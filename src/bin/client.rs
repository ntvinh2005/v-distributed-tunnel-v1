mod forward;
use blake3;
use hex;
use v_distributed_tunnel_v1::common::helper::config::{load_config, save_config};

use quinn::{ClientConfig, Endpoint, TransportConfig};
//use rpassword::read_password;
use clap::Parser;
use rustls::RootCertStore;
use rustls_pemfile::certs;
use std::io::{self, Write};
use std::time::Duration;
use std::{env, error::Error, fs::File, io::BufReader, net::SocketAddr, sync::Arc};

#[derive(Parser, Debug)]
#[command(author, version, about = "QUIC Tunnel Client", long_about = None)]
struct Args {
    #[arg(long)]
    node_id: Option<String>,

    #[arg(long)]
    password: Option<String>,
}

//Read a self-signed certificate of server and trust it
//The client will only connect if the server's certificate is matched.
fn load_root_certs(path: &str) -> Result<RootCertStore, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut roots = RootCertStore::empty();
    for cert in certs(&mut reader) {
        roots.add(cert?)?;
    }
    Ok(roots)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();
    println!("Starting QUIC client on UDP port 5000");

    //Load and trust the server cert (just for self-signed/testing)
    //Use quinn helper to build a Client Config from a root store
    let roots = load_root_certs("cert.pem")?;
    //let verifier = WebPkiServerVerifier::builder(Arc::new(roots)).build()?; //Build a server cert verifier

    //Here we add ping to keep the client alive
    let mut transport_config = TransportConfig::default();
    transport_config.max_idle_timeout(Some(Duration::from_secs(600).try_into().unwrap())); //600s = 10 min
    transport_config.keep_alive_interval(Some(Duration::from_secs(30))); //We ping each 30s

    //Create a config that trust server's certificate
    let client_config = ClientConfig::with_root_certificates(Arc::new(roots))?;

    //Here we cretaing endpoint and set default config
    let mut endpoint = Endpoint::client("[::]:0".parse()?)?;
    endpoint.set_default_client_config(client_config);

    //Here we connect to server
    //When we put this to server, we need to change the IP
    let ip = env::var("SERVER_PUBLIC_IP").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("TUNNEL_PORT").unwrap_or_else(|_| "5000".to_string());
    let addr = format!("{}:{}", ip, port);
    let server_addr: SocketAddr = addr.parse().unwrap();
    let quinn_conn = endpoint.connect(server_addr, "localhost")?.await?; //Connect to server IP and using name localhost and server's self-signed cert.
    println!("Connected to {}", quinn_conn.remote_address());

    //Agfter that we open the bidirectional stream
    let (mut send_stream, mut recv_stream) = quinn_conn.open_bi().await?;

    //here we prepare the new preimage to send to server for validate
    let config_path = "config.toml";
    let mut config = load_config(config_path);
    let seed_bytes = hex::decode(&config.seed).expect("Invalid hex seed");
    let mut hash = seed_bytes.to_vec();

    for i in 0..config.current_index {
        hash = blake3::hash(&hash).as_bytes().to_vec();
        let computed = blake3::hash(&hash);
        let computed_hex = computed.to_hex().to_string();
        println!("New hash {}: {}", i, computed_hex);
    }
    let preimage_hex = hex::encode(&hash);

    //First we send auth message to server in the format AUTH <node_id> <new hex preimage>

    let auth_message = format!("AUTH {} {}\n", config.node_id, preimage_hex);
    println!("[Auth Message]: {}", auth_message);
    send_stream.write_all(auth_message.as_bytes()).await?; //We can only send bytes in the stream
    send_stream.finish()?;

    let mut buf = vec![0; 1024];
    let mut linebuf = String::new();
    let mut authenticated = false;
    let mut assigned_port: Option<u16> = None;

    //here, we read lines in a loop until both "Success" and "ASSIGNED" are received
    loop {
        let n = match recv_stream.read(&mut buf).await? {
            Some(n) if n > 0 => n,
            _ => break,
        };
        linebuf.push_str(&String::from_utf8_lossy(&buf[..n]));

        while let Some(idx) = linebuf.find('\n') {
            let line = linebuf[..idx].trim();
            println!("Response: {}", line);

            if line.contains("Success") {
                println!("Authentication successful!");
                authenticated = true;
                config.current_index -= 1;
                save_config(config_path, &config);
                //We have ti check if current index reach 0 yet or not
                //if it already 0, we rotate the seed.
                //We only allow client that is already authenticated to rotate the seed.
                //So when currnet index == 1 -> In that last time, we force client to rotate the seed
                if config.current_index == 0 {
                    if line.starts_with("ROTATE") {
                        println!("Rotating seed");
                        const CHAIN_LENGTH: usize = 100;
                        config.seed = line[6..].trim().to_string();
                        config.current_index = CHAIN_LENGTH - 1;
                        save_config(config_path, &config);
                    }
                }
            } else if line.starts_with("ASSIGNED") {
                assigned_port = line[8..].trim().parse::<u16>().ok();
                if let Some(port) = assigned_port {
                    println!(
                        "Tunnel ready! Assigned port: {}. Connect your remote tester to this port.",
                        port
                    );
                }
            } else if line.contains("Unauthorized") {
                println!("Authentication failed!");
                return Ok(());
            }

            linebuf = linebuf[idx + 1..].to_string(); //remove processed line from our line buffer
        }

        //If both success and assigned, break to proceed
        //else, listening?
        //TODO: Is there a better way to handle reading?
        if authenticated && assigned_port.is_some() {
            break;
        }
    }
    if authenticated {
        if let Some(port) = assigned_port {
            println!(
                "Tunnel ready! Assigned port: {}. Waiting for incoming connections...",
                port
            );
            let mut warned = false;
            //Accept new bi-directional streams from the server (each represents a remote tester connection)
            //We only start forwarding things when there is a remote tester start connecting to server end of the tunnel
            //Then server send new stream, and we can start forwarding
            loop {
                match quinn_conn.accept_bi().await {
                    Ok((send_stream, recv_stream)) => {
                        println!("[Tunnel] Accepted new stream from server. Starting relay.");
                        warned = false;
                        //Each new remote tester connection gets its own tunnel handler
                        tokio::spawn(forward::client_tunnel_handler::handle_tunnel(
                            send_stream,
                            recv_stream,
                        ));
                    }
                    Err(e) => {
                        if e.to_string().contains("timed out") {
                            if !warned {
                                eprintln!(
                                    "[Tunnel] No incoming stream (timed out). Waiting again..."
                                );
                                warned = true;
                            }
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                            continue;
                        } else {
                            eprintln!("[Tunnel] Failed to accept new stream: {e}");
                            break;
                        }
                    }
                }
            }
            println!(
                "Tunnel loop for node '{}' on port {} has ended.",
                config.node_id, port
            );
        } else {
            println!("No assigned port received!");
        }
    }

    Ok(())
}
