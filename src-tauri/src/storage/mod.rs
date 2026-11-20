pub mod migrations;

use crate::error::{AppError, AppResult};
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};

pub(crate) fn configure(conn: &Connection) -> AppResult<()> {
    // secure_delete zeroes freed content so redacted notes don't linger in free pages.
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA secure_delete = ON;")?;
    Ok(())
}

/// A successfully opened database plus the pre-upgrade snapshot THIS launch made, if any.
pub struct Opened {
    pub conn: Connection,
    /// Set only when this very call migrated an existing database. The snapshot is the single
    /// recovery copy for that upgrade, so it is kept until the next launch (see `open_with`).
    pub snapshot: Option<PathBuf>,
}

/// Opens (and if needed upgrades) the user's database.
///
/// For an existing file, BEFORE any normal write:
/// 1. open it read-write (so SQLite can finish a crashed transaction by playing back a hot
///    rollback journal; a read-only open would call that corruption) and read `user_version`;
/// 2. `user_version` newer than this app supports -> `NewerDatabase`, file untouched;
/// 3. `PRAGMA integrity_check`; damage -> `CorruptDatabase`, file untouched;
/// 4. if migrations are pending, copy the database next to itself with `VACUUM INTO`
///    (`<name>.pre-migration-vN-to-vM.bak`, verified with integrity_check), then apply them in a
///    single transaction; failure -> `MigrationFailed`, rolled back, snapshot kept.
///
/// Snapshot lifecycle (a snapshot holds a full plaintext copy, including notes):
/// * one file per version pair, replaced (never accumulated) if the upgrade is retried;
/// * deleted automatically on the first later launch that opens the already-upgraded database
///   cleanly (that launch is not the migration launch, so the snapshot is no longer needed).
#[cfg(test)]
pub fn open(path: &Path) -> AppResult<Connection> {
    Ok(open_report(path)?.conn)
}

pub fn open_report(path: &Path) -> AppResult<Opened> {
    open_report_with(path, migrations::MIGRATIONS)
}

#[cfg(test)]
pub fn open_with(path: &Path, migrations: &[&str]) -> AppResult<Connection> {
    Ok(open_report_with(path, migrations)?.conn)
}

pub fn open_report_with(path: &Path, migrations: &[&str]) -> AppResult<Opened> {
    let target = migrations.len() as i64;
    let existing = path.metadata().map(|m| m.len() > 0).unwrap_or(false);
    let mut snapshot: Option<PathBuf> = None;
    let mut up_to_date = false;
    if existing {
        let version = preflight(path, target)?;
        up_to_date = version == target;
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
    if up_to_date {
        // The database was already at the current version and just opened cleanly, so any
        // snapshot is from an earlier launch's upgrade and its job is done.
        purge_snapshots(path, None);
    }
    Ok(Opened { conn, snapshot })
}

fn snapshot_prefix(path: &Path) -> String {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "user-data.sqlite".into());
    format!("{name}.pre-migration-v")
}

fn is_snapshot_file(prefix: &str, file_name: &str) -> bool {
    file_name.starts_with(prefix)
        && [".bak", ".bak.tmp", ".bak-journal", ".bak-wal", ".bak-shm", ".bak.tmp-journal"].iter().any(|s| file_name.ends_with(s))
}

/// Existing pre-upgrade snapshot files (and their stray journals) for this database.
pub fn snapshot_files(path: &Path) -> Vec<PathBuf> {
    let prefix = snapshot_prefix(path);
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| is_snapshot_file(&prefix, &e.file_name().to_string_lossy()))
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// Deletes every pre-upgrade snapshot except `keep` (the one made by the current launch).
/// Best effort: a snapshot that can't be removed is left for the next launch.
pub fn purge_snapshots(path: &Path, keep: Option<&Path>) {
    for f in snapshot_files(path) {
        let is_kept = keep.is_some_and(|k| {
            let k = k.as_os_str().to_string_lossy();
            let f = f.as_os_str().to_string_lossy();
            f == k || f.strip_prefix(k.as_ref()).is_some_and(|rest| rest.starts_with('-'))
        });
        if !is_kept {
            let _ = std::fs::remove_file(&f);
        }
    }
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

/// Read-write but never creating: lets SQLite recover a hot rollback journal left by a crash.
fn read_write_existing(path: &Path) -> AppResult<Connection> {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(|e| corrupt(&e.to_string()))
}

fn integrity_problem(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn.prepare("PRAGMA integrity_check").map_err(|e| e.to_string())?;
    let rows: Vec<String> =
        stmt.query_map([], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?.collect::<Result<_, _>>().map_err(|e| e.to_string())?;
    if rows.len() == 1 && rows[0] == "ok" {
        Ok(())
    } else {
        Err(rows.into_iter().take(3).collect::<Vec<_>>().join("; "))
    }
}

fn integrity_ok(conn: &Connection) -> AppResult<()> {
    integrity_problem(conn).map_err(|d| corrupt(&d))
}

/// Checks before any intentional write; returns the file's `user_version`. The newer-version
/// check comes first so a database from a newer app is refused as such, not as "damaged".
fn preflight(path: &Path, supported: i64) -> AppResult<i64> {
    let conn = read_write_existing(path)?;
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).map_err(|e| corrupt(&e.to_string()))?;
    if version > supported {
        return Err(migrations::newer(version, supported));
    }
    integrity_ok(&conn)?;
    Ok(version)
}

/// Writes the single snapshot for this version pair, replacing any earlier one (a retried
/// upgrade must not leave a pile of plaintext copies). Built under a temporary name and moved
/// into place only once verified.
fn snapshot_before_migration(path: &Path, from: i64, to: i64) -> AppResult<PathBuf> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let prefix = snapshot_prefix(path);
    let dest = dir.join(format!("{prefix}{from}-to-v{to}.bak"));
    let tmp = dir.join(format!("{prefix}{from}-to-v{to}.bak.tmp"));
    let fail = |e: String| AppError::MigrationFailed(format!("Could not make a safety copy before upgrading ({e}). Your data has not been changed."));
    let _ = std::fs::remove_file(&tmp);
    let src = read_only(path)?;
    src.execute("VACUUM INTO ?1", [tmp.to_string_lossy().as_ref()]).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        fail(e.to_string())
    })?;
    // The copy must itself be a healthy database at the old version, or we refuse to proceed.
    let check = (|| -> Result<(), String> {
        let c = Connection::open_with_flags(&tmp, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| e.to_string())?;
        integrity_problem(&c).map_err(|d| format!("the copy failed its check: {d}"))?;
        if migrations::user_version(&c).map_err(|_| "could not read the copy's version".to_string())? != from {
            return Err("the copy has the wrong data version".into());
        }
        Ok(())
    })();
    if let Err(e) = check {
        let _ = std::fs::remove_file(&tmp);
        return Err(fail(e));
    }
    std::fs::rename(&tmp, &dest).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        fail(e.to_string())
    })?;
    Ok(dest)
}

#[cfg(test)]
pub fn open_memory() -> AppResult<Connection> {
    let mut conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrations::migrate(&mut conn)?;
    Ok(conn)
}
