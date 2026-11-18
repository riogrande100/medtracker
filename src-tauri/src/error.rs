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
