use super::export::{self, csv_text, ExportOptions, Format};
use super::query::{self, Report};
use crate::application::{ledger, medication, schedule};
use crate::clock::{parse_local, TimeCtx, Zone};
use crate::domain::schedule::ScheduleMode;
use crate::dto::*;
use crate::storage;
use rusqlite::Connection;

const NY: &str = "America/New_York";

fn at(local: &str) -> TimeCtx {
    let z = Zone::iana(NY).unwrap();
    TimeCtx { now_utc_us: z.resolve(parse_local(local).unwrap()).unwrap().utc_us, zone: z }
}

fn rec(c: &mut Connection, now: &str, med: &str, kind: DoseKind, occ: Option<String>, when: Option<&str>, note: Option<&str>) -> String {
    ledger::record(
        c,
        at(now),
        RecordDose { medication_id: med.into(), kind, occurrence_id: occ, occurred_local: when.map(Into::into), note: note.map(Into::into), second_occurrence: None },
    )
    .unwrap()
}

/// The section 10 fixture: A daily 02:30 New York (nonexistent on 03-08, so the gap rule gives 07:30Z) 03-07..03-10, B as-needed.
fn fixture() -> (Connection, String, String) {
    let mut c = storage::open_memory().unwrap();
    let a = medication::create(&mut c, at("2026-03-07T01:00"), NewMedication { name: "Daily tablet A".into(), strength: Some("5 mg".into()), form: None, times: vec![] }).unwrap();
    let b = medication::create(&mut c, at("2026-03-07T01:00"), NewMedication { name: "As-needed liquid B".into(), strength: Some("10 mg/mL".into()), form: None, times: vec![] }).unwrap();
    schedule::create(
        &mut c,
        &at("2026-03-07T01:00"),
        NewSchedule {
            medication_id: a.clone(),
            definition: ScheduleInput { mode: ScheduleMode::HomeZone, zone_id: Some(NY.into()), slots: vec![SlotInput { time: "02:30".into(), weekdays: vec![] }], interval_seconds: None, anchor_local: None },
        },
    )
    .unwrap();
    let occ_for = |c: &mut Connection, now: &str| -> String {
        let d = &now[..10];
        schedule::generate_range(c, &at(now), d, d).unwrap();
        ledger::today(c, at(now)).unwrap().into_iter().find(|i| i.local_date == d).unwrap().occurrence_id
    };
    let o7 = occ_for(&mut c, "2026-03-07T09:00");
    let e7 = rec(&mut c, "2026-03-07T09:00", &a, DoseKind::Take, Some(o7), None, None);
    // one correction on the March 7 entry
    ledger::correct(&mut c, at("2026-03-07T09:30"), CorrectDose { event_id: e7, kind: None, occurred_local: Some("2026-03-07T08:10".into()), note: None, second_occurrence: None }).unwrap();
    let o8 = occ_for(&mut c, "2026-03-08T09:00");
    rec(&mut c, "2026-03-08T09:00", &a, DoseKind::Skip, Some(o8), None, None);
    occ_for(&mut c, "2026-03-09T12:00"); // nothing recorded
    let o10 = occ_for(&mut c, "2026-03-10T09:00");
    rec(&mut c, "2026-03-10T09:00", &a, DoseKind::Take, Some(o10.clone()), None, None);
    // a second take linked to the same occurrence (distinct event, counts once)
    c.execute(
        "INSERT INTO dose_event (id, medication_id, medication_version_id, kind, occurred_at_utc, entered_at_utc, original_local, original_offset_seconds, occurred_offset_seconds, original_zone_id)
         SELECT 'extra-take', medication_id, medication_version_id, 'take', occurred_at_utc + 3600000000, entered_at_utc, original_local, original_offset_seconds, occurred_offset_seconds, original_zone_id
         FROM dose_event WHERE kind='take' AND occurred_at_utc > 1773000000000000 ORDER BY occurred_at_utc DESC LIMIT 1",
        [],
    )
    .unwrap();
    c.execute("INSERT INTO dose_event_occurrence (dose_event_id, occurrence_id, linked_at_utc) VALUES ('extra-take', ?1, 0)", [&o10]).unwrap();
    rec(&mut c, "2026-03-10T10:00", &b, DoseKind::Take, None, Some("2026-03-10T09:15"), Some("2 mL"));
    rec(&mut c, "2026-03-10T11:00", &b, DoseKind::Take, None, Some("2026-03-10T10:45"), Some("1.5 mL"));
    (c, a, b)
}

fn rpt(c: &Connection) -> Report {
    query::report(c, &at("2026-03-10T12:00"), "2026-03-07", "2026-03-10").unwrap()
}

#[test]
fn spec_fixture_counts() {
    let (c, a, b) = fixture();
    let r = rpt(&c);
    let t = &r.totals;
    assert_eq!((t.occurrences, t.taken, t.skipped, t.not_logged, t.conflicts, t.pending), (4, 2, 1, 1, 0, 0));
    assert_eq!((t.unscheduled_taken, t.unscheduled_skipped), (2, 0));
    let row = |id: &str| r.by_medication.iter().find(|m| m.medication_id == id).unwrap();
    assert_eq!(row(&a).counts.occurrences, 4);
    assert_eq!(row(&b).counts.occurrences, 0, "B has no scheduled denominator");
    assert_eq!(row(&b).counts.unscheduled_taken, 2);
    let day = |d: &str| r.by_day.iter().find(|x| x.date == d).unwrap().counts.clone();
    assert_eq!(day("2026-03-07").taken, 1);
    assert_eq!(day("2026-03-08").skipped, 1);
    assert_eq!(day("2026-03-09").not_logged, 1);
    assert_eq!(day("2026-03-10").taken, 1, "two takes on one occurrence count once");
    assert_eq!(day("2026-03-10").unscheduled_taken, 2);
    // March 8 spring-forward occurrence resolves to 07:30Z.
    let us: i64 = c.query_row("SELECT scheduled_at_utc FROM scheduled_occurrence WHERE local_date='2026-03-08'", [], |r| r.get(0)).unwrap();
    assert_eq!(us, 1_772_955_000_000_000);
    let adj: i64 = c.query_row("SELECT adjusted FROM scheduled_occurrence WHERE local_date='2026-03-08'", [], |r| r.get(0)).unwrap();
    assert_eq!(adj, 1);
    assert_eq!(jiff::Timestamp::from_microsecond(us).unwrap().to_string(), "2026-03-08T07:30:00Z");
}

#[test]
fn wording_has_no_judgement() {
    let (c, ..) = fixture();
    let text = serde_json::to_string(&rpt(&c)).unwrap().to_lowercase();
    for w in ["overdue", "missed", "adherence", "compliance", "score"] {
        // "not a score" is the one allowed negation in the definitions.
        let t = text.replace("not a score", "");
        assert!(!t.contains(w), "found {w}");
    }
}

#[test]
fn later_occurrences_are_pending_not_not_logged() {
    let (c, ..) = fixture();
    let r = query::report(&c, &at("2026-03-09T07:00"), "2026-03-07", "2026-03-09").unwrap();
    assert_eq!((r.totals.pending, r.totals.not_logged), (1, 0));
    let r = query::report(&c, &at("2026-03-10T07:00"), "2026-03-07", "2026-03-09").unwrap();
    assert_eq!((r.totals.pending, r.totals.not_logged), (0, 1));
}

#[test]
fn conflict_is_surfaced_and_not_counted_as_taken_or_skipped() {
    let (mut c, a, _) = fixture();
    let o: String = c.query_row("SELECT id FROM scheduled_occurrence WHERE local_date='2026-03-07'", [], |r| r.get(0)).unwrap();
    c.execute(
        "INSERT INTO dose_event (id, medication_id, medication_version_id, kind, occurred_at_utc, entered_at_utc, original_local, original_offset_seconds, occurred_offset_seconds)
         SELECT 'sk', medication_id, medication_version_id, 'skip', occurred_at_utc, entered_at_utc, original_local, original_offset_seconds, occurred_offset_seconds FROM dose_event WHERE medication_id=?1 LIMIT 1",
        [&a],
    )
    .unwrap();
    c.execute("INSERT INTO dose_event_occurrence VALUES ('sk', ?1, 0)", [&o]).unwrap();
    let t = rpt(&c).totals;
    assert_eq!((t.taken, t.skipped, t.conflicts, t.occurrences), (1, 1, 1, 4));
}

#[test]
fn voided_and_deleted_handling() {
    let (mut c, a, _) = fixture();
    let id: String = c.query_row("SELECT id FROM dose_event WHERE id='extra-take'", [], |r| r.get(0)).unwrap();
    ledger::undo(&mut c, at("2026-03-10T12:00"), &id).unwrap();
    let o8: String = c.query_row("SELECT dose_event_id FROM dose_event_occurrence l JOIN scheduled_occurrence o ON o.id=l.occurrence_id WHERE o.local_date='2026-03-08'", [], |r| r.get(0)).unwrap();
    ledger::delete_entry(&mut c, at("2026-03-10T12:00"), &o8).unwrap();
    let r = rpt(&c);
    assert_eq!(r.voided_entries, 1);
    assert_eq!(r.deleted_entries, 1);
    // the deleted skip no longer counts as skipped; the voided second take didn't change the count
    assert_eq!((r.totals.taken, r.totals.skipped, r.totals.not_logged), (2, 0, 2));
    let _ = a;
}

#[test]
fn csv_escaping_and_injection() {
    assert_eq!(csv_text("plain"), "plain");
    assert_eq!(csv_text("a,b"), "\"a,b\"");
    assert_eq!(csv_text("say \"hi\""), "\"say \"\"hi\"\"\"");
    assert_eq!(csv_text("line1\nline2"), "\"line1\nline2\"");
    assert_eq!(csv_text("a\r\nb"), "\"a\r\nb\"");
    for s in ["=SUM(A1)", "+1", "-1", "@cmd", "\tx"] {
        assert!(csv_text(s).starts_with('\''), "{s}");
    }
    assert_eq!(csv_text("=1,2"), "\"'=1,2\"");
}

fn opts(format: Format, notes: bool, history: bool) -> ExportOptions {
    ExportOptions { from: Some("2026-03-07".into()), to: Some("2026-03-10".into()), format, include_notes: notes, include_history: history }
}

#[test]
fn csv_export_notes_are_opt_in_and_numeric_cells_unguarded() {
    let (mut c, a, _) = fixture();
    rec(&mut c, "2026-03-10T11:30", &a, DoseKind::Take, None, Some("2026-03-10T11:00"), Some("=HYPERLINK(\"x\")"));
    let without = export::build(&c, &opts(Format::Csv, false, false), 0).unwrap().content;
    assert!(!without.contains("note"), "no note column");
    assert!(!without.contains("HYPERLINK") && !without.contains("2 mL"));
    let with = export::build(&c, &opts(Format::Csv, true, false), 0).unwrap().content;
    assert!(with.lines().next().unwrap().ends_with(",note"));
    assert!(with.contains("\"'=HYPERLINK(\"\"x\"\")\""));
    assert!(with.contains("2 mL"));
    assert!(with.contains("\r\n"));
    // offsets stay numeric (negative number not apostrophe-prefixed)
    assert!(with.contains(",-14400,") && !with.contains("'-14400"));
    assert!(with.lines().nth(1).unwrap().starts_with("1,"), "schema_version first");
    assert!(with.contains("Daily tablet A") && with.contains("5 mg") && with.contains("2026-03-10T13:15:00Z"));
}

#[test]
fn history_option_controls_voided_and_deleted_rows() {
    let (mut c, ..) = fixture();
    ledger::undo(&mut c, at("2026-03-10T12:00"), "extra-take").unwrap();
    let skip: String = c.query_row("SELECT id FROM dose_event WHERE kind='skip'", [], |r| r.get(0)).unwrap();
    ledger::delete_entry(&mut c, at("2026-03-10T12:00"), &skip).unwrap();
    let plain = export::rows(&c, &opts(Format::Json, true, false)).unwrap();
    assert!(plain.iter().all(|r| r.status == "active"));
    let hist = export::rows(&c, &opts(Format::Json, true, true)).unwrap();
    assert_eq!(hist.iter().filter(|r| r.status == "undone").count(), 1);
    let del: Vec<_> = hist.iter().filter(|r| r.status == "deleted").collect();
    assert_eq!(del.len(), 1);
    assert_eq!(del[0].note, None, "tombstone never carries a note");
    let json = export::build(&c, &opts(Format::Json, true, true), 1_773_133_800_000_000).unwrap().content;
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["includes_notes"], true);
    assert_eq!(v["entry_counts"]["deleted"], 1);
    assert_eq!(v["entry_counts"]["undone"], 1);
}

#[test]
fn json_without_notes_has_no_note_key_and_range_filters() {
    let (c, ..) = fixture();
    let json = export::build(&c, &opts(Format::Json, false, false), 0).unwrap().content;
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(v["events"].as_array().unwrap().iter().all(|e| e.get("note").is_none()));
    assert_eq!(v["events"].as_array().unwrap().len(), 6);
    let mut o = opts(Format::Json, false, false);
    o.from = Some("2026-03-08".into());
    o.to = Some("2026-03-08".into());
    assert_eq!(export::rows(&c, &o).unwrap().len(), 1);
}

#[test]
fn file_name_is_stable() {
    let n = export::file_name(1_773_133_800_042_000, "csv");
    assert!(n.starts_with("medtracker-export-20260310-091000-042-") && n.ends_with(".csv"), "{n}");
    assert_ne!(n, export::file_name(1_773_133_800_042_000, "csv"), "same millisecond, different name");
}

#[test]
fn bad_ranges_are_rejected() {
    let (c, ..) = fixture();
    assert!(query::report(&c, &at("2026-03-10T12:00"), "2026-03-10", "2026-03-07").is_err());
    assert!(query::report(&c, &at("2026-03-10T12:00"), "2025-01-01", "2026-03-10").is_err());
    assert!(query::report(&c, &at("2026-03-10T12:00"), "nope", "2026-03-10").is_err());
}

#[test]
fn save_falls_back_when_the_preferred_folder_cannot_be_written() {
    let base = std::env::temp_dir().join(format!("medtracker-exp-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&base).unwrap();
    let blocker = base.join("a-file");
    std::fs::write(&blocker, b"x").unwrap();
    let bad = blocker.join("sub"); // a folder cannot be created under a regular file
    let good = base.join("exports");
    let p = export::save(&[bad.clone(), good.clone()], "e.csv", b"a,b\r\n").unwrap();
    assert_eq!(p, good.join("e.csv"));
    assert_eq!(std::fs::read(&p).unwrap(), b"a,b\r\n");
    let left: Vec<_> = std::fs::read_dir(&good).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(left, ["e.csv"], "no temp or partial file left");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&p).unwrap().permissions().mode() & 0o777, 0o600);
    }
    // Nowhere to write at all: a clear error, nothing created.
    assert!(export::save(&[bad], "e.csv", b"x").is_err());
    std::fs::remove_dir_all(base).unwrap();
}
