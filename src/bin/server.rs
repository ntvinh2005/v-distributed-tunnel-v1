mod auth;
mod pool;

use quinn::{Endpoint, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use std::{error::Error, fs::File, io::BufReader, net::SocketAddr, sync::Arc};
//use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

    //Also connect to pg db before enter the loop
    let pool = auth::connect_db::setup_pool().await;

    //Prepare our port pool (item to offer) before welcome our guesses (client)
    let port_pool = pool::port_pool::PortPool::new(5001, 5999);

    //Welcome some new clients.
    while let Some(connecting) = endpoint.accept().await {
        let pool = pool.clone(); //Every async spawn have its own handle to the pool;
        let port_pool = port_pool.clone(); //Getting another reference to use in each thread share same pool.
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

                        let node_id = parts[1];
                        let password = parts[2];

                        let is_authorized =
                            auth::login::verify_node(&pool, node_id, password).await;
                        if is_authorized {
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
                                send_stream
                                    .write(
                                        format!("Port assigned: {}\n", assigned_port.unwrap())
                                            .as_bytes(),
                                    )
                                    .await
                                    .unwrap();
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
