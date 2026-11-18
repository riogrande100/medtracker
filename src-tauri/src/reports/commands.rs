use super::export::{self, ExportOptions};
use super::query::{self, Report};
use crate::clock::TimeCtx;
use crate::commands::{with, Db};
use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{Manager, State};

#[derive(Serialize)]
pub struct ExportResult {
    pub path: String,
    pub file_name: String,
    pub rows: usize,
}

#[tauri::command]
pub fn report_range(db: State<Db>, from: String, to: String) -> AppResult<Report> {
    with(&db, |c| query::report(c, &TimeCtx::system(), &from, &to))
}

/// Where an export may go, preferred folder first: Downloads (or Documents), then the app's own
/// data folder. A folder that exists but cannot be written to is skipped by [`export::save`].
fn target_dirs(app: &tauri::AppHandle) -> AppResult<Vec<PathBuf>> {
    let p = app.path();
    let mut dirs = vec![];
    if let Ok(d) = p.download_dir().or_else(|_| p.document_dir()) {
        if d.is_dir() {
            dirs.push(d);
        }
    }
    dirs.push(p.app_data_dir().map_err(|e| AppError::Storage(e.to_string()))?.join("exports"));
    Ok(dirs)
}

#[tauri::command]
pub fn export_ledger(app: tauri::AppHandle, db: State<Db>, input: ExportOptions) -> AppResult<ExportResult> {
    let ctx = TimeCtx::system();
    let built = with(&db, |c| export::build(c, &input, ctx.now_utc_us))?;
    let name = export::file_name(ctx.now_utc_us, built.extension);
    let path = export::save(&target_dirs(&app)?, &name, built.content.as_bytes())?;
    Ok(ExportResult { path: path.display().to_string(), file_name: name, rows: built.rows })
}
