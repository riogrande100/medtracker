pub mod migrations;

use crate::error::{AppError, AppResult};
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};

fn configure(conn: &Connection) -> AppResult<()> {
    // secure_delete zeroes freed content so redacted notes don't linger in free pages.
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA secure_delete = ON;")?;
    Ok(())
}

/// Opens (and if needed upgrades) the user's database.
///
/// For an existing file, BEFORE any write:
/// 1. open read-only, run `PRAGMA integrity_check`; damage -> `CorruptDatabase`, file untouched;
/// 2. `user_version` newer than this app supports -> `NewerDatabase`, file untouched;
/// 3. if migrations are pending, copy the database next to itself with `VACUUM INTO`
///    (`<name>.pre-migration-vN-to-vM.bak`, verified with integrity_check), then apply them in a
///    single transaction; failure -> `MigrationFailed`, rolled back, snapshot kept.
pub fn open(path: &Path) -> AppResult<Connection> {
    open_with(path, migrations::MIGRATIONS)
}

pub fn open_with(path: &Path, migrations: &[&str]) -> AppResult<Connection> {
    let target = migrations.len() as i64;
    let existing = path.metadata().map(|m| m.len() > 0).unwrap_or(false);
    let mut snapshot: Option<PathBuf> = None;
    if existing {
        let version = preflight(path)?;
        if version > target {
            return Err(migrations::newer(version, target));
        }
        if version < target {
            snapshot = Some(snapshot_before_migration(path, version, target)?);
        }
    }
    let mut conn = Connection::open(path).map_err(|e| AppError::Storage(e.to_string()))?;
    configure(&conn)?;
    migrations::migrate_with(&mut conn, migrations).map_err(|e| match (e, &snapshot) {
        (AppError::MigrationFailed(m), Some(s)) => {
            AppError::MigrationFailed(format!("{m} A copy of your data from before the upgrade is at {}.", s.display()))
        }
        (e, _) => e,
    })?;
    Ok(conn)
}

fn corrupt(detail: &str) -> AppError {
    AppError::CorruptDatabase(format!(
        "The data file could not be read safely ({detail}). It has been left exactly as it is; do not delete it. \
         Restore from a backup or contact support with the file intact."
    ))
}

fn read_only(path: &Path) -> AppResult<Connection> {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| corrupt(&e.to_string()))
}

fn integrity_ok(conn: &Connection) -> AppResult<()> {
    let mut stmt = conn.prepare("PRAGMA integrity_check").map_err(|e| corrupt(&e.to_string()))?;
    let rows: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| corrupt(&e.to_string()))?
        .collect::<Result<_, _>>()
        .map_err(|e| corrupt(&e.to_string()))?;
    if rows.len() == 1 && rows[0] == "ok" {
        Ok(())
    } else {
        Err(corrupt(&rows.into_iter().take(3).collect::<Vec<_>>().join("; ")))
    }
}

/// Read-only checks; returns the file's `user_version`.
fn preflight(path: &Path) -> AppResult<i64> {
    let conn = read_only(path)?;
    integrity_ok(&conn)?;
    conn.query_row("PRAGMA user_version", [], |r| r.get(0)).map_err(|e| corrupt(&e.to_string()))
}

fn snapshot_before_migration(path: &Path, from: i64, to: i64) -> AppResult<PathBuf> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "user-data.sqlite".into());
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut dest = dir.join(format!("{name}.pre-migration-v{from}-to-v{to}.bak"));
    let mut n = 1;
    while dest.exists() {
        n += 1;
        dest = dir.join(format!("{name}.pre-migration-v{from}-to-v{to}-{n}.bak"));
    }
    let fail = |e: String| AppError::MigrationFailed(format!("Could not make a safety copy before upgrading ({e}). Your data has not been changed."));
    let src = read_only(path)?;
    src.execute("VACUUM INTO ?1", [dest.to_string_lossy().as_ref()]).map_err(|e| fail(e.to_string()))?;
    // The copy must itself be a healthy database at the old version, or we refuse to proceed.
    let check = (|| -> AppResult<()> {
        let c = read_only(&dest)?;
        integrity_ok(&c)?;
        if migrations::user_version(&c)? != from {
            return Err(corrupt("snapshot version mismatch"));
        }
        Ok(())
    })();
    if let Err(e) = check {
        let _ = std::fs::remove_file(&dest);
        return Err(fail(format!("{e:?}")));
    }
    Ok(dest)
}

#[cfg(test)]
pub fn open_memory() -> AppResult<Connection> {
    let mut conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrations::migrate(&mut conn)?;
    Ok(conn)
}
