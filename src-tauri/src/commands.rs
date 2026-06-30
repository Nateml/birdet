use crate::{AppState};
use serde::{Serialize, Deserialize};
use tauri::{State, AppHandle, Manager};
use tauri::path::BaseDirectory;
use anyhow::Result;

use crate::models::Pack;

#[derive(Serialize)]
pub struct Question {
    pub bird_id: i64,
    pub recording_id: i64,
    pub choices: Vec<String>, // For multiple choice questions
}

#[tauri::command]
pub async fn health() -> &'static str {
    "ok"
}
 
#[tauri::command]
pub async fn get_next_question(state: State<'_, AppState>, pack: Option<String>) -> Result<Question, String> {
    crate::services::quiz::next_question(&state.db, pack)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Deserialize)]
pub struct AnswerPayload {
    pub bird_id: i64,
    pub recording_id: i64,
    pub guess: String
}

#[derive(Serialize)]
pub struct AnswerResult {
    pub correct: bool,
    pub correct_name: String
}

#[tauri::command]
pub async fn submit_answer(state: State<'_, AppState>, payload: AnswerPayload) -> Result<AnswerResult, String> {
    crate::services::quiz::submit_answer(&state.db, payload)
        .await
        .map_err(|e| e.to_string())
}

/// Resolve a recording's bundled audio file to an absolute path on disk.
/// Frontend wraps this with `convertFileSrc()` to feed the asset protocol.
#[tauri::command]
pub async fn get_recording_path(
    app: AppHandle,
    state: State<'_, AppState>,
    recording_id: i64,
) -> Result<String, String> {
    let filename: String = sqlx::query_scalar("SELECT filename FROM recordings WHERE id = ?1")
        .bind(recording_id)
        .fetch_one(&state.db.0)
        .await
        .map_err(|e| e.to_string())?;

    let path = app
        .path()
        .resolve(format!("resources/recordings/{}", filename), BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;

    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn get_packs(state: State<'_, AppState>) -> Result<Vec<Pack>, String> {
    crate::services::packs::get_packs(&state.db)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct BirdStat {
    pub common_name: String,
    pub seen: i64,
    pub correct: i64,
}

#[derive(Serialize)]
pub struct Stats {
    pub total_seen: i64,
    pub total_correct: i64,
    pub birds: Vec<BirdStat>,
}

#[tauri::command]
pub async fn get_stats(state: State<'_, AppState>) -> Result<Stats, String> {
    crate::services::stats::get_stats(&state.db)
        .await
        .map_err(|e| e.to_string())
}

