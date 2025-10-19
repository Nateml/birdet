use sqlx::sqlite::{SqlitePoolOptions, SqlitePool};

// anyhow makes error handling easier
// we can return Result<T> where the error type is anyhow::Error
// instead of defining our own error types
use anyhow::{anyhow, Result};

#[derive(Clone)]
pub struct Db(pub SqlitePool);

pub async fn init_db(db_path: &str) -> Result<Db> {
    // Build the database pool
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&format!("sqlite://{}", db_path))
        .await
        .map_err(|e| anyhow!("Failed to connect to database: {}", e))?;

    // Run migrations
    // Migrations ensure that the database schema is up to date 
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| anyhow!("Failed to run migrations: {}", e))?;

    // Return the database wrapper
    // Okay is used to wrap the successful result
    Ok(Db(pool))
}

