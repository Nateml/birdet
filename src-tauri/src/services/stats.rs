use crate::db::Db;
use crate::commands::{Stats, BirdStat};
use anyhow::Result;
use sqlx::Row;

pub async fn get_stats(db: &Db) -> Result<Stats> {
    // Per-bird mastery, joined to names. Birds never answered simply don't appear.
    let rows = sqlx::query(
        r#"SELECT b.common_name AS common_name, m.seen AS seen, m.correct AS correct
           FROM mastery m
           JOIN birds b ON b.id = m.bird_id
           ORDER BY b.common_name"#
    )
    .fetch_all(&db.0).await?;

    let mut birds = Vec::with_capacity(rows.len());
    let mut total_seen = 0i64;
    let mut total_correct = 0i64;

    for row in rows {
        let seen: i64 = row.get("seen");
        let correct: i64 = row.get("correct");
        total_seen += seen;
        total_correct += correct;
        birds.push(BirdStat {
            common_name: row.get("common_name"),
            seen,
            correct,
        });
    }

    Ok(Stats { total_seen, total_correct, birds })
}
