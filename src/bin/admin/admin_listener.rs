use super::node_store::NodeStore;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

pub async fn start_admin_listener(node_store: Arc<NodeStore>) {
    let listener = TcpListener::bind("127.0.0.1:6969").await.unwrap();
    println!("Admin API listening on 127.0.0.1:6969");
    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let store = node_store.clone();
        tokio::spawn(async move {
            handle_admin(stream, store).await;
        });
    }
}

async fn handle_admin(stream: TcpStream, node_store: Arc<NodeStore>) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    while reader.read_line(&mut line).await.unwrap() > 0 {
        let parts: Vec<&str> = line.trim().splitn(2, ' ').collect();
        let mut out = String::new();
        match parts.as_slice() {
            ["add" | "create", node_id] => {
                let new_password = node_store.add_node(node_id.to_string());
                out.push_str("OK: Sir, node has been added\n");
                out.push_str(&format!("Node ID: {}\n", node_id));
                out.push_str(&format!("Password: {}\n", new_password));
                out.push_str(
                    "Please give this to your client node so that they can enter our dugeon.\n",
                );
                out.push_str("--END--\n");
            }
            ["remove" | "delete" | "destroy", node_id] => {
                node_store.remove_node(node_id.to_string());
                out.push_str("OK: Node removed\n");
                out.push_str("--END--\n");
            }
            ["view", node_id] => match node_store.get_node(node_id.to_string()) {
                Some(node) => {
                    out.push_str("\n+---------------+--------------------------------------------------------------+\n");
                    out.push_str(&format!("| {:<13} | {:<60} |\n", "Field", "Value"));
                    out.push_str("+---------------+--------------------------------------------------------------+\n");
                    out.push_str(&format!("| {:<13} | {:<60} |\n", "Node ID", node.node_id));
                    out.push_str(&format!(
                        "| {:<13} | {:<60} |\n",
                        "Anchor Hash", node.anchor
                    ));
                    out.push_str(&format!(
                        "| {:<13} | {:<60} |\n",
                        "Created At", node.created_at
                    ));
                    out.push_str(&format!(
                        "| {:<13} | {:<60} |\n",
                        "Last Login",
                        node.last_login
                            .map(|dt| dt.to_string())
                            .unwrap_or_else(|| "(Never)".to_string())
                    ));
                    out.push_str("+---------------+--------------------------------------------------------------+\n");
                    out.push_str("--END--\n");
                }
                None => {
                    out.push_str(
                        "I cannot find the node sir. Are you sure about the id of the node?\n",
                    );
                    out.push_str("--END--\n");
                }
            },
            ["list"] => {
                //this is our table header
                let id_width = 18;
                let hash_width = 32;
                let created_width = 22;
                let last_login_width = 22;

                out.push_str(&format!(
                    "{:<id_width$} | {:<hash_width$} | {:<created_width$} | {:<last_login_width$}\n",
                    "Node ID", "Password Hash", "Created At", "Last Login",
                    id_width = id_width, hash_width = hash_width, created_width = created_width, last_login_width = last_login_width,
                ));
                out.push_str(&format!(
                    "{:-<id_width$}-+-{:-<hash_width$}-+-{:-<created_width$}-+-{:-<last_login_width$}\n",
                    "", "", "", "",
                    id_width = id_width, hash_width = hash_width, created_width = created_width, last_login_width = last_login_width,
                ));

                for node in node_store.list_nodes() {
                    //This is to handle case when hash is too long
                    let hash_display = if node.anchor.len() > hash_width {
                        format!("{}...", &node.anchor[..(hash_width - 3)])
                    } else {
                        node.anchor.clone()
                    };
                    let last_login_display = node
                        .last_login
                        .map(|dt| dt.to_string())
                        .unwrap_or_else(|| "(Never)".to_string());

                    out.push_str(&format!(
                        "{:<id_width$} | {:<hash_width$} | {:<created_width$} | {:<last_login_width$}\n",
                        node.node_id,
                        hash_display,
                        node.created_at,
                        last_login_display,
                        id_width = id_width, hash_width = hash_width, created_width = created_width, last_login_width = last_login_width,
                    ));
                }
                out.push_str("--END--\n");
            }
            ["help"] => {
                out.push_str("Common spell you would like to use:\n");
                out.push_str("add/create <node_id>\n");
                out.push_str("remove/delete/destroy <node_id>\n");
                out.push_str("view <node_id>\n");
                out.push_str("list\n");
                out.push_str("Cast 'exit' or 'quit' to quit.\n");
                out.push_str("Cast 'help' to see what inside your magic book.\n");
                out.push_str("--END--\n");
            }
            _ => {
                out.push_str("ERR: I'm afraid... Wrong spell sir. Try again or cast 'help'\n");
                out.push_str("--END--\n");
            }
        }
        writer.write_all(out.as_bytes()).await.unwrap();
        line.clear();
    }
}
