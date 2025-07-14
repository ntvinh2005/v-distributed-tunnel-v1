use rpassword::read_password;
use sqlx::PgPool;

pub async fn edit_password(pool: &PgPool) -> Result<(), sqlx::Error> {
    use argon2::{
        Argon2,
        password_hash::{PasswordHasher, SaltString},
    };
    use rand_core::OsRng;
    use std::io::{self, Write};

    print!("Enter node ID to change password: ");
    io::stdout().flush().unwrap();
    let mut node_id = String::new();
    io::stdin().read_line(&mut node_id)?;
    let node_id = node_id.trim();

    print!("Enter new password: ");
    io::stdout().flush().unwrap();
    let password = read_password().unwrap();

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    let res = sqlx::query!(
        "UPDATE nodes SET password_hash = $2 WHERE node_id = $1",
        node_id,
        password_hash
    )
    .execute(pool)
    .await;

    match res {
        Ok(r) if r.rows_affected() > 0 => println!("✅ Password updated for '{}'.", node_id),
        Ok(_) => println!("Node not found."),
        Err(e) => println!("Error: {}", e),
    }
    Ok(())
}
