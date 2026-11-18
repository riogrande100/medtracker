use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum AppError {
    Validation(String),
    NotFound(String),
    Storage(String),
    /// A slot-linked request named a date that is no longer today (midnight passed,
    /// or the device was resumed on a later day). The caller must refresh Today.
    StaleDate(String),
    /// The database was written by a newer app version. It is left untouched.
    NewerDatabase(String),
    /// An upgrade failed and was rolled back; the data file is unchanged.
    MigrationFailed(String),
    /// SQLite reports the file is damaged or is not a database. It is left untouched.
    CorruptDatabase(String),
    /// Backup/restore (B2-09). Each failure mode is its own kind so the UI can say exactly what
    /// happened; every one of them leaves the live database untouched.
    /// The chosen file is not a MedTracker backup (wrong magic bytes).
    BackupNotABackup(String),
    /// The file ends before its header or encrypted data does (an interrupted copy or download).
    BackupTruncated(String),
    /// The file uses a backup format or encryption setting this app does not know (made by a newer app).
    BackupUnsupportedFormat(String),
    /// The backup holds data from a newer app version than this one can read.
    BackupNewerSchema(String),
    /// The passphrase does not match the one the backup was made with.
    BackupWrongPassphrase(String),
    /// The passphrase was right but the contents were changed or damaged, or are not a healthy database.
    BackupCorrupt(String),
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Storage(e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

pub fn invalid<T>(msg: &str) -> AppResult<T> {
    Err(AppError::Validation(msg.to_string()))
}
