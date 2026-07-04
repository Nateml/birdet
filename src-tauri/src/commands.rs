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

/// Read a recording's bundled audio file and return the raw bytes.
/// The frontend wraps these in a same-origin `Blob` URL so the Web Audio
/// AnalyserNode isn't cross-origin-tainted (needed for the live spectrogram).
#[tauri::command]
pub async fn get_recording_bytes(
    app: AppHandle,
    state: State<'_, AppState>,
    recording_id: i64,
) -> Result<tauri::ipc::Response, String> {
    let filename: String = sqlx::query_scalar("SELECT filename FROM recordings WHERE id = ?1")
        .bind(recording_id)
        .fetch_one(&state.db.0)
        .await
        .map_err(|e| e.to_string())?;

    let path = app
        .path()
        .resolve(format!("resources/recordings/{}", filename), BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;

    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    Ok(tauri::ipc::Response::new(bytes))
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


// --- Incoming eBird and Xeno-Canto API Definitions ---
#[derive(Deserialize, Debug)]
struct EBirdObservation {
    #[serde(rename = "speciesCode")]
    species_code: String,
    #[serde(rename = "comName")]
    com_name: String,
    #[serde(rename = "sciName")]
    sci_name: String,
}

#[derive(Deserialize, Debug)]
struct XcRecording {
    id: String,
    file: String,
    rec: String,
    lic: String,
}


#[derive(Deserialize, Debug)]
struct XcResponse {
    recordings: Vec<XcRecording>,
}

// --- Outbound eBird and Xeno-Canto API Definitions ---

#[derive(Serialize, Clone)]
pub struct BirdAsset {
    #[serde(rename = "ebirdCode")]
    ebird_code: String,
    #[serde(rename = "commonName")]
    common_name: String,
    #[serde(rename = "scientificName")]
    scientific_name: String,
    #[serde(rename = "audioUrl")]
    audio_url: String,
    recordist: String,
    #[serde(rename = "licenseUrl")]
    license_url: String,
    #[serde(rename = "xcId")]
    xc_id: String,
}

#[derive(Serialize)]
pub struct BirdPack {
    #[serde(rename = "packName")]
    pack_name: String,
    #[serde(rename = "regionCode")]
    region_code: String,
    birds: Vec<BirdAsset>,
}

// --- Pack creation command ---

/*
#[tauri::command]
pub async fn generate_official_pack(
    region_code: String,
    pack_size: usize,
    ebird_api_key: String,
) -> Result<BirdPack, String> {
    let client = reqwest::Client::new();

    println!("[1/4] Querying eBird observations for region: {}...", region_code);

    // 1. Fetch recent observations from eBird
    let ebird_url = format!("https://api.ebird.org/v2/data/obs/{}/recent?back=14&hotspot=true", region_code);
    let ebird_res = client
        .get(&ebird_url)
        .header("X-eBirdApiToken", &ebird_api_key)
        .send()
        .await
        .map_err(|e| format!("Failed calling eBird: {}", e))?;

    let observations: Vec<EbirdObservation> = ebird_res
        .json()
        .await
        .map_err(|e| format!("Failed parsing eBird JSON: {}", e))?;

    // 2. Deduplicate species
    let mut unique_birds = HashMap::new();
    for obs in observations {
        if !unique_birds.contains_key(&obs.species_code) {
            unique_birds.insert(obs.species_code.clone(), (obs.com_name, obs.sci_name));
        }
        if unique_birds.len() >= pack_size {
            break;
        }
    }

    println!("[2/4] Unique species targeted: {}. Querying Xeno-Canto sound files...", unique_birds.len());
    let mut compiled_birds = Vec::new();

    // 3. Sequential asset scraping loop
    for (species_code, (com_name, sci_name)) in unique_birds {
        // Search criteria: high quality (q:A) and explicit song type vocalization
        let mut xc_query = format!("\"{}\" q:A type:song", sci_name);
        let mut xc_url = format!("https://xeno-canto.org/api/2/recordings?query={}", urlencoding::encode(&xc_query));

        let mut xc_res = match client.get(&xc_url).send().await {
            Ok(res) => res,
            Err(_) => continue,
        };

        let mut xc_data: XcResponse = xc_res.json().await.unwrap_or(XcResponse { recordings: vec![] });

        // Fallback implementation: Lower quality requirements slightly if no explicit "A-grade song" matches
        if xc_data.recordings.is_empty() {
            xc_query = format!("\"{}\" q:B", sci_name);
            xc_url = format!("https://xeno-canto.org/api/2/recordings?query={}", urlencoding::encode(&xc_query));
            if let Ok(res) = client.get(&xc_url).send().await {
                if let Ok(fb_data) = res.json::<XcResponse>().await {
                    xc_data = fb_data;
                }
            }
        }

        // If an audio file matches, save it to the asset collection
        if let Some(prime_track) = xc_data.recordings.into_iter().next() {
            compiled_birds.push(BirdAsset {
                ebird_code,
                common_name: com_name.clone(),
                scientific_name: sci_name,
                audio_url: prime_track.file,
                recordist: prime_track.rec,
                license_url: prime_track.lic,
                xc_id: prime_track.id,
            });
            println!("✅ Aggregated audio for: {}", com_name);
        } else {
            println!("⚠️ Skipping {} - No viable audio on Xeno-Canto.", com_name);
        }

        // Rate limiting precaution: yield thread control for 1 second to safeguard API rules
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    Ok(BirdPack {
        pack_name: format!("Official {} Top {} Pack", region_code, compiled_birds.len()),
        region_code,
        birds: compiled_birds,
    })
}
*/
