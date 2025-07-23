use quinn::{RecvStream, SendStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

//This function is for client to act as a relay to connect QUIC stream to the local service through TCP stream
pub async fn handle_tunnel(
    mut send_stream: SendStream,
    mut recv_stream: RecvStream,
) -> anyhow::Result<()> {
    let local_service_port = 8080;
    //Connect to the local service on the assigned port. For example 8080
    // If the local service isn't up, retry a few times before failing
    let mut tcp_stream = loop {
        match TcpStream::connect(("127.0.0.1", local_service_port)).await {
            Ok(s) => break s,
            Err(e) => {
                eprintln!("[Tunnel] Failed to connect to local service: {e}. Retrying in 1s...");
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    };

    //Split TCP stream for independent reading/writing. I do the same for quic, eventhough it is unneccessary and just a rename convenience
    let (mut tcp_reader, mut tcp_writer) = tcp_stream.split();
    let (mut quic_writer, mut quic_reader) = (send_stream, recv_stream);

    //Forwarding: TCP -> QUIC
    //read from tcp, buffer it, then write to quic
    let tcp_to_quic = async {
        let mut buf = [0u8; 4096]; //12 bit
        loop {
            let n = tcp_reader.read(&mut buf).await?;
            if n == 0 {
                println!("[Tunnel] TCP -> QUIC: TCP closed connection");
                break;
            }
            quic_writer.write_all(&buf[..n]).await?;
        }
        quic_writer.finish()?;
        anyhow::Ok(())
    };

    //Forwarding: QUIC -> TCP
    //Read from quic, buffer it, then write to tcp
    let quic_to_tcp = async {
        let mut buf = [0u8; 4096]; //Also give this 12 bits
        loop {
            let n = quic_reader.read(&mut buf).await?;
            if let Some(n) = n {
                if n == 0 {
                    println!("[Tunnel] QUIC -> TCP: QUIC closed connection");
                    break;
                }
                tcp_writer.write_all(&buf[..n]).await?;
            } else {
                break;
            }
        }
        tcp_writer.shutdown().await?;
        anyhow::Ok(())
    };

    //Run both directions concurrently until both finishes. Think about it as a two-way road with different lanes.
    let (tcp_res, quic_res) = tokio::join!(tcp_to_quic, quic_to_tcp);

    if let Err(e) = tcp_res {
        eprintln!("[Tunnel] TCP -> QUIC error: {e}");
    }
    if let Err(e) = quic_res {
        eprintln!("[Tunnel] QUIC -> TCP error: {e}");
    }

    println!("[Tunnel] Bidirectional relay fully completed.");
    Ok(())
}