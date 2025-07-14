pub mod add;
pub mod delete;
pub mod edit;
pub mod list;
pub mod view;

pub fn print_help() {
    println!("Available commands:");
    println!("  add              - Add a new node");
    println!("  edit             - Edit an existing node's password");
    println!("  delete           - Delete a node");
    println!("  list             - List all nodes");
    println!("  view <node_id>   - View details of a node");
    println!("  help             - Show this help message");
    println!("  exit/quit        - Exit the CLI");
}
