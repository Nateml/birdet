//! Species blurbs: what a bird looks like, where it lives, how it behaves, and
//! how its voice is usually described.
//!
//! The library is user-built from the ~11k eBird taxa, so this text can't be
//! hand-authored — it's fetched per species and cached in `bird_info`, then read
//! offline. Two free, key-less sources:
//!
//! * **iNaturalist** (`/v1/taxa`) resolves a scientific name to its Wikipedia
//!   article, IUCN status and a licensed photo.
//! * **Wikipedia** (`action=query&prop=extracts&explaintext`) supplies the prose,
//!   which we split on its `== Heading ==` lines into our fields.
//!
//! Wikipedia text is CC BY-SA 4.0: `source_url` + `license` are stored alongside
//! it and must be shown wherever the text is.

use crate::db::Db;
use crate::services::import::ImportProgress;
use crate::services::regions::enc;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const INAT_TAXA: &str = "https://api.inaturalist.org/v1/taxa";
const WIKI_API: &str = "https://en.wikipedia.org/w/api.php";
const WIKI_LICENSE: &str = "CC BY-SA 4.0";

/// Wikimedia's UA policy wants a descriptive agent with a way to reach us, and
/// iNaturalist asks the same. Anonymous defaults get rate-limited harder.
const UA: &str = concat!(
    "birdet/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/Nateml/birdet)"
);

/// Per-field cap. Wikipedia sections run long; a couple of paragraphs is all the
/// detail pane wants, and it keeps the DB from bloating on verbose species.
const MAX_FIELD: usize = 1400;

/// Politeness gap between species during a backfill. iNaturalist asks for ≤60
/// requests/minute and we make two calls per bird, so pace at roughly one bird
/// per 1.2s rather than racing.
const BACKFILL_INTERVAL: Duration = Duration::from_millis(1200);

/// One notes run at a time, process-wide. Both entry points are long, paced and
/// network-bound, and the UI can't enforce this on its own: navigating away from
/// Settings destroys the component that was tracking the run, so without this a
/// second click would start a parallel loop and double the request rate against
/// iNaturalist's ~60/min.
static BACKFILL_RUNNING: AtomicBool = AtomicBool::new(false);

/// Held for the length of a run; clears the flag however the run ends, panics
/// and early returns included.
struct RunGuard;

impl RunGuard {
    /// `None` when a run is already in flight.
    fn acquire() -> Option<RunGuard> {
        BACKFILL_RUNNING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
            .then_some(RunGuard)
    }
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        BACKFILL_RUNNING.store(false, Ordering::Release);
    }
}

/// Is a species-notes run in flight? Lets the frontend recover its progress bar
/// after a reload rather than showing an idle button over a running job.
pub fn backfill_running() -> bool {
    BACKFILL_RUNNING.load(Ordering::Acquire)
}

fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(UA)
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(30))
        .build()?)
}

fn emit(app: &AppHandle, message: impl Into<String>, current: i64, total: i64) {
    let _ = app.emit(
        "import://progress",
        ImportProgress { stage: "info".into(), message: message.into(), current, total },
    );
}

// --- Public shape --------------------------------------------------------

/// One species' cached blurb. Every text field is optional: a species may have
/// no Wikipedia article, or an article with no "Voice" section.
#[derive(Serialize, Default, Clone)]
pub struct BirdInfo {
    pub bird_id: i64,
    pub summary: Option<String>,
    pub appearance: Option<String>,
    pub habitat: Option<String>,
    pub behaviour: Option<String>,
    pub voice: Option<String>,
    pub conservation: Option<String>,
    pub image_url: Option<String>,
    /// Filename under `app_data_dir/images/`, once the photo is downloaded.
    pub image_path: Option<String>,
    pub image_credit: Option<String>,
    pub image_license: Option<String>,
    /// Deed for `image_license`, and the photo's own page — both required to
    /// credit a CC image properly.
    pub image_license_url: Option<String>,
    pub image_source_url: Option<String>,
    pub source: Option<String>,
    pub source_url: Option<String>,
    pub license: Option<String>,
    pub fetched_at: Option<String>,
    /// The user's own field notes. Never overwritten by a refetch.
    pub user_notes: Option<String>,
}

pub async fn get_bird_info(db: &Db, bird_id: i64) -> Result<Option<BirdInfo>> {
    let row = sqlx::query(
        r#"SELECT bird_id, summary, appearance, habitat, behaviour, voice, conservation,
                  image_url, image_path, image_credit, image_license, image_license_url,
                  image_source_url, source, source_url, license, fetched_at, user_notes
           FROM bird_info WHERE bird_id = ?"#,
    )
    .bind(bird_id)
    .fetch_optional(&db.0)
    .await?;

    Ok(row.map(|r| BirdInfo {
        bird_id: r.get("bird_id"),
        summary: r.get("summary"),
        appearance: r.get("appearance"),
        habitat: r.get("habitat"),
        behaviour: r.get("behaviour"),
        voice: r.get("voice"),
        conservation: r.get("conservation"),
        image_url: r.get("image_url"),
        image_path: r.get("image_path"),
        image_credit: r.get("image_credit"),
        image_license: r.get("image_license"),
        image_license_url: r.get("image_license_url"),
        image_source_url: r.get("image_source_url"),
        source: r.get("source"),
        source_url: r.get("source_url"),
        license: r.get("license"),
        fetched_at: r.get("fetched_at"),
        user_notes: r.get("user_notes"),
    }))
}

/// Save (or clear) the user's own notes without disturbing the fetched text.
pub async fn set_bird_notes(db: &Db, bird_id: i64, notes: Option<String>) -> Result<()> {
    let notes = notes.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
    sqlx::query(
        r#"INSERT INTO bird_info (bird_id, user_notes) VALUES (?, ?)
           ON CONFLICT(bird_id) DO UPDATE SET user_notes = excluded.user_notes"#,
    )
    .bind(bird_id)
    .bind(&notes)
    .execute(&db.0)
    .await?;
    Ok(())
}

/// Fetch and cache one species' blurb. Returns true if any text was stored.
/// Unless `force`, a bird that already has a `fetched_at` is left alone — a
/// previous run that found nothing shouldn't re-hammer the APIs on every visit.
pub async fn fetch_bird_info(app: &AppHandle, db: &Db, bird_id: i64, force: bool) -> Result<bool> {
    if !force {
        let done: Option<String> =
            sqlx::query_scalar("SELECT fetched_at FROM bird_info WHERE bird_id = ?")
                .bind(bird_id)
                .fetch_optional(&db.0)
                .await?
                .flatten();
        if done.is_some() {
            return Ok(false);
        }
    }

    let row = sqlx::query("SELECT common_name, scientific_name FROM birds WHERE ID = ?")
        .bind(bird_id)
        .fetch_optional(&db.0)
        .await?
        .ok_or_else(|| anyhow!("bird {} not found", bird_id))?;
    let common: String = row.get("common_name");
    let scientific: String = row.get("scientific_name");

    let client = client()?;
    let mut fetched = lookup(&client, &common, &scientific).await;
    fetched.image_path = save_image(app, &client, bird_id, fetched.image_url.as_deref()).await;
    store(db, bird_id, &fetched).await?;
    Ok(fetched.has_text())
}

/// Fill in every bird that has no blurb yet (or all of them, with `force`).
/// Runs sequentially and paced; per-bird failures are tolerated and simply leave
/// that species blank. Returns how many birds gained text.
pub async fn backfill_bird_info(app: &AppHandle, db: &Db, force: bool) -> Result<i64> {
    let Some(_guard) = RunGuard::acquire() else {
        return Err(anyhow!("A species-notes lookup is already running — let it finish first."));
    };
    let sql = if force {
        "SELECT b.ID FROM birds b ORDER BY b.common_name"
    } else {
        r#"SELECT b.ID FROM birds b
           LEFT JOIN bird_info i ON i.bird_id = b.ID
           WHERE i.bird_id IS NULL OR i.fetched_at IS NULL
           ORDER BY b.common_name"#
    };
    let ids: Vec<i64> = sqlx::query_scalar(sql).fetch_all(&db.0).await?;

    let total = ids.len() as i64;
    if total == 0 {
        return Ok(0);
    }
    emit(app, format!("Looking up {} species…", total), 0, total);

    let client = client()?;
    let mut filled = 0i64;
    for (i, bird_id) in ids.into_iter().enumerate() {
        let current = i as i64 + 1;
        if i > 0 {
            tokio::time::sleep(BACKFILL_INTERVAL).await;
        }

        let Some(row) = sqlx::query("SELECT common_name, scientific_name FROM birds WHERE ID = ?")
            .bind(bird_id)
            .fetch_optional(&db.0)
            .await?
        else {
            continue; // deleted mid-run
        };
        let common: String = row.get("common_name");
        let scientific: String = row.get("scientific_name");
        emit(app, format!("{} ({}/{})", common, current, total), current, total);

        let mut fetched = lookup(&client, &common, &scientific).await;
        fetched.image_path = save_image(app, &client, bird_id, fetched.image_url.as_deref()).await;
        if fetched.has_text() {
            filled += 1;
        }
        // Store even an empty result: `fetched_at` marks the species as tried so
        // the next backfill skips it. `force` is the way to retry.
        if let Err(e) = store(db, bird_id, &fetched).await {
            emit(app, format!("skip {}: {}", common, e), current, total);
        }
    }

    emit(app, format!("Added notes for {} of {} species.", filled, total), total, total);
    Ok(filled)
}

/// Fill in blurbs for a specific set of birds without emitting progress.
/// Used by the import pipeline, which spawns this detached so a slow Wikipedia
/// doesn't hold up the import summary. Per-bird failures are swallowed: the
/// species simply has no notes until the next backfill.
pub async fn fetch_many_quiet(app: &AppHandle, db: &Db, bird_ids: &[i64]) {
    // A backfill already covers every bird without notes, these included, so
    // stepping aside costs nothing but a delay.
    let Some(_guard) = RunGuard::acquire() else { return };
    let Ok(client) = client() else { return };
    for (i, &bird_id) in bird_ids.iter().enumerate() {
        if i > 0 {
            tokio::time::sleep(BACKFILL_INTERVAL).await;
        }
        let row = sqlx::query("SELECT common_name, scientific_name FROM birds WHERE ID = ?")
            .bind(bird_id)
            .fetch_optional(&db.0)
            .await;
        let Ok(Some(row)) = row else { continue };
        let common: String = row.get("common_name");
        let scientific: String = row.get("scientific_name");

        let mut fetched = lookup(&client, &common, &scientific).await;
        fetched.image_path = save_image(app, &client, bird_id, fetched.image_url.as_deref()).await;
        let _ = store(db, bird_id, &fetched).await;
    }
}

/// Ceiling on a downloaded photo. iNaturalist "medium" images are ~50-150 KB;
/// anything near this is a redirect to something we didn't ask for.
const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;

/// Download the species photo into `app_data_dir/images/` so it shows offline.
/// Returns the filename to store, or None if there's no usable image — a
/// missing picture is never a reason to fail the whole lookup.
async fn save_image(
    app: &AppHandle,
    client: &reqwest::Client,
    bird_id: i64,
    url: Option<&str>,
) -> Option<String> {
    let url = url?;
    let dir = app.path().app_data_dir().ok()?.join("images");
    std::fs::create_dir_all(&dir).ok()?;

    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    // Trust the served type, not the URL: a redirect to an HTML error page would
    // otherwise be written out as a ".jpg" that no <img> can decode.
    let ctype = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !ctype.starts_with("image/") {
        return None;
    }
    let ext = if ctype.contains("png") {
        "png"
    } else if ctype.contains("webp") {
        "webp"
    } else {
        "jpg"
    };
    if resp.content_length().is_some_and(|len| len > MAX_IMAGE_BYTES) {
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    // Re-check: the server may have lied about (or omitted) Content-Length.
    if bytes.is_empty() || bytes.len() as u64 > MAX_IMAGE_BYTES {
        return None;
    }

    let name = format!("{}.{}", bird_id, ext);
    std::fs::write(dir.join(&name), &bytes).ok()?;
    Some(name)
}

// --- Fetch pipeline ------------------------------------------------------

#[derive(Default)]
struct Fetched {
    summary: Option<String>,
    appearance: Option<String>,
    habitat: Option<String>,
    behaviour: Option<String>,
    voice: Option<String>,
    conservation: Option<String>,
    image_url: Option<String>,
    image_path: Option<String>,
    image_credit: Option<String>,
    image_license: Option<String>,
    image_license_url: Option<String>,
    image_source_url: Option<String>,
    source_url: Option<String>,
}

impl Fetched {
    fn has_text(&self) -> bool {
        self.summary.is_some()
            || self.appearance.is_some()
            || self.habitat.is_some()
            || self.behaviour.is_some()
            || self.voice.is_some()
    }
}

/// iNaturalist first (it names the right Wikipedia article and adds status +
/// photo), then Wikipedia for the prose. Either leg may fail; whatever the other
/// found is still kept.
async fn lookup(client: &reqwest::Client, common: &str, scientific: &str) -> Fetched {
    let mut out = Fetched::default();

    let taxon = inat_taxon(client, scientific).await;
    if let Some(t) = &taxon {
        out.conservation = t
            .conservation_status
            .as_ref()
            .and_then(|c| c.status_name.clone())
            .map(|s| title_case(&s))
            .filter(|s| !s.is_empty());
        if let Some(p) = &t.default_photo {
            // Only surface a photo we're allowed to reuse. iNat marks
            // all-rights-reserved images with a null/empty licence code.
            if p.license_code.as_deref().is_some_and(|c| c.starts_with("cc")) {
                out.image_url = p.medium_url.clone();
                out.image_credit = p.attribution.clone();
                out.image_license = p.license_code.clone().map(|c| c.to_uppercase());
                out.image_license_url = p.license_code.as_deref().and_then(cc_deed_url);
                out.image_source_url =
                    p.id.map(|id| format!("https://www.inaturalist.org/photos/{}", id));
            }
        }
        out.summary = t
            .wikipedia_summary
            .as_deref()
            .map(strip_html)
            .map(|s| clamp_text(&s, MAX_FIELD))
            .filter(|s| !s.is_empty());
    }

    // Prefer the article iNaturalist points at; else let Wikipedia's redirects
    // resolve the binomial, and only then guess at the common name.
    let wiki_title = taxon
        .as_ref()
        .and_then(|t| t.wikipedia_url.as_deref())
        .and_then(wiki_title_from_url);
    let mut candidates: Vec<(String, bool)> = Vec::new();
    if let Some(t) = wiki_title {
        candidates.push((t, false));
    }
    candidates.push((scientific.to_string(), false));
    candidates.push((common.to_string(), true)); // needs a sanity check

    for (title, verify) in candidates {
        let Some(page) = wiki_extract(client, &title).await else {
            continue;
        };
        // A common-name title can land on an unrelated article ("Sanderling" is
        // safe, but plenty of bird names are also places or people). Require the
        // text to look like it's about this species before trusting it.
        if verify && !looks_like_species(&page.extract, scientific) {
            continue;
        }
        // iNaturalist files a good many species under an all-rights-reserved
        // default photo, which leaves the detail pane blank. The article's own
        // lead image is Commons-licensed, so fall back to it.
        if out.image_url.is_none() {
            if let Some((src, file)) = page.image.clone() {
                // Only take the file once its licence checks out — an image we
                // can't credit properly is worse than none.
                if let Some(credit) = commons_credit(client, &file).await {
                    out.image_url = Some(src);
                    out.image_credit = credit.artist;
                    out.image_license = credit.license;
                    out.image_license_url = credit.license_url;
                    out.image_source_url = Some(format!(
                        "https://commons.wikimedia.org/wiki/File:{}",
                        enc(&file)
                    ));
                }
            }
        }
        apply_wikipedia(&mut out, &page);
        break;
    }

    out
}

struct WikiPageText {
    title: String,
    extract: String,
    /// The article's lead photo, as (image URL, Commons file title). Only a
    /// fallback: a CC-licensed iNaturalist photo is preferred when there is one.
    image: Option<(String, String)>,
}

fn apply_wikipedia(out: &mut Fetched, page: &WikiPageText) {
    let (lead, sections) = split_sections(&page.extract);
    if !lead.is_empty() {
        out.summary = Some(clamp_text(&lead, MAX_FIELD));
    }
    for sec in sections {
        // A sub-heading usually names the field itself ("Calls and song"); when
        // it doesn't ("Subspecies"), it belongs to its enclosing section.
        let mut fields = classify(&sec.heading);
        if fields.is_empty() {
            fields = sec.parent.as_deref().map(classify).unwrap_or_default();
        }
        match (fields.voice, fields.other) {
            // A heading can name voice *and* something else ("Diet, feeding and
            // call"). Handing the whole body to voice buries the one line about
            // the call under a page of hunting behaviour, so split by sentence
            // and give each half its own field.
            (true, Some(other)) => {
                let (voice, rest) = partition_voice(&sec.body);
                if let Some(voice) = voice {
                    push_field(out, Field::Voice, voice);
                }
                // With no voice sentence found the heading over-promised, and
                // `rest` is the untouched body — it all goes to the other field,
                // leaving voice to the whole-article fallback below.
                if let Some(rest) = rest {
                    push_field(out, other, rest);
                }
            }
            (true, None) => push_field(out, Field::Voice, sec.body),
            (false, Some(other)) => push_field(out, other, sec.body),
            (false, None) => {}
        }
    }
    // Most bird articles have no "Voice" heading — the call description sits in
    // the middle of Description or Behaviour prose. Voice is the field an
    // ear-trainer most wants, so go looking for it sentence by sentence.
    if out.voice.is_none() {
        out.voice = voice_sentences(&page.extract);
    }

    out.source_url = Some(format!(
        "https://en.wikipedia.org/wiki/{}",
        enc(&page.title.replace(' ', "_"))
    ));
}

async fn store(db: &Db, bird_id: i64, f: &Fetched) -> Result<()> {
    let source = if f.has_text() { Some("wikipedia") } else { None };
    let license = f.source_url.as_ref().map(|_| WIKI_LICENSE);
    sqlx::query(
        r#"INSERT INTO bird_info
             (bird_id, summary, appearance, habitat, behaviour, voice, conservation,
              image_url, image_path, image_credit, image_license, image_license_url,
              image_source_url, source, source_url, license, fetched_at)
           VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,CURRENT_TIMESTAMP)
           ON CONFLICT(bird_id) DO UPDATE SET
             summary       = excluded.summary,
             appearance    = excluded.appearance,
             habitat       = excluded.habitat,
             behaviour     = excluded.behaviour,
             voice         = excluded.voice,
             conservation  = excluded.conservation,
             image_url     = excluded.image_url,
             image_path    = excluded.image_path,
             image_credit  = excluded.image_credit,
             image_license = excluded.image_license,
             image_license_url = excluded.image_license_url,
             image_source_url  = excluded.image_source_url,
             source        = excluded.source,
             source_url    = excluded.source_url,
             license       = excluded.license,
             fetched_at    = excluded.fetched_at"#,
    )
    .bind(bird_id)
    .bind(&f.summary)
    .bind(&f.appearance)
    .bind(&f.habitat)
    .bind(&f.behaviour)
    .bind(&f.voice)
    .bind(&f.conservation)
    .bind(&f.image_url)
    .bind(&f.image_path)
    .bind(&f.image_credit)
    .bind(&f.image_license)
    .bind(&f.image_license_url)
    .bind(&f.image_source_url)
    .bind(source)
    .bind(&f.source_url)
    .bind(license)
    .execute(&db.0)
    .await?;
    Ok(())
}

// --- iNaturalist ---------------------------------------------------------

#[derive(Deserialize)]
struct InatResp {
    #[serde(default)]
    results: Vec<InatTaxon>,
}

#[derive(Deserialize)]
struct InatTaxon {
    name: Option<String>,
    rank: Option<String>,
    wikipedia_url: Option<String>,
    wikipedia_summary: Option<String>,
    conservation_status: Option<InatConservation>,
    default_photo: Option<InatPhoto>,
}

#[derive(Deserialize)]
struct InatConservation {
    status_name: Option<String>,
}

#[derive(Deserialize)]
struct InatPhoto {
    id: Option<i64>,
    medium_url: Option<String>,
    attribution: Option<String>,
    license_code: Option<String>,
}

/// Map an iNaturalist licence code (`cc-by-nc`, `cc0`, …) to its deed. iNat
/// applies the 4.0 suite, and CC0 lives under `publicdomain/` rather than
/// `licenses/`. Anything unrecognised gets no link rather than a guessed one.
fn cc_deed_url(code: &str) -> Option<String> {
    let code = code.trim().to_ascii_lowercase();
    if code == "cc0" {
        return Some("https://creativecommons.org/publicdomain/zero/1.0/".into());
    }
    let rest = code.strip_prefix("cc-by")?;
    let suffix = rest.trim_start_matches('-');
    if !suffix.is_empty() && !suffix.split('-').all(|p| matches!(p, "nc" | "sa" | "nd")) {
        return None;
    }
    let slug = if suffix.is_empty() { "by".to_string() } else { format!("by-{}", suffix) };
    Some(format!("https://creativecommons.org/licenses/{}/4.0/", slug))
}

async fn inat_taxon(client: &reqwest::Client, scientific: &str) -> Option<InatTaxon> {
    let url = format!("{}?q={}&rank=species&per_page=5", INAT_TAXA, enc(scientific));
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: InatResp = resp.json().await.ok()?;

    // iNat's search is fuzzy, so prefer an exact binomial match and only fall
    // back to the top species hit.
    let target = scientific.to_lowercase();
    body.results
        .iter()
        .position(|t| t.name.as_deref().map(str::to_lowercase).as_deref() == Some(target.as_str()))
        .or_else(|| {
            body.results
                .iter()
                .position(|t| t.rank.as_deref() == Some("species"))
        })
        .map(|i| body.results.into_iter().nth(i).expect("index from position"))
}

// --- Wikipedia -----------------------------------------------------------

#[derive(Deserialize)]
struct WikiResp {
    query: Option<WikiQuery>,
}

#[derive(Deserialize)]
struct WikiQuery {
    #[serde(default)]
    pages: Vec<WikiPage>,
}

#[derive(Deserialize)]
struct WikiPage {
    title: Option<String>,
    extract: Option<String>,
    /// `piprop=original` — the lead photo at full size.
    original: Option<WikiImage>,
    /// `piprop=name` — that photo's file name, for the licence lookup.
    pageimage: Option<String>,
}

#[derive(Deserialize)]
struct WikiImage {
    source: Option<String>,
}

/// One file's `extmetadata`, which is where Commons keeps the credit line.
#[derive(Deserialize)]
struct ImageInfoResp {
    query: Option<ImageInfoQuery>,
}

#[derive(Deserialize)]
struct ImageInfoQuery {
    #[serde(default)]
    pages: Vec<ImageInfoPage>,
}

#[derive(Deserialize)]
struct ImageInfoPage {
    #[serde(default)]
    imageinfo: Vec<ImageInfo>,
}

#[derive(Deserialize)]
struct ImageInfo {
    extmetadata: Option<ExtMetadata>,
}

#[derive(Deserialize)]
struct ExtMetadata {
    #[serde(rename = "Artist")]
    artist: Option<MetaValue>,
    #[serde(rename = "LicenseShortName")]
    license: Option<MetaValue>,
    /// Machine-readable code — `cc-by-sa-4.0`, `pd`, `gfdl`, …
    #[serde(rename = "License")]
    license_code: Option<MetaValue>,
    #[serde(rename = "LicenseUrl")]
    license_url: Option<MetaValue>,
}

#[derive(Deserialize)]
struct MetaValue {
    value: Option<String>,
}

async fn wiki_extract(client: &reqwest::Client, title: &str) -> Option<WikiPageText> {
    let title = title.trim();
    if title.is_empty() {
        return None;
    }
    // `redirects=1` follows a binomial to the common-name article;
    // `formatversion=2` makes `pages` a plain array instead of a keyed object.
    // `pageimages` rides along on the same request: it costs nothing extra and
    // covers the species whose iNaturalist photo is all-rights-reserved.
    let url = format!(
        "{}?action=query&format=json&formatversion=2&redirects=1\
         &prop=extracts%7Cpageimages&explaintext=1&piprop=original%7Cname&titles={}",
        WIKI_API,
        enc(title)
    );
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: WikiResp = resp.json().await.ok()?;
    let page = body.query?.pages.into_iter().next()?;
    let image = page
        .original
        .as_ref()
        .and_then(|i| i.source.as_deref())
        // Drop the analytics query the API appends, and take only Commons-hosted
        // files: en-wiki also serves non-free fair-use images from /wikipedia/en/,
        // which can't be redistributed in the app.
        .map(|src| src.split('?').next().unwrap_or(src).to_string())
        .filter(|src| src.contains("/wikipedia/commons/"))
        .zip(page.pageimage.clone());

    let extract = page.extract?;
    if extract.trim().is_empty() {
        return None;
    }
    Some(WikiPageText {
        title: page.title.unwrap_or_else(|| title.to_string()),
        extract,
        image,
    })
}

/// One Commons file's credit: who made it, under what licence, and where to
/// read that licence.
#[derive(Default)]
struct CommonsCredit {
    artist: Option<String>,
    license: Option<String>,
    license_url: Option<String>,
}

/// Licence codes we're willing to ship a file under. Commons only hosts free
/// media, but "free" there includes GFDL-only uploads, which oblige us to
/// distribute the full licence text with the work — not worth it for a stock
/// photo, so take CC and public-domain files and skip the rest.
fn is_shippable_license(code: &str) -> bool {
    let c = code.trim().to_ascii_lowercase();
    c.starts_with("cc") || c == "pd" || c.contains("public domain") || c.starts_with("pd-")
}

/// Look up a Commons file's credit line. Costs one extra request, so it only
/// runs for the species that reached this point without a usable photo.
async fn commons_credit(client: &reqwest::Client, file: &str) -> Option<CommonsCredit> {
    let url = format!(
        "{}?action=query&format=json&formatversion=2&prop=imageinfo&iiprop=extmetadata&titles=File:{}",
        WIKI_API,
        enc(file)
    );
    let meta = async {
        let resp = client.get(&url).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let body: ImageInfoResp = resp.json().await.ok()?;
        body.query?.pages.into_iter().next()?.imageinfo.into_iter().next()?.extmetadata
    }
    .await;

    let meta = meta?;
    // The fields arrive as HTML — the artist is usually a link to a user page.
    let text = |m: Option<MetaValue>| {
        m.and_then(|v| v.value)
            .map(|v| strip_html(&v))
            .map(|v| clamp_text(&v, 200))
            .filter(|v| !v.is_empty())
    };
    let license = text(meta.license);
    let code = text(meta.license_code);
    // No licence at all, or one we can't ship: leave the species without a
    // photo rather than guess.
    match code.as_deref().or(license.as_deref()) {
        Some(c) if is_shippable_license(c) => {}
        _ => return None,
    }
    Some(CommonsCredit {
        artist: text(meta.artist),
        license,
        license_url: text(meta.license_url).filter(|u| u.starts_with("http")),
    })
}

/// Pull the article title out of an iNaturalist `wikipedia_url`.
fn wiki_title_from_url(url: &str) -> Option<String> {
    let seg = url.rsplit('/').next()?;
    let seg = seg.split(['#', '?']).next()?;
    if seg.is_empty() {
        return None;
    }
    Some(pct_decode(seg).replace('_', " "))
}

/// Cheap guard against a common-name title landing on an unrelated article.
fn looks_like_species(extract: &str, scientific: &str) -> bool {
    let lower = extract.to_lowercase();
    lower.contains(&scientific.to_lowercase()) || lower.contains("bird")
}

// --- Section parsing -----------------------------------------------------

enum Field {
    Appearance,
    Habitat,
    Behaviour,
    Voice,
}

/// What one heading names. Voice is tracked separately from the rest because a
/// heading routinely names both at once — "Diet, feeding and call", "Behaviour
/// and vocalisations" — and such a section has to be split, not filed whole.
#[derive(Default)]
struct HeadingFields {
    voice: bool,
    other: Option<Field>,
}

impl HeadingFields {
    /// Nothing recognisable — the caller falls back to the enclosing heading.
    fn is_empty(&self) -> bool {
        !self.voice && self.other.is_none()
    }
}

/// Map a Wikipedia heading onto our fields.
fn classify(heading: &str) -> HeadingFields {
    let h = heading.to_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| h.contains(n));

    let other = if has(&["habitat", "distribution", "range"]) {
        Some(Field::Habitat)
    } else if has(&["description", "appearance", "plumage", "identification"]) {
        Some(Field::Appearance)
    } else if has(&[
        "behaviour", "behavior", "ecology", "diet", "feeding", "food", "breeding", "nesting",
        "reproduction",
    ]) {
        Some(Field::Behaviour)
    } else {
        None
    };
    HeadingFields { voice: has(&["vocal", "voice", "call", "song"]), other }
}

/// Append a section's prose to a field. Two sections can map to one field
/// ("Diet" and "Breeding" both land in behaviour) — join rather than clobber.
fn push_field(out: &mut Fetched, field: Field, text: String) {
    let slot = match field {
        Field::Appearance => &mut out.appearance,
        Field::Habitat => &mut out.habitat,
        Field::Behaviour => &mut out.behaviour,
        Field::Voice => &mut out.voice,
    };
    let merged = match slot.take() {
        Some(prev) => format!("{}\n\n{}", prev, text),
        None => text,
    };
    let merged = clamp_text(&merged, MAX_FIELD);
    *slot = if merged.is_empty() { None } else { Some(merged) };
}

/// One heading's prose, plus the level-2 heading enclosing it (if it was a
/// sub-section). Both are needed to classify: Wikipedia bird articles file
/// "Calls and song" as a sub-heading of "Description" about as often as they
/// give voice its own top-level section.
struct Section {
    heading: String,
    parent: Option<String>,
    body: String,
}

/// Split an `explaintext` extract into its lead and its headed sections.
/// `explaintext` renders headings as `== Heading ==` / `=== Sub ===` lines; the
/// text before the first heading is the lead.
fn split_sections(extract: &str) -> (String, Vec<Section>) {
    let mut lead = String::new();
    let mut sections: Vec<Section> = Vec::new();
    let mut current: Option<Section> = None;
    let mut parent: Option<String> = None;

    for line in extract.lines() {
        if let Some((level, heading)) = heading(line.trim()) {
            if let Some(sec) = current.take() {
                sections.push(sec);
            }
            let enclosing = if level >= 3 { parent.clone() } else { None };
            if level <= 2 {
                parent = Some(heading.clone());
            }
            current = Some(Section { heading, parent: enclosing, body: String::new() });
            continue;
        }
        let buf = match current.as_mut() {
            Some(sec) => &mut sec.body,
            None => &mut lead,
        };
        buf.push_str(line);
        buf.push('\n');
    }
    if let Some(sec) = current.take() {
        sections.push(sec);
    }

    let sections = sections
        .into_iter()
        .map(|s| Section { body: tidy(&s.body), ..s })
        .filter(|s| !s.body.is_empty())
        .collect();
    (tidy(&lead), sections)
}

/// `== Heading ==` → `(2, "Heading")`. Returns None for a non-heading line or
/// one whose `=` runs don't match.
fn heading(line: &str) -> Option<(usize, String)> {
    if !line.starts_with("==") || !line.ends_with("==") || line.len() < 5 {
        return None;
    }
    let open = line.bytes().take_while(|b| *b == b'=').count();
    let close = line.bytes().rev().take_while(|b| *b == b'=').count();
    if open != close || open * 2 >= line.len() {
        return None;
    }
    let text = line[open..line.len() - close].trim();
    if text.is_empty() {
        return None;
    }
    Some((open, text.to_string()))
}

/// Words that mark a sentence as describing how a species sounds. Split into
/// whole words and stems so "whistling"/"whistles" both hit. Note "called" is
/// deliberately absent — it almost always means naming, not vocalising.
const VOICE_WORDS: &[&str] = &[
    "call", "calls", "calling", "song", "songs", "sing", "sings", "singing", "coo", "coos",
    "cooing", "note", "notes", "duet", "duets", "piping", "utter", "utters", "uttered", "voice",
];
const VOICE_STEMS: &[&str] =
    &["whistl", "trill", "chirp", "chatter", "warbl", "vocali", "cackl", "screech", "hoot",
      "croak", "squawk", "twitter", "chirrup", "yelp", "wail"];

/// Sentences shorter than this are usually fragments from a bad split.
const MIN_SENTENCE: usize = 25;
/// Enough to carry the mnemonic without dragging in half the article.
const MAX_VOICE_SENTENCES: usize = 6;

/// Harvest the sentences of an extract that describe the bird's voice.
fn voice_sentences(extract: &str) -> Option<String> {
    let body: String = extract
        .lines()
        .filter(|l| !l.trim_start().starts_with("=="))
        .collect::<Vec<_>>()
        .join(" ");

    let picked: Vec<&str> = split_sentences(&body)
        .into_iter()
        .filter(|s| s.len() >= MIN_SENTENCE && mentions_voice(s))
        .take(MAX_VOICE_SENTENCES)
        .collect();

    if picked.is_empty() {
        return None;
    }
    Some(clamp_text(&picked.join(" "), MAX_FIELD))
}

/// Split a mixed section's prose ("Diet, feeding and call") into the sentences
/// that describe the bird's voice and everything else. Order is preserved, and
/// so are the remainder's paragraph breaks; either half is None when it came out
/// empty, so a heading that promised a call but delivered none hands its whole
/// body back as the remainder.
fn partition_voice(body: &str) -> (Option<String>, Option<String>) {
    let mut voice: Vec<&str> = Vec::new();
    let mut rest: Vec<String> = Vec::new();

    for para in body.split("\n\n") {
        let mut kept: Vec<&str> = Vec::new();
        for sentence in split_sentences(para) {
            if sentence.len() >= MIN_SENTENCE && mentions_voice(sentence) {
                voice.push(sentence);
            } else {
                kept.push(sentence);
            }
        }
        if !kept.is_empty() {
            rest.push(kept.join(" "));
        }
    }
    voice.truncate(MAX_VOICE_SENTENCES);

    (
        (!voice.is_empty()).then(|| voice.join(" ")),
        (!rest.is_empty()).then(|| rest.join("\n\n")),
    )
}

fn mentions_voice(sentence: &str) -> bool {
    sentence
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|w| !w.is_empty())
        .any(|w| {
            let w = w.to_lowercase();
            VOICE_WORDS.contains(&w.as_str()) || VOICE_STEMS.iter().any(|st| w.starts_with(st))
        })
}

/// Split prose into sentences on terminal punctuation followed by a space and a
/// capital. The capital check keeps "c. 20 cm" and "e.g." from splitting.
fn split_sentences(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if matches!(bytes[i], b'.' | b'!' | b'?') {
            let mut chars = text[i + 1..].chars().peekable();
            // Consume the whole whitespace run, not one space: `voice_sentences`
            // joins the extract's lines with a space, so a paragraph break lands
            // here as several. Testing only the next character would see a space
            // rather than a capital and glue the two paragraphs into one
            // "sentence" — which then matches on a single stray voice word.
            let mut spaced = false;
            while chars.peek().is_some_and(|c| c.is_whitespace()) {
                spaced = true;
                chars.next();
            }
            let breaks = match chars.next() {
                Some(next) => spaced && next.is_uppercase(),
                None => true, // end of text
            };
            if breaks {
                out.push(text[start..=i].trim());
                start = i + 1;
            }
        }
        i += 1;
    }
    if start < text.len() {
        out.push(text[start..].trim());
    }
    out.into_iter().filter(|s| !s.is_empty()).collect()
}

// --- Text helpers --------------------------------------------------------

/// Trim trailing blank lines and collapse runs of them into paragraph breaks.
fn tidy(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut blanks = 0;
    for line in s.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            blanks += 1;
            continue;
        }
        if !out.is_empty() {
            out.push_str(if blanks > 0 { "\n\n" } else { "\n" });
        }
        blanks = 0;
        out.push_str(line);
    }
    out
}

/// Cut to `max` bytes at the last sentence end (else the last space), so a
/// truncated blurb doesn't stop mid-word.
fn clamp_text(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.len() <= max {
        return s.to_string();
    }
    // Find a char boundary at or below `max` to slice at safely.
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    let head = &s[..end];
    let cut = head
        .rfind(['.', '!', '?'])
        .map(|i| i + 1)
        .filter(|i| *i > max / 2)
        .or_else(|| head.rfind(char::is_whitespace))
        .unwrap_or(end);
    format!("{}…", head[..cut].trim_end())
}

/// Strip tags from iNaturalist's HTML summary. Only used as a fallback — the
/// Wikipedia extract arrives as plain text.
fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    tidy(&out.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&#39;", "'").replace("&quot;", "\""))
}

fn pct_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// iNaturalist returns IUCN statuses in lower case ("least concern").
fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shape of a real bird article: voice hides in a level-3 sub-heading under
    /// "Description", and "Diet"/"Breeding" sit under "Behaviour".
    const EXTRACT: &str = "\
The Cape robin-chat is a small passerine bird. It has a disjunct range.

== Taxonomy ==
Named by Linnaeus in 1771.

=== Subspecies ===
Six subspecies are recognised.

== Description ==
Adults are 16-17 cm long with an orange breast.

=== Calls and song ===
The alarm call is a low guttural wa-dur-dra. The song is a rambling jumble.

== Distribution and habitat ==
Found in forest edge and gardens from South Sudan to South Africa.

== Behaviour ==
Usually solitary and territorial.

=== Food and feeding ===
Eats insects taken from the ground.

== References ==
Citations here.
";

    fn sections_by_field(extract: &str) -> Fetched {
        let mut out = Fetched::default();
        apply_wikipedia(
            &mut out,
            &WikiPageText {
                title: "Cape robin-chat".into(),
                extract: extract.into(),
                image: None,
            },
        );
        out
    }

    #[test]
    fn voice_subheading_beats_its_parent_section() {
        let f = sections_by_field(EXTRACT);
        let voice = f.voice.expect("voice");
        assert!(voice.contains("wa-dur-dra"), "got: {voice}");
        // The parent's own prose must stay in appearance, not follow voice out.
        let appearance = f.appearance.expect("appearance");
        assert!(appearance.contains("orange breast"));
        assert!(!appearance.contains("wa-dur-dra"));
    }

    #[test]
    fn subheadings_fold_into_their_parent_when_unclassified() {
        let f = sections_by_field(EXTRACT);
        let behaviour = f.behaviour.expect("behaviour");
        assert!(behaviour.contains("solitary"));
        assert!(behaviour.contains("insects")); // "Food and feeding"
        assert!(f.habitat.expect("habitat").contains("forest edge"));
        assert!(f.summary.expect("summary").contains("small passerine"));
    }

    #[test]
    fn references_and_taxonomy_are_dropped() {
        let f = sections_by_field(EXTRACT);
        for field in [&f.appearance, &f.habitat, &f.behaviour, &f.voice] {
            let t = field.as_deref().unwrap_or_default();
            assert!(!t.contains("Citations"), "references leaked: {t}");
            assert!(!t.contains("Linnaeus"), "taxonomy leaked: {t}");
        }
    }

    /// A heading that names two fields at once must not drag the other one's
    /// prose into voice — the rock kestrel's "Diet, feeding and call" is a page
    /// of hunting behaviour with a single sentence about the call.
    #[test]
    fn a_mixed_heading_splits_between_its_two_fields() {
        let extract = "\
Lead paragraph about the rock kestrel.

== Behavior ==
=== Diet, feeding and call ===
Rock kestrels feed on a wide variety of organisms, mostly invertebrates.
Hover hunting describes the method whereby the kestrel remains stationary in the air.
Their call is a harsh chay-chay-chay, unlike the common kestrel's kee-kee-kee.
";
        let f = sections_by_field(extract);
        let voice = f.voice.expect("voice");
        assert!(voice.contains("chay-chay-chay"), "got: {voice}");
        assert!(!voice.contains("Hover hunting"), "got: {voice}");

        let behaviour = f.behaviour.expect("behaviour");
        assert!(behaviour.contains("Hover hunting"));
        assert!(behaviour.contains("invertebrates"));
        assert!(!behaviour.contains("chay-chay-chay"));
    }

    /// When the split finds nothing that sounds like a call, the other field
    /// still gets the whole section rather than losing it.
    #[test]
    fn a_mixed_heading_with_no_call_prose_keeps_its_other_half() {
        let extract = "\
Lead paragraph.

== Diet, feeding and call ==
Rock kestrels feed on a wide variety of organisms, mostly invertebrates.
";
        let f = sections_by_field(extract);
        assert!(f.voice.is_none(), "got: {:?}", f.voice);
        assert!(f.behaviour.expect("behaviour").contains("invertebrates"));
    }

    /// The guard is what stops a second Settings visit starting a parallel run.
    #[test]
    fn only_one_notes_run_at_a_time() {
        let first = RunGuard::acquire().expect("first run starts");
        assert!(RunGuard::acquire().is_none(), "a second run must be refused");
        drop(first);
        assert!(RunGuard::acquire().is_some(), "the flag clears when a run ends");
    }

    #[test]
    fn cc_codes_map_to_their_deeds() {
        assert_eq!(
            cc_deed_url("cc-by-nc").as_deref(),
            Some("https://creativecommons.org/licenses/by-nc/4.0/")
        );
        assert_eq!(
            cc_deed_url("CC-BY").as_deref(),
            Some("https://creativecommons.org/licenses/by/4.0/")
        );
        assert_eq!(
            cc_deed_url("cc0").as_deref(),
            Some("https://creativecommons.org/publicdomain/zero/1.0/")
        );
        // Not a licence we know how to link — better no link than a wrong one.
        assert_eq!(cc_deed_url("c").as_deref(), None);
        assert_eq!(cc_deed_url("cc-by-weird").as_deref(), None);
    }

    #[test]
    fn only_cc_and_public_domain_commons_files_ship() {
        assert!(is_shippable_license("cc-by-sa-4.0"));
        assert!(is_shippable_license("PD"));
        assert!(is_shippable_license("Public domain"));
        // GFDL obliges us to ship the licence text with the file.
        assert!(!is_shippable_license("GFDL"));
        assert!(!is_shippable_license("attribution"));
    }

    /// A paragraph break reaches `voice_sentences` as a run of spaces. Splitting
    /// on only the first one glued the paragraphs together, so one call sentence
    /// dragged the description and range prose into voice with it.
    #[test]
    fn voice_fallback_splits_across_paragraph_breaks() {
        let prose = "\
Juveniles have less extensive violet on their ear coverts.

The call is a high-pitched insect-like tsip-tsip given in flight.

The African pygmy kingfisher is distributed widely south of the Sahara.
";
        let voice = voice_sentences(prose).expect("voice");
        assert!(voice.contains("tsip-tsip"), "got: {voice}");
        assert!(!voice.contains("ear coverts"), "got: {voice}");
        assert!(!voice.contains("Sahara"), "got: {voice}");
    }

    /// Most articles have no voice heading at all — the fallback has to find the
    /// call description in the middle of ordinary prose.
    #[test]
    fn voice_fallback_picks_call_sentences_out_of_prose() {
        let prose = "\
== Description ==
The Karoo prinia is 13 cm long and heavily streaked below. \
The calls of this species include a sharp chleet-chleet, and a fast buzzy tit-tit-tit. \
It is found in fynbos and scrub throughout the region.
";
        let voice = voice_sentences(prose).expect("voice");
        assert!(voice.contains("chleet-chleet"), "got: {voice}");
        assert!(!voice.contains("heavily streaked"));
        assert!(!voice.contains("fynbos"));
    }

    /// "called" means naming, not vocalising — the commonest false positive.
    #[test]
    fn voice_fallback_ignores_naming_sentences() {
        let prose = "The species is also called the Cape robin in older field guides.";
        assert!(voice_sentences(prose).is_none());
    }

    #[test]
    fn clamp_cuts_at_a_sentence_end() {
        let s = "One sentence here. Two sentence here. Three sentence here.";
        let out = clamp_text(s, 40);
        assert_eq!(out, "One sentence here. Two sentence here.…");
        assert_eq!(clamp_text("Short.", 40), "Short.");
    }

    #[test]
    fn wiki_title_is_decoded_from_the_inat_url() {
        assert_eq!(
            wiki_title_from_url("https://en.wikipedia.org/wiki/Cape_robin-chat"),
            Some("Cape robin-chat".to_string())
        );
        assert_eq!(
            wiki_title_from_url("https://en.wikipedia.org/wiki/Gurney%27s_sugarbird"),
            Some("Gurney's sugarbird".to_string())
        );
    }
}
