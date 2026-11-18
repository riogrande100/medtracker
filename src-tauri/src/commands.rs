use crate::application::{ledger, medication};
use crate::clock::TimeCtx;
use crate::dto::*;
use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use std::sync::Mutex;
use tauri::State;

pub struct Db(pub Mutex<Connection>);

fn with<T>(db: &State<Db>, f: impl FnOnce(&mut Connection) -> AppResult<T>) -> AppResult<T> {
    let mut c = db.0.lock().map_err(|_| AppError::Storage("database unavailable".into()))?;
    f(&mut c)
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
pub fn list_history(db: State<Db>) -> AppResult<Vec<HistoryEntry>> {
    with(&db, |c| ledger::history(c, 200))
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
    with(&db, |c| medication::delete(c, &id))
}
