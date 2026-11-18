use crate::domain::schedule::{ScheduleMode, ScheduleState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoseKind {
    Take,
    Skip,
}

impl DoseKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            DoseKind::Take => "take",
            DoseKind::Skip => "skip",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "take" => Some(DoseKind::Take),
            "skip" => Some(DoseKind::Skip),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct NewMedication {
    pub name: String,
    pub strength: Option<String>,
    pub form: Option<String>,
    /// Daily local times, "HH:MM", resolved in the device's zone. Empty means as-needed. For
    /// weekdays, intervals or another zone, create the schedule with `create_schedule`.
    pub times: Vec<String>,
}

/// Derived from the occurrence plus its active dose events and the clock; never stored.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TodayStatus {
    /// Later today, not yet at its scheduled time.
    Upcoming,
    /// Scheduled time reached, same local day, nothing recorded.
    Due,
    Taken,
    Skipped,
    /// Active take and skip both linked: shown for review, never silently resolved.
    Conflict,
    /// An earlier local day with nothing recorded. Deliberately neutral ("not logged"): the app
    /// never says "missed" or "overdue" (spec invariant 6).
    EarlierNotLogged,
}

#[derive(Debug, Serialize, Clone)]
pub struct TodayItem {
    /// The explicit link target for Taken/Skip (spec invariant 10).
    pub occurrence_id: String,
    pub schedule_id: String,
    pub medication_id: String,
    pub name: String,
    pub strength: Option<String>,
    pub form: Option<String>,
    pub mode: ScheduleMode,
    pub scheduled_at_utc_us: i64,
    /// Local date and time of the resolved instant in the occurrence's own zone.
    pub local_date: String,
    pub local_time: String,
    pub zone_id: Option<String>,
    pub zone_abbreviation: Option<String>,
    pub offset_seconds: i32,
    /// The intended wall time did not exist (clock change) and was moved.
    pub adjusted: bool,
    /// The same instant on the device's clock, present only when it reads differently.
    pub device_time: Option<String>,
    pub status: TodayStatus,
    /// Active (not undone) event ids linked to this occurrence.
    pub event_ids: Vec<String>,
}

// ---- schedules ----

#[derive(Debug, Deserialize, Clone)]
pub struct SlotInput {
    /// "HH:MM"
    pub time: String,
    /// ISO weekdays, 1 = Monday ... 7 = Sunday. Empty = every day.
    #[serde(default)]
    pub weekdays: Vec<u8>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ScheduleInput {
    pub mode: ScheduleMode,
    /// `home_zone`: required IANA zone id. Must be absent for `elapsed_interval`.
    pub zone_id: Option<String>,
    #[serde(default)]
    pub slots: Vec<SlotInput>,
    /// `elapsed_interval`: seconds between doses.
    pub interval_seconds: Option<i64>,
    /// `elapsed_interval`: the first dose, "YYYY-MM-DDTHH:MM" on the device clock. Absent = now
    /// when creating, or keep the current anchor when editing.
    pub anchor_local: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NewSchedule {
    pub medication_id: String,
    pub definition: ScheduleInput,
}

#[derive(Debug, Deserialize)]
pub struct EditSchedule {
    pub schedule_id: String,
    pub definition: ScheduleInput,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct SlotDto {
    pub time: String,
    pub weekdays: Vec<u8>,
}

/// One expected occurrence shown before saving, in the schedule summary, or as "next due".
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct PreviewOccurrence {
    pub scheduled_at_utc_us: i64,
    pub local_date: String,
    /// Resolved local time (after any clock-change adjustment).
    pub local_time: String,
    pub offset_seconds: i32,
    pub zone_id: Option<String>,
    pub zone_abbreviation: Option<String>,
    pub adjusted: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct ScheduleSummary {
    pub id: String,
    pub state: ScheduleState,
    pub version_number: i64,
    pub mode: ScheduleMode,
    pub zone_id: Option<String>,
    pub slots: Vec<SlotDto>,
    pub interval_seconds: Option<i64>,
    pub anchor_utc_us: Option<i64>,
    pub effective_from_utc_us: i64,
    /// The next seven occurrences while active; empty when paused or ended.
    pub next: Vec<PreviewOccurrence>,
}

#[derive(Debug, Serialize, Clone)]
pub struct NextDue {
    pub medication_id: String,
    pub name: String,
    pub strength: Option<String>,
    pub occurrence: PreviewOccurrence,
}

#[derive(Debug, Deserialize)]
pub struct RecordDose {
    pub medication_id: String,
    pub kind: DoseKind,
    /// The scheduled occurrence this action is for (the explicit link). Absent = unscheduled dose.
    pub occurrence_id: Option<String>,
    /// Backdated local datetime "YYYY-MM-DDTHH:MM"; absent means now.
    pub occurred_local: Option<String>,
    pub note: Option<String>,
    /// Only meaningful when `occurred_local` falls in an hour that happens twice (clocks going
    /// back). `true` selects the second (later) occurrence; absent/false selects the earlier.
    #[serde(default)]
    pub second_occurrence: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CorrectDose {
    pub event_id: String,
    pub kind: Option<DoseKind>,
    pub occurred_local: Option<String>,
    pub note: Option<String>,
    /// As `RecordDose::second_occurrence`. When `occurred_local` equals the entry's own current
    /// local time and this is absent, the stored instant is kept untouched.
    #[serde(default)]
    pub second_occurrence: Option<bool>,
}

/// Position in History (newest first); pass the previous page's `next_cursor` as `before`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct HistoryCursor {
    pub occurred_at_utc_us: i64,
    pub entered_at_utc_us: i64,
    pub event_id: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct HistoryPage {
    pub entries: Vec<HistoryEntry>,
    /// Present when older entries exist.
    pub next_cursor: Option<HistoryCursor>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Revision {
    pub revision_number: i64,
    pub changed_at_utc_us: i64,
    pub reason: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct HistoryEntry {
    pub event_id: String,
    pub medication_id: String,
    /// Medication name/strength as snapshotted when the dose was logged.
    pub name: String,
    pub strength: Option<String>,
    pub kind: DoseKind,
    /// Soft-undone (reversible). False for deleted tombstones.
    pub undone: bool,
    /// Privacy-deleted tombstone: note and revision payloads are gone; cannot be restored.
    pub deleted: bool,
    pub occurred_at_utc_us: i64,
    pub entered_at_utc_us: i64,
    pub original_local: String,
    pub original_offset_seconds: i32,
    /// IANA zone the entry was recorded in; None when only a fixed offset was known.
    pub original_zone_id: Option<String>,
    pub current_local: String,
    /// Offset in force at the entry's (possibly corrected) instant.
    pub current_offset_seconds: i32,
    pub note: Option<String>,
    pub slot_time: Option<String>,
    pub revisions: Vec<Revision>,
}

#[derive(Debug, Serialize, Clone)]
pub struct MedicationSummary {
    pub id: String,
    pub name: String,
    pub strength: Option<String>,
    pub form: Option<String>,
    pub archived: bool,
    /// Most recent schedule for this medication; None = as needed.
    pub schedule: Option<ScheduleSummary>,
}
