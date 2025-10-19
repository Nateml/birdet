use crate::db::Db;
use crate::models::Pack;
use anyhow::Result;
use sqlx::Row;
use anyhow::Context;

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
