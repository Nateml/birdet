use crate::db::Db;
use crate::services::regions::{self, enc};
use crate::services::settings;
use crate::services::taxonomy::{self, Taxon};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinSet;

const EBIRD_BASE: &str = "https://api.ebird.org/v2";
const XC_BASE: &str = "https://xeno-canto.org/api/3/recordings";

/// Number of species downloaded concurrently during an import. XC API queries
/// are still spaced by `Throttle` regardless, so this only overlaps the
/// download + DB work between species.
const SPECIES_CONCURRENCY: usize = 4;
/// Ceiling on in-flight media downloads across the whole run. Kept modest: the
/// XC download host answers 503 when hit with too large a burst, so this trades
/// a little speed for far fewer transient failures.
const DOWNLOAD_CONCURRENCY: usize = 4;
/// Minimum spacing between Xeno-Canto *API* queries (the rate-limited endpoint).
const XC_API_MIN_INTERVAL: Duration = Duration::from_millis(300);

/// Shared throttles for one concurrent import run: paces the XC API queries and
/// caps concurrent media downloads. Cheap to `clone` into each species task.
#[derive(Clone)]
struct Throttle {
    api_gate: Arc<Mutex<Instant>>,
    downloads: Arc<Semaphore>,
}

impl Throttle {
    fn new() -> Self {
        Throttle {
            api_gate: Arc::new(Mutex::new(
                Instant::now()
                    .checked_sub(XC_API_MIN_INTERVAL)
                    .unwrap_or_else(Instant::now),
            )),
            downloads: Arc::new(Semaphore::new(DOWNLOAD_CONCURRENCY)),
        }
    }

    /// Block until the next XC API query is allowed, then claim the slot. Holding
    /// the lock across the sleep serializes callers, so queries stay ≥
    /// `XC_API_MIN_INTERVAL` apart no matter how many species run in parallel.
    async fn api_slot(&self) {
        let mut last = self.api_gate.lock().await;
        let earliest = *last + XC_API_MIN_INTERVAL;
        let now = Instant::now();
        if earliest > now {
            tokio::time::sleep(earliest - now).await;
        }
        *last = Instant::now();
    }
}

/// Cap on a single downloaded recording. XC files are a few MB; this only
/// exists to stop a malicious pack/URL from exhausting memory or disk.
const MAX_RECORDING_BYTES: u64 = 60 * 1024 * 1024;

/// Build a filesystem-safe recording filename from an *untrusted* Xeno-Canto id.
/// `id` and `file_name` come from remote JSON or a shared `.birdet` pack file,
/// so an attacker could set `id` to `../../…` and escape the recordings dir on
/// import (arbitrary file write) or later reads. Strip the id to a safe
/// character set and whitelist the extension so the result is always a plain
/// leaf name under `recordings/`.
fn recording_file_name(id: &str, file_name: &str) -> String {
    let safe_id: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    let safe_id = if safe_id.is_empty() {
        "unknown".to_string()
    } else {
        safe_id
    };
    let ext = file_name
        .rsplit('.')
        .next()
        .map(str::to_ascii_lowercase)
        .filter(|e| matches!(e.as_str(), "mp3" | "wav" | "ogg" | "flac" | "m4a" | "mpga" | "aac"))
        .unwrap_or_else(|| "mp3".to_string());
    format!("xc_{}.{}", safe_id, ext)
}

// --- Import parameters (from the UI) ------------------------------------
#[derive(Deserialize, Debug, Clone)]
pub struct ImportParams {
    pub region: String,             // eBird region code, e.g. "GB-ENG" or "ZA"
    pub max_species: i64,           // cap on distinct species imported
    pub max_per_species: i64,       // cap on recordings downloaded per species
    pub quality: Option<String>,    // XC quality floor, e.g. "A"
    pub rec_type: Option<String>,   // XC vocalization type, e.g. "song"
    pub family: Option<String>,     // restrict to a family common name
    pub create_pack: bool,          // auto-create a pack from the import
    #[serde(default = "default_true")]
    pub skip_existing: bool,        // skip species already in the library (fill the cap with new ones)
}

fn default_true() -> bool {
    true
}

// --- Progress events emitted to the frontend ----------------------------
#[derive(Serialize, Clone)]
pub struct ImportProgress {
    pub stage: String, // 'fetching' | 'species' | 'downloading' | 'done' | 'error'
    pub message: String,
    pub current: i64,
    pub total: i64,
}

#[derive(Serialize, Clone)]
pub struct ImportSummary {
    pub species_imported: i64,
    pub species_skipped: i64,
    pub recordings_added: i64,
    pub pack_id: Option<String>,
}

fn emit(app: &AppHandle, stage: &str, message: impl Into<String>, current: i64, total: i64) {
    let _ = app.emit(
        "import://progress",
        ImportProgress { stage: stage.into(), message: message.into(), current, total },
    );
}

// eBird taxonomy uses the shared `taxonomy::Taxon` shape.

// --- Xeno-Canto v3 shapes (lenient) -------------------------------------
#[derive(Deserialize, Debug)]
struct XcResponse {
    #[serde(default)]
    recordings: Vec<XcRecording>,
}

#[derive(Deserialize, Debug, Clone)]
struct XcRecording {
    id: String,
    #[serde(default)]
    rec: String,
    #[serde(default)]
    lic: String,
    #[serde(default)]
    file: String,
    #[serde(rename = "file-name", default)]
    file_name: String,
    #[serde(default)]
    loc: String,
    #[serde(default)]
    q: String, // quality rating A–E
    #[serde(default)]
    length: String, // duration "m:ss"
    #[serde(rename = "type", default)]
    rec_type: String, // vocalization type
}

/// A Xeno-Canto recording not yet downloaded, offered for adding to a bird.
#[derive(Serialize, Clone)]
pub struct RecordingCandidate {
    pub xc_id: String,
    pub recordist: String,
    pub quality: String,
    pub length: String,
    pub rec_type: String,
    pub location: String,
    pub license_url: String,
}

/// Run a full import: eBird region species list (spplist) → taxonomy →
/// Xeno-Canto audio → download + upsert. Emits `import://progress` throughout.
/// The spplist endpoint resolves the region hierarchy itself, so a country code
/// returns its complete species list with no fan-out.
pub async fn import_birds(app: &AppHandle, db: &Db, params: ImportParams) -> Result<ImportSummary> {
    let ebird_key = regions::ebird_key(db).await?;
    let xc_key = settings::get_setting(db, "xc_api_key")
        .await?
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| anyhow!("Missing Xeno-Canto API key — add it in Settings."))?;

    let client = regions::build_client()?;

    // 1. Authoritative species list for the region.
    emit(app, "fetching", format!("Fetching species list for {}…", params.region), 0, 0);
    let codes = regions::fetch_spplist(&client, &ebird_key, &params.region).await?;
    if codes.is_empty() {
        return Err(anyhow!("No species found for region {}.", params.region));
    }

    // 2. Rank by local abundance. eBird has no per-species frequency endpoint,
    // so reconstruct it the way the bar charts do: sample checklists and tally
    // how many report each species. Sample across twelve mid-month dates over
    // the last year (not just recent checklists) so seasonal birds — summer
    // migrants, winter visitors — are counted in their own season rather than
    // scoring zero and getting cut. Stable-sort the taxonomic spplist by the
    // tally; ties keep taxon order. Tolerant — an empty sample leaves the order
    // unchanged (taxonomic fallback).
    const LISTS_PER_DATE: usize = 10;
    const FETCH_CONCURRENCY: usize = 5; // eBird burst cap is ~25 req / 5s
    let dates = regions::year_sample_dates(db).await.unwrap_or_default();
    emit(app, "fetching", "Ranking species by year-round abundance…", 0, 0);

    // Gather checklist IDs across all sample dates (cheap, one call per month).
    let mut subids: Vec<String> = Vec::new();
    for date in &dates {
        subids.extend(
            regions::checklist_subids(&client, &ebird_key, &params.region, Some(date), LISTS_PER_DATE).await,
        );
    }

    // Fetch each checklist's species concurrently (bounded) — this is the bulk
    // of the network cost, so cap in-flight requests rather than serialize.
    let mut freq: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let total_lists = subids.len();
    let mut done = 0usize;
    let mut iter = subids.into_iter();
    let mut set: tokio::task::JoinSet<Vec<String>> = tokio::task::JoinSet::new();
    for sid in iter.by_ref().take(FETCH_CONCURRENCY) {
        let (c, k) = (client.clone(), ebird_key.clone());
        set.spawn(async move { regions::checklist_species(&c, &k, &sid).await });
    }
    while let Some(res) = set.join_next().await {
        if let Ok(species) = res {
            for sp in species {
                *freq.entry(sp).or_insert(0) += 1;
            }
        }
        done += 1;
        if done % 10 == 0 || done == total_lists {
            emit(app, "fetching", format!("Sampling checklists… {}/{}", done, total_lists), 0, 0);
        }
        if let Some(sid) = iter.next() {
            let (c, k) = (client.clone(), ebird_key.clone());
            set.spawn(async move { regions::checklist_species(&c, &k, &sid).await });
        }
    }

    let mut ranked = codes;
    ranked.sort_by(|a, b| freq.get(b).unwrap_or(&0).cmp(freq.get(a).unwrap_or(&0)));

    // 3. Resolve taxonomy (names, family) + apply family filter + cap. Done
    // lazily in ranked order, one 100-code chunk at a time, stopping as soon as
    // the cap is filled — so a small `max_species` on a big region only costs a
    // chunk or two of taxonomy calls instead of resolving the whole region.
    let family_filter = params.family.as_deref().map(str::to_lowercase);
    let cap = params.max_species.max(1) as usize;
    // Species already in the library, so the cap fills with new birds (and we
    // don't re-query/re-download ones the user already has). Skipped unless the
    // caller opts to re-import.
    let owned: std::collections::HashSet<String> = if params.skip_existing {
        sqlx::query_scalar("SELECT ebird_code FROM birds WHERE ebird_code IS NOT NULL")
            .fetch_all(&db.0)
            .await?
            .into_iter()
            .collect()
    } else {
        std::collections::HashSet::new()
    };
    let mut taxa_by_code: std::collections::HashMap<String, Taxon> = std::collections::HashMap::new();
    let mut targets: Vec<String> = Vec::new();
    for chunk in ranked.chunks(100) {
        let joined = chunk.join(",");
        let taxa: Vec<Taxon> = client
            .get(format!("{}/ref/taxonomy/ebird?species={}&fmt=json", EBIRD_BASE, enc(&joined)))
            .header("X-eBirdApiToken", &ebird_key)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .unwrap_or_default();
        for t in taxa {
            taxa_by_code.insert(t.species_code.clone(), t);
        }
        // Select from this chunk in ranked order, respecting name + family filter.
        for code in chunk {
            if targets.len() >= cap {
                break;
            }
            if owned.contains(code) {
                continue; // already in the library — don't spend the cap on it
            }
            let Some(t) = taxa_by_code.get(code) else { continue }; // no name
            if let Some(fam) = &family_filter {
                let matches = t
                    .family_com_name
                    .as_ref()
                    .map(|f| f.to_lowercase().contains(fam))
                    .unwrap_or(false);
                if !matches {
                    continue;
                }
            }
            targets.push(code.clone());
        }
        if targets.len() >= cap {
            break;
        }
    }

    if targets.is_empty() {
        return Err(if params.skip_existing && !owned.is_empty() {
            anyhow!("No new species matched — you may already have them all. Turn off “skip existing” to re-import.")
        } else {
            anyhow!("No species matched the filter.")
        });
    }

    let rec_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| anyhow!("app data dir: {}", e))?
        .join("recordings");
    std::fs::create_dir_all(&rec_dir)?;

    let taxa: Vec<Taxon> = targets.iter().map(|c| taxa_by_code[c].clone()).collect();
    let (imported_bird_ids, species_imported, species_skipped, recordings_added) = import_taxa(
        app, db, &client, &xc_key, &rec_dir, &taxa, &params.region,
        params.quality.as_deref(), params.rec_type.as_deref(), params.max_per_species,
    )
    .await?;
    let total = taxa.len() as i64;

    let pack_id = if params.create_pack && !imported_bird_ids.is_empty() {
        let name = format!(
            "{} · {} species",
            params.family.clone().unwrap_or_else(|| params.region.clone()),
            species_imported
        );
        Some(crate::services::packs::create_pack_from_birds(db, &name, &imported_bird_ids).await?)
    } else {
        None
    };

    let summary = ImportSummary { species_imported, species_skipped, recordings_added, pack_id };
    emit(
        app,
        "done",
        format!("Imported {} species, {} recordings.", species_imported, recordings_added),
        total,
        total,
    );
    Ok(summary)
}

/// Import specific species by eBird code — the manual search-and-add path.
/// Reuses the per-species XC fetch/download/upsert. Imported birds are added to
/// each pack in `pack_ids` and, if `new_pack_name` is set, to a fresh pack.
pub async fn import_species(
    app: &AppHandle,
    db: &Db,
    ebird_codes: Vec<String>,
    quality: Option<String>,
    rec_type: Option<String>,
    max_per_species: i64,
    pack_ids: Vec<String>,
    new_pack_name: Option<String>,
) -> Result<ImportSummary> {
    let xc_key = settings::get_setting(db, "xc_api_key")
        .await?
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| anyhow!("Missing Xeno-Canto API key — add it in Settings."))?;
    let client = regions::build_client()?;
    let taxa = taxonomy::taxa_for(db, &ebird_codes).await?;
    if taxa.is_empty() {
        return Err(anyhow!("No matching species found."));
    }

    let rec_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| anyhow!("app data dir: {}", e))?
        .join("recordings");
    std::fs::create_dir_all(&rec_dir)?;

    let total = taxa.len() as i64;
    let (imported_bird_ids, species_imported, species_skipped, recordings_added) = import_taxa(
        app, db, &client, &xc_key, &rec_dir, &taxa, "",
        quality.as_deref(), rec_type.as_deref(), max_per_species.max(1),
    )
    .await?;

    // Attach imported birds to the chosen packs.
    let mut pack_id = None;
    if !imported_bird_ids.is_empty() {
        for pid in &pack_ids {
            crate::services::packs::add_birds_to_pack(db, pid, &imported_bird_ids).await?;
        }
        if let Some(name) = new_pack_name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) {
            pack_id = Some(
                crate::services::packs::create_pack_from_birds(db, &name, &imported_bird_ids).await?,
            );
        }
    }

    emit(
        app,
        "done",
        format!("Added {} species, {} recordings.", species_imported, recordings_added),
        total,
        total,
    );
    Ok(ImportSummary { species_imported, species_skipped, recordings_added, pack_id })
}

/// Import a pack from an exported `birdet-pack` JSON file. Birds already in the
/// library are linked as-is; missing ones (by eBird code) are downloaded from
/// Xeno-Canto. Creates a new pack and returns a per-category tally.
pub async fn import_pack(
    app: &AppHandle,
    db: &Db,
    contents: String,
    max_per_species: i64,
) -> Result<crate::models::PackImportResult> {
    let file: crate::models::PackFile = serde_json::from_str(&contents)
        .map_err(|e| anyhow!("Not a valid Birdet pack file: {}", e))?;
    if file.format != "birdet-pack" {
        return Err(anyhow!("Unrecognised file — expected a Birdet pack export."));
    }
    let name = {
        let n = file.name.trim();
        if n.is_empty() { "Imported pack".to_string() } else { n.to_string() }
    };

    // Resolve each bird against the library; collect the ones we must download.
    let mut bird_ids: Vec<i64> = Vec::new();
    let mut linked_existing = 0i64;
    let mut skipped = 0i64;
    let mut missing_codes: Vec<String> = Vec::new();
    for b in &file.birds {
        let mut id: Option<i64> = None;
        if let Some(code) = b.ebird_code.as_ref().map(|c| c.trim()).filter(|c| !c.is_empty()) {
            id = sqlx::query_scalar("SELECT id FROM birds WHERE ebird_code = ?1")
                .bind(code)
                .fetch_optional(&db.0)
                .await?;
        }
        if id.is_none() {
            id = sqlx::query_scalar("SELECT id FROM birds WHERE scientific_name = ?1")
                .bind(&b.scientific_name)
                .fetch_optional(&db.0)
                .await?;
        }
        match id {
            Some(i) => {
                bird_ids.push(i);
                linked_existing += 1;
            }
            None => match b.ebird_code.as_ref().map(|c| c.trim()).filter(|c| !c.is_empty()) {
                Some(code) => missing_codes.push(code.to_string()),
                None => skipped += 1, // no eBird code → nothing to download
            },
        }
    }

    // Download the missing birds from Xeno-Canto.
    let mut downloaded_new = 0i64;
    let mut recordings_added = 0i64;
    if !missing_codes.is_empty() {
        let xc_key = settings::get_setting(db, "xc_api_key")
            .await?
            .filter(|k| !k.trim().is_empty())
            .ok_or_else(|| anyhow!("Missing Xeno-Canto API key — add it in Settings."))?;
        let client = regions::build_client()?;
        let rec_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| anyhow!("app data dir: {}", e))?
            .join("recordings");
        std::fs::create_dir_all(&rec_dir)?;

        taxonomy::ensure(db).await?; // make sure taxa_for can resolve the codes
        let taxa = taxonomy::taxa_for(db, &missing_codes).await?;
        skipped += missing_codes.len() as i64 - taxa.len() as i64; // unresolvable codes
        let (new_ids, imported, no_audio, recs) = import_taxa(
            app, db, &client, &xc_key, &rec_dir, &taxa, "",
            None, None, max_per_species.max(1),
        )
        .await?;
        bird_ids.extend(new_ids);
        downloaded_new += imported;
        skipped += no_audio; // species with no audio available
        recordings_added += recs;
    }

    if bird_ids.is_empty() {
        return Err(anyhow!("No birds from this pack could be imported."));
    }
    bird_ids.sort_unstable();
    bird_ids.dedup();

    let pack_id = crate::services::packs::create_pack_from_birds(db, &name, &bird_ids).await?;
    emit(app, "done", format!("Imported pack “{}”.", name), 1, 1);

    Ok(crate::models::PackImportResult {
        pack_id,
        name,
        linked_existing,
        downloaded_new,
        skipped,
        recordings_added,
    })
}

/// Species resolved from a pasted eBird list (URL or CSV), for the import
/// preview. `source` describes what was recognised; `unresolved` lists CSV names
/// that matched no eBird species (hybrids, "sp." entries, typos).
#[derive(Serialize, Clone)]
pub struct ListPreview {
    pub source: String,
    pub species: Vec<taxonomy::SpeciesLite>,
    pub unresolved: Vec<String>,
}

/// Extract an eBird checklist submission id (`S…`) from a URL or bare code.
fn extract_checklist_id(s: &str) -> Option<String> {
    let grab = |seg: &str| {
        seg.chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
    };
    if let Some(idx) = s.find("/checklist/") {
        let code = grab(&s[idx + "/checklist/".len()..]);
        if code.starts_with('S') && code.len() > 1 {
            return Some(code);
        }
    }
    let t = s.trim();
    if t.len() > 1 && t.starts_with('S') && t[1..].chars().all(|c| c.is_ascii_digit()) {
        return Some(t.to_string());
    }
    None
}

/// Extract an eBird region or hotspot code from a URL or bare code. Hotspots
/// (`L…`) and region codes (`US`, `GB-ENG`, `US-NY-109`) both work with spplist.
fn extract_region_code(s: &str) -> Option<String> {
    let grab = |seg: &str| {
        seg.chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect::<String>()
    };
    for marker in ["/hotspot/", "/region/"] {
        if let Some(idx) = s.find(marker) {
            let code = grab(&s[idx + marker.len()..]);
            if !code.is_empty() {
                return Some(code);
            }
        }
    }
    // A bare single token: L-hotspot or an uppercase region code.
    let t = s.trim();
    let is_token = !t.is_empty() && !t.contains(char::is_whitespace);
    if is_token {
        if t.starts_with('L') && t[1..].chars().all(|c| c.is_ascii_digit()) && t.len() > 1 {
            return Some(t.to_string());
        }
        if t.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
            && t.chars().any(|c| c.is_ascii_uppercase())
        {
            return Some(t.to_string());
        }
    }
    None
}

/// Pull species names out of a CSV blob (an eBird life-list / My-Data export) or,
/// failing a recognisable header, treat each line as a name. Prefers the
/// Scientific Name column, falling back to Common Name.
fn extract_csv_names(blob: &str) -> Vec<String> {
    let mut rdr = csv::ReaderBuilder::new().flexible(true).from_reader(blob.as_bytes());
    if let Ok(headers) = rdr.headers().cloned() {
        let find = |want: &str| {
            headers.iter().position(|h| h.trim().eq_ignore_ascii_case(want))
        };
        let col = find("Scientific Name").or_else(|| find("Common Name"));
        if let Some(col) = col {
            let mut out = Vec::new();
            for rec in rdr.records().flatten() {
                if let Some(v) = rec.get(col) {
                    let v = v.trim();
                    if !v.is_empty() {
                        out.push(v.to_string());
                    }
                }
            }
            return out;
        }
    }
    // No eBird header — one name per line.
    blob.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

/// Resolve a pasted eBird list (checklist URL, hotspot/region URL or code, or a
/// life-list / My-Data CSV) into the species it contains, without downloading
/// anything. The frontend previews the result, then imports via `import_species`
/// with the returned eBird codes.
pub async fn resolve_ebird_list(db: &Db, input: &str) -> Result<ListPreview> {
    let t = input.trim();
    if t.is_empty() {
        return Err(anyhow!("Nothing to import — paste an eBird list URL or a CSV export."));
    }

    if let Some(sub_id) = extract_checklist_id(t) {
        let key = regions::ebird_key(db).await?;
        let client = regions::build_client()?;
        let codes = regions::checklist_species(&client, &key, &sub_id).await;
        if codes.is_empty() {
            return Err(anyhow!("No species found on checklist {} (is it public?).", sub_id));
        }
        let species = taxonomy::species_lite_for(db, &codes).await?;
        return Ok(ListPreview { source: format!("Checklist {}", sub_id), species, unresolved: vec![] });
    }

    if let Some(code) = extract_region_code(t) {
        let key = regions::ebird_key(db).await?;
        let client = regions::build_client()?;
        let codes = regions::fetch_spplist(&client, &key, &code).await?;
        if codes.is_empty() {
            return Err(anyhow!("No species found for {}.", code));
        }
        let species = taxonomy::species_lite_for(db, &codes).await?;
        return Ok(ListPreview { source: format!("Region {}", code), species, unresolved: vec![] });
    }

    // Otherwise treat the input as a CSV / newline-separated name list.
    let names = extract_csv_names(t);
    if names.is_empty() {
        return Err(anyhow!("Couldn't find any species names — expected an eBird CSV or a URL."));
    }
    let (species, unresolved) = taxonomy::resolve_names(db, &names).await?;
    if species.is_empty() {
        return Err(anyhow!("None of the {} names matched an eBird species.", names.len()));
    }
    Ok(ListPreview { source: "Pasted list".to_string(), species, unresolved })
}

/// Query Xeno-Canto for a species (by scientific name) and return the raw
/// recording list. Silent (no progress events) — used by the per-bird
/// recording browser. Tolerant: returns [] on any request/parse error.
async fn xc_recordings_for(
    client: &reqwest::Client,
    xc_key: &str,
    sci_name: &str,
    quality: Option<&str>,
    rec_type: Option<&str>,
) -> Vec<XcRecording> {
    let mut query = format!("sp:\"{}\"", sci_name);
    if let Some(q) = quality.filter(|s| !s.is_empty()) {
        query.push_str(&format!(" q:{}", q));
    }
    if let Some(t) = rec_type.filter(|s| !s.is_empty()) {
        query.push_str(&format!(" type:{}", t));
    }
    match client
        .get(format!("{}?query={}&key={}", XC_BASE, enc(&query), enc(xc_key)))
        .send()
        .await
    {
        Ok(resp) => {
            let body = resp.text().await.unwrap_or_default();
            serde_json::from_str::<XcResponse>(&body)
                .map(|x| x.recordings)
                .unwrap_or_default()
        }
        Err(_) => Vec::new(),
    }
}

/// Look up a bird's scientific name.
async fn bird_sci_name(db: &Db, bird_id: i64) -> Result<String> {
    sqlx::query_scalar::<_, String>("SELECT scientific_name FROM birds WHERE id = ?1")
        .bind(bird_id)
        .fetch_optional(&db.0)
        .await?
        .ok_or_else(|| anyhow!("Bird {} not found.", bird_id))
}

/// Search Xeno-Canto for more recordings of a bird already in the library,
/// omitting any already downloaded (matched by xc_id). For the per-bird
/// "add recordings" browser.
pub async fn search_recordings(
    db: &Db,
    bird_id: i64,
    quality: Option<String>,
    rec_type: Option<String>,
) -> Result<Vec<RecordingCandidate>> {
    let xc_key = settings::get_setting(db, "xc_api_key")
        .await?
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| anyhow!("Missing Xeno-Canto API key — add it in Settings."))?;
    let sci = bird_sci_name(db, bird_id).await?;
    let client = regions::build_client()?;

    let have: Vec<String> =
        sqlx::query_scalar("SELECT xc_id FROM recordings WHERE bird_id = ?1 AND xc_id IS NOT NULL")
            .bind(bird_id)
            .fetch_all(&db.0)
            .await?;
    let have: std::collections::HashSet<String> = have.into_iter().collect();

    let recs = xc_recordings_for(&client, &xc_key, &sci, quality.as_deref(), rec_type.as_deref()).await;
    Ok(recs
        .into_iter()
        .filter(|r| !r.file.is_empty() && !have.contains(&r.id))
        .map(|r| RecordingCandidate {
            xc_id: r.id,
            recordist: r.rec,
            quality: r.q,
            length: r.length,
            rec_type: r.rec_type,
            location: r.loc,
            license_url: r.lic,
        })
        .collect())
}

/// Download and insert specific Xeno-Canto recordings (by xc_id) for a bird
/// already in the library. Returns the number added.
pub async fn add_recordings(
    app: &AppHandle,
    db: &Db,
    bird_id: i64,
    xc_ids: Vec<String>,
) -> Result<i64> {
    if xc_ids.is_empty() {
        return Ok(0);
    }
    let xc_key = settings::get_setting(db, "xc_api_key")
        .await?
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| anyhow!("Missing Xeno-Canto API key — add it in Settings."))?;
    let sci = bird_sci_name(db, bird_id).await?;
    let client = regions::build_client()?;

    let rec_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| anyhow!("app data dir: {}", e))?
        .join("recordings");
    std::fs::create_dir_all(&rec_dir)?;

    let wanted: std::collections::HashSet<String> = xc_ids.into_iter().collect();
    let recs = xc_recordings_for(&client, &xc_key, &sci, None, None).await;
    let mut added = 0i64;
    for r in recs.iter().filter(|r| wanted.contains(&r.id) && !r.file.is_empty()) {
        let file_url = if r.file.starts_with("//") {
            format!("https:{}", r.file)
        } else {
            r.file.clone()
        };
        let filename = recording_file_name(&r.id, &r.file_name);
        if download_to(&client, &xc_key, &file_url, &rec_dir.join(&filename)).await.is_ok() {
            insert_recording(db, bird_id, &filename, r).await?;
            added += 1;
        }
    }
    Ok(added)
}

/// Back-fill quality + type on existing recordings that predate those columns.
/// One Xeno-Canto query per bird (not per recording): fetch the species' list,
/// map xc_id → (q, type), update matching rows. Paced for the XC rate limit.
/// Returns the number of recordings updated.
pub async fn backfill_recording_meta(db: &Db) -> Result<i64> {
    let xc_key = settings::get_setting(db, "xc_api_key")
        .await?
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| anyhow!("Missing Xeno-Canto API key — add it in Settings."))?;
    let client = regions::build_client()?;

    // Birds that have at least one XC recording still missing quality AND type.
    let bird_ids: Vec<i64> = sqlx::query_scalar(
        r#"SELECT DISTINCT bird_id FROM recordings
           WHERE xc_id IS NOT NULL AND quality IS NULL AND rec_type IS NULL"#,
    )
    .fetch_all(&db.0)
    .await?;

    let mut updated = 0i64;
    for bird_id in bird_ids {
        let sci = match bird_sci_name(db, bird_id).await {
            Ok(s) => s,
            Err(_) => continue,
        };
        let recs = xc_recordings_for(&client, &xc_key, &sci, None, None).await;
        let meta: std::collections::HashMap<String, (String, String)> = recs
            .into_iter()
            .map(|r| (r.id, (r.q, r.rec_type)))
            .collect();

        // Rows for this bird still needing a backfill.
        let rows: Vec<(i64, String)> = sqlx::query_as(
            r#"SELECT id, xc_id FROM recordings
               WHERE bird_id = ?1 AND xc_id IS NOT NULL AND quality IS NULL AND rec_type IS NULL"#,
        )
        .bind(bird_id)
        .fetch_all(&db.0)
        .await?;

        for (rec_id, xc_id) in rows {
            if let Some((q, t)) = meta.get(&xc_id) {
                if q.is_empty() && t.is_empty() {
                    continue;
                }
                sqlx::query("UPDATE recordings SET quality = ?1, rec_type = ?2 WHERE id = ?3")
                    .bind(if q.is_empty() { None } else { Some(q.clone()) })
                    .bind(if t.is_empty() { None } else { Some(t.clone()) })
                    .bind(rec_id)
                    .execute(&db.0)
                    .await?;
                updated += 1;
            }
        }
        tokio::time::sleep(Duration::from_millis(1000)).await; // XC rate limit
    }
    Ok(updated)
}

#[derive(Serialize, Clone)]
pub struct RepairSummary {
    pub checked: i64,
    pub repaired: i64,
    pub failed: i64,
}

/// Chromium/WebView2 (and GStreamer) can't decode 24/32-bit PCM WAV, so those
/// recordings fail to play. Re-quantize such files to 16-bit PCM in place. No-op
/// for non-WAV files or WAVs already ≤16-bit int. Returns whether it rewrote.
fn normalize_wav(path: &Path) -> Result<bool> {
    let mut reader = match hound::WavReader::open(path) {
        Ok(r) => r,
        Err(_) => return Ok(false), // not a WAV we can parse — leave untouched
    };
    let spec = reader.spec();
    if spec.sample_format == hound::SampleFormat::Int && spec.bits_per_sample <= 16 {
        return Ok(false); // already webview-playable
    }
    let out_spec = hound::WavSpec {
        channels: spec.channels,
        sample_rate: spec.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let tmp = path.with_extension("wavtmp");
    {
        let mut writer = hound::WavWriter::create(&tmp, out_spec)?;
        match spec.sample_format {
            hound::SampleFormat::Int => {
                let shift = spec.bits_per_sample.saturating_sub(16) as u32;
                for s in reader.samples::<i32>() {
                    writer.write_sample((s? >> shift) as i16)?;
                }
            }
            hound::SampleFormat::Float => {
                for s in reader.samples::<f32>() {
                    let v = (s? * 32767.0).clamp(-32768.0, 32767.0);
                    writer.write_sample(v as i16)?;
                }
            }
        }
        writer.finalize()?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(true)
}

/// Build the playable https URL for an XC `file` field (may be protocol-relative).
fn xc_file_url(file: &str) -> String {
    if file.starts_with("//") {
        format!("https:{}", file)
    } else {
        file.to_string()
    }
}

/// Does the file look like real, non-truncated audio? Missing, implausibly tiny,
/// or bytes starting like HTML/JSON (an XC error page saved as audio) → broken.
fn looks_broken(path: &Path) -> bool {
    use std::io::Read;
    match std::fs::metadata(path) {
        Err(_) => true,
        Ok(m) if m.len() < 2048 => true,
        Ok(_) => {
            let mut buf = [0u8; 16];
            let n = std::fs::File::open(path)
                .and_then(|mut f| f.read(&mut buf))
                .unwrap_or(0);
            buf[..n]
                .iter()
                .find(|b| !b.is_ascii_whitespace())
                .map(|&b| b == b'<' || b == b'{')
                .unwrap_or(n == 0)
        }
    }
}

/// Fetch a single XC recording by its catalogue number (the `nr:` tag).
async fn xc_recording_by_id(
    client: &reqwest::Client,
    xc_key: &str,
    xc_id: &str,
) -> Option<XcRecording> {
    let query = format!("nr:{}", xc_id);
    let body = client
        .get(format!("{}?query={}&key={}", XC_BASE, enc(&query), enc(xc_key)))
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;
    serde_json::from_str::<XcResponse>(&body)
        .ok()?
        .recordings
        .into_iter()
        .find(|r| r.id == xc_id)
}

/// Scan every downloaded Xeno-Canto recording and re-download any whose local
/// file is missing, truncated, or a saved error page. Seed audio (no `xc_id`,
/// read-only in resources) is left alone. Returns a summary for the UI.
pub async fn repair_recordings(app: &AppHandle, db: &Db) -> Result<RepairSummary> {
    let xc_key = settings::get_setting(db, "xc_api_key")
        .await?
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| anyhow!("Missing Xeno-Canto API key — add it in Settings."))?;
    let client = regions::build_client()?;

    let rec_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| anyhow!("app data dir: {}", e))?
        .join("recordings");
    std::fs::create_dir_all(&rec_dir)?;

    let rows: Vec<(i64, String, String)> = sqlx::query_as(
        r#"SELECT id, xc_id, filename FROM recordings
           WHERE xc_id IS NOT NULL AND source = 'xeno-canto'"#,
    )
    .fetch_all(&db.0)
    .await?;

    let mut summary = RepairSummary { checked: 0, repaired: 0, failed: 0 };
    for (id, xc_id, filename) in rows {
        summary.checked += 1;

        // A valid imported file needs no re-download — but a 24/32-bit WAV won't
        // decode in the webview even though it's intact, so fix that in place.
        let current = rec_dir.join(&filename);
        if current.exists() && !looks_broken(&current) {
            if matches!(normalize_wav(&current), Ok(true)) {
                summary.repaired += 1;
            }
            continue;
        }

        // Re-download to a guaranteed-safe leaf name (also self-heals any legacy
        // unsanitized filename), then verify the result actually looks like audio.
        let safe_name = recording_file_name(&xc_id, &filename);
        let dest = rec_dir.join(&safe_name);
        let ok = match xc_recording_by_id(&client, &xc_key, &xc_id).await {
            Some(r) if !r.file.is_empty() => {
                download_to(&client, &xc_key, &xc_file_url(&r.file), &dest)
                    .await
                    .is_ok()
            }
            _ => false,
        };
        if ok && !looks_broken(&dest) {
            if safe_name != filename {
                let _ = sqlx::query("UPDATE recordings SET filename = ?1 WHERE id = ?2")
                    .bind(&safe_name)
                    .bind(id)
                    .execute(&db.0)
                    .await;
            }
            summary.repaired += 1;
        } else {
            summary.failed += 1;
        }
        tokio::time::sleep(Duration::from_millis(300)).await; // gentle on XC
    }
    Ok(summary)
}

/// Recording metadata for the per-bird recording list.
#[derive(Serialize, Clone)]
pub struct RecordingInfo {
    pub id: i64,
    pub filename: Option<String>,
    pub source: Option<String>,
    pub xc_id: Option<String>,
    pub recordist: Option<String>,
    pub license_url: Option<String>,
    pub location: Option<String>,
    pub quality: Option<String>,
    pub rec_type: Option<String>,
}

/// All recordings stored for a bird.
pub async fn get_bird_recordings(db: &Db, bird_id: i64) -> Result<Vec<RecordingInfo>> {
    let rows = sqlx::query(
        r#"SELECT id AS id, filename AS filename, source AS source, xc_id AS xc_id,
                  recordist AS recordist, license_url AS license_url, location AS location,
                  quality AS quality, rec_type AS rec_type
           FROM recordings WHERE bird_id = ?1 ORDER BY id"#,
    )
    .bind(bird_id)
    .fetch_all(&db.0)
    .await?;
    use sqlx::Row;
    Ok(rows
        .into_iter()
        .map(|row| RecordingInfo {
            id: row.get("id"),
            filename: row.get("filename"),
            source: row.get("source"),
            xc_id: row.get("xc_id"),
            recordist: row.get("recordist"),
            license_url: row.get("license_url"),
            location: row.get("location"),
            quality: row.get("quality"),
            rec_type: row.get("rec_type"),
        })
        .collect())
}

/// Delete a recording: its DB row, pack links, history refs, and — if the audio
/// lives in the writable app-data dir (an import, not bundled seed audio) — the
/// file on disk. Errors if this is the bird's last recording (a bird with no
/// audio can't be quizzed).
pub async fn delete_recording(app: &AppHandle, db: &Db, recording_id: i64) -> Result<()> {
    let row: Option<(i64, Option<String>)> =
        sqlx::query_as("SELECT bird_id, filename FROM recordings WHERE id = ?1")
            .bind(recording_id)
            .fetch_optional(&db.0)
            .await?;
    let (bird_id, filename) = row.ok_or_else(|| anyhow!("Recording {} not found.", recording_id))?;

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM recordings WHERE bird_id = ?1")
        .bind(bird_id)
        .fetch_one(&db.0)
        .await?;
    if count <= 1 {
        return Err(anyhow!(
            "Can't delete the bird's only recording — remove the bird from the library instead."
        ));
    }

    let mut tx = db.0.begin().await?;
    sqlx::query("DELETE FROM pack_recordings WHERE recording_id = ?1")
        .bind(recording_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM history WHERE recording_id = ?1")
        .bind(recording_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM recordings WHERE id = ?1")
        .bind(recording_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    // Remove the downloaded file (only the writable copy; never bundled seeds).
    if let Some(name) = filename {
        if let Ok(dir) = app.path().app_data_dir() {
            let path = dir.join("recordings").join(&name);
            if path.exists() {
                let _ = std::fs::remove_file(path);
            }
        }
    }
    Ok(())
}

/// Delete a bird entirely: its recordings (rows + downloaded files), pack links,
/// mastery/history references, and the bird row. Errors if the id doesn't exist.
pub async fn delete_bird(app: &AppHandle, db: &Db, bird_id: i64) -> Result<()> {
    // Downloaded filenames to remove from disk after the DB delete commits.
    let filenames: Vec<Option<String>> =
        sqlx::query_scalar("SELECT filename FROM recordings WHERE bird_id = ?1")
            .bind(bird_id)
            .fetch_all(&db.0)
            .await?;

    let mut tx = db.0.begin().await?;
    // Detach history refs (foreign keys may not be enforced, so be explicit).
    sqlx::query("UPDATE history SET recording_ID = NULL WHERE recording_ID IN (SELECT id FROM recordings WHERE bird_id = ?1)")
        .bind(bird_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE history SET bird_ID = NULL WHERE bird_ID = ?1")
        .bind(bird_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM pack_recordings WHERE recording_id IN (SELECT id FROM recordings WHERE bird_id = ?1)")
        .bind(bird_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM mastery WHERE bird_id = ?1")
        .bind(bird_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM recordings WHERE bird_id = ?1")
        .bind(bird_id)
        .execute(&mut *tx)
        .await?;
    let res = sqlx::query("DELETE FROM birds WHERE id = ?1")
        .bind(bird_id)
        .execute(&mut *tx)
        .await?;
    if res.rows_affected() == 0 {
        return Err(anyhow!("Bird {} not found.", bird_id));
    }
    tx.commit().await?;

    // Remove downloaded audio files (writable copies only; never bundled seeds).
    if let Ok(dir) = app.path().app_data_dir() {
        let rec_dir = dir.join("recordings");
        for name in filenames.into_iter().flatten() {
            let path = rec_dir.join(&name);
            if path.exists() {
                let _ = std::fs::remove_file(path);
            }
        }
    }
    Ok(())
}

/// Import a ranked list of taxa concurrently: up to `SPECIES_CONCURRENCY`
/// species in flight at once, each fetching + downloading its recordings while
/// the shared `Throttle` keeps XC API queries paced. Returns
/// `(bird_ids_in_input_order, imported, skipped, recordings_added)`.
#[allow(clippy::too_many_arguments)]
async fn import_taxa(
    app: &AppHandle,
    db: &Db,
    client: &reqwest::Client,
    xc_key: &str,
    rec_dir: &Path,
    taxa: &[Taxon],
    region: &str,
    quality: Option<&str>,
    rec_type: Option<&str>,
    max_per_species: i64,
) -> Result<(Vec<i64>, i64, i64, i64)> {
    let throttle = Throttle::new();
    let total = taxa.len() as i64;

    // Launch one species task. Clones everything it needs so the future is
    // 'static (spawnable); returns its input index so results stay ordered.
    let launch = |set: &mut JoinSet<Result<(usize, Option<i64>, i64)>>, i: usize, taxon: Taxon| {
        let (app, db, client) = (app.clone(), db.clone(), client.clone());
        let xc_key = xc_key.to_string();
        let rec_dir = rec_dir.to_path_buf();
        let region = region.to_string();
        let quality = quality.map(str::to_string);
        let rec_type = rec_type.map(str::to_string);
        let throttle = throttle.clone();
        set.spawn(async move {
            let (bird_id, recs) = fetch_species(
                &app, &db, &client, &xc_key, &rec_dir, &taxon, &region,
                quality.as_deref(), rec_type.as_deref(), max_per_species, i as i64, total, &throttle,
            )
            .await?;
            Ok((i, bird_id, recs))
        });
    };

    let mut set: JoinSet<Result<(usize, Option<i64>, i64)>> = JoinSet::new();
    let mut pending = taxa.iter().cloned().enumerate();
    for (i, taxon) in pending.by_ref().take(SPECIES_CONCURRENCY) {
        launch(&mut set, i, taxon);
    }

    let mut slots: Vec<Option<i64>> = vec![None; taxa.len()];
    let mut skipped = 0i64;
    let mut recordings_added = 0i64;
    let mut done = 0i64;
    while let Some(res) = set.join_next().await {
        // A task's own Result: propagate a hard error (DB failure etc.) rather
        // than silently dropping species; a join panic counts as a skip.
        match res {
            Ok(Ok((i, bird_id, recs))) => {
                recordings_added += recs;
                match bird_id {
                    Some(id) => slots[i] = Some(id),
                    None => skipped += 1,
                }
            }
            Ok(Err(e)) => return Err(e),
            Err(_) => skipped += 1,
        }
        done += 1;
        emit(app, "species", format!("{}/{} species", done, total), done, total);
        if let Some((i, taxon)) = pending.next() {
            launch(&mut set, i, taxon);
        }
    }

    let bird_ids: Vec<i64> = slots.into_iter().flatten().collect();
    let imported = bird_ids.len() as i64;
    Ok((bird_ids, imported, skipped, recordings_added))
}

/// Fetch one species' Xeno-Canto audio, download up to `max_per_species` clips,
/// and upsert the bird + recordings. Returns `(Some(bird_id), n)` if any audio
/// was found (bird stored even if a download fails), else `(None, 0)`. The XC
/// API query is spaced by the shared `Throttle`; downloads run concurrently
/// under its download semaphore. `idx`/`total` drive the progress display only.
#[allow(clippy::too_many_arguments)]
async fn fetch_species(
    app: &AppHandle,
    db: &Db,
    client: &reqwest::Client,
    xc_key: &str,
    rec_dir: &Path,
    taxon: &Taxon,
    region: &str,
    quality: Option<&str>,
    rec_type: Option<&str>,
    max_per_species: i64,
    idx: i64,
    total: i64,
    throttle: &Throttle,
) -> Result<(Option<i64>, i64)> {
    // XC v3 query. v3 requires tagged terms — a bare "Genus species" errors;
    // the species is given via the sp: tag with the full binomial quoted.
    let mut query = format!("sp:\"{}\"", taxon.sci_name);
    if let Some(q) = quality.filter(|s| !s.is_empty()) {
        query.push_str(&format!(" q:{}", q));
    }
    if let Some(t) = rec_type.filter(|s| !s.is_empty()) {
        query.push_str(&format!(" type:{}", t));
    }

    // Pace the rate-limited API query (shared across concurrent species tasks).
    throttle.api_slot().await;

    // Fetch as text first so we can surface exactly what XC returned when
    // parsing or the query is off (v3 shape/syntax diagnostics).
    let xc: XcResponse = match client
        .get(format!("{}?query={}&key={}", XC_BASE, enc(&query), enc(xc_key)))
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            match serde_json::from_str::<XcResponse>(&body) {
                Ok(x) => {
                    emit(app, "downloading", format!("XC {} → {} recs [{}]", status.as_u16(), x.recordings.len(), query), idx, total);
                    x
                }
                Err(e) => {
                    let snippet: String = body.chars().take(400).collect();
                    emit(app, "downloading", format!("XC {} parse-fail: {} | {}", status.as_u16(), e, snippet), idx, total);
                    XcResponse { recordings: vec![] }
                }
            }
        }
        Err(e) => {
            emit(app, "downloading", format!("XC request error: {}", e), idx, total);
            XcResponse { recordings: vec![] }
        }
    };

    let picks: Vec<XcRecording> = xc
        .recordings
        .into_iter()
        .filter(|r| !r.file.is_empty())
        .take(max_per_species.max(1) as usize)
        .collect();

    if picks.is_empty() {
        return Ok((None, 0));
    }

    let bird_id = upsert_bird(db, taxon, region).await?;

    // Download this species' picks concurrently (media CDN, bounded by the
    // shared download semaphore); insert the successes into the DB afterwards.
    emit(app, "downloading", format!("↓ {} ({} recs)", taxon.com_name, picks.len()), idx, total);
    let mut dl: JoinSet<std::result::Result<(String, XcRecording), (String, String)>> = JoinSet::new();
    for r in picks {
        let file_url = xc_file_url(&r.file);
        let filename = recording_file_name(&r.id, &r.file_name);
        let dest = rec_dir.join(&filename);
        let (client, xc_key) = (client.clone(), xc_key.to_string());
        let sem = throttle.downloads.clone();
        dl.spawn(async move {
            let _permit = sem.acquire_owned().await;
            match download_to(&client, &xc_key, &file_url, &dest).await {
                Ok(()) => Ok((filename, r)),
                Err(e) => Err((r.id, e.to_string())),
            }
        });
    }

    let mut recs = 0i64;
    while let Some(res) = dl.join_next().await {
        match res {
            Ok(Ok((filename, r))) => {
                insert_recording(db, bird_id, &filename, &r).await?;
                recs += 1;
            }
            Ok(Err((id, e))) => {
                emit(app, "downloading", format!("skip XC{}: {}", id, e), idx, total);
            }
            Err(_) => {} // download task panicked — treat as a skipped clip
        }
    }

    Ok((Some(bird_id), recs))
}

/// Is this status worth retrying? XC's download host answers 503/502/504 under
/// load and 429 when throttling — all transient. 4xx (except 429) is not.
fn is_transient(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

/// Honour a `Retry-After` header (seconds), clamped to a sane range.
fn retry_after(resp: &reqwest::Response) -> Option<Duration> {
    resp.headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .map(|s| Duration::from_secs(s.clamp(1, 10)))
}

async fn download_to(
    client: &reqwest::Client,
    xc_key: &str,
    url: &str,
    dest: &std::path::Path,
) -> Result<()> {
    let sep = if url.contains('?') { '&' } else { '?' };
    let full = format!("{}{}key={}", url, sep, enc(xc_key));

    // Retry transient failures (503/502/504/429 or a dropped connection) with
    // Retry-After or exponential backoff, so a momentarily busy XC download host
    // doesn't permanently skip an otherwise-good recording.
    const MAX_TRIES: u32 = 4;
    let mut resp = None;
    let mut last: Option<anyhow::Error> = None;
    for attempt in 0..MAX_TRIES {
        let is_last = attempt + 1 == MAX_TRIES;
        match client.get(&full).send().await {
            Ok(r) if r.status().is_success() => {
                resp = Some(r);
                break;
            }
            Ok(r) if is_transient(r.status()) && !is_last => {
                let wait = retry_after(&r).unwrap_or_else(|| Duration::from_millis(500 * (1 << attempt)));
                last = Some(anyhow!("HTTP {}", r.status()));
                tokio::time::sleep(wait).await;
            }
            // Non-transient status (404 etc.), or transient but out of tries.
            Ok(r) => return Err(r.error_for_status().expect_err("non-success").into()),
            Err(e) if !is_last => {
                last = Some(e.into());
                tokio::time::sleep(Duration::from_millis(500 * (1 << attempt))).await;
            }
            Err(e) => return Err(e.into()),
        }
    }
    let resp = resp.ok_or_else(|| last.unwrap_or_else(|| anyhow!("download failed")))?;

    // Reject a non-audio body: XC occasionally answers a 200 with an HTML/JSON
    // error page instead of the file. Saving that as `.mp3` yields a "recording"
    // that later fails to decode ("format not supported"/0:00). Bail early.
    if let Some(ct) = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
    {
        let ct = ct.to_ascii_lowercase();
        if ct.starts_with("text/") || ct.contains("json") || ct.contains("html") {
            return Err(anyhow!("expected audio, got {}", ct));
        }
    }
    // Reject an oversized download before buffering it: a malicious pack/URL
    // could otherwise exhaust memory or fill the disk. XC recordings are small.
    if let Some(len) = resp.content_length() {
        if len > MAX_RECORDING_BYTES {
            return Err(anyhow!("recording too large ({} bytes)", len));
        }
    }
    let bytes = resp.bytes().await?;
    if bytes.is_empty() {
        return Err(anyhow!("empty file"));
    }
    // Guard against a server that lied about (or omitted) Content-Length.
    if bytes.len() as u64 > MAX_RECORDING_BYTES {
        return Err(anyhow!("recording too large ({} bytes)", bytes.len()));
    }
    std::fs::write(dest, &bytes)?;
    // Down-convert 24/32-bit WAV to 16-bit so the webview can decode it.
    // Best-effort: a failure here still leaves the raw download in place.
    let _ = normalize_wav(dest);
    Ok(())
}

/// Insert the bird if new (keyed on eBird code), else return the existing id,
/// backfilling descriptors on a scientific-name match (seed data).
async fn upsert_bird(db: &Db, taxon: &Taxon, region: &str) -> Result<i64> {
    if let Some(id) = sqlx::query_scalar::<_, i64>("SELECT id FROM birds WHERE ebird_code = ?1")
        .bind(&taxon.species_code)
        .fetch_optional(&db.0)
        .await?
    {
        return Ok(id);
    }
    if let Some(id) = sqlx::query_scalar::<_, i64>("SELECT id FROM birds WHERE scientific_name = ?1")
        .bind(&taxon.sci_name)
        .fetch_optional(&db.0)
        .await?
    {
        sqlx::query(
            "UPDATE birds SET ebird_code=?1, family=?2, family_sci=?3, taxon_order=?4, region=?5 WHERE id=?6",
        )
        .bind(&taxon.species_code)
        .bind(taxon.family_com_name.clone())
        .bind(taxon.family_sci_name.clone())
        .bind(taxon.order.clone())
        .bind(region)
        .bind(id)
        .execute(&db.0)
        .await?;
        return Ok(id);
    }

    let bird_id = sqlx::query_scalar::<_, i64>(
        r#"INSERT INTO birds (common_name, scientific_name, ebird_code, family, family_sci, taxon_order, region)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) RETURNING id"#,
    )
    .bind(&taxon.com_name)
    .bind(&taxon.sci_name)
    .bind(&taxon.species_code)
    .bind(taxon.family_com_name.clone())
    .bind(taxon.family_sci_name.clone())
    .bind(taxon.order.clone())
    .bind(region)
    .fetch_one(&db.0)
    .await?;

    Ok(bird_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checklist_id_from_url_and_bare() {
        assert_eq!(extract_checklist_id("https://ebird.org/checklist/S123456789").as_deref(), Some("S123456789"));
        assert_eq!(extract_checklist_id("https://ebird.org/checklist/S99?foo=1").as_deref(), Some("S99"));
        assert_eq!(extract_checklist_id("S42").as_deref(), Some("S42"));
        assert_eq!(extract_checklist_id("Setophaga"), None); // not a checklist code
        assert_eq!(extract_checklist_id("https://ebird.org/region/US"), None);
    }

    #[test]
    fn region_and_hotspot_codes() {
        assert_eq!(extract_region_code("https://ebird.org/hotspot/L99381").as_deref(), Some("L99381"));
        assert_eq!(extract_region_code("https://ebird.org/region/GB-ENG?season=all").as_deref(), Some("GB-ENG"));
        assert_eq!(extract_region_code("US-NY-109").as_deref(), Some("US-NY-109"));
        assert_eq!(extract_region_code("L12345").as_deref(), Some("L12345"));
        assert_eq!(extract_region_code("Turdus migratorius"), None); // has a space
    }

    #[test]
    fn csv_prefers_scientific_name_and_falls_back_to_lines() {
        // eBird life-list style CSV.
        let csv = "Common Name,Scientific Name,Count\nMallard,Anas platyrhynchos,3\nRobin,Turdus migratorius,1\n";
        assert_eq!(extract_csv_names(csv), vec!["Anas platyrhynchos", "Turdus migratorius"]);

        // Only a common-name column present.
        let csv2 = "Row,Common Name\n1,Mallard\n2,Robin\n";
        assert_eq!(extract_csv_names(csv2), vec!["Mallard", "Robin"]);

        // No recognizable header — one name per line.
        let plain = "Anas platyrhynchos\nTurdus migratorius\n";
        assert_eq!(extract_csv_names(plain), vec!["Anas platyrhynchos", "Turdus migratorius"]);
    }
}

async fn insert_recording(db: &Db, bird_id: i64, filename: &str, r: &XcRecording) -> Result<()> {
    let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM recordings WHERE xc_id = ?1")
        .bind(&r.id)
        .fetch_optional(&db.0)
        .await?;
    if exists.is_some() {
        return Ok(());
    }
    sqlx::query(
        r#"INSERT INTO recordings (bird_id, filename, location, source, xc_id, recordist, license_url, quality, rec_type)
           VALUES (?1, ?2, ?3, 'xeno-canto', ?4, ?5, ?6, ?7, ?8)"#,
    )
    .bind(bird_id)
    .bind(filename)
    .bind(if r.loc.is_empty() { None } else { Some(r.loc.clone()) })
    .bind(&r.id)
    .bind(if r.rec.is_empty() { None } else { Some(r.rec.clone()) })
    .bind(if r.lic.is_empty() { None } else { Some(r.lic.clone()) })
    .bind(if r.q.is_empty() { None } else { Some(r.q.clone()) })
    .bind(if r.rec_type.is_empty() { None } else { Some(r.rec_type.clone()) })
    .execute(&db.0)
    .await?;
    Ok(())
}
