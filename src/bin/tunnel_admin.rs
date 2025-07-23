use clap::Parser;
use std::io::{self, Write};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[derive(Parser, Debug)]
#[command(author, version, about = "Tunnel admin CLI for QUIC nodes", long_about = None)]
struct Args {
    #[arg()]
    command: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let mut stream = TcpStream::connect("127.0.0.1:6969").await?;
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    let mut response = String::new();

    println!("Connected to admin port on 127.0.0.1:6969");

    if !args.command.is_empty() {
        let cmd = args.command.join(" ");
        writer.write_all(cmd.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        loop {
            response.clear();
            reader.read_line(&mut response).await?;
            if response.trim_end() == "--END--" {
                break;
            }
            print!("{}", response);
        }
        return Ok(());
    }

    //Here is our interactive mode
    println!(
        "Cast the spell, e.g.:create/add node123 $argon2id$...\ndestroy/remove/delete node123\nview node123\nlist\nType 'exit' or 'quit' to quit.\n"
    );

    loop {
        print!("dugeon-master> ");
        io::stdout().flush().unwrap();
        line.clear();
        io::stdin().read_line(&mut line)?;
        let spell = line.trim();
        if spell == "exit" || spell == "quit" {
            break;
        }
        writer.write_all(spell.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        loop {
            response.clear();
            reader.read_line(&mut response).await?;
            if response.trim_end() == "--END--" {
                break;
            }
            print!("{}", response);
        }
    }
    Ok(())
}
