use quinn::{RecvStream, SendStream};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

//Returns Arc for use in TCP listener code (Checkout server)
pub fn make_forward_fn()
-> Arc<dyn Fn(TcpStream, SendStream, RecvStream) -> tokio::task::JoinHandle<()> + Send + Sync> {
    Arc::new(|mut tcp_stream, mut send_stream, mut recv_stream| {
        tokio::spawn(async move {
            let (mut tcp_reader, mut tcp_writer) = tcp_stream.split();
            let (mut quic_writer, mut quic_reader) = (send_stream, recv_stream);

            let tcp_to_quic = async {
                let mut buf = [0u8; 4096];
                loop {
                    let n = tcp_reader.read(&mut buf).await?;
                    if n == 0 {
                        break;
                    }
                    quic_writer.write_all(&buf[..n]).await?;
                }
                quic_writer.finish()?;
                Ok::<(), std::io::Error>(())
            };

            let quic_to_tcp = async {
                let mut buf = [0u8; 4096];
                loop {
                    let n = quic_reader.read(&mut buf).await?;
                    if let Some(n) = n {
                        if n == 0 {
                            break;
                        }
                        tcp_writer.write_all(&buf[..n]).await?;
                    } else {
                        break;
                    }
                }
                tcp_writer.shutdown().await?;
                Ok::<(), std::io::Error>(())
            };

            let _ = tokio::try_join!(tcp_to_quic, quic_to_tcp);
        })
    })
}
