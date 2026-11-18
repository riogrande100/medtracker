use crate::error::AppResult;
use rusqlite::Connection;

/// Numbered, embedded migrations. Index + 1 is the `user_version` after applying.
const MIGRATIONS: &[&str] = &[include_str!("sql/001_initial.sql")];

pub fn latest_version() -> i64 {
    MIGRATIONS.len() as i64
}

pub fn migrate(conn: &mut Connection) -> AppResult<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if current > latest_version() {
        return Err(crate::error::AppError::Storage(
            "database was created by a newer version; refusing to downgrade".into(),
        ));
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.execute_batch(&format!("PRAGMA user_version = {}", i + 1))?;
        tx.commit()?;
    }
    let bad: i64 = conn.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r.get(0))?;
    if bad > 0 {
        return Err(crate::error::AppError::Storage("foreign key check failed".into()));
    }
    Ok(())
}
