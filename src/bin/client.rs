use quinn::{ClientConfig, Endpoint};
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    println!("Starting QUIC client on UDP port 5000");
    Ok(())
}
