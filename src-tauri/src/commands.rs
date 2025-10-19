use crate::{AppState};
use serde::{Serialize, Deserialize};
use tauri::State;
use anyhow::Result;

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

