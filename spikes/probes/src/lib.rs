//! Throwaway Phase 0 probes for MedTracker.
//!
//! Everything here is host-side. Nothing in this crate proves behavior on an iOS, Android,
//! Windows, or macOS *device*; `evidence` exists to make that distinction machine-checkable.

pub mod evidence;
pub mod health;
pub mod reminder;
pub mod tz;
