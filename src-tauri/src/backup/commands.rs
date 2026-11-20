use super::{BackupCreated, BackupFile, BackupFolder, OpGuard, RestoreResult};
use crate::application::schedule;
use crate::clock::{TimeCtx, Zone};
use crate::commands::Db;
use crate::error::{AppError, AppResult};
use crate::fsutil;
use crate::storage::migrations::MIGRATIONS;
use crate::transfer;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use zeroize::Zeroizing;

/// Where backups are created: always the app's own folder (owner-only, never shared). The user
/// gets a copy out of the app with `transfer::save_copy` (system save screen), and brings one back
/// with `transfer::pick_backup_file`; neither depends on a user-visible folder.
fn backup_dirs(app: &AppHandle) -> AppResult<Vec<PathBuf>> {
    Ok(vec![transfer::backups_dir(app)?])
}

/// The app's backup folder and the backups found in it. Nothing is opened or decrypted.
#[tauri::command]
pub fn backup_folder(app: AppHandle) -> AppResult<BackupFolder> {
    let dirs = backup_dirs(&app)?;
    let per_dir: Vec<(PathBuf, Vec<BackupFile>)> = dirs.into_iter().map(|d| { let f = super::list_backups(&d); (d, f) }).collect();
    let folder = per_dir.iter().find(|(_, f)| !f.is_empty()).or_else(|| per_dir.first()).map(|(d, _)| d.to_string_lossy().into_owned()).unwrap_or_default();
    let mut files: Vec<BackupFile> = per_dir.into_iter().flat_map(|(_, f)| f).collect();
    files.sort_by(|a, b| b.modified_utc_us.cmp(&a.modified_utc_us).then_with(|| a.name.cmp(&b.name)));
    Ok(BackupFolder { folder, files })
}

fn join_err(e: tauri::Error) -> AppError {
    AppError::Storage(format!("The operation did not finish ({e}). Nothing was changed."))
}

/// Async, with the slow work (password hashing, about a second) on a blocking thread so the window
/// does not freeze. The one-at-a-time guard is taken before anything else and held until done.
#[tauri::command]
pub async fn create_backup(app: AppHandle, passphrase: String) -> AppResult<BackupCreated> {
    let passphrase = Zeroizing::new(passphrase);
    let guard = OpGuard::acquire()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let db = app.state::<Db>();
        if let Some(e) = &db.startup_error {
            return Err(e.clone());
        }
        let dirs = backup_dirs(&app)?;
        let now = TimeCtx::system().now_utc_us;
        let name = fsutil::unique_name("MedTracker-backup", now, super::EXTENSION);
        let lock = db.conn.lock().map_err(|_| AppError::Storage("database unavailable".into()))?;
        let conn = lock.as_ref().ok_or_else(|| AppError::Storage("database unavailable".into()))?;
        super::create_backup_in(conn, db.path(), &dirs, &name, &passphrase, now, super::format::DEFAULT_KDF)
    })
    .await
    .map_err(join_err)?
}

/// Restores from `path`, which must be one of the app's own backups or the staged copy of a file
/// the user picked (`transfer::pick_backup_file`); anything else is refused before it is read. The live database is replaced only after the backup has been fully
/// verified; the whole swap happens while the database lock is held, so no other command can
/// write in between. `path` is only ever read, and anything that is not a valid backup under this
/// passphrase is refused with a typed error before any live file is touched. Only one backup or
/// restore runs at a time: the guard is taken before verification starts.
#[tauri::command]
pub async fn restore_backup(app: AppHandle, path: String, passphrase: String) -> AppResult<RestoreResult> {
    let passphrase = Zeroizing::new(passphrase);
    let guard = OpGuard::acquire()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let db = app.state::<Db>();
        if let Some(e) = &db.startup_error {
            return Err(e.clone());
        }
        let live = db.path().to_path_buf();
        let staging = transfer::staging_dir(&app)?;
        let file = transfer::ensure_inside(&path, &[transfer::backups_dir(&app)?, staging.clone()])?;
        // Phase 1 needs no database access, so the lock is not held during the slow part.
        let staged = super::prepare_restore(&live, &file, &passphrase, MIGRATIONS)?;
        let mut lock = db.conn.lock().map_err(|_| AppError::Storage("database unavailable".into()))?;
        let conn = lock.take().ok_or_else(|| AppError::Storage("database unavailable".into()))?;
        let (new_conn, result) = super::commit_restore(&live, conn, staged);
        if let Some(c) = &new_conn {
            if result.is_ok() {
                // Same as at startup: backups from before schedules had zones get the device's.
                if let Some(zone) = Zone::detect().id() {
                    let _ = schedule::adopt_device_zone(c, zone);
                }
            }
        }
        *lock = new_conn;
        if result.is_ok() {
            // The picked copy has done its job; a failed attempt keeps it so a retry (for example
            // with the right passphrase) does not need the picker again.
            transfer::clear_staging(&staging);
        }
        result
    })
    .await
    .map_err(join_err)?
}
