use argon2::{Argon2, PasswordHash, PasswordVerifier};
use sqlx::PgPool;

pub async fn verify_node(pool: &PgPool, node_id: &str, password: &str) -> bool {
    if let Some(row) = sqlx::query!(
        "SELECT password_hash FROM nodes WHERE node_id = $1 AND is_active = TRUE",
        node_id
    )
    .fetch_optional(pool)
    .await
    .unwrap()
    {
        let hash = PasswordHash::new(&row.password_hash).unwrap();
        println!("Password hash: {}", row.password_hash);
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    } else {
        false
    }
}
