use crate::application::{ledger, medication};
use crate::clock::{parse_local, TimeCtx, Zone};
use crate::error::AppError;
use crate::dto::*;
use crate::notifications::{NotificationAdapter, NotificationError, UnsupportedAdapter};
use crate::storage::{self, migrations};
use rusqlite::Connection;

const OFFSET: i32 = -6 * 3600;

fn utc_at(zone: &Zone, local: &str) -> i64 {
    zone.resolve(parse_local(local).unwrap()).unwrap().utc_us
}

fn ctx(local: &str) -> TimeCtx {
    TimeCtx::fixed(utc_at(&Zone::fixed(OFFSET), local), OFFSET)
}

fn ctx_in(zone: &str, local: &str) -> TimeCtx {
    let z = Zone::iana(zone).unwrap();
    TimeCtx { now_utc_us: utc_at(&z, local), zone: z }
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
            second_occurrence: None,
        },
    )
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
fn undo_is_reversible_and_keeps_everything() {
    let (mut c, med) = setup();
    let now = "2026-03-10T09:30";
    let item = ledger::today(&c, ctx(now)).unwrap().remove(0);
    let id = ledger::record(
        &mut c,
        ctx(now),
        RecordDose {
            medication_id: med.clone(),
            kind: DoseKind::Take,
            slot_id: Some(item.slot_id.clone()),
            local_date: Some(item.local_date.clone()),
            occurred_local: None,
            note: Some("took with food".into()),
            second_occurrence: None,
        },
    )
    .unwrap();
    ledger::undo(&mut c, ctx(now), &id).unwrap();
    assert_eq!(ledger::today(&c, ctx(now)).unwrap()[0].status, TodayStatus::Due);
    // Undone entries can't be edited, and restoring puts the same entry (note intact) back.
    assert!(ledger::correct(&mut c, ctx(now), CorrectDose { event_id: id.clone(), kind: None, occurred_local: None, note: Some("x".into()), second_occurrence: None }).is_err());
    ledger::restore(&mut c, ctx(now), &id).unwrap();
    assert_eq!(ledger::today(&c, ctx(now)).unwrap()[0].status, TodayStatus::Taken);
    let h = &ledger::history(&c, 10).unwrap()[0];
    assert!(!h.undone && !h.deleted);
    assert_eq!(h.note.as_deref(), Some("took with food"));
    assert_eq!(h.revisions.iter().map(|r| r.reason.as_str()).collect::<Vec<_>>(), ["undo", "restore"]);
    assert!(ledger::restore(&mut c, ctx(now), &id).is_err(), "only undone entries can be restored");

    // Restore is refused if the slot was logged again with the same outcome in the meantime.
    ledger::undo(&mut c, ctx(now), &id).unwrap();
    record(&mut c, now, &med, DoseKind::Take, &item).unwrap();
    assert!(ledger::restore(&mut c, ctx(now), &id).is_err());
}

fn db_bytes_contain(path: &std::path::Path, needle: &str) -> bool {
    let bytes = std::fs::read(path).unwrap();
    bytes.windows(needle.len()).any(|w| w == needle.as_bytes())
}

#[test]
fn privacy_deletion_is_permanent_and_leaves_no_note_text() {
    let path = std::env::temp_dir().join(format!("medtracker-del-{}.sqlite", uuid::Uuid::new_v4()));
    let mut c = storage::open(&path).unwrap();
    let med = medication::create(
        &mut c,
        ctx("2026-03-01T08:00"),
        NewMedication { name: "Metformin".into(), strength: None, form: None, times: vec!["08:00".into()] },
    )
    .unwrap();
    let now = "2026-03-10T09:30";
    let item = ledger::today(&c, ctx(now)).unwrap().remove(0);
    let secret1 = "SECRET-NOTE-ALPHA-7731";
    let secret2 = "SECRET-NOTE-BRAVO-9042";
    let id = ledger::record(
        &mut c,
        ctx(now),
        RecordDose {
            medication_id: med.clone(),
            kind: DoseKind::Take,
            slot_id: Some(item.slot_id.clone()),
            local_date: Some(item.local_date.clone()),
            occurred_local: None,
            note: Some(secret1.into()),
            second_occurrence: None,
        },
    )
    .unwrap();
    // A correction copies the note into before/after revision payloads.
    ledger::correct(&mut c, ctx(now), CorrectDose { event_id: id.clone(), kind: None, occurred_local: None, note: Some(secret2.into()), second_occurrence: None }).unwrap();
    assert!(ledger::history(&c, 10).unwrap()[0].revisions[0].after.as_ref().unwrap().contains(secret2));
    assert!(db_bytes_contain(&path, secret1) && db_bytes_contain(&path, secret2), "sanity: notes are in the file before deletion");

    ledger::undo(&mut c, ctx(now), &id).unwrap(); // deletion works on undone entries too
    ledger::delete_entry(&mut c, ctx(now), &id).unwrap();

    let h = &ledger::history(&c, 10).unwrap()[0];
    assert!(h.deleted && !h.undone);
    assert_eq!(h.event_id, id, "tombstone keeps the id");
    assert!(h.note.is_none());
    assert!(h.revisions.iter().all(|r| r.before.is_none() && r.after.is_none()));
    assert_eq!(h.revisions.last().unwrap().reason, "delete");
    assert_eq!(ledger::today(&c, ctx(now)).unwrap()[0].status, TodayStatus::Due, "tombstone is not a dose");
    // Not reversible, not editable, not deletable twice.
    assert!(ledger::restore(&mut c, ctx(now), &id).is_err());
    assert!(ledger::undo(&mut c, ctx(now), &id).is_err());
    assert!(ledger::correct(&mut c, ctx(now), CorrectDose { event_id: id.clone(), kind: None, occurred_local: None, note: Some("new".into()), second_occurrence: None }).is_err());
    assert!(ledger::delete_entry(&mut c, ctx(now), &id).is_err());
    // The slot can be logged afresh.
    record(&mut c, now, &med, DoseKind::Take, &item).unwrap();

    drop(c);
    assert!(!db_bytes_contain(&path, secret1), "first note text must not remain in the database file");
    assert!(!db_bytes_contain(&path, secret2), "corrected note text must not remain in the database file");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_entry_unknown_id_is_not_found() {
    let (mut c, _) = setup();
    assert!(matches!(ledger::delete_entry(&mut c, ctx("2026-03-10T09:30"), "nope"), Err(AppError::NotFound(_))));
}

fn plain(med: &str, when: &str) -> RecordDose {
    RecordDose { medication_id: med.into(), kind: DoseKind::Take, slot_id: None, local_date: None, occurred_local: Some(when.into()), note: None, second_occurrence: None }
}

#[test]
fn backdated_entry_stores_offset_and_zone_for_the_entered_date() {
    let (mut c, med) = setup();
    // Device "now" is July (EDT, -4h) in New York; the user logs a dose from January (EST, -5h).
    let now = ctx_in("America/New_York", "2026-07-20T12:00");
    ledger::record(&mut c, now, plain(&med, "2026-01-15T09:00")).unwrap();
    let h = &ledger::history(&c, 10).unwrap()[0];
    assert_eq!(h.original_offset_seconds, -5 * 3600, "January offset, not today's");
    assert_eq!(h.current_offset_seconds, -5 * 3600);
    assert_eq!(h.original_zone_id.as_deref(), Some("America/New_York"));
    assert_eq!(h.original_local, "2026-01-15T09:00");
    assert_eq!(h.occurred_at_utc_us, utc_at(&Zone::fixed(0), "2026-01-15T14:00"));
    assert_eq!(h.current_local, "2026-01-15T09:00");
}

#[test]
fn dst_gap_and_overlap_are_resolved_deterministically_on_record() {
    let (mut c, med) = setup();
    let now = || ctx_in("America/New_York", "2026-12-01T12:00");
    ledger::record(&mut c, now(), plain(&med, "2026-03-08T02:30")).unwrap(); // gap
    ledger::record(&mut c, now(), plain(&med, "2026-11-01T01:30")).unwrap(); // overlap
    let h = ledger::history(&c, 10).unwrap();
    let gap = h.iter().find(|e| e.original_local.starts_with("2026-03-08")).unwrap();
    assert_eq!(gap.original_local, "2026-03-08T03:30", "skipped time moves forward by the gap");
    assert_eq!(gap.original_offset_seconds, -4 * 3600);
    let fold = h.iter().find(|e| e.original_local.starts_with("2026-11-01")).unwrap();
    assert_eq!(fold.original_local, "2026-11-01T01:30");
    assert_eq!(fold.original_offset_seconds, -4 * 3600, "repeated time uses the earlier instant");
    assert_eq!(fold.occurred_at_utc_us, utc_at(&Zone::fixed(0), "2026-11-01T05:30"));
}

#[test]
fn southern_hemisphere_backdating_and_fixed_offset_fallback() {
    let (mut c, med) = setup();
    // Sydney: now is July (AEST +10); backdating to January must use AEDT +11.
    ledger::record(&mut c, ctx_in("Australia/Sydney", "2026-07-20T12:00"), plain(&med, "2026-01-15T09:00")).unwrap();
    let h = &ledger::history(&c, 10).unwrap()[0];
    assert_eq!(h.original_offset_seconds, 11 * 3600);
    assert_eq!(h.original_zone_id.as_deref(), Some("Australia/Sydney"));

    // Fixed-offset device (unknown IANA id): no zone stored, constant offset.
    let fixed = TimeCtx { now_utc_us: utc_at(&Zone::fixed(0), "2026-07-20T12:00"), zone: Zone::from_name_or_fixed(Some("Nowhere/Land"), -6 * 3600) };
    ledger::record(&mut c, fixed, plain(&med, "2026-01-15T09:00")).unwrap();
    let h = ledger::history(&c, 10).unwrap().into_iter().find(|e| e.original_zone_id.is_none()).unwrap();
    assert_eq!(h.original_offset_seconds, -6 * 3600);
}

#[test]
fn correction_across_dst_uses_the_new_dates_offset() {
    let (mut c, med) = setup();
    let now = || ctx_in("America/New_York", "2026-12-01T12:00");
    ledger::record(&mut c, now(), plain(&med, "2026-03-07T20:00")).unwrap(); // EST (-5)
    let id = ledger::history(&c, 1).unwrap()[0].event_id.clone();
    ledger::correct(&mut c, now(), CorrectDose { event_id: id, kind: None, occurred_local: Some("2026-03-09T20:00".into()), note: None, second_occurrence: None }).unwrap(); // EDT (-4)
    let h = &ledger::history(&c, 1).unwrap()[0];
    assert_eq!(h.original_offset_seconds, -5 * 3600, "original context is preserved");
    assert_eq!(h.current_offset_seconds, -4 * 3600);
    assert_eq!(h.current_local, "2026-03-09T20:00");
    assert_eq!(h.occurred_at_utc_us, utc_at(&Zone::fixed(0), "2026-03-10T00:00"));
}

#[test]
fn stale_slot_dates_are_rejected_with_a_typed_error() {
    let (mut c, med) = setup();
    let item = ledger::today(&c, ctx("2026-03-10T23:50")).unwrap().remove(0);
    assert_eq!(item.local_date, "2026-03-10");
    // Still valid just before midnight.
    record(&mut c, "2026-03-10T23:59", &med, DoseKind::Take, &item).unwrap();
    // After midnight the same request (Today was left open) is stale.
    let err = record(&mut c, "2026-03-11T00:01", &med, DoseKind::Skip, &item).unwrap_err();
    assert!(matches!(err, AppError::StaleDate(_)), "{err:?}");
    // Also stale if a later day, and a future date is not "today" either.
    assert!(matches!(record(&mut c, "2026-03-12T09:00", &med, DoseKind::Skip, &item), Err(AppError::StaleDate(_))));
    let mut future = item.clone();
    future.local_date = "2026-03-11".into();
    assert!(matches!(record(&mut c, "2026-03-10T23:00", &med, DoseKind::Skip, &future), Err(AppError::StaleDate(_))));
    // Nothing was written for the stale attempts.
    assert_eq!(ledger::history(&c, 10).unwrap().len(), 1);
    // Wire format the UI relies on.
    assert_eq!(serde_json::to_string(&AppError::StaleDate("x".into())).unwrap(), r#"{"kind":"stale_date","message":"x"}"#);
}

// ---- storage: upgrades, snapshots and recoverable failures ----

fn temp_db(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("medtracker-{tag}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("user-data.sqlite")
}

fn cleanup(path: &std::path::Path) {
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

fn siblings(path: &std::path::Path) -> Vec<String> {
    let mut v: Vec<String> =
        std::fs::read_dir(path.parent().unwrap()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    v.sort();
    v
}

/// A real v1 database (migration 001 only) with an active entry, an undone entry and a note.
fn make_v1_fixture(path: &std::path::Path) {
    let c = Connection::open(path).unwrap();
    c.execute_batch(migrations::MIGRATIONS[0]).unwrap();
    c.execute_batch(
        "PRAGMA user_version = 1;
         INSERT INTO user_medication (id, current_version_id, status, created_at_utc, updated_at_utc) VALUES ('m1', NULL, 'active', 1, 1);
         INSERT INTO medication_version (id, medication_id, version_number, effective_from_utc, name, strength, form, source_kind, created_at_utc)
           VALUES ('v1', 'm1', 1, 1, 'Metformin', '500 mg', 'tablet', 'manual', 1);
         UPDATE user_medication SET current_version_id = 'v1';
         INSERT INTO dose_event (id, medication_id, medication_version_id, kind, occurred_at_utc, entered_at_utc, original_local, original_offset_seconds, note, current_revision, deleted)
           VALUES ('e-active', 'm1', 'v1', 'take', 1773133800000000, 1773133800000000, '2026-03-10T03:30', -21600, 'old note', 0, 0),
                  ('e-undone', 'm1', 'v1', 'skip', 1773047400000000, 1773047400000000, '2026-03-09T03:30', -21600, NULL, 1, 1);
         INSERT INTO dose_event_revision (id, dose_event_id, revision_number, changed_at_utc, reason) VALUES ('r1', 'e-undone', 1, 1773047500000000, 'undo');",
    )
    .unwrap();
}

#[test]
fn v1_database_is_snapshotted_then_migrated_to_latest_without_data_loss() {
    let path = temp_db("upgrade");
    make_v1_fixture(&path);
    let before = std::fs::read(&path).unwrap();

    let c = storage::open(&path).unwrap();
    assert_eq!(migrations::user_version(&c).unwrap(), migrations::latest_version());
    assert!(migrations::latest_version() >= 2);
    let fk: i64 = c.query_row("PRAGMA foreign_keys", [], |r| r.get(0)).unwrap();
    assert_eq!(fk, 1);

    // v1 `deleted` (undo) became `undone`; nothing is a privacy tombstone; offsets carried over.
    let h = ledger::history(&c, 10).unwrap();
    assert_eq!(h.len(), 2);
    let active = h.iter().find(|e| e.event_id == "e-active").unwrap();
    assert!(!active.undone && !active.deleted);
    assert_eq!(active.note.as_deref(), Some("old note"));
    assert_eq!(active.current_offset_seconds, -21600);
    assert!(active.original_zone_id.is_none(), "legacy rows have only a fixed offset");
    let undone = h.iter().find(|e| e.event_id == "e-undone").unwrap();
    assert!(undone.undone && !undone.deleted);
    assert_eq!(undone.revisions[0].reason, "undo");
    // A verified pre-migration snapshot sits next to the database and is byte-for-byte the old data.
    let snaps: Vec<String> = siblings(&path).into_iter().filter(|n| n.ends_with(".bak")).collect();
    assert_eq!(snaps, ["user-data.sqlite.pre-migration-v1-to-v2.bak"]);
    let snap = Connection::open(path.parent().unwrap().join(&snaps[0])).unwrap();
    assert_eq!(migrations::user_version(&snap).unwrap(), 1);
    let n: i64 = snap.query_row("SELECT count(*) FROM dose_event", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 2);
    let note: String = snap.query_row("SELECT note FROM dose_event WHERE id='e-active'", [], |r| r.get(0)).unwrap();
    assert_eq!(note, "old note");
    drop(snap);
    assert_ne!(before, std::fs::read(&path).unwrap(), "live database was upgraded");
    assert!(siblings(&path).iter().all(|n| !n.ends_with(".tmp")), "no temporary copy left behind");
    drop(c);

    // The next launch opens the upgraded database cleanly; the snapshot has done its job and is
    // removed (it would otherwise keep plaintext notes forever).
    let mut c = storage::open(&path).unwrap();
    assert!(siblings(&path).iter().all(|n| !n.contains(".bak")), "{:?}", siblings(&path));
    // The new revision reasons are accepted after the upgrade.
    ledger::restore(&mut c, ctx("2026-03-12T09:00"), "e-undone").unwrap();
    drop(c);
    cleanup(&path);
}

#[test]
fn fresh_database_needs_no_snapshot_and_reopen_keeps_data() {
    let path = temp_db("fresh");
    {
        let mut c = storage::open(&path).unwrap();
        medication::create(&mut c, ctx("2026-03-01T08:00"), NewMedication { name: "A".into(), strength: None, form: None, times: vec![] }).unwrap();
    }
    let c = storage::open(&path).unwrap();
    assert_eq!(medication::list(&c).unwrap().len(), 1);
    assert!(siblings(&path).iter().all(|n| !n.ends_with(".bak")));
    cleanup(&path);
}

#[test]
fn newer_database_is_refused_and_left_untouched() {
    let path = temp_db("newer");
    make_v1_fixture(&path);
    Connection::open(&path).unwrap().execute_batch("PRAGMA user_version = 99; CREATE TABLE from_the_future (x TEXT); INSERT INTO from_the_future VALUES ('keep me');").unwrap();
    let before = std::fs::read(&path).unwrap();
    let names_before = siblings(&path);

    let err = storage::open(&path).unwrap_err();
    assert!(matches!(err, AppError::NewerDatabase(_)), "{err:?}");
    assert_eq!(before, std::fs::read(&path).unwrap(), "file bytes unchanged");
    assert_eq!(names_before, siblings(&path), "no snapshot, journal or other file created");
    cleanup(&path);
}

#[test]
fn corrupt_database_reports_a_typed_error_and_is_left_untouched() {
    // Not a database at all.
    let path = temp_db("garbage");
    std::fs::write(&path, b"this is definitely not an sqlite file, just some text padding it out to a few hundred bytes ........................................................................................................................................................................................................................................................").unwrap();
    let before = std::fs::read(&path).unwrap();
    let err = storage::open(&path).unwrap_err();
    assert!(matches!(err, AppError::CorruptDatabase(_)), "{err:?}");
    assert_eq!(before, std::fs::read(&path).unwrap());
    assert_eq!(siblings(&path), ["user-data.sqlite"]);
    cleanup(&path);

    // A real database with a damaged table page (header intact, so only integrity_check can tell).
    let path = temp_db("damaged");
    {
        let mut c = storage::open(&path).unwrap();
        for i in 0..40 {
            medication::create(&mut c, ctx("2026-03-01T08:00"), NewMedication { name: format!("Med {i}"), strength: None, form: None, times: vec![] }).unwrap();
        }
    }
    let mut bytes = std::fs::read(&path).unwrap();
    let page = 4096;
    assert!(bytes.len() > page * 3);
    // Scribble over the interior of every page after the first; keeps page 1 (schema) readable.
    for p in 1..bytes.len() / page {
        for b in &mut bytes[p * page + 8..(p + 1) * page] {
            *b = 0xFF;
        }
    }
    std::fs::write(&path, &bytes).unwrap();
    let err = storage::open(&path).unwrap_err();
    assert!(matches!(err, AppError::CorruptDatabase(_)), "{err:?}");
    assert_eq!(bytes, std::fs::read(&path).unwrap());
    assert!(siblings(&path).iter().all(|n| !n.ends_with(".bak")), "no snapshot of a damaged file");
    cleanup(&path);
}

#[test]
fn failed_migration_rolls_back_everything_and_keeps_a_snapshot() {
    let path = temp_db("failmig");
    make_v1_fixture(&path);
    let before = std::fs::read(&path).unwrap();
    // Step 2 does real work and then fails: the whole upgrade must roll back as one unit.
    let bad: Vec<&str> = vec![migrations::MIGRATIONS[0], "ALTER TABLE dose_event ADD COLUMN extra TEXT; SELECT * FROM table_that_does_not_exist;"];
    let err = storage::open_with(&path, &bad).unwrap_err();
    match &err {
        AppError::MigrationFailed(m) => assert!(m.contains("not been changed") && m.contains(".bak"), "{m}"),
        e => panic!("{e:?}"),
    }
    assert_eq!(before, std::fs::read(&path).unwrap(), "database file is exactly as it was");
    let c = Connection::open(&path).unwrap();
    assert_eq!(migrations::user_version(&c).unwrap(), 1);
    assert!(c.prepare("SELECT extra FROM dose_event").is_err(), "partial DDL was rolled back");
    assert_eq!(siblings(&path).iter().filter(|n| n.ends_with(".bak")).count(), 1);
    drop(c);
    // The real migrations still apply afterwards.
    assert_eq!(migrations::user_version(&storage::open(&path).unwrap()).unwrap(), migrations::latest_version());
    cleanup(&path);
}

#[test]
fn startup_failure_is_surfaced_through_db_state_not_a_crash() {
    let path = temp_db("startup");
    make_v1_fixture(&path);
    Connection::open(&path).unwrap().execute_batch("PRAGMA user_version = 99;").unwrap();
    let db = crate::commands::Db::open(&path);
    assert!(matches!(db.startup_error, Some(AppError::NewerDatabase(_))));
    assert!(db.conn.lock().unwrap().is_none());
    assert_eq!(serde_json::to_value(db.startup_error.as_ref().unwrap()).unwrap()["kind"], "newer_database");
    cleanup(&path);
}

/// Every file in the database's directory (database, snapshots, journals, WAL) that contains `needle`.
fn files_containing(path: &std::path::Path, needle: &str) -> Vec<String> {
    siblings(path)
        .into_iter()
        .filter(|n| std::fs::read(path.parent().unwrap().join(n)).map(|b| b.windows(needle.len()).any(|w| w == needle.as_bytes())).unwrap_or(false))
        .collect()
}

#[test]
fn pre_upgrade_snapshot_never_outlives_a_privacy_deletion() {
    let secret = "SECRET-V1-NOTE-5521";
    let path = temp_db("snap-privacy");
    make_v1_fixture(&path);
    Connection::open(&path).unwrap().execute("UPDATE dose_event SET note=?1 WHERE id='e-active'", [secret]).unwrap();

    // Migration launch: the snapshot is the only way back from a bad upgrade, so it must survive
    // a deletion made during this same launch, and the UI is told it exists.
    let db = crate::commands::Db::open(&path);
    assert!(db.startup_error.is_none());
    assert!(db.upgrade_copy_exists());
    {
        let mut guard = db.conn.lock().unwrap();
        ledger::delete_entry(guard.as_mut().unwrap(), ctx("2026-03-12T09:00"), "e-active").unwrap();
    }
    // An older snapshot (e.g. one a previous launch could not remove) plus its journal is purged on deletion...
    let dir = path.parent().unwrap();
    std::fs::write(dir.join("user-data.sqlite.pre-migration-v0-to-v1.bak"), format!("old copy {secret}")).unwrap();
    std::fs::write(dir.join("user-data.sqlite.pre-migration-v0-to-v1.bak-journal"), format!("old journal {secret}")).unwrap();
    db.purge_older_snapshots();
    assert_eq!(
        siblings(&path).into_iter().filter(|n| n.contains(".bak")).collect::<Vec<_>>(),
        ["user-data.sqlite.pre-migration-v1-to-v2.bak"],
        "only this launch's own snapshot remains"
    );
    assert!(db.upgrade_copy_exists());
    assert_eq!(files_containing(&path, secret), ["user-data.sqlite.pre-migration-v1-to-v2.bak"], "the live file no longer holds the note");
    drop(db);

    // ...and the next launch removes this launch's snapshot too: no file of any kind keeps the note.
    let db = crate::commands::Db::open(&path);
    assert!(db.startup_error.is_none());
    assert!(!db.upgrade_copy_exists());
    drop(db);
    assert!(siblings(&path).iter().all(|n| !n.contains(".bak")), "{:?}", siblings(&path));
    assert!(files_containing(&path, secret).is_empty(), "{:?}", files_containing(&path, secret));
    cleanup(&path);
}

#[test]
fn deleting_a_medication_also_purges_older_snapshots() {
    let path = temp_db("snap-medication");
    {
        let _ = storage::open(&path).unwrap();
    }
    let db = crate::commands::Db::open(&path);
    let med = {
        let mut g = db.conn.lock().unwrap();
        medication::create(g.as_mut().unwrap(), ctx("2026-03-01T08:00"), NewMedication { name: "A".into(), strength: None, form: None, times: vec![] }).unwrap()
    };
    std::fs::write(path.parent().unwrap().join("user-data.sqlite.pre-migration-v1-to-v2-2.bak"), "stale copy").unwrap();
    assert!(!db.upgrade_copy_exists());
    {
        let mut g = db.conn.lock().unwrap();
        medication::delete(g.as_mut().unwrap(), &med).unwrap();
    }
    db.purge_older_snapshots();
    assert!(siblings(&path).iter().all(|n| !n.contains(".bak")));
    cleanup(&path);
}

#[test]
fn repeated_failed_upgrades_reuse_one_snapshot() {
    let path = temp_db("failmig-repeat");
    make_v1_fixture(&path);
    let bad: Vec<&str> = vec![migrations::MIGRATIONS[0], "SELECT * FROM table_that_does_not_exist;"];
    for _ in 0..3 {
        assert!(matches!(storage::open_with(&path, &bad), Err(AppError::MigrationFailed(_))));
    }
    let names = siblings(&path);
    assert_eq!(names.iter().filter(|n| n.contains(".bak")).collect::<Vec<_>>(), ["user-data.sqlite.pre-migration-v1-to-v2.bak"], "{names:?}");
    assert!(names.iter().all(|n| !n.ends_with(".tmp")));
    // The eventual successful upgrade replaces it; the launch after that removes it.
    drop(storage::open(&path).unwrap());
    assert_eq!(siblings(&path).iter().filter(|n| n.contains(".bak")).count(), 1);
    drop(storage::open(&path).unwrap());
    assert_eq!(siblings(&path).iter().filter(|n| n.contains(".bak")).count(), 0);
    cleanup(&path);
}

#[test]
fn newer_database_is_reported_as_newer_even_if_otherwise_unreadable_for_integrity() {
    // The version check runs before integrity_check and its message never contains debug output.
    let path = temp_db("newer-first");
    {
        let mut c = storage::open(&path).unwrap();
        for i in 0..40 {
            medication::create(&mut c, ctx("2026-03-01T08:00"), NewMedication { name: format!("Med {i}"), strength: None, form: None, times: vec![] }).unwrap();
        }
        c.execute_batch("PRAGMA user_version = 99;").unwrap();
    }
    let mut bytes = std::fs::read(&path).unwrap();
    for p in 1..bytes.len() / 4096 {
        for b in &mut bytes[p * 4096 + 8..(p + 1) * 4096] {
            *b = 0xFF;
        }
    }
    std::fs::write(&path, &bytes).unwrap();
    assert!(matches!(storage::open(&path), Err(AppError::NewerDatabase(_))));
    assert_eq!(bytes, std::fs::read(&path).unwrap());
    cleanup(&path);

    // A failed safety copy yields a plain-language message, not Rust debug syntax.
    let path = temp_db("snapfail");
    make_v1_fixture(&path);
    // A directory squatting on the temporary name makes the snapshot impossible.
    std::fs::create_dir(path.parent().unwrap().join("user-data.sqlite.pre-migration-v1-to-v2.bak.tmp")).unwrap();
    let err = storage::open(&path).unwrap_err();
    match err {
        AppError::MigrationFailed(m) => assert!(m.contains("safety copy") && !m.contains("Corrupt") && !m.contains("{") && !m.contains("\\\""), "{m}"),
        e => panic!("{e:?}"),
    }
    cleanup(&path);
}

#[test]
fn hot_rollback_journal_from_a_crash_is_recovered_not_reported_as_corruption() {
    let src = temp_db("hotjournal-src");
    {
        let mut c = storage::open(&src).unwrap();
        medication::create(&mut c, ctx("2026-03-01T08:00"), NewMedication { name: "Keep".into(), strength: None, form: None, times: vec![] }).unwrap();
        c.execute_batch("CREATE TABLE filler (x BLOB); INSERT INTO filler VALUES (zeroblob(100));").unwrap();
    }
    let crashed = |tag: &str| {
        let dst = temp_db(tag);
        std::fs::remove_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        dst
    };
    let (a, b) = (crashed("hotjournal-a"), crashed("hotjournal-b"));
    {
        // A big transaction on a tiny page cache spills dirty pages to the database file (after
        // journaling the originals). Copying both files mid-transaction is what a crash leaves.
        let c = Connection::open(&src).unwrap();
        c.execute_batch("PRAGMA synchronous = OFF; PRAGMA cache_size = 5; BEGIN;").unwrap();
        for _ in 0..400 {
            c.execute("INSERT INTO filler VALUES (zeroblob(2000))", []).unwrap();
        }
        let journal = std::path::PathBuf::from(format!("{}-journal", src.display()));
        assert!(journal.exists() && std::fs::metadata(&journal).unwrap().len() > 512, "transaction left a journal behind");
        for dst in [&a, &b] {
            std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
            std::fs::copy(&src, dst).unwrap();
            std::fs::copy(&journal, format!("{}-journal", dst.display())).unwrap();
        }
        c.execute_batch("ROLLBACK").unwrap();
    }
    // Sanity: the simulation really is a hot journal, which a read-only open cannot play back.
    let ro = Connection::open_with_flags(&b, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert!(ro.query_row("SELECT count(*) FROM filler", [], |r| r.get::<_, i64>(0)).is_err(), "read-only open cannot recover a hot journal");
    drop(ro);

    let c = storage::open(&a).expect("hot journal is recovered on a read-write open");
    let rows: i64 = c.query_row("SELECT count(*) FROM filler", [], |r| r.get(0)).unwrap();
    assert_eq!(rows, 1, "the interrupted transaction was rolled back to the last committed state");
    assert_eq!(medication::list(&c).unwrap().len(), 1);
    assert!(!std::path::PathBuf::from(format!("{}-journal", a.display())).exists(), "journal consumed");
    cleanup(&src);
    cleanup(&a);
    cleanup(&b);
}

#[test]
fn fall_back_overlap_round_trips_and_second_occurrence_is_selectable() {
    let (mut c, med) = setup();
    let now = || ctx_in("America/New_York", "2026-12-01T12:00");
    let utc = |s: &str| utc_at(&Zone::fixed(0), s);
    let rec = |second: Option<bool>| RecordDose { second_occurrence: second, ..plain(&med, "2026-11-01T01:30") };
    let id = ledger::record(&mut c, now(), rec(Some(true))).unwrap();
    let h = ledger::history(&c, 1).unwrap().remove(0);
    assert_eq!((h.current_offset_seconds, h.occurred_at_utc_us), (-5 * 3600, utc("2026-11-01T06:30")), "second (EST) occurrence");
    assert_eq!(h.current_local, "2026-11-01T01:30");

    // Editing only the note while the form re-submits the same wall time must not move the instant.
    let fix = |id: &str, local: Option<&str>, note: Option<&str>, second: Option<bool>| CorrectDose {
        event_id: id.into(),
        kind: None,
        occurred_local: local.map(String::from),
        note: note.map(String::from),
        second_occurrence: second,
    };
    ledger::correct(&mut c, now(), fix(&id, Some("2026-11-01T01:30"), Some("hi"), None)).unwrap();
    let h = ledger::history(&c, 1).unwrap().remove(0);
    assert_eq!((h.current_offset_seconds, h.occurred_at_utc_us), (-5 * 3600, utc("2026-11-01T06:30")), "stored instant kept");
    assert_eq!(h.note.as_deref(), Some("hi"));
    // Same wall time and nothing else is still "nothing changed".
    assert!(ledger::correct(&mut c, now(), fix(&id, Some("2026-11-01T01:30"), None, None)).is_err());

    // Explicitly choosing the earlier occurrence moves it an hour earlier and is recorded.
    ledger::correct(&mut c, now(), fix(&id, Some("2026-11-01T01:30"), None, Some(false))).unwrap();
    let h = ledger::history(&c, 1).unwrap().remove(0);
    assert_eq!((h.current_offset_seconds, h.occurred_at_utc_us), (-4 * 3600, utc("2026-11-01T05:30")));
    assert_eq!(h.current_local, "2026-11-01T01:30");
    assert!(h.revisions.last().unwrap().before.as_ref().unwrap().contains("-18000"));
    assert!(h.revisions.last().unwrap().after.as_ref().unwrap().contains("-14400"));
    // ...and a re-submit of that wall time with no choice keeps the earlier instant.
    ledger::correct(&mut c, now(), fix(&id, Some("2026-11-01T01:30"), Some("again"), None)).unwrap();
    assert_eq!(ledger::history(&c, 1).unwrap()[0].occurred_at_utc_us, utc("2026-11-01T05:30"));

    // Backdating INTO the repeated hour from elsewhere: default earlier, optional second.
    let other = ledger::record(&mut c, now(), plain(&med, "2026-10-31T20:00")).unwrap();
    ledger::correct(&mut c, now(), fix(&other, Some("2026-11-01T01:15"), None, None)).unwrap();
    let find = |c: &Connection, id: &str| ledger::history(c, 10).unwrap().into_iter().find(|e| e.event_id == id).unwrap();
    assert_eq!(find(&c, &other).occurred_at_utc_us, utc("2026-11-01T05:15"));
    ledger::correct(&mut c, now(), fix(&other, Some("2026-11-01T01:15"), None, Some(true))).unwrap();
    assert_eq!(find(&c, &other).occurred_at_utc_us, utc("2026-11-01T06:15"));
    // The flag is ignored outside a repeated hour.
    ledger::correct(&mut c, now(), fix(&other, Some("2026-11-01T09:00"), None, Some(true))).unwrap();
    assert_eq!(find(&c, &other).occurred_at_utc_us, utc("2026-11-01T14:00"));
}

#[test]
fn history_pages_reach_every_entry_in_order() {
    let (mut c, med) = setup();
    let now = || ctx("2026-03-20T12:00");
    // Seven entries; three share an identical time AND entry time so the cursor must tie-break on id.
    for t in ["2026-03-01T08:00", "2026-03-02T08:00", "2026-03-03T08:00", "2026-03-04T08:00", "2026-03-04T08:00", "2026-03-04T08:00", "2026-03-05T08:00"] {
        ledger::record(&mut c, now(), plain(&med, t)).unwrap();
    }
    let all = ledger::history_page(&c, Some(100), None).unwrap();
    assert_eq!(all.entries.len(), 7);
    assert!(all.next_cursor.is_none());
    let mut seen = Vec::new();
    let mut cursor = None;
    let mut pages = 0;
    loop {
        let p = ledger::history_page(&c, Some(3), cursor).unwrap();
        pages += 1;
        assert!(p.entries.len() <= 3);
        seen.extend(p.entries.iter().map(|e| e.event_id.clone()));
        cursor = p.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(pages, 3);
    assert_eq!(seen, all.entries.iter().map(|e| e.event_id.clone()).collect::<Vec<_>>(), "no gaps, no repeats, same order");
    // An exact multiple has no trailing empty page.
    assert!(ledger::history_page(&c, Some(7), None).unwrap().next_cursor.is_none());
    // The oldest entry is reachable and deletable.
    let oldest = all.entries.last().unwrap().event_id.clone();
    ledger::delete_entry(&mut c, now(), &oldest).unwrap();
    assert!(ledger::history_page(&c, Some(100), None).unwrap().entries.last().unwrap().deleted);
    // Limits are clamped, never an error or an unbounded read.
    assert_eq!(ledger::history_page(&c, Some(0), None).unwrap().entries.len(), 1);
    assert_eq!(serde_json::to_value(all.entries.len()).unwrap(), 7);
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
        CorrectDose { event_id: id.clone(), kind: Some(DoseKind::Skip), occurred_local: Some("2026-03-10T08:05".into()), note: Some("wrong tap".into()), second_occurrence: None },
    )
    .unwrap();
    let h = &ledger::history(&c, 10).unwrap()[0];
    assert_eq!(h.kind, DoseKind::Skip);
    assert_eq!(h.original_local, "2026-03-10T09:30");
    assert_eq!(h.current_local, "2026-03-10T08:05");
    assert_eq!(h.original_offset_seconds, OFFSET);
    assert_eq!(h.current_offset_seconds, OFFSET);
    assert_eq!(h.revisions.len(), 1);
    assert!(h.revisions[0].before.as_ref().unwrap().contains("\"take\""));
    assert!(h.revisions[0].after.as_ref().unwrap().contains("\"skip\""));
    assert!(h.entered_at_utc_us < ctx("2026-03-10T10:00").now_utc_us);
    assert_eq!(ledger::today(&c, ctx(now)).unwrap()[0].status, TodayStatus::Skipped);
    // No-op corrections are rejected rather than creating empty revisions.
    assert!(ledger::correct(&mut c, ctx(now), CorrectDose { event_id: id, kind: None, occurred_local: None, note: None, second_occurrence: None }).is_err());
}

#[test]
fn backdating_works_and_future_is_rejected() {
    let (mut c, med) = setup();
    let now = || ctx("2026-03-10T09:30");
    let mk = |when: &str| RecordDose {
        medication_id: med.clone(),
        kind: DoseKind::Take,
        slot_id: None,
        local_date: None,
        occurred_local: Some(when.into()),
        note: None,
        second_occurrence: None,
    };
    ledger::record(&mut c, now(), mk("2026-03-08T21:15")).unwrap();
    let h = &ledger::history(&c, 10).unwrap()[0];
    assert_eq!(h.original_local, "2026-03-08T21:15");
    assert!(h.entered_at_utc_us > h.occurred_at_utc_us);
    assert!(ledger::record(&mut c, now(), mk("2026-03-12T09:00")).is_err());
    assert!(ledger::record(&mut c, now(), mk("not a date")).is_err());
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
    ledger::correct(&mut c, ctx(now), CorrectDose { event_id: id, kind: Some(DoseKind::Skip), occurred_local: None, note: None, second_occurrence: None }).unwrap();

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
