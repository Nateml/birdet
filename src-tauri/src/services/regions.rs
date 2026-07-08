use crate::db::Db;
use crate::services::settings;
use anyhow::{anyhow, Result};
use serde::Deserialize;
use sqlx::Row;
use std::collections::HashSet;
use std::time::Duration;

const EBIRD_BASE: &str = "https://api.ebird.org/v2";

/// GET an eBird endpoint, retrying on 429 (rate limit) and transport errors.
/// eBird enforces a burst cap (25 req / 5s) and an hourly cap (500 req / hr);
/// on 429 it sends `Retry-After`. Honour it, else back off, up to 4 tries.
/// Returns None if it never succeeds — callers treat that as an empty result.
async fn ebird_get(client: &reqwest::Client, key: &str, url: &str) -> Option<reqwest::Response> {
    for attempt in 0..4u32 {
        match client.get(url).header("X-eBirdApiToken", key).send().await {
            Ok(r) if r.status() == reqwest::StatusCode::TOO_MANY_REQUESTS => {
                let wait = r
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(5)
                    .clamp(1, 10);
                tokio::time::sleep(Duration::from_secs(wait)).await;
            }
            Ok(r) => return Some(r),
            Err(_) => tokio::time::sleep(Duration::from_millis(400 * (attempt as u64 + 1))).await,
        }
    }
    None
}

/// Percent-encode a URL path/query segment (unreserved set kept as-is).
pub(crate) fn enc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

pub fn build_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent("birdet/0.1")
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .build()?)
}

pub async fn ebird_key(db: &Db) -> Result<String> {
    settings::get_setting(db, "ebird_api_key")
        .await?
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| anyhow!("Missing eBird API token — add it in Settings."))
}

#[derive(Deserialize)]
struct EbirdRegion {
    code: String,
    name: String,
}

/// List the sub-regions of a parent for the region picker, caching the eBird
/// response so repeat opens are offline. `level` ∈ country | subnational1 |
/// subnational2; `parent` is 'world' for countries, else a region code.
pub async fn list_regions(
    db: &Db,
    level: &str,
    parent: &str,
) -> Result<Vec<crate::commands::RegionItem>> {
    // Cache hit?
    let cached = sqlx::query("SELECT code, name FROM regions WHERE parent = ?1 AND level = ?2 ORDER BY name")
        .bind(parent)
        .bind(level)
        .fetch_all(&db.0)
        .await?;
    if !cached.is_empty() {
        return Ok(cached
            .into_iter()
            .map(|r| crate::commands::RegionItem { code: r.get("code"), name: r.get("name") })
            .collect());
    }

    let key = ebird_key(db).await?;
    let client = build_client()?;
    let items: Vec<EbirdRegion> = client
        .get(format!("{}/ref/region/list/{}/{}?fmt=json", EBIRD_BASE, enc(level), enc(parent)))
        .header("X-eBirdApiToken", &key)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| anyhow!("eBird region list failed: {}", e))?
        .json()
        .await?;

    for it in &items {
        sqlx::query("INSERT OR IGNORE INTO regions (code, name, parent, level) VALUES (?1, ?2, ?3, ?4)")
            .bind(&it.code)
            .bind(&it.name)
            .bind(parent)
            .bind(level)
            .execute(&db.0)
            .await?;
    }

    Ok(items
        .into_iter()
        .map(|it| crate::commands::RegionItem { code: it.code, name: it.name })
        .collect())
}

/// eBird species-code list for a region (every species ever recorded there).
/// The endpoint resolves the region hierarchy itself, so any code level works.
pub async fn fetch_spplist(client: &reqwest::Client, key: &str, region: &str) -> Result<Vec<String>> {
    let codes: Vec<String> = client
        .get(format!("{}/product/spplist/{}", EBIRD_BASE, enc(region)))
        .header("X-eBirdApiToken", key)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| anyhow!("eBird species list failed (check region code): {}", e))?
        .json()
        .await?;
    Ok(codes)
}

/// Twelve mid-month sample dates spanning the last year, as `y/m/d` path
/// segments for the dated checklist endpoint. Uses SQLite for the calendar
/// math (no Rust date deps).
pub async fn year_sample_dates(db: &Db) -> Result<Vec<String>> {
    let rows = sqlx::query_scalar::<_, String>(
        r#"WITH RECURSIVE m(n) AS (SELECT 0 UNION ALL SELECT n+1 FROM m WHERE n<11)
           SELECT strftime('%Y/%m/15', date('now','start of month','-'||n||' months')) FROM m"#,
    )
    .fetch_all(&db.0)
    .await?;
    Ok(rows)
}

/// Checklist submission IDs for a region. `date` is an optional `y/m/d` path
/// segment — `None` uses the recent-checklists endpoint, `Some` the dated one.
/// eBird caps `maxResults` at 200. Tolerant: returns [] on any error so callers
/// can fall back to taxonomic order.
pub async fn checklist_subids(
    client: &reqwest::Client,
    key: &str,
    region: &str,
    date: Option<&str>,
    max: usize,
) -> Vec<String> {
    #[derive(Deserialize)]
    struct ListItem {
        #[serde(rename = "subId")]
        sub_id: String,
    }
    let path = match date {
        Some(d) => format!("{}/{}", enc(region), d),
        None => enc(region),
    };
    let url = format!("{}/product/lists/{}?maxResults={}", EBIRD_BASE, path, max.clamp(1, 200));
    let items: Vec<ListItem> = match ebird_get(client, key, &url).await {
        Some(r) => r.json().await.unwrap_or_default(),
        None => Vec::new(),
    };
    items.into_iter().map(|i| i.sub_id).collect()
}

/// Species codes reported on a single checklist. Tolerant: [] on error.
pub async fn checklist_species(client: &reqwest::Client, key: &str, sub_id: &str) -> Vec<String> {
    #[derive(Deserialize)]
    struct Obs {
        #[serde(rename = "speciesCode")]
        species_code: String,
    }
    #[derive(Deserialize, Default)]
    struct Checklist {
        #[serde(default)]
        obs: Vec<Obs>,
    }
    let url = format!("{}/product/checklist/view/{}", EBIRD_BASE, enc(sub_id));
    let cl: Checklist = match ebird_get(client, key, &url).await {
        Some(r) => r.json().await.unwrap_or_default(),
        None => Checklist::default(),
    };
    cl.obs.into_iter().map(|o| o.species_code).collect()
}

/// Local birds whose eBird range includes `region` (spplist ∩ library),
/// optionally narrowed to a family. Region membership comes from eBird, not
/// from how the bird was imported.
pub async fn birds_in_region(
    db: &Db,
    region: &str,
    family: Option<&str>,
) -> Result<Vec<i64>> {
    let key = ebird_key(db).await?;
    let client = build_client()?;
    let codes: HashSet<String> = fetch_spplist(&client, &key, region).await?.into_iter().collect();

    let rows = sqlx::query("SELECT id, ebird_code, family FROM birds WHERE ebird_code IS NOT NULL")
        .fetch_all(&db.0)
        .await?;
    let fam = family.map(str::to_lowercase);
    let ids = rows
        .into_iter()
        .filter(|r| {
            let code: String = r.get("ebird_code");
            if !codes.contains(&code) {
                return false;
            }
            match &fam {
                Some(f) => r
                    .get::<Option<String>, _>("family")
                    .map(|bf| bf.to_lowercase().contains(f.as_str()))
                    .unwrap_or(false),
                None => true,
            }
        })
        .map(|r| r.get::<i64, _>("id"))
        .collect();
    Ok(ids)
}
