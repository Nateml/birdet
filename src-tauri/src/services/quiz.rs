use crate::db::Db;
use crate::commands::{Question, AnswerPayload, AnswerResult, QueueCounts};
use anyhow::Result;
use sqlx::Row;

// How far ahead a learning card may be shown when nothing else is queued —
// Anki's `learnAheadLimit` (default 20 min). Both learning steps (1m, 10m) fall
// inside it, so once the rest of the queue is empty a card is drilled to
// graduation rather than making the user wait on the clock.
const COLLAPSE_MIN: i64 = 20;

pub async fn next_question(
    db: &Db,
    pack: Option<String>,
    new_remaining: i64,
    include_reviews: bool,
    cram: bool,
) -> Result<Option<Question>> {
    // Cram / "practice anyway": ignore the schedule entirely and serve any card
    // in the pool, soonest-due first (study-ahead), new birds last. Answers still
    // reschedule normally. The frontend caps the count, so no drain check here.
    if cram {
        let row = sqlx::query(
            r#"SELECT b.id AS id, b.common_name AS common_name,
                      CASE WHEN m.bird_id IS NULL OR m.state = 'new' THEN 1 ELSE 0 END AS is_new
               FROM birds b
               LEFT JOIN mastery m ON m.bird_id = b.id
               WHERE (?1 IS NULL OR EXISTS (
                   SELECT 1 FROM pack_recordings pr
                   JOIN recordings r ON pr.recording_id = r.id
                   WHERE r.bird_id = b.id AND pr.pack_id = ?1
               ))
               ORDER BY (m.due_at IS NULL), m.due_at ASC, RANDOM()
               LIMIT 1"#,
        )
        .bind(pack)
        .fetch_optional(&db.0)
        .await?;
        return match row {
            Some(r) => Ok(Some(build_question(db, r.get("id"), r.get("common_name"), r.get::<i64, _>("is_new") == 1).await?)),
            None => Ok(None),
        };
    }

    // Anki-style queue pick, in priority order:
    //   prio 0 — strictly due now (learning time arrived, or review overdue)
    //   prio 1 — a brand-new bird, but only while the new-card budget lasts
    //   prio 2 — learn-ahead: a learning card due within the collapse window,
    //            surfaced early only because nothing else is left
    // Anything else is ineligible (prio 3) — when no eligible row exists the
    // session queue is drained and we return None (→ results screen).
    // `?4` (include_reviews) gates the due/learning prios so a "new-only" run
    // ignores the review queue entirely.
    let row = sqlx::query(
        r#"SELECT b.id AS id, b.common_name AS common_name,
                  CASE
                    WHEN ?4 AND m.bird_id IS NOT NULL AND m.state != 'new'
                         AND m.due_at IS NOT NULL AND m.due_at <= datetime('now') THEN 0
                    WHEN (m.bird_id IS NULL OR m.state = 'new') AND ?2 > 0 THEN 1
                    WHEN ?4 AND m.bird_id IS NOT NULL AND m.state != 'new'
                         AND m.due_at IS NOT NULL
                         AND m.due_at <= datetime('now', ?3) THEN 2
                    ELSE 3
                  END AS prio
           FROM birds b
           LEFT JOIN mastery m ON m.bird_id = b.id
           WHERE (?1 IS NULL OR EXISTS (
               SELECT 1 FROM pack_recordings pr
               JOIN recordings r ON pr.recording_id = r.id
               WHERE r.bird_id = b.id AND pr.pack_id = ?1
           ))
           ORDER BY prio ASC,
                    CASE WHEN prio = 1 THEN NULL ELSE m.due_at END ASC,
                    CASE WHEN prio = 1 THEN RANDOM() ELSE 0 END
           LIMIT 1"#,
    )
    .bind(pack)
    .bind(new_remaining)
    .bind(format!("+{} minutes", COLLAPSE_MIN))
    .bind(include_reviews)
    .fetch_optional(&db.0)
    .await?;

    let row = match row {
        Some(r) if r.get::<i64, _>("prio") < 3 => r,
        _ => return Ok(None), // queue drained
    };

    let prio: i64 = row.get("prio");
    Ok(Some(
        build_question(db, row.get("id"), row.get("common_name"), prio == 1).await?,
    ))
}

/// Summarise the session's remaining work for the live progress readout, scoped
/// exactly like `next_question` (pack filter, `include_reviews` gate). Buckets:
///   new      — available new birds (capped by the remaining budget)
///   learning — learning cards still being drilled (all due within the window)
///   due      — review cards due within the learn-ahead window
/// and `to_go`, the minimum questions to finish if every answer is correct:
/// `steps` per new bird, `steps - learning_step` per learning card, 1 per due
/// review. This decreases by one on each correct answer instead of sitting still
/// when a new bird merely shifts into the learning queue.
pub async fn queue_counts(
    db: &Db,
    pack: Option<String>,
    new_remaining: i64,
    include_reviews: bool,
) -> Result<QueueCounts> {
    let steps = LEARNING_STEPS_MIN.len() as i64;
    let row = sqlx::query(
        r#"SELECT
             SUM(CASE WHEN (m.bird_id IS NULL OR m.state = 'new') THEN 1 ELSE 0 END) AS new_avail,
             SUM(CASE WHEN ?2 AND m.state = 'learning'
                       AND m.due_at IS NOT NULL AND m.due_at <= datetime('now', ?3)
                      THEN 1 ELSE 0 END) AS learning_cnt,
             COALESCE(SUM(CASE WHEN ?2 AND m.state = 'learning'
                       AND m.due_at IS NOT NULL AND m.due_at <= datetime('now', ?3)
                      THEN ?4 - m.learning_step ELSE 0 END), 0) AS learning_reps,
             SUM(CASE WHEN ?2 AND m.state = 'review'
                       AND m.due_at IS NOT NULL AND m.due_at <= datetime('now', ?3)
                      THEN 1 ELSE 0 END) AS review_soon
           FROM birds b
           LEFT JOIN mastery m ON m.bird_id = b.id
           WHERE (?1 IS NULL OR EXISTS (
               SELECT 1 FROM pack_recordings pr
               JOIN recordings r ON pr.recording_id = r.id
               WHERE r.bird_id = b.id AND pr.pack_id = ?1
           ))"#,
    )
    .bind(pack)
    .bind(include_reviews)
    .bind(format!("+{} minutes", COLLAPSE_MIN))
    .bind(steps)
    .fetch_one(&db.0)
    .await?;

    let new_avail: i64 = row.try_get::<Option<i64>, _>("new_avail")?.unwrap_or(0);
    let learning: i64 = row.try_get::<Option<i64>, _>("learning_cnt")?.unwrap_or(0);
    let learning_reps: i64 = row.try_get::<Option<i64>, _>("learning_reps")?.unwrap_or(0);
    let due: i64 = row.try_get::<Option<i64>, _>("review_soon")?.unwrap_or(0);
    let new = new_remaining.max(0).min(new_avail);

    let to_go = steps * new + learning_reps + due;
    Ok(QueueCounts { new, learning, due, to_go })
}

// Pick a random recording for the bird and 3 random distractors, returning a
// ready-to-serve question (the UI shuffles the options).
async fn build_question(db: &Db, id: i64, name: String, is_new: bool) -> Result<Question> {
    let recording_id: i64 = sqlx::query_scalar(
        r#"SELECT id FROM recordings WHERE bird_id = ?1 ORDER BY RANDOM() LIMIT 1"#,
    )
    .bind(id)
    .fetch_one(&db.0)
    .await?;

    let choices = sqlx::query(
        r#"SELECT common_name FROM birds WHERE id != ?1 ORDER BY RANDOM() LIMIT 3"#,
    )
    .bind(id)
    .fetch_all(&db.0)
    .await?
    .into_iter()
    .map(|r| r.get::<String, _>("common_name"))
    .collect::<Vec<_>>();

    let mut options = choices;
    options.push(name);

    Ok(Question { bird_id: id, recording_id, choices: options, is_new })
}

// --- SM-2 + learning-steps scheduler ------------------------------------
// Learning/relearning steps (minutes) a card walks before graduating to a
// day-scale review interval. The short first step means a missed bird
// resurfaces within the same sitting.
const LEARNING_STEPS_MIN: [f64; 2] = [1.0, 10.0];
const GRAD_INTERVAL_DAYS: f64 = 1.0; // interval on graduating from learning
const MIN_EASE: f64 = 1.3;
const EASE_PENALTY: f64 = 0.2; // ease drop on a lapse

#[derive(Clone)]
struct Card {
    state: String,
    ease: f64,
    interval_days: f64,
    reps: i64,
    lapses: i64,
    learning_step: i64,
    seen: i64,
    correct: i64,
}

impl Default for Card {
    fn default() -> Self {
        Card {
            state: "new".into(),
            ease: 2.5,
            interval_days: 0.0,
            reps: 0,
            lapses: 0,
            learning_step: 0,
            seen: 0,
            correct: 0,
        }
    }
}

pub async fn submit_answer(db: &Db, payload: AnswerPayload) -> Result<AnswerResult> {
    let correct_name: String = sqlx::query_scalar("SELECT common_name FROM birds WHERE id = ?1")
        .bind(payload.bird_id)
        .fetch_one(&db.0).await?;

    let correct = payload.guess == correct_name;

    // Load the current card state (defaults for a brand-new bird).
    let mut card = sqlx::query(
        r#"SELECT state, ease, interval_days, reps, lapses, learning_step, seen, correct
           FROM mastery WHERE bird_id = ?1"#,
    )
    .bind(payload.bird_id)
    .fetch_optional(&db.0)
    .await?
    .map(|r| Card {
        state: r.get("state"),
        ease: r.get("ease"),
        interval_days: r.get("interval_days"),
        reps: r.get("reps"),
        lapses: r.get("lapses"),
        learning_step: r.get("learning_step"),
        seen: r.get("seen"),
        correct: r.get("correct"),
    })
    .unwrap_or_default();

    card.seen += 1;
    if correct {
        card.correct += 1;
    }

    // Compute the next schedule. `due_modifier` is a SQLite datetime() modifier
    // applied to 'now', so all clock math stays in the database.
    let due_modifier: String;
    if correct {
        if card.state == "review" {
            // Mature card recalled: stretch the interval by the ease factor.
            card.reps += 1;
            card.interval_days = (card.interval_days * card.ease).max(1.0);
            card.learning_step = 0;
            due_modifier = format!("+{} days", card.interval_days);
        } else {
            // New/learning card: advance a step, graduating off the last one.
            let next = card.learning_step + 1;
            if next as usize >= LEARNING_STEPS_MIN.len() {
                card.state = "review".into();
                card.reps += 1;
                card.interval_days = GRAD_INTERVAL_DAYS;
                card.learning_step = 0;
                due_modifier = format!("+{} days", card.interval_days);
            } else {
                card.state = "learning".into();
                card.learning_step = next;
                due_modifier = format!("+{} minutes", LEARNING_STEPS_MIN[next as usize]);
            }
        }
    } else {
        // "Again" (wrong or skipped): only a mature card counts as a lapse.
        if card.state == "review" {
            card.lapses += 1;
        }
        card.ease = (card.ease - EASE_PENALTY).max(MIN_EASE);
        card.state = "learning".into();
        card.learning_step = 0;
        card.interval_days = 0.0;
        due_modifier = format!("+{} minutes", LEARNING_STEPS_MIN[0]);
    }

    // Upsert card state (aggregate seen/correct kept for the stats screen).
    sqlx::query(
        r#"
      INSERT INTO mastery
        (bird_id, seen, correct, ease, interval_days, due_at, reps, lapses, learning_step, state, last_reviewed)
      VALUES (?1, ?2, ?3, ?4, ?5, datetime('now', ?6), ?7, ?8, ?9, ?10, datetime('now'))
      ON CONFLICT(bird_id) DO UPDATE SET
        seen = excluded.seen, correct = excluded.correct, ease = excluded.ease,
        interval_days = excluded.interval_days, due_at = excluded.due_at,
        reps = excluded.reps, lapses = excluded.lapses,
        learning_step = excluded.learning_step, state = excluded.state,
        last_reviewed = excluded.last_reviewed
    "#,
    )
    .bind(payload.bird_id)
    .bind(card.seen)
    .bind(card.correct)
    .bind(card.ease)
    .bind(card.interval_days)
    .bind(&due_modifier)
    .bind(card.reps)
    .bind(card.lapses)
    .bind(card.learning_step)
    .bind(&card.state)
    .execute(&db.0)
    .await?;

    // Log the individual attempt.
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

