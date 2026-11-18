use super::export::{self, ExportOptions};
use super::query::{self, Report};
use crate::clock::TimeCtx;
use crate::commands::{with, Db};
use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::io::Write;
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

fn target_dir(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    let p = app.path();
    if let Ok(d) = p.download_dir().or_else(|_| p.document_dir()) {
        if d.is_dir() {
            return Ok(d);
        }
    }
    let d = p.app_data_dir().map_err(|e| AppError::Storage(e.to_string()))?.join("exports");
    std::fs::create_dir_all(&d).map_err(|e| AppError::Storage(e.to_string()))?;
    Ok(d)
}

#[tauri::command]
pub fn export_ledger(app: tauri::AppHandle, db: State<Db>, input: ExportOptions) -> AppResult<ExportResult> {
    let ctx = TimeCtx::system();
    let built = with(&db, |c| export::build(c, &input, ctx.now_utc_us))?;
    let name = export::file_name(ctx.now_utc_us, built.extension);
    let path = target_dir(&app)?.join(&name);
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| AppError::Storage(format!("Couldn't save the export: {e}")))?;
    f.write_all(built.content.as_bytes()).map_err(|e| AppError::Storage(format!("Couldn't save the export: {e}")))?;
    Ok(ExportResult { path: path.display().to_string(), file_name: name, rows: built.rows })
}
