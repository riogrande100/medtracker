//! Throwaway model of the `ReminderScheduler` port from docs/product-spec.md section 6/8.
//!
//! It exists to pin down, in executable form, the *decision rules* the device spikes must
//! confirm: how the app degrades when the OS integration is denied, unavailable, expired, or
//! has silently lost its requests, and the invariant that no notification event writes a dose.
//! It performs no OS calls and is not a production integration.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Permission {
    NotDetermined,
    Granted,
    Denied,
}

/// What the native integration reports about itself when the app reconciles.
#[derive(Clone, Debug)]
pub struct AdapterState {
    /// False when the platform/plugin cannot schedule (desktop plugin is immediate-only, plugin
    /// missing, API error, OS too old).
    pub available: bool,
    pub permission: Permission,
    /// Android: `AlarmManager.canScheduleExactAlarms()`. `None` where not applicable or unknown.
    pub exact_alarms: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Desired {
    pub occurrence_id: String,
    pub fire_utc: i64,
    pub schedule_version: u32,
}

/// A request the OS (or plugin) reports as pending.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pending {
    pub request_id: i32,
    pub occurrence_id: String,
    pub fire_utc: i64,
    pub schedule_version: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Rolling horizon. The database schedule stays authoritative; the OS only gets a window.
    pub horizon_s: i64,
    /// Never enqueue more than this many requests (set from measured per-platform capacity).
    pub cap: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Timing {
    Exact,
    /// OS may defer delivery (for example Android without exact-alarm access).
    Approximate,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DegradedReason {
    PermissionNotDetermined,
    PermissionDenied,
    ServiceUnavailable,
    /// Exact-alarm access was lost after requests were accepted (the system cancels them).
    ExactAlarmLost,
    /// Requests the app recorded as accepted are no longer pending (reboot, force-stop,
    /// reinstall, restore, user cleared them).
    RequestsRemoved { count: usize },
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ReminderStatus {
    Active(Timing),
    Degraded(DegradedReason),
}

#[derive(Clone, Debug)]
pub struct Plan {
    pub schedule: Vec<Desired>,
    pub cancel: Vec<i32>,
    pub keep: Vec<i32>,
    pub status: ReminderStatus,
    /// Neutral user-facing text. Never claims a dose was missed or not taken.
    pub notices: Vec<String>,
}

pub fn reconcile(
    state: &AdapterState,
    desired: &[Desired],
    pending: &[Pending],
    accepted_before: &BTreeSet<String>,
    now_utc: i64,
    limits: Limits,
) -> Plan {
    let mut window: Vec<&Desired> =
        desired.iter().filter(|d| d.fire_utc > now_utc && d.fire_utc <= now_utc + limits.horizon_s).collect();
    window.sort_by_key(|d| (d.fire_utc, d.occurrence_id.clone()));
    window.truncate(limits.cap);

    let mut keep = Vec::new();
    let mut cancel = Vec::new();
    let mut matched: BTreeSet<&str> = BTreeSet::new();
    for p in pending {
        let hit = window.iter().find(|d| {
            d.occurrence_id == p.occurrence_id && d.fire_utc == p.fire_utc && d.schedule_version == p.schedule_version
        });
        match hit {
            Some(d) if !matched.contains(d.occurrence_id.as_str()) => {
                matched.insert(d.occurrence_id.as_str());
                keep.push(p.request_id);
            }
            // Wrong version/instant, outside the window, duplicate, or no longer desired.
            _ => cancel.push(p.request_id),
        }
    }

    let usable = state.available && state.permission == Permission::Granted;
    let missing: Vec<&Desired> = window.iter().copied().filter(|d| !matched.contains(d.occurrence_id.as_str())).collect();
    let schedule: Vec<Desired> = if usable { missing.iter().map(|d| (*d).clone()).collect() } else { Vec::new() };

    let removed = accepted_before
        .iter()
        .filter(|id| {
            let still_future = desired.iter().any(|d| &d.occurrence_id == *id && d.fire_utc > now_utc);
            still_future && !pending.iter().any(|p| &p.occurrence_id == *id)
        })
        .count();

    let mut notices = Vec::new();
    let status = if !state.available {
        notices.push("Reminders can't be scheduled on this device. Your schedule and log still work.".to_string());
        ReminderStatus::Degraded(DegradedReason::ServiceUnavailable)
    } else if state.permission == Permission::NotDetermined {
        notices.push("Reminders are off until you allow notifications.".to_string());
        ReminderStatus::Degraded(DegradedReason::PermissionNotDetermined)
    } else if state.permission == Permission::Denied {
        notices.push("Notifications are turned off for this app. Your schedule and log still work.".to_string());
        ReminderStatus::Degraded(DegradedReason::PermissionDenied)
    } else if state.exact_alarms == Some(false) && removed > 0 && pending.is_empty() {
        notices.push("Exact reminder timing was turned off. Reminders may be late or missing.".to_string());
        ReminderStatus::Degraded(DegradedReason::ExactAlarmLost)
    } else if removed > 0 {
        notices.push("Some reminders were removed by the system and were re-created.".to_string());
        ReminderStatus::Degraded(DegradedReason::RequestsRemoved { count: removed })
    } else if state.exact_alarms == Some(false) {
        notices.push("Reminder timing is approximate on this device.".to_string());
        ReminderStatus::Active(Timing::Approximate)
    } else {
        ReminderStatus::Active(Timing::Exact)
    };

    Plan { schedule, cancel, keep, status, notices }
}

/// Bookkeeping states from spec section 8. Delivery is only ever `Unknown` unless the OS says so.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NotificationState {
    Scheduled,
    RequestAccepted,
    Delivered,
    Opened,
    Actioned,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotificationEvent {
    Delivered,
    Opened,
    Dismissed,
    ActionTapped(String),
    /// Android plugin re-fires past-due requests ~15 s after boot (source-read, unverified).
    StaleFiredAfterBoot,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DoseKind {
    Taken,
    Skipped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoseEvent {
    pub occurrence_id: Option<String>,
    pub kind: DoseKind,
}

/// Only the UI layer constructs this, from an explicit user action.
#[derive(Clone, Debug)]
pub struct ExplicitUserEntry {
    occurrence_id: Option<String>,
    kind: DoseKind,
}

impl ExplicitUserEntry {
    pub fn from_user_action(occurrence_id: Option<&str>, kind: DoseKind) -> Self {
        Self { occurrence_id: occurrence_id.map(str::to_string), kind }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OccurrenceStatus {
    Upcoming,
    Taken,
    Skipped,
    NotLogged,
    Conflicting,
}

#[derive(Default)]
pub struct Ledger {
    events: Vec<DoseEvent>,
    notifications: BTreeMap<String, NotificationState>,
}

impl Ledger {
    pub fn record(&mut self, entry: ExplicitUserEntry) {
        self.events.push(DoseEvent { occurrence_id: entry.occurrence_id, kind: entry.kind });
    }

    pub fn events(&self) -> &[DoseEvent] {
        &self.events
    }

    pub fn notification_state(&self, occurrence_id: &str) -> Option<NotificationState> {
        self.notifications.get(occurrence_id).copied()
    }

    pub fn note_request_accepted(&mut self, occurrence_id: &str) {
        self.notifications.insert(occurrence_id.to_string(), NotificationState::RequestAccepted);
    }

    /// Bookkeeping only. Returns the number of dose events written, which must always be 0.
    pub fn apply_notification_event(&mut self, occurrence_id: &str, event: &NotificationEvent) -> usize {
        let next = match event {
            NotificationEvent::Delivered => Some(NotificationState::Delivered),
            NotificationEvent::Opened => Some(NotificationState::Opened),
            NotificationEvent::ActionTapped(_) => Some(NotificationState::Actioned),
            NotificationEvent::Dismissed | NotificationEvent::StaleFiredAfterBoot => None,
        };
        if let Some(n) = next {
            self.notifications.insert(occurrence_id.to_string(), n);
        }
        0
    }

    /// There is deliberately no "missed" status: an unlogged past occurrence is `NotLogged`.
    pub fn status(&self, occurrence_id: &str, fire_utc: i64, now_utc: i64) -> OccurrenceStatus {
        let linked: Vec<_> = self.events.iter().filter(|e| e.occurrence_id.as_deref() == Some(occurrence_id)).collect();
        let taken = linked.iter().any(|e| e.kind == DoseKind::Taken);
        let skipped = linked.iter().any(|e| e.kind == DoseKind::Skipped);
        match (taken, skipped) {
            (true, true) => OccurrenceStatus::Conflicting,
            (true, false) => OccurrenceStatus::Taken,
            (false, true) => OccurrenceStatus::Skipped,
            (false, false) if fire_utc > now_utc => OccurrenceStatus::Upcoming,
            (false, false) => OccurrenceStatus::NotLogged,
        }
    }
}
