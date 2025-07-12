use quinn::{Endpoint, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use std::{error::Error, fs::File, io::BufReader, net::SocketAddr, sync::Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
    let mut server_config = ServerConfig::with_single_cert(certs, key)?;
    Arc::get_mut(&mut server_config.transport)
        .unwrap()
        .max_concurrent_bidi_streams(100u32.into());

    let address: SocketAddr = "0.0.0.0:5000".parse()?;
    let endpoint = Endpoint::server(server_config, address)?;

    while let Some(connecting) = endpoint.accept().await {
        tokio::spawn(async move {
            match connecting.await {
                Ok(conn) => {
                    println!("Accepted new connection from {}", conn.remote_address());
                    while let Ok((mut send_stream, mut recv_stream)) = conn.accept_bi().await {
                        let mut buf = vec![0; 1024];
                        if let Some(n) = recv_stream.read(&mut buf).await.unwrap() {
                            println!("Received: {:?}", &buf[..n]);
                            send_stream.write_all(&buf[..n]).await.unwrap();
                        }
                    }
                }
                Err(e) => eprintln!("Connection error: {e:?}"),
            }
        });
    }
    Ok(())
}
