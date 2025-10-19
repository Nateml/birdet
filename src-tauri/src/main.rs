// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod models;
mod commands;
mod errors;
mod services;

// Import db wrapper
use db::{init_db, Db};
use tauri::Manager;

// App state to be shared across Tauri commands
// Keeps a reference to the database connection pool
#[derive(Clone)]
struct AppState {
    db: Db,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the database
    let app = tauri::Builder::default()
        .setup(|app| {
            tauri::async_runtime::spawn(async move {
                // no-op
            });
            Ok(())
        });

    let handle = app.build(tauri::generate_context!())?;

    // Resolve app data path
    let app_dir = handle
        .path().app_data_dir().expect("app dir").to_string_lossy().to_string();

    std::fs::create_dir_all(&app_dir).ok();
    let db_path = format!("{}/birdet.sqlite", app_dir);

    let db = init_db(&db_path).await?;
    let state = AppState { db };

    handle.manage(state.clone());

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::get_next_question,
            commands::submit_answer
            // Add commands here
        ])
        .run(tauri::generate_context!());

    Ok(())
}

