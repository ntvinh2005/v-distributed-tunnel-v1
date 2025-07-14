mod admin;

use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok(); //Load .env
    //Load db url else fallback
    //Pretty similar to in other language like in Go
    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:Vinh12345678@localhost:5432/hilio-tunnel-db".to_string()
    });

    //Now let connect to our lovely pg db with the url
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&db_url)
        .await?;

    println!("");
    println!("Connect to the Hilio Network database");
    println!("");
    println!("Welcome to dugeon master!");
    println!("Sir, to start you can type 'help' for available commands.");

    loop {
        print!("dugeon-master>");
        io::stdout().flush().unwrap();
        let mut master_command = String::new();
        //If error, we exit our admin tool
        if io::stdin().read_line(&mut master_command).is_err() {
            println!();
            break;
        }
        let master_command = master_command.trim();
        if master_command == "exit" || master_command == "quit" {
            break;
        }
        match master_command {
            "help" => admin::print_help(),
            "list" => admin::list::list_nodes(&pool).await?,
            "add" => admin::add::add_node(&pool).await?,
            "edit" => admin::edit::edit_node(&pool).await?,
            "delete" => admin::delete::delete_node(&pool).await?,
            //view <node id>
            cmd if cmd.starts_with("view ") => {
                let node_id = cmd.trim_start_matches("view ").trim();
                admin::view::view_node(&pool, node_id).await?;
            }
            "" => println!("Type 'help' for available commands."),
            _ => println!(
                "Unknown command: {}. Type 'help' for available commands.",
                master_command
            ),
        }
    }
    println!("Goodbye dugeon-master! :)");
    println!("We gonna meet again soon...");
    Ok(())
}
