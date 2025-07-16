mod admin;
mod forward;
mod pool;

use admin::node_store::NodeStore;
use quinn::{Endpoint, RecvStream, SendStream, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use std::{error::Error, fs::File, io::BufReader, net::SocketAddr, sync::Arc};
//use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

fn load_certs(path: &str) -> Result<Vec<CertificateDer<'static>>, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let certs = certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|c| c.to_owned()) // Clone to 'static
        .collect();
    Ok(certs)
}

fn load_key(path: &str) -> Result<PrivateKeyDer<'static>, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    if let Some(Ok(key)) = pkcs8_private_keys(&mut reader).next() {
        let bytes = key.secret_pkcs8_der();
        return Ok(PrivateKeyDer::Pkcs8(bytes.to_vec().into()));
    }

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    if let Some(Ok(key)) = rsa_private_keys(&mut reader).next() {
        let bytes = key.secret_pkcs1_der();
        return Ok(PrivateKeyDer::Pkcs1(bytes.to_vec().into()));
    }

    Err("Failed to load private key".into())
}

pub async fn start_tcp_listener_for_port(
    port: u16,
    port_registry: Arc<pool::port_registry::PortRegistry>,
    forward_fn: Arc<
        dyn Fn(TcpStream, SendStream, RecvStream) -> tokio::task::JoinHandle<()> + Send + Sync,
    >,
) {
    let listener = match TcpListener::bind(("0.0.0.0", port)).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind TCP listener on port {}: {:?}", port, e);
            return;
        }
    };

    println!("Listening for public TCP connections on port {}", port);

    loop {
        let (tcp_stream, remote_addr) = match listener.accept().await {
            Ok(x) => x,
            Err(e) => {
                eprintln!("Accept error on port {}: {:?}", port, e);
                continue;
            }
        };

        let registry_clone = port_registry.clone();
        let forward_fn = forward_fn.clone();

        tokio::spawn(async move {
            let node_info = match registry_clone.get(&port) {
                Some(info) => info,
                None => {
                    eprintln!("No node registered for port {}, dropping connection", port);
                    return;
                }
            };

            let (send_stream, recv_stream) = match node_info.conn.open_bi().await {
                Ok(x) => x,
                Err(e) => {
                    eprintln!(
                        "Failed to open QUIC stream to node {}: {:?}",
                        node_info.node_id, e
                    );
                    return;
                }
            };

            // Start bidirectional forwarding
            forward_fn(tcp_stream, send_stream, recv_stream);
            println!("Closed tunnel from {} on port {}", remote_addr, port);
        });
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    println!("Starting QUIC server on UDP port 5000");

    let certs = load_certs("cert.pem")?;
    let key = load_key("key.pem")?;

    //The key is used intentional by server to prove to client that server is the one who control the cert
    //When a client connect, server only present its cert as part of the TLS handshake.
    let mut server_config = ServerConfig::with_single_cert(certs, key)?;
    Arc::get_mut(&mut server_config.transport)
        .unwrap()
        .max_concurrent_bidi_streams(100u32.into()); //TODO: Change the number of concurrent connections later. This 100 just for test

    let address: SocketAddr = "0.0.0.0:5000".parse()?;
    let endpoint = Endpoint::server(server_config, address)?;

    //Create node store
    let node_store = Arc::new(NodeStore::new());

    //Start admin CLI listener
    //This help us add new node info to our memory!
    tokio::spawn(admin::admin_listener::start_admin_listener(
        node_store.clone(),
    ));

    //Prepare our port pool (item to offer) before welcome our guesses (client)
    let port_pool = Arc::new(pool::port_pool::PortPool::new(5001, 5999));

    let port_registry = pool::port_registry::PortRegistry::new();
    let port_registry = Arc::new(port_registry);

    //Welcome some new clients.
    while let Some(connecting) = endpoint.accept().await {
        let node_store = node_store.clone();
        let port_pool = port_pool.clone(); //Getting another reference to use in each thread share same pool.
        let port_registry = port_registry.clone();
        tokio::spawn(async move {
            match connecting.await {
                Ok(conn) => {
                    println!("Accepted new connection from {}", conn.remote_address());
                    while let Ok((mut send_stream, mut recv_stream)) = conn.accept_bi().await {
                        let mut buf = vec![0; 128];
                        let n = match recv_stream.read(&mut buf).await.unwrap() {
                            Some(n) => n,
                            None => continue, //Just skip if receive nothing from client
                        };
                        let auth_line = String::from_utf8_lossy(&buf[..n]);
                        let parts: Vec<&str> = auth_line.trim().splitn(3, ' ').collect();
                        //parts will have three parts: Header (AUTH), <node id> and <password>.
                        if parts.len() < 3 {
                            send_stream
                                .write(b"Unauthorized: Auth line lack of arguments\n")
                                .await
                                .unwrap();
                            continue;
                        }

                        if parts.len() > 3 {
                            send_stream
                                .write(b"Unauthorized: Auth line has too many arguments\n")
                                .await
                                .unwrap();
                            continue;
                        }

                        if parts[0] != "AUTH" {
                            send_stream
                                .write(b"Unauthorized: Invalid auth header\n")
                                .await
                                .unwrap();
                            continue;
                        }

                        let node_id = parts[1].trim();
                        let password = parts[2].trim();

                        let is_authorized =
                            admin::login::verify_node(&node_store, node_id, password);
                        if !is_authorized {
                            send_stream
                                .write(b"Unauthorized: Invalid node id or password\n")
                                .await
                                .unwrap();
                            continue;
                        } else {
                            send_stream.write(b"Authorized: Success\n").await.unwrap();
                            let assigned_port = port_pool.assign_random_port(node_id);

                            if assigned_port.is_none() {
                                send_stream
                                    .write(b"Service unavailable: No port available\n")
                                    .await
                                    .unwrap();
                                continue;
                            } else {
                                println!(
                                    "Assigned port {} to node '{}'",
                                    assigned_port.unwrap(),
                                    node_id
                                );
                                send_stream
                                    .write(
                                        format!("ASSIGNED {}\n", assigned_port.unwrap()).as_bytes(), //Send back protocal message ASSIGNED <port>
                                    )
                                    .await
                                    .unwrap();
                                println!(
                                    "Sent port {} to node '{}'",
                                    assigned_port.unwrap(),
                                    node_id,
                                );

                                //Create an instance of port guard to release the port after the client is disconnected or something go wrong.
                                let port_guard = pool::port_pool::PortGuard {
                                    port_pool: port_pool.clone(),
                                    port: assigned_port.unwrap(),
                                    node_id: node_id.to_string(),
                                };

                                //Each assigned port will have it own tcp listener
                                let node_info = pool::port_registry::NodeInfo {
                                    conn: conn.clone(),
                                    node_id: node_id.to_string(),
                                };
                                port_registry.insert(assigned_port.unwrap(), node_info);

                                let port_registry = port_registry.clone(); // clone for the listener task
                                let forward_fn = forward::server_tunnel_handler::make_forward_fn(); // supply your forward_fn Arc<dyn Fn...>
                                tokio::spawn(async move {
                                    let _guard = port_guard;
                                    start_tcp_listener_for_port(
                                        assigned_port.unwrap(),
                                        port_registry,
                                        forward_fn,
                                    )
                                    .await;
                                });
                            }
                        }
                    }
                }
                Err(e) => eprintln!("Connection error: {e:?}"),
            }
        });
    }
    Ok(())
}
