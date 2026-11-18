mod application;
mod backup;
mod clock;
mod commands;
mod domain;
mod dto;
mod error;
mod fsutil;
mod notifications;
mod reports;
mod storage;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_schedule;

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
            commands::upgrade_copy_exists,
            commands::create_medication,
            commands::list_medications,
            commands::archive_medication,
            commands::delete_medication,
            commands::create_schedule,
            commands::edit_schedule,
            commands::pause_schedule,
            commands::resume_schedule,
            commands::end_schedule,
            commands::preview_schedule,
            commands::next_due,
            commands::generate_occurrences,
            backup::commands::backup_folder,
            backup::commands::create_backup,
            backup::commands::restore_backup,
            reports::commands::report_range,
            reports::commands::export_ledger,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MedTracker");
}
