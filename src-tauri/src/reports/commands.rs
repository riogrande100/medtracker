use super::export::{self, ExportOptions};
use super::query::{self, Report};
use crate::clock::TimeCtx;
use crate::commands::{with, Db};
use crate::error::AppResult;
use serde::Serialize;
use crate::transfer;
use tauri::State;

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

#[tauri::command]
pub fn export_ledger(app: tauri::AppHandle, db: State<Db>, input: ExportOptions) -> AppResult<ExportResult> {
    let ctx = TimeCtx::system();
    let built = with(&db, |c| export::build(c, &input, ctx.now_utc_us))?;
    let name = export::file_name(ctx.now_utc_us, built.extension);
    // Always created in the app's own folder; `transfer::save_copy` hands a copy to the user.
    let path = export::save(&[transfer::exports_dir(&app)?], &name, built.content.as_bytes())?;
    Ok(ExportResult { path: path.display().to_string(), file_name: name, rows: built.rows })
}
