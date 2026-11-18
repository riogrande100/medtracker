use serde::Serialize;

#[derive(Debug, Serialize, PartialEq)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum AppError {
    Validation(String),
    NotFound(String),
    Storage(String),
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
