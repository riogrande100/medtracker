use crate::application::{ledger, medication};
use crate::clock::{parse_local, utc_from_local, TimeCtx};
use crate::dto::*;
use crate::notifications::{NotificationAdapter, NotificationError, UnsupportedAdapter};
use crate::storage::{self, migrations};
use rusqlite::Connection;

const OFFSET: i32 = -6 * 3600;

fn ctx(local: &str) -> TimeCtx {
    TimeCtx { now_utc_us: utc_from_local(parse_local(local).unwrap(), OFFSET), offset_seconds: OFFSET }
}

fn setup() -> (Connection, String) {
    let mut c = storage::open_memory().unwrap();
    let id = medication::create(
        &mut c,
        ctx("2026-03-01T08:00"),
        NewMedication { name: "Metformin".into(), strength: Some("500 mg".into()), form: Some("tablet".into()), times: vec!["08:00".into()] },
    )
    .unwrap();
    (c, id)
}

fn record(c: &mut Connection, now: &str, med: &str, kind: DoseKind, slot: &TodayItem) -> Result<String, crate::error::AppError> {
    ledger::record(
        c,
        ctx(now),
        RecordDose {
            medication_id: med.into(),
            kind,
            slot_id: Some(slot.slot_id.clone()),
            local_date: Some(slot.local_date.clone()),
            occurred_local: None,
            note: None,
        },
    )
}

#[test]
fn migrations_are_versioned_idempotent_and_refuse_downgrade() {
    let path = std::env::temp_dir().join(format!("medtracker-{}.sqlite", uuid::Uuid::new_v4()));
    {
        let c = storage::open(&path).unwrap();
        let v: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, migrations::latest_version());
        let fk: i64 = c.query_row("PRAGMA foreign_keys", [], |r| r.get(0)).unwrap();
        assert_eq!(fk, 1);
    }
    storage::open(&path).unwrap(); // reopen applies nothing and keeps data
    {
        let c = Connection::open(&path).unwrap();
        c.execute_batch("PRAGMA user_version = 99").unwrap();
    }
    assert!(storage::open(&path).is_err());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn due_then_taken_then_undo_restores_due() {
    let (mut c, med) = setup();
    let now = "2026-03-10T09:30";
    let item = ledger::today(&c, ctx(now)).unwrap().remove(0);
    assert_eq!(item.status, TodayStatus::Due);
    let id = record(&mut c, now, &med, DoseKind::Take, &item).unwrap();
    assert_eq!(ledger::today(&c, ctx(now)).unwrap()[0].status, TodayStatus::Taken);

    ledger::undo(&mut c, ctx(now), &id).unwrap();
    assert_eq!(ledger::today(&c, ctx(now)).unwrap()[0].status, TodayStatus::Due);
    let h = ledger::history(&c, 10).unwrap();
    assert!(h[0].undone);
    assert_eq!(h[0].revisions[0].reason, "undo");
    assert!(ledger::undo(&mut c, ctx(now), &id).is_err());
}

#[test]
fn upcoming_before_scheduled_time() {
    let (c, _) = setup();
    assert_eq!(ledger::today(&c, ctx("2026-03-10T07:00")).unwrap()[0].status, TodayStatus::Upcoming);
}

#[test]
fn skip_is_distinct_from_taken_and_conflict_is_flagged() {
    let (mut c, med) = setup();
    let now = "2026-03-10T09:30";
    let item = ledger::today(&c, ctx(now)).unwrap().remove(0);
    record(&mut c, now, &med, DoseKind::Skip, &item).unwrap();
    assert_eq!(ledger::today(&c, ctx(now)).unwrap()[0].status, TodayStatus::Skipped);
    assert!(record(&mut c, now, &med, DoseKind::Skip, &item).is_err());
    record(&mut c, now, &med, DoseKind::Take, &item).unwrap();
    assert_eq!(ledger::today(&c, ctx(now)).unwrap()[0].status, TodayStatus::Conflict);
}

#[test]
fn correction_keeps_original_context_and_appends_revision() {
    let (mut c, med) = setup();
    let now = "2026-03-10T09:30";
    let item = ledger::today(&c, ctx(now)).unwrap().remove(0);
    let id = record(&mut c, now, &med, DoseKind::Take, &item).unwrap();
    ledger::correct(
        &mut c,
        ctx("2026-03-10T10:00"),
        CorrectDose { event_id: id.clone(), kind: Some(DoseKind::Skip), occurred_local: Some("2026-03-10T08:05".into()), note: Some("wrong tap".into()) },
    )
    .unwrap();
    let h = &ledger::history(&c, 10).unwrap()[0];
    assert_eq!(h.kind, DoseKind::Skip);
    assert_eq!(h.original_local, "2026-03-10T09:30");
    assert_eq!(h.current_local, "2026-03-10T08:05");
    assert_eq!(h.original_offset_seconds, OFFSET);
    assert_eq!(h.revisions.len(), 1);
    assert!(h.revisions[0].before.as_ref().unwrap().contains("\"take\""));
    assert!(h.revisions[0].after.as_ref().unwrap().contains("\"skip\""));
    assert!(h.entered_at_utc_us < ctx("2026-03-10T10:00").now_utc_us);
    assert_eq!(ledger::today(&c, ctx(now)).unwrap()[0].status, TodayStatus::Skipped);
    // No-op corrections are rejected rather than creating empty revisions.
    assert!(ledger::correct(&mut c, ctx(now), CorrectDose { event_id: id, kind: None, occurred_local: None, note: None }).is_err());
}

#[test]
fn backdating_works_and_future_is_rejected() {
    let (mut c, med) = setup();
    let now = ctx("2026-03-10T09:30");
    let mk = |when: &str| RecordDose {
        medication_id: med.clone(),
        kind: DoseKind::Take,
        slot_id: None,
        local_date: None,
        occurred_local: Some(when.into()),
        note: None,
    };
    ledger::record(&mut c, now, mk("2026-03-08T21:15")).unwrap();
    let h = &ledger::history(&c, 10).unwrap()[0];
    assert_eq!(h.original_local, "2026-03-08T21:15");
    assert!(h.entered_at_utc_us > h.occurred_at_utc_us);
    assert!(ledger::record(&mut c, now, mk("2026-03-12T09:00")).is_err());
    assert!(ledger::record(&mut c, now, mk("not a date")).is_err());
}

#[test]
fn medication_versions_are_immutable_snapshots() {
    let (mut c, med) = setup();
    let now = "2026-03-10T09:30";
    let item = ledger::today(&c, ctx(now)).unwrap().remove(0);
    record(&mut c, now, &med, DoseKind::Take, &item).unwrap();
    assert!(c.execute("UPDATE medication_version SET name='Changed'", []).is_err());
    assert_eq!(ledger::history(&c, 10).unwrap()[0].name, "Metformin");
}

#[test]
fn archive_keeps_history_but_delete_removes_everything() {
    let (mut c, med) = setup();
    let now = "2026-03-10T09:30";
    let item = ledger::today(&c, ctx(now)).unwrap().remove(0);
    let id = record(&mut c, now, &med, DoseKind::Take, &item).unwrap();
    ledger::correct(&mut c, ctx(now), CorrectDose { event_id: id, kind: Some(DoseKind::Skip), occurred_local: None, note: None }).unwrap();

    medication::archive(&c, ctx(now), &med).unwrap();
    assert!(ledger::today(&c, ctx(now)).unwrap().is_empty());
    assert_eq!(ledger::history(&c, 10).unwrap().len(), 1);
    assert!(medication::list(&c).unwrap()[0].archived);

    medication::delete(&mut c, &med).unwrap();
    assert!(ledger::history(&c, 10).unwrap().is_empty());
    assert!(medication::list(&c).unwrap().is_empty());
}

#[test]
fn validation_rejects_bad_input() {
    let mut c = storage::open_memory().unwrap();
    let bad = |name: &str, times: Vec<&str>| NewMedication {
        name: name.into(),
        strength: None,
        form: None,
        times: times.into_iter().map(String::from).collect(),
    };
    assert!(medication::create(&mut c, ctx("2026-03-01T08:00"), bad("  ", vec![])).is_err());
    assert!(medication::create(&mut c, ctx("2026-03-01T08:00"), bad("A", vec!["25:00"])).is_err());
    assert!(medication::create(&mut c, ctx("2026-03-01T08:00"), bad("A", vec!["08:00", "08:00"])).is_err());
    assert!(medication::create(&mut c, ctx("2026-03-01T08:00"), bad("A", vec![])).is_ok());
}

#[test]
fn notification_adapter_is_an_unsupported_stub() {
    let a = UnsupportedAdapter;
    assert_eq!(a.schedule("r", 0), Err(NotificationError::Unsupported));
    assert_eq!(a.cancel("r"), Err(NotificationError::Unsupported));
}
