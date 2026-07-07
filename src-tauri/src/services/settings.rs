use crate::db::Db;
use anyhow::Result;

/// Read a persisted setting (e.g. an API key). None if unset.
pub async fn get_setting(db: &Db, key: &str) -> Result<Option<String>> {
    let value: Option<String> =
        sqlx::query_scalar("SELECT value FROM settings WHERE key = ?1")
            .bind(key)
            .fetch_optional(&db.0)
            .await?
            .flatten();
    Ok(value)
}

/// Upsert a persisted setting.
pub async fn set_setting(db: &Db, key: &str, value: &str) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO settings (key, value) VALUES (?1, ?2)
           ON CONFLICT(key) DO UPDATE SET value = excluded.value"#,
    )
    .bind(key)
    .bind(value)
    .execute(&db.0)
    .await?;
    Ok(())
}
