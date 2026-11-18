use crate::error::{AppError, AppResult};
use rusqlite::Connection;

/// Numbered, embedded migrations. Index + 1 is the `user_version` after applying.
pub const MIGRATIONS: &[&str] = &[include_str!("sql/001_initial.sql"), include_str!("sql/002_undo_delete_zone.sql")];

#[cfg(test)]
pub fn latest_version() -> i64 {
    MIGRATIONS.len() as i64
}

pub fn user_version(conn: &Connection) -> AppResult<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}

#[cfg(test)]
pub fn migrate(conn: &mut Connection) -> AppResult<()> {
    migrate_with(conn, MIGRATIONS)
}

/// Applies every pending migration in ONE transaction together with the `user_version` bump, so
/// a failure at any step (including the final foreign-key check) rolls the database back to
/// exactly the state it was in. A database from a newer app version is never touched.
pub fn migrate_with(conn: &mut Connection, migrations: &[&str]) -> AppResult<()> {
    let target = migrations.len() as i64;
    let current = user_version(conn)?;
    if current > target {
        return Err(newer(current, target));
    }
    if current == target {
        return Ok(());
    }
    let tx = conn.transaction().map_err(|e| failed(current + 1, &e.to_string()))?;
    for (i, sql) in migrations.iter().enumerate().skip(current as usize) {
        tx.execute_batch(sql).map_err(|e| failed(i as i64 + 1, &e.to_string()))?;
    }
    tx.execute_batch(&format!("PRAGMA user_version = {target}")).map_err(|e| failed(target, &e.to_string()))?;
    let bad: i64 = tx
        .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r.get(0))
        .map_err(|e| failed(target, &e.to_string()))?;
    if bad > 0 {
        return Err(failed(target, "foreign key check failed"));
    }
    tx.commit().map_err(|e| failed(target, &e.to_string()))?;
    Ok(())
}

pub fn newer(found: i64, supported: i64) -> AppError {
    AppError::NewerDatabase(format!(
        "This data file was created by a newer version of MedTracker (data format {found}; this version supports {supported}). \
         Your data has not been changed. Update MedTracker to open it."
    ))
}

fn failed(step: i64, detail: &str) -> AppError {
    AppError::MigrationFailed(format!("Upgrade step {step} failed ({detail}). Your data has not been changed."))
}
