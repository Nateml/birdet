use crate::db::Db;
use crate::commands::{Stats, BirdStat};
use anyhow::Result;
use sqlx::Row;

// A review card is "mastered" (Anki-mature) once its interval reaches 21 days.
const MATURE_DAYS: f64 = 21.0;

pub async fn get_stats(db: &Db) -> Result<Stats> {
    // Total library size — includes birds never studied (no mastery row).
    let total_birds: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM birds")
        .fetch_one(&db.0)
        .await?;

    // Aggregate deck composition across the scheduled cards.
    let agg = sqlx::query(
        r#"SELECT
             COALESCE(SUM(seen), 0)    AS total_seen,
             COALESCE(SUM(correct), 0) AS total_correct,
             COUNT(*)                  AS scheduled,
             COALESCE(SUM(CASE WHEN state = 'learning' THEN 1 ELSE 0 END), 0) AS learning,
             COALESCE(SUM(CASE WHEN state = 'review' AND interval_days <  ?1 THEN 1 ELSE 0 END), 0) AS review,
             COALESCE(SUM(CASE WHEN state = 'review' AND interval_days >= ?1 THEN 1 ELSE 0 END), 0) AS mastered,
             COALESCE(SUM(CASE WHEN state != 'new' AND due_at IS NOT NULL
                               AND due_at <= datetime('now') THEN 1 ELSE 0 END), 0) AS due
           FROM mastery"#,
    )
    .bind(MATURE_DAYS)
    .fetch_one(&db.0)
    .await?;

    let total_seen: i64 = agg.get("total_seen");
    let total_correct: i64 = agg.get("total_correct");
    let scheduled: i64 = agg.get("scheduled");
    let learning_count: i64 = agg.get("learning");
    let review_count: i64 = agg.get("review");
    let mastered_count: i64 = agg.get("mastered");
    let due_count: i64 = agg.get("due");
    // Birds with no mastery row (and any leftover 'new'-state rows) are new.
    let new_count = (total_birds - scheduled).max(0);

    // Per-bird detail, ordered so the actionable cards (due / weak) sort first.
    let rows = sqlx::query(
        r#"SELECT b.common_name AS common_name, m.seen AS seen, m.correct AS correct,
                  m.state AS state, m.interval_days AS interval_days, m.ease AS ease,
                  m.lapses AS lapses, m.due_at AS due_at
           FROM mastery m
           JOIN birds b ON b.id = m.bird_id
           ORDER BY
             CASE WHEN m.state != 'new' AND m.due_at IS NOT NULL
                       AND m.due_at <= datetime('now') THEN 0 ELSE 1 END,
             CAST(m.correct AS REAL) / NULLIF(m.seen, 0) ASC,
             b.common_name"#,
    )
    .fetch_all(&db.0)
    .await?;

    let birds = rows
        .into_iter()
        .map(|row| {
            let state: String = row.get("state");
            let interval_days: f64 = row.get("interval_days");
            // Fold the mature bucket into a display-only "mastered" state.
            let display_state = if state == "review" && interval_days >= MATURE_DAYS {
                "mastered".to_string()
            } else {
                state
            };
            BirdStat {
                common_name: row.get("common_name"),
                seen: row.get("seen"),
                correct: row.get("correct"),
                state: display_state,
                interval_days,
                ease: row.get("ease"),
                lapses: row.get("lapses"),
                due_at: row.get("due_at"),
            }
        })
        .collect();

    Ok(Stats {
        total_seen,
        total_correct,
        total_birds,
        new_count,
        learning_count,
        review_count,
        mastered_count,
        due_count,
        birds,
    })
}
