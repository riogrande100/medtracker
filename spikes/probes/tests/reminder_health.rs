use std::collections::BTreeSet;

use medtracker_phase0_probes::health::*;
use medtracker_phase0_probes::reminder::*;

fn desired(id: &str, fire: i64) -> Desired {
    Desired { occurrence_id: id.into(), fire_utc: fire, schedule_version: 1 }
}
fn ok_state() -> AdapterState {
    AdapterState { available: true, permission: Permission::Granted, exact_alarms: Some(true) }
}
const LIM: Limits = Limits { horizon_s: 7 * 86_400, cap: 3 };

#[test]
fn schedules_missing_within_cap_and_horizon() {
    let d: Vec<_> = (1..=5).map(|i| desired(&format!("o{i}"), 1000 + i * 60)).collect();
    let p = reconcile(&ok_state(), &d, &[], &BTreeSet::new(), 1000, LIM);
    assert_eq!(p.schedule.len(), 3);
    assert_eq!(p.status, ReminderStatus::Active(Timing::Exact));
}

#[test]
fn permission_denied_schedules_nothing_and_stays_neutral() {
    let st = AdapterState { permission: Permission::Denied, ..ok_state() };
    let p = reconcile(&st, &[desired("o1", 2000)], &[], &BTreeSet::new(), 1000, LIM);
    assert!(p.schedule.is_empty());
    assert_eq!(p.status, ReminderStatus::Degraded(DegradedReason::PermissionDenied));
    assert_no_missed_wording(&p);
}

#[test]
fn unavailable_and_undetermined() {
    let st = AdapterState { available: false, ..ok_state() };
    let p = reconcile(&st, &[desired("o1", 2000)], &[], &BTreeSet::new(), 1000, LIM);
    assert_eq!(p.status, ReminderStatus::Degraded(DegradedReason::ServiceUnavailable));
    assert!(p.schedule.is_empty());
    let st = AdapterState { permission: Permission::NotDetermined, ..ok_state() };
    let p = reconcile(&st, &[desired("o1", 2000)], &[], &BTreeSet::new(), 1000, LIM);
    assert_eq!(p.status, ReminderStatus::Degraded(DegradedReason::PermissionNotDetermined));
}

#[test]
fn exact_alarm_lost_after_acceptance() {
    let st = AdapterState { exact_alarms: Some(false), ..ok_state() };
    let accepted = BTreeSet::from(["o1".to_string()]);
    let p = reconcile(&st, &[desired("o1", 2000)], &[], &accepted, 1000, LIM);
    assert_eq!(p.status, ReminderStatus::Degraded(DegradedReason::ExactAlarmLost));
    assert_no_missed_wording(&p);
}

#[test]
fn exact_alarm_unavailable_is_approximate_not_degraded() {
    let st = AdapterState { exact_alarms: Some(false), ..ok_state() };
    let p = reconcile(&st, &[desired("o1", 2000)], &[], &BTreeSet::new(), 1000, LIM);
    assert_eq!(p.status, ReminderStatus::Active(Timing::Approximate));
}

#[test]
fn requests_removed_by_system_are_detected_and_recreated() {
    let accepted = BTreeSet::from(["o1".to_string(), "o2".to_string()]);
    let pending = [Pending { request_id: 7, occurrence_id: "o2".into(), fire_utc: 2060, schedule_version: 1 }];
    let d = [desired("o1", 2000), desired("o2", 2060)];
    let p = reconcile(&ok_state(), &d, &pending, &accepted, 1000, LIM);
    assert_eq!(p.status, ReminderStatus::Degraded(DegradedReason::RequestsRemoved { count: 1 }));
    assert_eq!(p.keep, vec![7]);
    assert_eq!(p.schedule, vec![desired("o1", 2000)]);
}

#[test]
fn stale_version_or_instant_is_cancelled_and_rescheduled() {
    let pending = [Pending { request_id: 1, occurrence_id: "o1".into(), fire_utc: 1500, schedule_version: 1 }];
    let p = reconcile(&ok_state(), &[desired("o1", 2000)], &pending, &BTreeSet::new(), 1000, LIM);
    assert_eq!(p.cancel, vec![1]);
    assert_eq!(p.schedule.len(), 1);
}

#[test]
fn duplicate_pending_request_is_cancelled() {
    let mk = |id| Pending { request_id: id, occurrence_id: "o1".into(), fire_utc: 2000, schedule_version: 1 };
    let p = reconcile(&ok_state(), &[desired("o1", 2000)], &[mk(1), mk(2)], &BTreeSet::new(), 1000, LIM);
    assert_eq!(p.keep, vec![1]);
    assert_eq!(p.cancel, vec![2]);
}

fn assert_no_missed_wording(p: &Plan) {
    for n in &p.notices {
        let n = n.to_lowercase();
        assert!(!n.contains("missed") && !n.contains("not taken") && !n.contains("failed to take"), "{n}");
    }
}

#[test]
fn all_notices_avoid_missed_wording() {
    for perm in [Permission::Granted, Permission::Denied, Permission::NotDetermined] {
        for available in [true, false] {
            for ex in [None, Some(true), Some(false)] {
                for acc in [BTreeSet::new(), BTreeSet::from(["o1".to_string()])] {
                    let st = AdapterState { available, permission: perm, exact_alarms: ex };
                    assert_no_missed_wording(&reconcile(&st, &[desired("o1", 2000)], &[], &acc, 1000, LIM));
                }
            }
        }
    }
}

#[test]
fn notification_events_write_zero_doses() {
    let mut l = Ledger::default();
    l.note_request_accepted("o1");
    for ev in [
        NotificationEvent::Delivered,
        NotificationEvent::Opened,
        NotificationEvent::Dismissed,
        NotificationEvent::ActionTapped("taken".into()),
        NotificationEvent::StaleFiredAfterBoot,
    ] {
        assert_eq!(l.apply_notification_event("o1", &ev), 0);
    }
    assert!(l.events().is_empty());
    assert_eq!(l.status("o1", 100, 200), OccurrenceStatus::NotLogged);
    assert_eq!(l.status("o1", 300, 200), OccurrenceStatus::Upcoming);
}

#[test]
fn only_explicit_user_entry_creates_dose() {
    let mut l = Ledger::default();
    l.record(ExplicitUserEntry::from_user_action(Some("o1"), DoseKind::Taken));
    assert_eq!(l.status("o1", 100, 200), OccurrenceStatus::Taken);
    l.record(ExplicitUserEntry::from_user_action(Some("o1"), DoseKind::Skipped));
    assert_eq!(l.status("o1", 100, 200), OccurrenceStatus::Conflicting);
}

#[test]
fn health_never_creates_doses_for_any_combination() {
    use Auth::*;
    use Signal::*;
    use Source::*;
    for s in [HealthKit, HealthConnectFitness, HealthConnectMedicalRecords, GoogleCloudApi] {
        for a in [Granted, LimitedWindow, Denied, NotDetermined, Unavailable, Revoked, Expired, SourceRemoved] {
            for g in [ClaimTaken, ClaimSkipped, ReminderPipelineStatus, MedicationStatement, NoRecords] {
                let o = classify(s, a, g);
                assert_eq!(o.ledger_doses_created, 0);
                assert!(!o.may_change_occurrence_status);
                if g == NoRecords {
                    assert_eq!(o.absence, Absence::Unknown);
                }
                if s == GoogleCloudApi {
                    assert_eq!(o.effect, Effect::BlockedByConstraint);
                }
                if matches!(a, Denied | NotDetermined | Unavailable | Revoked | Expired | SourceRemoved) && s != GoogleCloudApi {
                    assert_eq!(o.effect, Effect::Ignore);
                }
                if g == ReminderPipelineStatus {
                    assert_ne!(o.effect, Effect::ExternalClaim);
                }
            }
        }
    }
}
