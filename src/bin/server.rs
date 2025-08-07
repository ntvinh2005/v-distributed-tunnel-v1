mod admin;
mod forward;
mod pool;
mod reverse_proxy;

use admin::node_store::NodeStore;
use quinn::{Endpoint, RecvStream, SendStream, ServerConfig, TransportConfig};
use reverse_proxy::helper::{extract_host, extract_path};
use reverse_proxy::routing_table::RoutingTable;
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use std::time::Duration;
use std::{env, error::Error, fs::File, io::BufReader, net::SocketAddr, sync::Arc};
use v_distributed_tunnel_v1::common::helper;
//use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::reverse_proxy::routing_table;

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
    routing_table: Arc<routing_table::RoutingTable>,
) {
    let ip = env::var("TUNNEL_IP").unwrap_or_else(|_| "0.0.0.0".to_string());
    let listener = match TcpListener::bind((ip, port)).await {
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

        //as usual, before feed routing these class into our as
        let registry_clone = port_registry.clone();
        let forward_fn = forward_fn.clone();
        let routing_table = routing_table.clone();

        tokio::spawn(async move {
            //firstly, we take a look at top of http data
            let mut buf = [0; 1024];
            //we read without removing. therefore we peek 😉
            let n = match tcp_stream.peek(&mut buf).await {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("Failed to read from TCP stream: {}", e);
                    return;
                }
            };

            let http_data = String::from_utf8_lossy(&buf[..n]);
            let host = extract_host(&http_data);
            let path = extract_path(&http_data);

            //here, we use our routing table as a dictionary to look/map to our wanted backend
            let backend = routing_table.lookup_with_path(
                host.as_ref().unwrap().to_string(),
                path.as_ref().unwrap().to_string(),
            );

            if let Some(backend_id) = backend {
                let mut split = backend_id.split(':');
                let node_id = split.next().unwrap();
                let port: u16 = split.next().unwrap().parse().unwrap_or(8080);
                let node_info = match registry_clone.get_by_node_id(node_id) {
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
            } else {
                eprintln!(
                    "No backend found for host {:?} and path {:?}, dropping connection",
                    host, path
                );
                return;
            }
        });
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();
    println!("Starting QUIC server on UDP port 5000");

    let certs = load_certs("cert.pem")?;
    let key = load_key("key.pem")?;

    //we add ping to keep the server alive
    let mut transport_config = TransportConfig::default();
    transport_config.max_idle_timeout(Some(Duration::from_secs(600).try_into().unwrap())); //600s = 10 min
    transport_config.keep_alive_interval(Some(Duration::from_secs(30))); //We ping each 30s

    //The key is used intentional by server to prove to client that server is the one who control the cert
    //When a client connect, server only present its cert as part of the TLS handshake.
    let mut server_config = ServerConfig::with_single_cert(certs, key)?;
    Arc::get_mut(&mut server_config.transport)
        .unwrap()
        .max_concurrent_bidi_streams(100u32.into()); //TODO: Change the number of concurrent connections later. This 100 just for test

    let ip = env::var("TUNNEL_IP").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("TUNNEL_PORT").unwrap_or_else(|_| "5000".to_string());
    let addr = format!("{}:{}", ip, port);
    let address: SocketAddr = addr.parse()?;
    let endpoint = Endpoint::server(server_config, address)?;

    //Load routing table (for our reverse proxy)
    let routing_table = Arc::new(routing_table::setup_routing_table());

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
        //Getting another reference to use in each thread share same pool.
        let port_pool = port_pool.clone();
        let port_registry = port_registry.clone();
        let routing_table = routing_table.clone();
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
                        //parts will have three parts: Header (AUTH), <node id> and <hex preimage>.
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
                        let preimage = parts[2].trim();

                        let (is_authorized, new_seed) =
                            admin::login::verify_node(&node_store, node_id, preimage);
                        if !is_authorized {
                            send_stream
                                .write(b"Unauthorized: Invalid node id or preimage\n")
                                .await
                                .unwrap();
                            continue;
                        } else {
                            send_stream.write(b"Authorized: Success\n").await.unwrap();
                            if new_seed.is_some() {
                                send_stream
                                    .write(format!("ROTATE {}\n", new_seed.unwrap()).as_bytes())
                                    .await
                                    .unwrap();
                            }
                            let node_seed_opt = node_store.get_seed(node_id);
                            let assigned_result =
                                port_pool.assign_static_port(node_id, node_seed_opt.as_deref());
                            match assigned_result {
                                pool::port_pool::StaticPortAssignResult::Success(port) => {
                                    println!("Assigned port {} to node '{}'", port, node_id);
                                    send_stream
                                        .write(
                                            format!("ASSIGNED {}\n", port).as_bytes(), //Send back protocal message ASSIGNED <port>
                                        )
                                        .await
                                        .unwrap();
                                    println!("Sent port {} to node '{}'", port, node_id,);

                                    //Create an instance of port guard to release the port after the client is disconnected or something go wrong.
                                    let port_guard = pool::port_pool::PortGuard {
                                        port_pool: port_pool.clone(),
                                        port: port,
                                        node_id: node_id.to_string(),
                                    };

                                    //Each assigned port will have it own tcp listener
                                    let node_info = pool::port_registry::NodeInfo {
                                        conn: conn.clone(),
                                        node_id: node_id.to_string(),
                                    };
                                    port_registry.insert(port, node_info);

                                    //Create a clone to feed into each async tcp listener
                                    let port_registry = port_registry.clone();
                                    let forward_fn =
                                        forward::server_tunnel_handler::make_forward_fn();
                                    let routing_table = routing_table.clone();
                                    tokio::spawn(async move {
                                        start_tcp_listener_for_port(
                                            port,
                                            port_registry,
                                            forward_fn,
                                            routing_table,
                                        )
                                        .await;
                                    });

                                    //MAIN SESSION LOOP
                                    //accept new bidirectional streams from client as long as session is alive
                                    let _guard = port_guard;
                                    loop {
                                        match conn.accept_bi().await {
                                            Ok((send_stream, recv_stream)) => {
                                                //ignore rn
                                            }
                                            Err(_) => {
                                                //session end, portguard release.
                                                break;
                                            }
                                        }
                                    }
                                }
                                pool::port_pool::StaticPortAssignResult::SeedMissing => {
                                    send_stream
                                        .write(b"Service unavailable: Seed missing\n")
                                        .await
                                        .unwrap();
                                    continue;
                                }
                                pool::port_pool::StaticPortAssignResult::SeedHexInvalid => {
                                    send_stream
                                        .write(b"Service unavailable: Seed hex invalid\n")
                                        .await
                                        .unwrap();
                                    continue;
                                }
                                pool::port_pool::StaticPortAssignResult::PortInUse(port) => {
                                    send_stream
                                        .write(
                                            format!(
                                                "Service unavailable: Port {} is in use\n",
                                                port
                                            )
                                            .as_bytes(),
                                        )
                                        .await
                                        .unwrap();
                                    continue;
                                }
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
