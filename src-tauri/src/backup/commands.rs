use super::{BackupCreated, BackupFolder, RestoreResult};
use crate::application::schedule;
use crate::clock::{TimeCtx, Zone};
use crate::commands::Db;
use crate::error::{AppError, AppResult};
use crate::storage::migrations::MIGRATIONS;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

/// Where backups are written and listed: "MedTracker Backups" inside the user's Documents folder
/// (visible in their file manager), or the app's own data folder where the OS has no such folder.
fn backup_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let p = app.path();
    if let Ok(d) = p.document_dir() {
        if d.is_dir() {
            return Ok(d.join("MedTracker Backups"));
        }
    }
    Ok(p.app_data_dir().map_err(|e| AppError::Storage(e.to_string()))?.join("backups"))
}

/// The backup folder and the backups found in it. Nothing is opened or decrypted.
#[tauri::command]
pub fn backup_folder(app: AppHandle) -> AppResult<BackupFolder> {
    let dir = backup_dir(&app)?;
    Ok(BackupFolder { folder: dir.to_string_lossy().into_owned(), files: super::list_backups(&dir) })
}

/// Async so the deliberately slow password hashing (about a second) does not freeze the window.
#[tauri::command]
pub async fn create_backup(app: AppHandle, db: State<'_, Db>, passphrase: String) -> AppResult<BackupCreated> {
    let dir = backup_dir(&app)?;
    if let Some(e) = &db.startup_error {
        return Err(e.clone());
    }
    let guard = db.conn.lock().map_err(|_| AppError::Storage("database unavailable".into()))?;
    let conn = guard.as_ref().ok_or_else(|| AppError::Storage("database unavailable".into()))?;
    let name = format!("MedTracker-backup-{}.{}", chrono::Local::now().format("%Y-%m-%d-%H%M"), super::EXTENSION);
    super::create_backup(conn, db.path(), &dir, &name, &passphrase, TimeCtx::system().now_utc_us, super::format::DEFAULT_KDF)
}

/// Restores from `path`. The live database is replaced only after the backup has been fully
/// verified; the whole swap happens while the database lock is held, so no other command can
/// write in between. `path` is only ever read, and anything that is not a valid backup under this
/// passphrase is refused with a typed error before any live file is touched.
#[tauri::command]
pub async fn restore_backup(db: State<'_, Db>, path: String, passphrase: String) -> AppResult<RestoreResult> {
    if let Some(e) = &db.startup_error {
        return Err(e.clone());
    }
    let live = db.path().to_path_buf();
    // Phase 1 needs no database access, so the lock is not held during the slow part.
    let staged = super::prepare_restore(&live, std::path::Path::new(path.trim()), &passphrase, MIGRATIONS)?;
    let mut guard = db.conn.lock().map_err(|_| AppError::Storage("database unavailable".into()))?;
    let conn = guard.take().ok_or_else(|| AppError::Storage("database unavailable".into()))?;
    let (new_conn, result) = super::commit_restore(&live, conn, staged);
    if let Some(c) = &new_conn {
        if result.is_ok() {
            // Same as at startup: backups from before schedules had zones get the device's.
            if let Some(zone) = Zone::detect().id() {
                let _ = schedule::adopt_device_zone(c, zone);
            }
        }
    }
    *guard = new_conn;
    result
}
