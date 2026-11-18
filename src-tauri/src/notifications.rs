#![allow(dead_code)]
//! Notification adapter stub. Native reminders are not implemented in Phase 1.
//! Contract: an adapter may schedule/cancel OS requests but must never write dose
//! events, and must never log medication or dose details.

pub trait NotificationAdapter {
    fn schedule(&self, request_id: &str, due_at_utc_us: i64) -> Result<(), NotificationError>;
    fn cancel(&self, request_id: &str) -> Result<(), NotificationError>;
}

#[derive(Debug, PartialEq)]
pub enum NotificationError {
    Unsupported,
}

pub struct UnsupportedAdapter;

impl NotificationAdapter for UnsupportedAdapter {
    fn schedule(&self, _: &str, _: i64) -> Result<(), NotificationError> {
        Err(NotificationError::Unsupported)
    }
    fn cancel(&self, _: &str) -> Result<(), NotificationError> {
        Err(NotificationError::Unsupported)
    }
}
