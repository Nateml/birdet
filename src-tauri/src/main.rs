// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod models;
mod commands;
mod errors;
mod services;

use db::{init_db, Db};
use tauri::Manager;
use dotenv::dotenv;

// App state to be shared across Tauri commands
// Keeps a reference to the database connection pool
#[derive(Clone)]
pub struct AppState {
    pub db: Db,
}

fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file!()
    dotenv().ok();
    // Initialize the database
    tauri::Builder::default()
        .setup(|app| {
            // clone a handle to move to async task
            let handle = app.handle();

            let db = tauri::async_runtime::block_on(async {
                match init_db(&handle).await {
                    Ok(db) => {
                        println!("Database initialized successfully.");
                        db
                    }
                    Err(e) => {
                        panic!("Failed to initialize database: {}", e);
                    }
                }
            });

            app.manage(AppState { db });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_next_question,
            commands::submit_answer,
            commands::get_packs,
            // Add commands here
        ])
        .run(tauri::generate_context!())?;

    Ok(())
}

