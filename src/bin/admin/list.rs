use sqlx::PgPool;

pub async fn list_nodes(pool: &PgPool) -> Result<(), sqlx::Error> {
    let rows =
        sqlx::query!("SELECT node_id, created_at, is_active FROM nodes ORDER BY node_id ASC")
            .fetch_all(pool)
            .await?;

    println!("{:<20} | {:<24} | {}", "Node ID", "Created At", "Active");
    println!("{:-<65}", "");
    for row in rows {
        println!(
            "{:<20} | {:?} | {}",
            row.node_id,
            row.created_at,
            row.is_active.unwrap_or(false)
        );
    }
    Ok(())
}
