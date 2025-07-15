use quinn::{ClientConfig, Endpoint};
use rpassword::read_password;
use rustls::RootCertStore;
use rustls_pemfile::certs;
use std::io::{self, Write};
use std::{error::Error, fs::File, io::BufReader, net::SocketAddr, sync::Arc};

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
    tracing_subscriber::fmt::init();
    println!("Starting QUIC client on UDP port 5000");

    //Load and trust the server cert (just for self-signed/testing)
    //Use quinn helper to build a Client Config from a root store
    let roots = load_root_certs("cert.pem")?;
    //let verifier = WebPkiServerVerifier::builder(Arc::new(roots)).build()?; //Build a server cert verifier

    //Create a config that trust server's certificate
    let client_config = ClientConfig::with_root_certificates(Arc::new(roots))?;

    //Here we cretaing endpoint and set default config
    let mut endpoint = Endpoint::client("[::]:0".parse()?)?;
    endpoint.set_default_client_config(client_config);

    //Here we connect to server
    //When we put this to server, we need to change the IP
    let server_addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    let quinn_conn = endpoint.connect(server_addr, "localhost")?.await?; //Connect to server IP and using name localhost and server's self-signed cert.
    println!("Connected to {}", quinn_conn.remote_address());

    //Agfter that we open the bidirectional stream
    let (mut send_stream, mut recv_stream) = quinn_conn.open_bi().await?;

    //First we send auth message to server in the format AUTH <node_id> <password>
    //Alow user (client node) to type in node id and password and send it to server
    print!("Enter node ID: ");
    io::stdout().flush().unwrap();
    let mut node_id = String::new();
    io::stdin().read_line(&mut node_id)?;

    print!("Enter password: ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    let password = read_password().unwrap();

    let auth_message = format!("AUTH {} {}\n", node_id, password);
    send_stream.write_all(auth_message.as_bytes()).await?; //We can only send bytes in the stream
    send_stream.finish()?;

    //Afrer sending, now we read frmo server (echo back)
    let mut buf = vec![0; 1024];
    let n = recv_stream.read(&mut buf).await?.unwrap();
    let response = String::from_utf8_lossy(&buf[..n]); //Convert response in byte into string
    if response.contains("Success") {
        println!("Authentication successful!");
    } else {
        println!("Authentication failed!");
        println!("Response: {}", response);
    }

    Ok(())
}
