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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
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
            commands::get_recording_path,
            commands::get_recording_bytes,
            commands::get_stats,
            commands::get_setting,
            commands::set_setting,
            commands::get_birds,
            commands::get_bird_packs,
            commands::get_bird_recordings,
            commands::delete_recording,
            commands::delete_bird,
            commands::search_bird_recordings,
            commands::add_bird_recordings,
            commands::add_recording_by_number,
            commands::backfill_recording_meta,
            commands::get_bird_info,
            commands::fetch_bird_info,
            commands::backfill_bird_info,
            commands::bird_info_backfill_running,
            commands::cancel_bird_info_backfill,
            commands::set_bird_notes,
            commands::get_bird_image_bytes,
            commands::repair_recordings,
            commands::recordings_storage,
            commands::open_recordings_folder,
            commands::get_queue_counts,
            commands::create_pack,
            commands::create_pack_from_filter,
            commands::list_regions,
            commands::import_birds,
            commands::search_species,
            commands::import_species,
            commands::resolve_ebird_list,
            commands::get_pack_birds,
            commands::rename_pack,
            commands::set_pack_icon,
            commands::delete_pack,
            commands::add_birds_to_pack,
            commands::remove_bird_from_pack,
            commands::export_pack,
            commands::import_pack
            //commands::generate_official_pack
            // Add commands here
        ])
        .run(tauri::generate_context!())?;

    Ok(())
}

