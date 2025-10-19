use sqlx::sqlite::{SqlitePoolOptions, SqlitePool, SqliteConnectOptions};
use std::fs;
use std::str::FromStr;
use tauri::Manager;

// anyhow makes error handling easier
// we can return Result<T> where the error type is anyhow::Error
// instead of defining our own error types
use anyhow::{anyhow, Result};

#[derive(Clone)]
pub struct Db(pub SqlitePool);


pub async fn init_db(app_handle: &tauri::AppHandle) -> Result<Db> {

    // Resolve the db path
    let mut db_path = app_handle.path().app_data_dir()
        .map_err(|e| anyhow!("Failed to get app data directory: {}", e))?;

    fs::create_dir_all(&db_path)?; // Ensure the directory exists
    db_path.push("birdet.sqlite"); // Append the database file name
   
    // Create the SQLite URL
    let db_url = format!("sqlite://{}", db_path.to_string_lossy());  
    println!("Database path: {}", db_url);

    // Connection options
    let opts = SqliteConnectOptions::from_str(&db_url)
        .map_err(|e| anyhow!("Failed to create connection options: {}", e))?
        .create_if_missing(true); // Create the database file if it doesn't exist
    
    // Build the database pool
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
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

