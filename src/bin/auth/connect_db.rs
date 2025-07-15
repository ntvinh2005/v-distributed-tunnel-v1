use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;

pub async fn setup_pool() -> sqlx::Pool<sqlx::Postgres> {
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
        .await
        .map_err(|e| {
            println!("Failed to connect to database: {}", e);
            e
        })
        .unwrap();

    println!("Connected to database");

    return pool;
}
