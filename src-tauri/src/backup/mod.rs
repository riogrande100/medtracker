//! Encrypted local backup and restore (B2-09).
//!
//! * **Create**: a consistent snapshot of the live database (`VACUUM INTO`, never a raw file copy),
//!   integrity-checked, wrapped with a small manifest and sealed in the versioned container from
//!   [`format`] under a key derived from the user's passphrase. The file is written under a
//!   temporary name, flushed, moved into place, then read back and decrypted once more, so a
//!   backup the app reports as made is a backup that opens.
//! * **Restore** has two phases so the live database is never at risk:
//!   1. [`prepare_restore`] touches nothing live. It authenticates and decrypts, writes the
//!      database to a staging file NEXT TO the live one (same filesystem, so the final rename is
//!      atomic), then checks version, `integrity_check`, `foreign_key_check` and runs the normal
//!      migrations on that staging file. Any failure deletes the staging file and returns a typed
//!      error.
//!   2. [`commit_restore`] takes a pre-restore copy of the live data, closes the live connection,
//!      and swaps the verified staging file into place with one rename. A failed swap reopens the
//!      untouched original.
//! * **Pre-restore copy** follows the existing snapshot lifecycle in `storage`: it is named so
//!   `storage::purge_snapshots` owns it (kept for the rest of this launch, removed by the first
//!   later clean launch and by a privacy deletion), and one copy is kept (replaced, never piled up).
//! * **No key escrow.** The passphrase is never stored or sent anywhere. Forgotten means unrecoverable.
//! * **Where the file goes.** No file-dialog plugin: it would add a dependency and capability
//!   permissions to an app whose capability set is deliberately `core:default` with an empty network
//!   allowlist. Instead backups are written to a folder the user can open in their file manager
//!   (the Documents folder where the OS has one, otherwise the app's data folder), the full path is
//!   returned and shown, and restore offers the files found there plus a typed path for a backup
//!   copied from elsewhere. A native picker can replace this later without touching this module.

pub mod commands;
pub mod format;
#[cfg(test)]
mod tests;

use crate::error::{AppError, AppResult};
use format::KdfParams;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

pub const EXTENSION: &str = "medbackup";
const MIN_PASSPHRASE_CHARS: usize = 8;
const MAX_MANIFEST: usize = 64 * 1024;

/// Describes what is inside a backup. It is INSIDE the encrypted payload, so nothing about the
/// user's data (not even counts) is readable without the passphrase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub format_version: u16,
    pub app_schema_version: u32,
    pub app_version: String,
    pub created_at_utc_us: i64,
    pub medications: i64,
    pub doses: i64,
    pub schedules: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BackupCreated {
    pub path: String,
    pub file_name: String,
    pub created_at_utc_us: i64,
    pub medications: i64,
    pub doses: i64,
    pub schedules: i64,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BackupFile {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_utc_us: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BackupFolder {
    pub folder: String,
    pub files: Vec<BackupFile>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RestoreResult {
    pub backup_created_at_utc_us: i64,
    pub medications: i64,
    pub doses: i64,
    pub schedules: i64,
    /// The data format the backup was made in, when it was older and had to be upgraded.
    pub upgraded_from: Option<u32>,
    /// A copy of what was on this device before the restore is kept until MedTracker next starts.
    pub safety_copy_kept: bool,
}

fn io_err(what: &str, e: std::io::Error) -> AppError {
    AppError::Storage(format!("{what} ({e})."))
}

pub fn validate_passphrase(p: &str) -> AppResult<()> {
    if p.chars().count() < MIN_PASSPHRASE_CHARS {
        return Err(AppError::Validation(format!("Use a passphrase of at least {MIN_PASSPHRASE_CHARS} characters.")));
    }
    Ok(())
}

// ---- paths ----------------------------------------------------------------------------------

fn sibling(live: &Path, suffix: &str) -> PathBuf {
    let name = live.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "user-data.sqlite".into());
    live.with_file_name(format!("{name}{suffix}"))
}

// All three share the `<db>.pre-migration-v...bak[.tmp]` shape, which is what makes
// `storage::purge_snapshots` (next clean launch, privacy deletion) sweep up any leftover.
fn pre_restore_copy(live: &Path) -> PathBuf {
    sibling(live, ".pre-migration-vrestore.bak")
}
fn backup_scratch(live: &Path) -> PathBuf {
    sibling(live, ".pre-migration-vbackup.bak.tmp")
}
fn staging_path(live: &Path) -> PathBuf {
    sibling(live, ".pre-migration-vrestore-staging.bak.tmp")
}

fn remove_with_sidecars(p: &Path) {
    let _ = std::fs::remove_file(p);
    for s in ["-journal", "-wal", "-shm"] {
        let mut o = p.as_os_str().to_owned();
        o.push(s);
        let _ = std::fs::remove_file(PathBuf::from(o));
    }
}

// ---- checks on a database file --------------------------------------------------------------

fn bad_data(detail: &str) -> AppError {
    AppError::BackupCorrupt(format!("The backup opened, but the data inside it is not healthy ({detail}). Nothing was changed."))
}

/// `integrity_check` and `foreign_key_check` must both come back clean.
fn verify_health(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn.prepare("PRAGMA integrity_check").map_err(|e| e.to_string())?;
    let rows: Vec<String> = stmt.query_map([], |r| r.get(0)).map_err(|e| e.to_string())?.collect::<Result<_, _>>().map_err(|e| e.to_string())?;
    if rows != ["ok"] {
        return Err(rows.into_iter().take(3).collect::<Vec<_>>().join("; "));
    }
    let fk: i64 = conn.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r.get(0)).map_err(|e| e.to_string())?;
    if fk > 0 {
        return Err("broken links between records".into());
    }
    Ok(())
}

fn user_version(conn: &Connection) -> Result<i64, String> {
    conn.query_row("PRAGMA user_version", [], |r| r.get(0)).map_err(|e| e.to_string())
}

fn counts(conn: &Connection) -> Result<(i64, i64, i64), String> {
    let n = |sql: &str| conn.query_row(sql, [], |r| r.get::<_, i64>(0)).map_err(|e| e.to_string());
    Ok((n("SELECT count(*) FROM user_medication")?, n("SELECT count(*) FROM dose_event WHERE deleted = 0")?, n("SELECT count(*) FROM schedule")?))
}

// ---- create ---------------------------------------------------------------------------------

/// Counts for the manifest come from the snapshot, so they describe exactly what is backed up.
fn snapshot_bytes(conn: &Connection, live: &Path) -> AppResult<(Zeroizing<Vec<u8>>, u32, (i64, i64, i64))> {
    let scratch = backup_scratch(live);
    remove_with_sidecars(&scratch);
    let result = (|| -> AppResult<(Zeroizing<Vec<u8>>, u32, (i64, i64, i64))> {
        conn.execute("VACUUM INTO ?1", [scratch.to_string_lossy().as_ref()])
            .map_err(|e| AppError::Storage(format!("Could not make a consistent copy of your data to back up ({e}). Nothing was changed.")))?;
        let snap = Connection::open_with_flags(&scratch, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| AppError::Storage(e.to_string()))?;
        verify_health(&snap).map_err(|d| AppError::CorruptDatabase(format!("Your data failed its health check ({d}), so no backup was made. Nothing was changed.")))?;
        let version = user_version(&snap).map_err(AppError::Storage)?;
        let c = counts(&snap).map_err(AppError::Storage)?;
        drop(snap);
        let bytes = std::fs::read(&scratch).map_err(|e| io_err("Could not read the copy of your data", e))?;
        Ok((Zeroizing::new(bytes), u32::try_from(version).map_err(|_| AppError::Storage("bad data version".into()))?, c))
    })();
    // The scratch copy is plaintext: it never outlives this call, success or not.
    remove_with_sidecars(&scratch);
    result
}

fn unique_path(dir: &Path, file_name: &str) -> PathBuf {
    let first = dir.join(file_name);
    if !first.exists() {
        return first;
    }
    let stem = file_name.strip_suffix(&format!(".{EXTENSION}")).unwrap_or(file_name);
    (2..)
        .map(|i| dir.join(format!("{stem}-{i}.{EXTENSION}")))
        .find(|p| !p.exists())
        .expect("an unused name exists")
}

/// Makes an encrypted backup of the database behind `conn` (whose file is `live`) in `dest_dir`.
/// On any failure nothing is left behind and the live database is unchanged.
pub fn create_backup(
    conn: &Connection,
    live: &Path,
    dest_dir: &Path,
    file_name: &str,
    passphrase: &str,
    now_utc_us: i64,
    kdf: KdfParams,
) -> AppResult<BackupCreated> {
    validate_passphrase(passphrase)?;
    std::fs::create_dir_all(dest_dir).map_err(|e| io_err("Could not create the backup folder", e))?;

    let (db, schema, (medications, doses, schedules)) = snapshot_bytes(conn, live)?;
    let manifest = Manifest {
        format_version: format::FORMAT_VERSION,
        app_schema_version: schema,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        created_at_utc_us: now_utc_us,
        medications,
        doses,
        schedules,
    };
    let manifest_json = serde_json::to_vec(&manifest).map_err(|e| AppError::Storage(e.to_string()))?;
    let mut plain = Zeroizing::new(Vec::with_capacity(4 + manifest_json.len() + db.len()));
    plain.extend_from_slice(&(manifest_json.len() as u32).to_be_bytes());
    plain.extend_from_slice(&manifest_json);
    plain.extend_from_slice(&db);
    drop(db);

    let sealed = format::seal(&plain, passphrase, schema, kdf)?;

    let dest = unique_path(dest_dir, file_name);
    let mut partial = dest.as_os_str().to_owned();
    partial.push(".partial");
    let partial = PathBuf::from(partial);
    let written = (|| -> AppResult<()> {
        let mut f = std::fs::File::create(&partial).map_err(|e| io_err("Could not write the backup file", e))?;
        f.write_all(&sealed).map_err(|e| io_err("Could not write the backup file (is the disk full?)", e))?;
        f.sync_all().map_err(|e| io_err("Could not finish writing the backup file", e))?;
        drop(f);
        std::fs::rename(&partial, &dest).map_err(|e| io_err("Could not save the backup file", e))
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_file(&partial);
        return Err(e);
    }

    // Read it back from disk and open it with the passphrase: a backup that cannot be opened is
    // worse than none, because it looks safe.
    let verified = std::fs::read(&dest)
        .map_err(|e| io_err("Could not read the backup back to check it", e))
        .and_then(|bytes| format::open(&bytes, passphrase, schema).map(|(_, p)| p))
        .and_then(|p| if p.as_slice() == plain.as_slice() { Ok(()) } else { Err(AppError::Storage("The saved backup did not match what was written.".into())) });
    if let Err(e) = verified {
        let _ = std::fs::remove_file(&dest);
        return Err(e);
    }

    Ok(BackupCreated {
        path: dest.to_string_lossy().into_owned(),
        file_name: dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        created_at_utc_us: now_utc_us,
        medications,
        doses,
        schedules,
        size_bytes: sealed.len() as u64,
    })
}

/// Backups in `dir`, newest first. Names only; nothing is opened, so no passphrase is needed.
pub fn list_backups(dir: &Path) -> Vec<BackupFile> {
    let mut out: Vec<BackupFile> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|x| x == EXTENSION))
                .filter_map(|e| {
                    let md = e.metadata().ok()?;
                    let modified = md.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_micros() as i64;
                    Some(BackupFile {
                        name: e.file_name().to_string_lossy().into_owned(),
                        path: e.path().to_string_lossy().into_owned(),
                        size_bytes: md.len(),
                        modified_utc_us: modified,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| b.modified_utc_us.cmp(&a.modified_utc_us).then_with(|| a.name.cmp(&b.name)));
    out
}

// ---- restore: phase 1, verify in a staging file ---------------------------------------------

/// A fully verified (and, if needed, upgraded) copy of the backup's database, waiting next to the
/// live database. Dropping it without committing deletes it.
pub struct Staged {
    path: PathBuf,
    keep: bool,
    pub manifest: Manifest,
    pub upgraded_from: Option<u32>,
    pub medications: i64,
    pub doses: i64,
    pub schedules: i64,
}

impl Drop for Staged {
    fn drop(&mut self) {
        if !self.keep {
            remove_with_sidecars(&self.path);
        }
    }
}

fn parse_payload(plain: &[u8], header_schema: u32) -> AppResult<(Manifest, &[u8])> {
    let bad = |m: &str| AppError::BackupCorrupt(format!("The backup's contents are not in the expected layout ({m}). Nothing was changed."));
    if plain.len() < 4 {
        return Err(bad("no manifest"));
    }
    let n = u32::from_be_bytes(plain[..4].try_into().expect("4")) as usize;
    if n == 0 || n > MAX_MANIFEST || plain.len() < 4 + n {
        return Err(bad("manifest length"));
    }
    let manifest: Manifest = serde_json::from_slice(&plain[4..4 + n]).map_err(|_| bad("malformed manifest"))?;
    if manifest.app_schema_version != header_schema || manifest.format_version != format::FORMAT_VERSION {
        return Err(bad("manifest does not match header"));
    }
    let db = &plain[4 + n..];
    if !db.starts_with(b"SQLite format 3\0") {
        return Err(bad("not a database"));
    }
    Ok((manifest, db))
}

/// Authenticates, decrypts and verifies `backup_file`, leaving a migrated candidate database next
/// to `live`. Nothing live is read or written. `migrations` is the app's migration list (a
/// parameter so tests can simulate older and newer apps).
pub fn prepare_restore(live: &Path, backup_file: &Path, passphrase: &str, migrations: &[&str]) -> AppResult<Staged> {
    let supported = migrations.len() as u32;
    let md = std::fs::metadata(backup_file).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => AppError::NotFound("That backup file was not found. Check the path, or choose one from the list.".into()),
        _ => io_err("Could not open the backup file", e),
    })?;
    if md.len() > format::MAX_PAYLOAD + format::HEADER_LEN as u64 {
        return Err(AppError::BackupCorrupt("This file is far larger than any MedTracker backup, so it was not opened. Nothing was changed.".into()));
    }
    let bytes = std::fs::read(backup_file).map_err(|e| io_err("Could not read the backup file", e))?;
    let (header, plain) = format::open(&bytes, passphrase, supported)?;
    drop(bytes);
    let (manifest, db) = parse_payload(&plain, header.app_schema_version)?;

    let staging = staging_path(live);
    remove_with_sidecars(&staging);
    let mut staged = Staged {
        path: staging.clone(),
        keep: false,
        manifest: manifest.clone(),
        upgraded_from: None,
        medications: 0,
        doses: 0,
        schedules: 0,
    };
    // From here on, any early return drops `staged`, which deletes the staging file.
    std::fs::write(&staging, db).map_err(|e| io_err("Could not prepare the backup for checking (is the disk full?). Your data was not changed", e))?;
    drop(plain);

    {
        let c = Connection::open_with_flags(&staging, OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(|e| bad_data(&e.to_string()))?;
        let version = user_version(&c).map_err(|e| bad_data(&e))?;
        if version > i64::from(supported) {
            return Err(AppError::BackupNewerSchema(format!(
                "This backup holds data from a newer version of MedTracker (data format {version}; this version supports {supported}). Update MedTracker, then restore it. Nothing was changed."
            )));
        }
        if version != i64::from(header.app_schema_version) {
            return Err(bad_data("its data version does not match its label"));
        }
        verify_health(&c).map_err(|d| bad_data(&d))?;
    }

    // Upgrade the candidate with the normal migration path (which takes its own snapshot of the
    // candidate and checks foreign keys); the live database is nowhere near this.
    let opened = crate::storage::open_report_with(&staging, migrations).map_err(|e| match e {
        AppError::NewerDatabase(m) => AppError::BackupNewerSchema(m),
        AppError::CorruptDatabase(m) => AppError::BackupCorrupt(m),
        other => other,
    });
    let opened = match opened {
        Ok(o) => o,
        Err(e) => {
            crate::storage::purge_snapshots(&staging, None);
            return Err(e);
        }
    };
    let conn = opened.conn;
    let finish = (|| -> Result<(i64, i64, i64), String> {
        verify_health(&conn)?;
        // Leave a single self-contained file: no journal or WAL beside it when it is renamed.
        conn.query_row("PRAGMA journal_mode = DELETE", [], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
        counts(&conn)
    })();
    drop(conn);
    crate::storage::purge_snapshots(&staging, None);
    let (m, d, s) = finish.map_err(|e| bad_data(&e))?;
    staged.upgraded_from = (header.app_schema_version != supported).then_some(header.app_schema_version);
    staged.medications = m;
    staged.doses = d;
    staged.schedules = s;
    Ok(staged)
}

// ---- restore: phase 2, swap -----------------------------------------------------------------

/// Takes a verified copy of the live data under the pre-restore name, replacing any earlier one.
fn pre_restore_snapshot(conn: &Connection, live: &Path) -> AppResult<()> {
    let fail = |e: String| AppError::Storage(format!("Could not make a safety copy of your current data first ({e}). Nothing was changed."));
    let dest = pre_restore_copy(live);
    let mut tmp = dest.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    remove_with_sidecars(&tmp);
    let made = (|| -> Result<(), String> {
        conn.execute("VACUUM INTO ?1", [tmp.to_string_lossy().as_ref()]).map_err(|e| e.to_string())?;
        let c = Connection::open_with_flags(&tmp, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| e.to_string())?;
        verify_health(&c)?;
        drop(c);
        std::fs::rename(&tmp, &dest).map_err(|e| e.to_string())
    })();
    made.map_err(|e| {
        remove_with_sidecars(&tmp);
        fail(e)
    })
}

fn open_live(live: &Path) -> AppResult<Connection> {
    let conn = Connection::open(live).map_err(|e| AppError::Storage(e.to_string()))?;
    crate::storage::configure(&conn)?;
    Ok(conn)
}

/// Swaps a verified `staged` database in for the live one. Always hands back a usable connection
/// when it can: the new database after success, the ORIGINAL after any failure.
pub fn commit_restore(live: &Path, conn: Connection, staged: Staged) -> (Option<Connection>, AppResult<RestoreResult>) {
    commit_restore_with(live, conn, staged, &|from, to| std::fs::rename(from, to))
}

pub fn commit_restore_with(
    live: &Path,
    conn: Connection,
    mut staged: Staged,
    rename: &dyn Fn(&Path, &Path) -> std::io::Result<()>,
) -> (Option<Connection>, AppResult<RestoreResult>) {
    if let Err(e) = pre_restore_snapshot(&conn, live) {
        return (Some(conn), Err(e));
    }
    if let Err((conn, e)) = conn.close() {
        return (Some(conn), Err(AppError::Storage(format!("Could not close your current data to replace it ({e}). Nothing was changed."))));
    }
    // A stale journal next to the live file must never be replayed into the new database.
    for s in ["-journal", "-wal", "-shm"] {
        let mut o = live.as_os_str().to_owned();
        o.push(s);
        let _ = std::fs::remove_file(PathBuf::from(o));
    }
    if let Err(e) = rename(&staged.path, live) {
        // The original file was never touched; reopen it and report.
        let reopened = open_live(live).ok();
        return (reopened, Err(io_err("Could not put the restored data in place. Your data was not changed", e)));
    }
    staged.keep = true; // the staging name no longer exists; the file is now the live database
    let result = RestoreResult {
        backup_created_at_utc_us: staged.manifest.created_at_utc_us,
        medications: staged.medications,
        doses: staged.doses,
        schedules: staged.schedules,
        upgraded_from: staged.upgraded_from,
        safety_copy_kept: pre_restore_copy(live).exists(),
    };
    match open_live(live) {
        Ok(c) => (Some(c), Ok(result)),
        Err(e) => (None, Err(AppError::Storage(format!("Your data was restored but could not be reopened; close and reopen MedTracker. ({e:?})")))),
    }
}
