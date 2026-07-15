use crate::{AppState};
use serde::{Serialize, Deserialize};
use tauri::{State, AppHandle, Manager};
use tauri::path::BaseDirectory;
use anyhow::Result;

use crate::models::Pack;

/// A species heard in the background of a recording (Xeno-Canto `also`).
#[derive(Serialize)]
pub struct BackgroundBird {
    pub scientific: String,
    pub common: Option<String>, // common name if the species is in the library
}

#[derive(Serialize)]
pub struct Question {
    pub bird_id: i64,
    pub recording_id: i64,
    pub choices: Vec<String>, // For multiple choice questions
    pub is_new: bool,         // first-ever exposure (counts against the new-card cap)
    // Attribution for the recording being played (Creative Commons requires
    // credit). Shown after the answer is revealed so it doesn't hint the bird.
    pub source: Option<String>,
    pub xc_id: Option<String>,
    pub recordist: Option<String>,
    pub license_url: Option<String>,
    pub location: Option<String>,
    // Other species audible in the clip. Never offered as wrong options; shown
    // to the user when the "background birds" option is enabled.
    pub background: Vec<BackgroundBird>,
}

#[tauri::command]
pub async fn health() -> &'static str {
    "ok"
}
 
/// Return the next card to study, or `None` when the session queue is drained.
/// `pack` filters the candidate pool (None = the whole library). `new_remaining`
/// is the remaining new-card budget. `include_reviews=false` studies only new
/// cards (a "new-only" session); with `new_remaining=0` you get a review-only run.
#[tauri::command]
pub async fn get_next_question(
    state: State<'_, AppState>,
    pack: Option<String>,
    new_remaining: i64,
    include_reviews: bool,
    cram: bool,
) -> Result<Option<Question>, String> {
    crate::services::quiz::next_question(&state.db, pack, new_remaining, include_reviews, cram)
        .await
        .map_err(|e| e.to_string())
}

/// A snapshot of the session's remaining work, for the live progress readout.
/// `to_go` is the headline number: the minimum questions left if every answer is
/// correct (a new bird needs 2 passes, a learning card its remaining steps, a due
/// review 1). It drops by one per correct answer and rises on a miss — unlike a
/// raw card count, which sits still while a new bird just shifts to learning.
/// `new`/`learning`/`due` are card counts kept for the breakdown tooltip.
#[derive(Serialize)]
pub struct QueueCounts {
    pub new: i64,      // new birds still to introduce (capped by remaining budget)
    pub learning: i64, // learning cards being drilled
    pub due: i64,      // review cards due within the learn-ahead window
    pub to_go: i64,    // min questions to finish if all correct
}

/// Count the cards `get_next_question` would still serve, with the same scoping.
#[tauri::command]
pub async fn get_queue_counts(
    state: State<'_, AppState>,
    pack: Option<String>,
    new_remaining: i64,
    include_reviews: bool,
) -> Result<QueueCounts, String> {
    crate::services::quiz::queue_counts(&state.db, pack, new_remaining, include_reviews)
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

/// Resolve a recording filename to an absolute path, preferring imported audio
/// in the writable app-data dir and falling back to the bundled resources.
/// Imported recordings download to `app_data_dir/recordings/`; the original seed
/// audio ships read-only under `BaseDirectory::Resource`.
pub fn resolve_recording(app: &AppHandle, filename: &str) -> Result<std::path::PathBuf, String> {
    // `filename` is stored per-recording and may predate sanitization. Reject
    // anything that isn't a plain leaf name so it can't escape the recordings
    // dir (path traversal → arbitrary file read via the asset protocol).
    if filename.is_empty()
        || filename.contains('/')
        || filename.contains('\\')
        || filename.contains("..")
    {
        return Err("invalid recording filename".into());
    }
    if let Ok(dir) = app.path().app_data_dir() {
        let imported = dir.join("recordings").join(filename);
        if imported.exists() {
            return Ok(imported);
        }
    }
    app.path()
        .resolve(format!("resources/recordings/{}", filename), BaseDirectory::Resource)
        .map_err(|e| e.to_string())
}

async fn recording_filename(state: &AppState, recording_id: i64) -> Result<String, String> {
    sqlx::query_scalar("SELECT filename FROM recordings WHERE id = ?1")
        .bind(recording_id)
        .fetch_one(&state.db.0)
        .await
        .map_err(|e| e.to_string())
}

/// Resolve a recording's audio file to an absolute path on disk.
/// Frontend wraps this with `convertFileSrc()` to feed the asset protocol.
#[tauri::command]
pub async fn get_recording_path(
    app: AppHandle,
    state: State<'_, AppState>,
    recording_id: i64,
) -> Result<String, String> {
    let filename = recording_filename(&state, recording_id).await?;
    let path = resolve_recording(&app, &filename)?;
    Ok(path.to_string_lossy().into_owned())
}

/// Read a recording's audio file and return the raw bytes.
/// The frontend wraps these in a same-origin `Blob` URL so the Web Audio
/// AnalyserNode isn't cross-origin-tainted (needed for the live spectrogram).
#[tauri::command]
pub async fn get_recording_bytes(
    app: AppHandle,
    state: State<'_, AppState>,
    recording_id: i64,
) -> Result<tauri::ipc::Response, String> {
    let filename = recording_filename(&state, recording_id).await?;
    let path = resolve_recording(&app, &filename)?;
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    Ok(tauri::ipc::Response::new(bytes))
}

/// Read a persisted app setting (API keys, defaults). Empty string = unset,
/// so the frontend can treat it uniformly.
#[tauri::command]
pub async fn get_setting(state: State<'_, AppState>, key: String) -> Result<String, String> {
    crate::services::settings::get_setting(&state.db, &key)
        .await
        .map(|v| v.unwrap_or_default())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_setting(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    crate::services::settings::set_setting(&state.db, &key, &value)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_packs(state: State<'_, AppState>) -> Result<Vec<Pack>, String> {
    crate::services::packs::get_packs(&state.db)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct BirdListItem {
    pub id: i64,
    pub common_name: String,
    pub scientific_name: String,
    pub family: Option<String>,
    pub region: Option<String>,
    pub recording_count: i64,
}

#[tauri::command]
pub async fn get_birds(state: State<'_, AppState>) -> Result<Vec<BirdListItem>, String> {
    crate::services::packs::get_birds(&state.db)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct BirdPackTag {
    pub bird_id: i64,
    pub pack_id: String,
    pub pack_name: String,
}

/// One row per (bird, pack) pair, for rendering pack tags in the library.
#[tauri::command]
pub async fn get_bird_packs(state: State<'_, AppState>) -> Result<Vec<BirdPackTag>, String> {
    crate::services::packs::get_bird_packs(&state.db)
        .await
        .map_err(|e| e.to_string())
}

/// Recordings stored for a bird (for the per-bird recording manager).
#[tauri::command]
pub async fn get_bird_recordings(
    state: State<'_, AppState>,
    bird_id: i64,
) -> Result<Vec<crate::services::import::RecordingInfo>, String> {
    crate::services::import::get_bird_recordings(&state.db, bird_id)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a single recording (DB row, links, and downloaded file).
#[tauri::command]
pub async fn delete_recording(
    app: AppHandle,
    state: State<'_, AppState>,
    recording_id: i64,
) -> Result<(), String> {
    crate::services::import::delete_recording(&app, &state.db, recording_id)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a bird entirely (recordings, files, pack links, and the bird row).
#[tauri::command]
pub async fn delete_bird(
    app: AppHandle,
    state: State<'_, AppState>,
    bird_id: i64,
) -> Result<(), String> {
    crate::services::import::delete_bird(&app, &state.db, bird_id)
        .await
        .map_err(|e| e.to_string())
}

/// Search Xeno-Canto for more recordings of a bird (excludes ones already had).
#[tauri::command]
pub async fn search_bird_recordings(
    state: State<'_, AppState>,
    bird_id: i64,
    quality: Option<String>,
    rec_type: Option<String>,
) -> Result<crate::services::import::RecordingSearch, String> {
    crate::services::import::search_recordings(&state.db, bird_id, quality, rec_type)
        .await
        .map_err(|e| e.to_string())
}

/// Back-fill quality + type on older recordings. Returns the count updated.
#[tauri::command]
pub async fn backfill_recording_meta(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    crate::services::import::backfill_recording_meta(&app, &state.db)
        .await
        .map_err(|e| e.to_string())
}

/// Total disk usage of downloaded recordings.
#[derive(Serialize)]
pub struct RecordingStorage {
    pub bytes: u64,
    pub file_count: u64,
    pub path: String,
}

/// Sum the size of every downloaded recording in the writable app-data dir.
/// Bundled seed audio (read-only resources) isn't counted — it isn't user data.
#[tauri::command]
pub fn recordings_storage(app: AppHandle) -> Result<RecordingStorage, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("recordings");
    let (mut bytes, mut file_count) = (0u64, 0u64);
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    bytes += meta.len();
                    file_count += 1;
                }
            }
        }
    }
    Ok(RecordingStorage { bytes, file_count, path: dir.to_string_lossy().into_owned() })
}

/// Open the recordings folder in the OS file manager. Creates it first so the
/// action works even before the first import.
#[tauri::command]
pub fn open_recordings_folder(app: AppHandle) -> Result<(), String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("recordings");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    let mut cmd = std::process::Command::new("explorer");
    #[cfg(target_os = "macos")]
    let mut cmd = std::process::Command::new("open");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut cmd = std::process::Command::new("xdg-open");

    // explorer.exe returns a non-zero exit code even on success, so only report a
    // failure to *launch* the command, not its exit status.
    cmd.arg(&dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Couldn't open the folder: {}", e))
}

/// Re-download any XC recordings whose local audio file is missing or corrupt.
#[tauri::command]
pub async fn repair_recordings(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::services::import::RepairSummary, String> {
    crate::services::import::repair_recordings(&app, &state.db)
        .await
        .map_err(|e| e.to_string())
}

/// Download and add specific Xeno-Canto recordings (by xc_id) to a bird.
#[tauri::command]
pub async fn add_bird_recordings(
    app: AppHandle,
    state: State<'_, AppState>,
    bird_id: i64,
    xc_ids: Vec<String>,
) -> Result<i64, String> {
    crate::services::import::add_recordings(&app, &state.db, bird_id, xc_ids)
        .await
        .map_err(|e| e.to_string())
}

/// Add one specific Xeno-Canto recording to a bird by its catalogue number
/// ("XC123", "123", or an XC URL).
#[tauri::command]
pub async fn add_recording_by_number(
    app: AppHandle,
    state: State<'_, AppState>,
    bird_id: i64,
    catalogue: String,
) -> Result<crate::services::import::AddByNumberResult, String> {
    crate::services::import::add_recording_by_number(&app, &state.db, bird_id, &catalogue)
        .await
        .map_err(|e| e.to_string())
}

/// Export a pack to a `birdet-pack` JSON file in the user's Downloads folder.
/// Returns the absolute path written, for display.
#[tauri::command]
pub async fn export_pack(
    app: AppHandle,
    state: State<'_, AppState>,
    pack_id: String,
) -> Result<String, String> {
    let (name, json) = crate::services::packs::export_pack(&state.db, &pack_id)
        .await
        .map_err(|e| e.to_string())?;

    // Slugify the pack name for a safe filename.
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let slug = if slug.is_empty() { "pack".to_string() } else { slug };
    let filename = format!("{}.birdet-pack.json", slug);

    // Prefer the Downloads dir; fall back to app-data/exports if unavailable.
    let dir = app.path().download_dir().unwrap_or_else(|_| {
        app.path()
            .app_data_dir()
            .map(|d| d.join("exports"))
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
    });
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(filename);
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

/// Import a pack from the text contents of an exported `birdet-pack` file.
#[tauri::command]
pub async fn import_pack(
    app: AppHandle,
    state: State<'_, AppState>,
    contents: String,
    max_per_species: Option<i64>,
) -> Result<crate::models::PackImportResult, String> {
    crate::services::import::import_pack(&app, &state.db, contents, max_per_species.unwrap_or(3))
        .await
        .map_err(|e| e.to_string())
}

/// Manual pack: name + explicit bird selection.
#[tauri::command]
pub async fn create_pack(
    state: State<'_, AppState>,
    name: String,
    bird_ids: Vec<i64>,
    icon: Option<String>,
) -> Result<String, String> {
    let id = crate::services::packs::create_pack_from_birds(&state.db, &name, &bird_ids)
        .await
        .map_err(|e| e.to_string())?;
    if icon.is_some() {
        crate::services::packs::set_pack_icon(&state.db, &id, icon.as_deref())
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(id)
}

/// Birds contained in a pack (for the pack editor).
#[tauri::command]
pub async fn get_pack_birds(
    state: State<'_, AppState>,
    pack_id: String,
) -> Result<Vec<BirdListItem>, String> {
    crate::services::packs::get_pack_birds(&state.db, &pack_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_pack(
    state: State<'_, AppState>,
    pack_id: String,
    name: String,
) -> Result<(), String> {
    crate::services::packs::rename_pack(&state.db, &pack_id, &name)
        .await
        .map_err(|e| e.to_string())
}

/// Set (or clear) a pack's icon emoji.
#[tauri::command]
pub async fn set_pack_icon(
    state: State<'_, AppState>,
    pack_id: String,
    icon: Option<String>,
) -> Result<(), String> {
    crate::services::packs::set_pack_icon(&state.db, &pack_id, icon.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_pack(state: State<'_, AppState>, pack_id: String) -> Result<(), String> {
    crate::services::packs::delete_pack(&state.db, &pack_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_birds_to_pack(
    state: State<'_, AppState>,
    pack_id: String,
    bird_ids: Vec<i64>,
) -> Result<(), String> {
    crate::services::packs::add_birds_to_pack(&state.db, &pack_id, &bird_ids)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_bird_from_pack(
    state: State<'_, AppState>,
    pack_id: String,
    bird_id: i64,
) -> Result<(), String> {
    crate::services::packs::remove_bird_from_pack(&state.db, &pack_id, bird_id)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct RegionItem {
    pub code: String,
    pub name: String,
}

/// List sub-regions for the region picker (cached). `level` ∈ country |
/// subnational1 | subnational2; `parent` is 'world' for countries.
#[tauri::command]
pub async fn list_regions(
    state: State<'_, AppState>,
    level: String,
    parent: String,
) -> Result<Vec<RegionItem>, String> {
    crate::services::regions::list_regions(&state.db, &level, &parent)
        .await
        .map_err(|e| e.to_string())
}

/// Filter pack: a region (eBird spplist ∩ library) and/or a family.
#[tauri::command]
pub async fn create_pack_from_filter(
    state: State<'_, AppState>,
    name: Option<String>,
    region: Option<String>,
    family: Option<String>,
    icon: Option<String>,
) -> Result<String, String> {
    let region = region.filter(|r| !r.trim().is_empty());
    let family = family.filter(|f| !f.trim().is_empty());
    let result = match (region, family) {
        (Some(r), fam) => {
            crate::services::packs::create_pack_from_region(&state.db, name, &r, fam.as_deref()).await
        }
        (None, Some(f)) => crate::services::packs::create_pack_from_family(&state.db, name, &f).await,
        (None, None) => Err(anyhow::anyhow!("Set a region or family to filter by.")),
    };
    let id = result.map_err(|e| e.to_string())?;
    if icon.is_some() {
        crate::services::packs::set_pack_icon(&state.db, &id, icon.as_deref())
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(id)
}

/// Import birds from eBird + Xeno-Canto. Emits `import://progress` events and
/// returns a summary when finished.
#[tauri::command]
pub async fn import_birds(
    app: AppHandle,
    state: State<'_, AppState>,
    params: crate::services::import::ImportParams,
) -> Result<crate::services::import::ImportSummary, String> {
    crate::services::import::import_birds(&app, &state.db, params)
        .await
        .map_err(|e| e.to_string())
}

/// Search the cached eBird taxonomy by name for the manual add-birds flow.
#[tauri::command]
pub async fn search_species(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<crate::services::taxonomy::SpeciesResult>, String> {
    crate::services::taxonomy::search(&state.db, &query, 30)
        .await
        .map_err(|e| e.to_string())
}

/// Import specific species by eBird code (manual search-and-add). Emits
/// `import://progress` and returns a summary.
#[tauri::command]
pub async fn import_species(
    app: AppHandle,
    state: State<'_, AppState>,
    ebird_codes: Vec<String>,
    quality: Option<String>,
    rec_type: Option<String>,
    max_per_species: Option<i64>,
    pack_ids: Option<Vec<String>>,
    new_pack_name: Option<String>,
) -> Result<crate::services::import::ImportSummary, String> {
    crate::services::import::import_species(
        &app,
        &state.db,
        ebird_codes,
        quality,
        rec_type,
        max_per_species.unwrap_or(1),
        pack_ids.unwrap_or_default(),
        new_pack_name,
    )
    .await
    .map_err(|e| e.to_string())
}

/// Resolve a pasted eBird list (checklist/hotspot/region URL or code, or a
/// life-list / My-Data CSV) into its species, for the import preview. No
/// download happens here — the frontend imports the codes via `import_species`.
#[tauri::command]
pub async fn resolve_ebird_list(
    state: State<'_, AppState>,
    input: String,
) -> Result<crate::services::import::ListPreview, String> {
    crate::services::import::resolve_ebird_list(&state.db, &input)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct BirdStat {
    pub common_name: String,
    pub seen: i64,
    pub correct: i64,
    pub state: String,          // new | learning | review | mastered
    pub interval_days: f64,
    pub ease: f64,
    pub lapses: i64,
    pub due_at: Option<String>, // SQLite timestamp; None = new/unscheduled
}

#[derive(Serialize)]
pub struct Stats {
    pub total_seen: i64,
    pub total_correct: i64,
    pub total_birds: i64,   // every bird in the library
    pub new_count: i64,     // never studied
    pub learning_count: i64,
    pub review_count: i64,  // young reviews (interval < mature threshold)
    pub mastered_count: i64,// mature reviews (interval >= threshold)
    pub due_count: i64,     // cards due now
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
