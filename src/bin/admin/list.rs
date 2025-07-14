use sqlx::PgPool;

pub async fn list_nodes(pool: &PgPool) -> Result<(), sqlx::Error> {
    let rows =
        sqlx::query!("SELECT node_id, created_at, is_active FROM nodes ORDER BY node_id ASC")
            .fetch_all(pool)
            .await?;

    println!("{:<20} | {:<24} | {}", "Node ID", "Created At", "Active");
    println!("{:-<65}", "");
    for row in rows {
        let created_at_str = row
            .created_at
            .map(|dt| dt.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        println!(
            "{:<20} | {:?} | {}",
            row.node_id,
            created_at_str,
            row.is_active.unwrap_or(false)
        );
    }
    Ok(())
}
