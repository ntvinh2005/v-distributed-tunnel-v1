use sqlx::PgPool;

pub async fn view_node(pool: &PgPool, node_id: &str) -> Result<(), sqlx::Error> {
    let row = sqlx::query!(
        "SELECT node_id, created_at, is_active FROM nodes WHERE node_id = $1",
        node_id
    )
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let created_at_str = r
                .created_at
                .map(|dt| dt.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            println!("Node ID:    {}", r.node_id);
            println!("Created At: {:?}", created_at_str);
            println!("Active:     {}", r.is_active.unwrap_or(false));
        }
        None => println!("Node '{}' not found.", node_id),
    }
    Ok(())
}
