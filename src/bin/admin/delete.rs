use sqlx::PgPool;
use std::io::{self, Write};

pub async fn delete_node(pool: &PgPool) -> Result<(), sqlx::Error> {
    print!("Enter node ID to delete: ");
    io::stdout().flush().unwrap();
    let mut node_id = String::new();
    io::stdin().read_line(&mut node_id)?;
    let node_id = node_id.trim();

    let res = sqlx::query!("DELETE FROM nodes WHERE node_id = $1", node_id)
        .execute(pool)
        .await;

    match res {
        Ok(r) if r.rows_affected() > 0 => println!("Node '{}' deleted.", node_id),
        Ok(_) => println!("Node not found."),
        Err(e) => println!("Error: {}", e),
    }
    Ok(())
}
