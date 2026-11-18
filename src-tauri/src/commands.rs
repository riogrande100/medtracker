use crate::application::{ledger, medication};
use crate::clock::TimeCtx;
use crate::dto::*;
use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::State;

/// App database state. If opening failed (newer version, corrupt, failed upgrade) there is no
/// connection and `startup_error` explains why; the data file has been left untouched and every
/// command returns that same typed error so the UI can show a recoverable screen.
pub struct Db {
    pub conn: Mutex<Option<Connection>>,
    pub startup_error: Option<AppError>,
    path: PathBuf,
    /// The pre-upgrade snapshot made by THIS launch (the only recovery copy for that upgrade).
    /// It is kept until the next launch; every other snapshot is purged on privacy deletion.
    snapshot: Option<PathBuf>,
}

impl Db {
    pub fn open(path: &Path) -> Db {
        match crate::storage::open_report(path) {
            Ok(o) => Db { conn: Mutex::new(Some(o.conn)), startup_error: None, path: path.to_path_buf(), snapshot: o.snapshot },
            Err(e) => Db { conn: Mutex::new(None), startup_error: Some(e), path: path.to_path_buf(), snapshot: None },
        }
    }

    /// True while this launch's pre-upgrade copy (which still contains deleted content) is on disk.
    pub fn upgrade_copy_exists(&self) -> bool {
        self.snapshot.as_ref().is_some_and(|s| s.exists())
    }

    /// After a privacy deletion: remove any pre-upgrade snapshot older than this launch. This
    /// launch's own snapshot stays, because it is the only way back from a bad upgrade.
    pub fn purge_older_snapshots(&self) {
        crate::storage::purge_snapshots(&self.path, self.snapshot.as_deref());
    }
}

fn with<T>(db: &State<Db>, f: impl FnOnce(&mut Connection) -> AppResult<T>) -> AppResult<T> {
    if let Some(e) = &db.startup_error {
        return Err(e.clone());
    }
    let mut guard = db.conn.lock().map_err(|_| AppError::Storage("database unavailable".into()))?;
    let c = guard.as_mut().ok_or_else(|| AppError::Storage("database unavailable".into()))?;
    f(c)
}

/// `None` when the database opened normally; otherwise the reason it did not.
#[tauri::command]
pub fn startup_status(db: State<Db>) -> Option<AppError> {
    db.startup_error.clone()
}

#[tauri::command]
pub fn list_today(db: State<Db>) -> AppResult<Vec<TodayItem>> {
    with(&db, |c| ledger::today(c, TimeCtx::system()))
}

#[tauri::command]
pub fn record_dose(db: State<Db>, input: RecordDose) -> AppResult<String> {
    with(&db, |c| ledger::record(c, TimeCtx::system(), input))
}

#[tauri::command]
pub fn correct_dose(db: State<Db>, input: CorrectDose) -> AppResult<()> {
    with(&db, |c| ledger::correct(c, TimeCtx::system(), input))
}

#[tauri::command]
pub fn undo_dose(db: State<Db>, event_id: String) -> AppResult<()> {
    with(&db, |c| ledger::undo(c, TimeCtx::system(), &event_id))
}

#[tauri::command]
pub fn restore_dose(db: State<Db>, event_id: String) -> AppResult<()> {
    with(&db, |c| ledger::restore(c, TimeCtx::system(), &event_id))
}

/// Permanent privacy deletion of one entry's content. Distinct from `undo_dose`.
#[tauri::command]
pub fn delete_dose_entry(db: State<Db>, event_id: String) -> AppResult<()> {
    with(&db, |c| ledger::delete_entry(c, TimeCtx::system(), &event_id))?;
    db.purge_older_snapshots();
    Ok(())
}

/// Whether a temporary pre-upgrade copy of the data (made by this launch) still exists; the
/// deletion dialogs must say so because that copy still holds what is being deleted.
#[tauri::command]
pub fn upgrade_copy_exists(db: State<Db>) -> bool {
    db.upgrade_copy_exists()
}

/// One page of History, newest first. Pass the previous page's `next_cursor` as `before`.
#[tauri::command]
pub fn list_history(db: State<Db>, limit: Option<u32>, before: Option<HistoryCursor>) -> AppResult<HistoryPage> {
    with(&db, |c| ledger::history_page(c, limit, before))
}

#[tauri::command]
pub fn create_medication(db: State<Db>, input: NewMedication) -> AppResult<String> {
    with(&db, |c| medication::create(c, TimeCtx::system(), input))
}

#[tauri::command]
pub fn list_medications(db: State<Db>) -> AppResult<Vec<MedicationSummary>> {
    with(&db, |c| medication::list(c))
}

#[tauri::command]
pub fn archive_medication(db: State<Db>, id: String, archived: bool) -> AppResult<()> {
    with(&db, |c| {
        if archived {
            medication::archive(c, TimeCtx::system(), &id)
        } else {
            medication::unarchive(c, TimeCtx::system(), &id)
        }
    })
}

#[tauri::command]
pub fn delete_medication(db: State<Db>, id: String) -> AppResult<()> {
    with(&db, |c| medication::delete(c, &id))?;
    db.purge_older_snapshots();
    Ok(())
}
