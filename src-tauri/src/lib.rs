mod application;
mod clock;
mod commands;
mod dto;
mod error;
mod notifications;
mod storage;

#[cfg(test)]
mod tests;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            // A failed open (newer/corrupt/failed upgrade) must not abort the app or touch the file:
            // the UI shows a recoverable screen from `startup_status`.
            app.manage(commands::Db::open(&dir.join("user-data.sqlite")));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::startup_status,
            commands::list_today,
            commands::record_dose,
            commands::correct_dose,
            commands::undo_dose,
            commands::restore_dose,
            commands::delete_dose_entry,
            commands::list_history,
            commands::create_medication,
            commands::list_medications,
            commands::archive_medication,
            commands::delete_medication,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MedTracker");
}
