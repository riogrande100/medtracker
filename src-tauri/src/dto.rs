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
    /// Daily local times, "HH:MM". Empty means as-needed.
    pub times: Vec<String>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TodayStatus {
    Upcoming,
    Due,
    Taken,
    Skipped,
    Conflict,
}

#[derive(Debug, Serialize, Clone)]
pub struct TodayItem {
    pub slot_id: String,
    pub medication_id: String,
    pub name: String,
    pub strength: Option<String>,
    pub form: Option<String>,
    pub local_date: String,
    pub local_time: String,
    pub status: TodayStatus,
    /// Active (not undone) event ids for this slot today.
    pub event_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RecordDose {
    pub medication_id: String,
    pub kind: DoseKind,
    pub slot_id: Option<String>,
    /// Local date of the slot, "YYYY-MM-DD". Required with slot_id.
    pub local_date: Option<String>,
    /// Backdated local datetime "YYYY-MM-DDTHH:MM"; absent means now.
    pub occurred_local: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CorrectDose {
    pub event_id: String,
    pub kind: Option<DoseKind>,
    pub occurred_local: Option<String>,
    pub note: Option<String>,
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
    pub times: Vec<String>,
}
