use crate::db::Db;
use crate::services::regions;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::Row;

const EBIRD_BASE: &str = "https://api.ebird.org/v2";

/// One eBird species. Shared by the taxonomy cache and the importer — both
/// deserialize the same eBird taxonomy JSON. `order` is the taxonomic order
/// name; it lands in `birds.taxon_order` on import.
#[derive(Deserialize, Debug, Default, Clone)]
pub struct Taxon {
    #[serde(rename = "speciesCode")]
    pub species_code: String,
    #[serde(rename = "comName")]
    pub com_name: String,
    #[serde(rename = "sciName")]
    pub sci_name: String,
    #[serde(rename = "familyComName")]
    pub family_com_name: Option<String>,
    #[serde(rename = "familySciName")]
    pub family_sci_name: Option<String>,
    #[serde(rename = "order")]
    pub order: Option<String>,
}

/// A search hit returned to the UI, flagged if already in the library.
#[derive(Serialize)]
pub struct SpeciesResult {
    pub ebird_code: String,
    pub common_name: String,
    pub scientific_name: String,
    pub family: Option<String>,
    pub in_library: bool,
}

/// Download + cache the full eBird species taxonomy once (~11k rows, ~3.5MB).
/// No-op if already populated. eBird has no name-search endpoint, so the whole
/// list is cached locally and searched with SQL.
pub async fn ensure(db: &Db) -> Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM taxonomy")
        .fetch_one(&db.0)
        .await?;
    if count > 0 {
        return Ok(());
    }

    let key = regions::ebird_key(db).await?;
    let client = regions::build_client()?;
    let taxa: Vec<Taxon> = client
        .get(format!("{}/ref/taxonomy/ebird?fmt=json&cat=species", EBIRD_BASE))
        .header("X-eBirdApiToken", &key)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let mut tx = db.0.begin().await?;
    for t in &taxa {
        sqlx::query(
            r#"INSERT OR IGNORE INTO taxonomy
               (ebird_code, common_name, scientific_name, family, family_sci, taxon_order)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
        )
        .bind(&t.species_code)
        .bind(&t.com_name)
        .bind(&t.sci_name)
        .bind(&t.family_com_name)
        .bind(&t.family_sci_name)
        .bind(&t.order)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Search cached taxonomy by common or scientific name. Prefix matches rank
/// first, then alphabetical. Populates the cache on first call.
pub async fn search(db: &Db, query: &str, limit: i64) -> Result<Vec<SpeciesResult>> {
    ensure(db).await?;
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Ok(vec![]);
    }
    let contains = format!("%{}%", q);
    let prefix = format!("{}%", q);
    let rows = sqlx::query(
        r#"SELECT t.ebird_code, t.common_name, t.scientific_name, t.family,
                  (SELECT 1 FROM birds b WHERE b.ebird_code = t.ebird_code) AS in_lib
           FROM taxonomy t
           WHERE lower(t.common_name) LIKE ?1 OR lower(t.scientific_name) LIKE ?1
           ORDER BY (lower(t.common_name) LIKE ?2) DESC, t.common_name
           LIMIT ?3"#,
    )
    .bind(&contains)
    .bind(&prefix)
    .bind(limit.clamp(1, 100))
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| SpeciesResult {
            ebird_code: r.get("ebird_code"),
            common_name: r.get("common_name"),
            scientific_name: r.get("scientific_name"),
            family: r.get("family"),
            in_library: r.get::<Option<i64>, _>("in_lib").is_some(),
        })
        .collect())
}

/// Load full `Taxon` records for the given eBird codes from the cache,
/// preserving input order. Ensures the cache is populated first.
pub async fn taxa_for(db: &Db, codes: &[String]) -> Result<Vec<Taxon>> {
    ensure(db).await?;
    let mut out = Vec::with_capacity(codes.len());
    for code in codes {
        if let Some(r) = sqlx::query(
            "SELECT ebird_code, common_name, scientific_name, family, family_sci, taxon_order
             FROM taxonomy WHERE ebird_code = ?1",
        )
        .bind(code)
        .fetch_optional(&db.0)
        .await?
        {
            out.push(Taxon {
                species_code: r.get("ebird_code"),
                com_name: r.get("common_name"),
                sci_name: r.get("scientific_name"),
                family_com_name: r.get("family"),
                family_sci_name: r.get("family_sci"),
                order: r.get("taxon_order"),
            });
        }
    }
    Ok(out)
}
