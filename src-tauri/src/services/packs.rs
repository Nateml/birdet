use crate::db::Db;
use crate::models::{Pack, PackFile, PackFileBird};
use anyhow::{anyhow, Result};
use sqlx::Row;
use anyhow::Context;

/// Serialize a pack to the portable `birdet-pack` JSON shape. Returns
/// `(pack_name, pretty_json)`. Only species identity is exported — no audio.
pub async fn export_pack(db: &Db, pack_id: &str) -> Result<(String, String)> {
    let name: Option<String> = sqlx::query_scalar("SELECT name FROM packs WHERE id = ?1")
        .bind(pack_id)
        .fetch_optional(&db.0)
        .await?;
    let name = name.ok_or_else(|| anyhow!("Pack {} not found.", pack_id))?;

    let rows = sqlx::query(
        r#"SELECT b.ebird_code AS ebird_code, b.common_name AS common_name,
                  b.scientific_name AS scientific_name
           FROM birds b
           JOIN recordings r ON r.bird_id = b.id
           JOIN pack_recordings pr ON pr.recording_id = r.id AND pr.pack_id = ?1
           GROUP BY b.id
           ORDER BY b.common_name"#,
    )
    .bind(pack_id)
    .fetch_all(&db.0)
    .await?;

    let birds = rows
        .into_iter()
        .map(|row| PackFileBird {
            ebird_code: row.get("ebird_code"),
            common_name: row.get("common_name"),
            scientific_name: row.get("scientific_name"),
        })
        .collect();

    let file = PackFile { format: "birdet-pack".into(), version: 1, name: name.clone(), birds };
    let json = serde_json::to_string_pretty(&file)?;
    Ok((name, json))
}

/// Build a URL/PK-safe pack id (packs.id is VARCHAR(31)) from a name plus a
/// millisecond suffix for uniqueness.
fn pack_id_from(name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let slug: String = slug.split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-");
    let slug: String = slug.chars().take(20).collect();
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() % 1_000_000)
        .unwrap_or(0);
    format!("{}-{}", if slug.is_empty() { "pack" } else { &slug }, suffix)
}

/// All birds in the library, with descriptors for manual selection & filtering.
pub async fn get_birds(db: &Db) -> Result<Vec<crate::commands::BirdListItem>> {
    let rows = sqlx::query(
        r#"SELECT b.id AS id, b.common_name AS common_name, b.scientific_name AS scientific_name,
                  b.family AS family, b.region AS region,
                  COUNT(r.id) AS recording_count
           FROM birds b
           LEFT JOIN recordings r ON r.bird_id = b.id
           GROUP BY b.id
           ORDER BY b.common_name"#,
    )
    .fetch_all(&db.0)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| crate::commands::BirdListItem {
            id: row.get("id"),
            common_name: row.get("common_name"),
            scientific_name: row.get("scientific_name"),
            family: row.get("family"),
            region: row.get("region"),
            recording_count: row.get("recording_count"),
        })
        .collect())
}

/// Which packs each bird belongs to (via its recordings). One row per
/// (bird, pack) pair, so the frontend can render pack tags per bird.
pub async fn get_bird_packs(db: &Db) -> Result<Vec<crate::commands::BirdPackTag>> {
    let rows = sqlx::query(
        r#"SELECT DISTINCT r.bird_id AS bird_id, p.id AS pack_id, p.name AS pack_name
           FROM pack_recordings pr
           JOIN recordings r ON r.id = pr.recording_id
           JOIN packs p ON p.id = pr.pack_id
           ORDER BY p.name"#,
    )
    .fetch_all(&db.0)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| crate::commands::BirdPackTag {
            bird_id: row.get("bird_id"),
            pack_id: row.get("pack_id"),
            pack_name: row.get("pack_name"),
        })
        .collect())
}

/// Create a pack containing every recording of the given birds. Returns the id.
pub async fn create_pack_from_birds(db: &Db, name: &str, bird_ids: &[i64]) -> Result<String> {
    if bird_ids.is_empty() {
        return Err(anyhow!("A pack needs at least one bird."));
    }
    let id = pack_id_from(name);
    sqlx::query("INSERT INTO packs (id, name) VALUES (?1, ?2)")
        .bind(&id)
        .bind(name)
        .execute(&db.0)
        .await?;

    // Link all recordings belonging to those birds.
    for bird_id in bird_ids {
        sqlx::query(
            r#"INSERT OR IGNORE INTO pack_recordings (pack_id, recording_id)
               SELECT ?1, id FROM recordings WHERE bird_id = ?2"#,
        )
        .bind(&id)
        .bind(bird_id)
        .execute(&db.0)
        .await?;
    }
    Ok(id)
}

/// Birds in a pack (via its linked recordings). `recording_count` is how many
/// of the bird's recordings the pack contains.
pub async fn get_pack_birds(db: &Db, pack_id: &str) -> Result<Vec<crate::commands::BirdListItem>> {
    let rows = sqlx::query(
        r#"SELECT b.id AS id, b.common_name AS common_name, b.scientific_name AS scientific_name,
                  b.family AS family, b.region AS region,
                  COUNT(DISTINCT r.id) AS recording_count
           FROM birds b
           JOIN recordings r ON r.bird_id = b.id
           JOIN pack_recordings pr ON pr.recording_id = r.id AND pr.pack_id = ?1
           GROUP BY b.id
           ORDER BY b.common_name"#,
    )
    .bind(pack_id)
    .fetch_all(&db.0)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| crate::commands::BirdListItem {
            id: row.get("id"),
            common_name: row.get("common_name"),
            scientific_name: row.get("scientific_name"),
            family: row.get("family"),
            region: row.get("region"),
            recording_count: row.get("recording_count"),
        })
        .collect())
}

/// Rename a pack. Errors if the id doesn't exist.
pub async fn rename_pack(db: &Db, pack_id: &str, name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(anyhow!("Pack name can't be empty."));
    }
    let res = sqlx::query("UPDATE packs SET name = ?2 WHERE id = ?1")
        .bind(pack_id)
        .bind(name)
        .execute(&db.0)
        .await?;
    if res.rows_affected() == 0 {
        return Err(anyhow!("Pack {} not found.", pack_id));
    }
    Ok(())
}

/// Delete a pack and its recording links. Birds/recordings are untouched.
pub async fn delete_pack(db: &Db, pack_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM pack_recordings WHERE pack_id = ?1")
        .bind(pack_id)
        .execute(&db.0)
        .await?;
    sqlx::query("DELETE FROM packs WHERE id = ?1")
        .bind(pack_id)
        .execute(&db.0)
        .await?;
    Ok(())
}

/// Remove a bird (all its recordings) from a pack.
pub async fn remove_bird_from_pack(db: &Db, pack_id: &str, bird_id: i64) -> Result<()> {
    sqlx::query(
        r#"DELETE FROM pack_recordings
           WHERE pack_id = ?1
             AND recording_id IN (SELECT id FROM recordings WHERE bird_id = ?2)"#,
    )
    .bind(pack_id)
    .bind(bird_id)
    .execute(&db.0)
    .await?;
    Ok(())
}

/// Add birds (all their recordings) to an existing pack. No-op for a bird
/// already in the pack. Returns an error if the pack id doesn't exist.
pub async fn add_birds_to_pack(db: &Db, pack_id: &str, bird_ids: &[i64]) -> Result<()> {
    let exists: Option<String> = sqlx::query_scalar("SELECT id FROM packs WHERE id = ?1")
        .bind(pack_id)
        .fetch_optional(&db.0)
        .await?;
    if exists.is_none() {
        return Err(anyhow!("Pack {} not found.", pack_id));
    }
    for bird_id in bird_ids {
        sqlx::query(
            r#"INSERT OR IGNORE INTO pack_recordings (pack_id, recording_id)
               SELECT ?1, id FROM recordings WHERE bird_id = ?2"#,
        )
        .bind(pack_id)
        .bind(bird_id)
        .execute(&db.0)
        .await?;
    }
    Ok(())
}

/// Create a pack of every library bird in a family (local-only, no network).
pub async fn create_pack_from_family(
    db: &Db,
    name: Option<String>,
    family: &str,
) -> Result<String> {
    let ids: Vec<i64> = sqlx::query_scalar(
        "SELECT id FROM birds WHERE family IS NOT NULL AND lower(family) LIKE '%' || lower(?1) || '%'",
    )
    .bind(family)
    .fetch_all(&db.0)
    .await?;
    if ids.is_empty() {
        return Err(anyhow!("No birds match that family yet."));
    }
    create_pack_from_birds(db, &name.unwrap_or_else(|| family.to_string()), &ids).await
}

/// Create a pack from a region (via eBird spplist ∩ library), optionally
/// narrowed to a family. Region membership is authoritative, not import-based.
pub async fn create_pack_from_region(
    db: &Db,
    name: Option<String>,
    region: &str,
    family: Option<&str>,
) -> Result<String> {
    let ids = crate::services::regions::birds_in_region(db, region, family).await?;
    if ids.is_empty() {
        return Err(anyhow!(
            "No birds in your library occur in {} yet — import some first.",
            region
        ));
    }
    let auto = name.unwrap_or_else(|| match family {
        Some(f) => format!("{} · {}", region, f),
        None => region.to_string(),
    });
    create_pack_from_birds(db, &auto, &ids).await
}

pub async fn get_packs(db: &Db) -> Result<Vec<Pack>> {
    println!("Fetching packs from database...");
    // Query all packs and count associated birds
    let rows = sqlx::query!(
        r#"
        SELECT 
            p.id AS id, 
            p.name AS name, 
            p.description AS description, 
            COALESCE(COUNT(DISTINCT b.id), 0) AS bird_count
        FROM packs AS p 
        LEFT JOIN pack_recordings AS pr ON p.id = pr.pack_id
        LEFT JOIN recordings AS r on pr.recording_id = r.id
        LEFT JOIN birds AS b on r.bird_id = b.id 
        GROUP BY p.id, p.name, p.description 
        ORDER BY p.name;
        "#
    )
    .fetch_all(&db.0).await.context("Failed to fetch packs from database")?;

    println!("Query executed successfully, processing results...");

    let packs = rows.into_iter().map(|row| {
        Pack {
            id: row.id,
            name: row.name,
            description: row.description,
            bird_count: row.bird_count,
        }
    }).collect::<Vec<_>>();

    println!("Fetched {} packs.", packs.len());

    Ok(packs)
}
