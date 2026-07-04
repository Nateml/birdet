use crate::db::Db;
use crate::commands::{Question, AnswerPayload, AnswerResult};
use anyhow::Result;
use sqlx::Row;

pub async fn next_question(db: &Db, pack: Option<String>) -> Result<Question> {
    // pick a random bird from the pack
    let row = sqlx::query(
        r#"SELECT b.id AS id, b.common_name AS common_name FROM birds b
           WHERE (?1 IS NULL OR EXISTS (
               SELECT 1 FROM pack_recordings pr
               JOIN recordings r ON pr.recording_id = r.id
               WHERE r.bird_id = b.id AND pr.pack_id = ?1
           ))
           ORDER BY RANDOM() LIMIT 1"#
    )
    .bind(pack)
    .fetch_one(&db.0).await?;

    // pick a random recording for that bird
    let bird_id: i64 = row.get("id");
    let recording_row = sqlx::query(
        r#"SELECT id AS id FROM recordings
              WHERE bird_id = ?1
                ORDER BY RANDOM() LIMIT 1"#
    )
    .bind(bird_id)
    .fetch_one(&db.0).await?;

    let recording_id: i64 = recording_row.get("id");

    let id: i64 = row.get("id");
    let name: String = row.get("common_name");

    // Randomly select multiple choice options
    let choices = sqlx::query(r#"SELECT common_name FROM birds WHERE id != ?1 ORDER BY RANDOM() LIMIT 3"#)
        .bind(id)
        .fetch_all(&db.0).await?
        .into_iter().map(|r| r.get::<String,_>("common_name")).collect::<Vec<_>>();

    let mut options = choices;
    options.push(name.clone());
    // Simple shuffle in Rust side or let UI shuffle

    Ok(Question { bird_id: id, recording_id: recording_id, choices: options })
}

pub async fn submit_answer(db: &Db, payload: AnswerPayload) -> Result<AnswerResult> {
    let correct_name: String = sqlx::query_scalar("SELECT common_name FROM birds WHERE id = ?1")
        .bind(payload.bird_id)
        .fetch_one(&db.0).await?;

    let correct = payload.guess == correct_name;

    // Upsert aggregate mastery
    sqlx::query(r#"
      INSERT INTO mastery (bird_id, seen, correct)
      VALUES (?1, 1, CASE WHEN ?2 THEN 1 ELSE 0 END)
      ON CONFLICT(bird_id) DO UPDATE SET
        seen = seen + 1,
        correct = correct + CASE WHEN excluded.correct=1 THEN 1 ELSE 0 END
    "#)
    .bind(payload.bird_id)
    .bind(correct)
    .execute(&db.0).await?;

    // Log the individual attempt
    sqlx::query(r#"
      INSERT INTO history (bird_id, recording_id, answered_correctly)
      VALUES (?1, ?2, ?3)
    "#)
    .bind(payload.bird_id)
    .bind(payload.recording_id)
    .bind(correct)
    .execute(&db.0).await?;

    Ok(AnswerResult { correct, correct_name })
}

