use rand::Rng;
use rand::seq::SliceRandom;
use sqlx::PgPool;

fn generate_password() -> String {
    let mut rng = rand::thread_rng();

    //Create vector of all uppercase chars, lowercase chars, digits, special characters
    let upper = ('A'..='Z').collect::<Vec<_>>();
    let lower = ('a'..='z').collect::<Vec<_>>();
    let digit = ('0'..='9').collect::<Vec<_>>();
    let special = b"!@#$%^&*()-_=+[]{};:,.<>?"
        .iter()
        .map(|&b| b as char) //Map each as char
        .collect::<Vec<_>>();

    let upper_count = rng.gen_range(2..4);
    let lower_count = rng.gen_range(3..5);
    let digit_count = rng.gen_range(3..5);
    let special_count = rng.gen_range(2..4);

    //Next we just gonna choose randomly char from different collections n times.
    let mut password = Vec::new();
    //Sample without replacement, so no repeat
    password.extend(upper.choose_multiple(&mut rng, upper_count).cloned());
    password.extend(lower.choose_multiple(&mut rng, lower_count).cloned());
    password.extend(digit.choose_multiple(&mut rng, digit_count).cloned());
    password.extend(special.choose_multiple(&mut rng, special_count).cloned());

    password.shuffle(&mut rng); //Shuffle so you guys cannot guess any pridictable postition anymore. Hahaha

    password.into_iter().collect()
}

pub async fn add_node(pool: &PgPool) -> Result<(), sqlx::Error> {
    use argon2::{
        Argon2,
        password_hash::{PasswordHasher, SaltString},
    };
    use rand_core::OsRng;
    use std::io::{self, Write};

    print!("Enter new node ID: ");
    io::stdout().flush().unwrap();
    let mut node_id = String::new();
    io::stdin().read_line(&mut node_id)?;
    let node_id = node_id.trim();

    let password = generate_password();

    //Hash the password before save it in db.
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    //Prevent SQL injection with placeholder 1 and 2 for node id and password hash.
    let res = sqlx::query!(
        r#"INSERT INTO nodes (node_id, password_hash) VALUES ($1, $2)"#,
        node_id,
        password_hash,
    )
    .execute(pool)
    .await;
    match res {
        Ok(_) => {
            println!("Node '{}' added.", node_id);
            println!("Generated password: {}", password);
            println!("Sir, please give this key to the user securely.");
        }
        Err(e) => println!("Error: {}", e),
    }
    Ok(())
}
