//! Descriptive reports and plain-file export (spec section 7, backlog B2-04 / B2-07).
//!
//! Everything here is counting. There is no score, no target and no judgement: an occurrence
//! with nothing recorded is "earlier, not logged" (never "missed" or "overdue"), and an
//! occurrence that has not reached its day yet is "later, not yet logged".
//!
//! Definitions used (shown in the UI and carried in every export):
//! * Occurrences are the active, derived scheduled opportunities whose stored schedule-local
//!   date (in each schedule's own zone) falls in the range. Superseded/cancelled ones are not.
//! * An occurrence is "taken" when it has an active take and no active skip, "skipped" with an
//!   active skip and no take, "conflicting" with both (excluded from taken/skipped until the
//!   user resolves it). Several takes linked to one occurrence count once.
//! * Unscheduled events are active events linked to no occurrence, grouped by the local day
//!   recorded with the event (its date and offset at the time it happened).
//! * Voided (undone) entries are excluded from every outcome and total, and are only counted
//!   separately. Privacy-deleted tombstones are likewise never outcomes; they are reported as
//!   existing entries whose details were erased, and are exported only as such.

pub mod commands;
pub mod export;
pub mod query;

#[cfg(test)]
mod tests;
