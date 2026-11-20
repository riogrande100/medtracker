//! Schedule, occurrence and migration-003 tests. Time is always passed in; nothing reads the
//! system clock.

use crate::application::{ledger, medication, schedule};
use crate::clock::{parse_local, TimeCtx, Zone};
use crate::domain::schedule::ScheduleMode;
use crate::dto::*;
use crate::error::AppError;
use crate::storage::{self, migrations};
use rusqlite::Connection;

const NY: &str = "America/New_York";
const SYD: &str = "Australia/Sydney";

fn utc(local: &str) -> i64 {
    Zone::fixed(0).resolve(parse_local(local).unwrap()).unwrap().utc_us
}

fn at(zone: &str, local: &str) -> TimeCtx {
    let z = Zone::iana(zone).unwrap();
    let now_utc_us = z.resolve(parse_local(local).unwrap()).unwrap().utc_us;
    TimeCtx { now_utc_us, zone: z }
}

fn home(zone: &str, times: &[&str], weekdays: &[u8]) -> ScheduleInput {
    ScheduleInput {
        mode: ScheduleMode::HomeZone,
        zone_id: Some(zone.into()),
        slots: times.iter().map(|t| SlotInput { time: (*t).into(), weekdays: weekdays.to_vec() }).collect(),
        interval_seconds: None,
        anchor_local: None,
    }
}

fn every(hours: i64, first: &str) -> ScheduleInput {
    ScheduleInput { mode: ScheduleMode::ElapsedInterval, zone_id: None, slots: vec![], interval_seconds: Some(hours * 3600), anchor_local: Some(first.into()) }
}

/// A medication with no schedule, created at `now`.
fn med(c: &mut Connection, now: &TimeCtx) -> String {
    medication::create(c, now.clone(), NewMedication { name: "Metformin".into(), strength: Some("500 mg".into()), form: None, times: vec![] }).unwrap()
}

fn make(c: &mut Connection, now: &TimeCtx, med: &str, def: ScheduleInput) -> String {
    schedule::create(c, now, NewSchedule { medication_id: med.into(), definition: def }).unwrap()
}

fn items(c: &Connection, now: &TimeCtx) -> Vec<TodayItem> {
    ledger::today(c, now.clone()).unwrap()
}

fn act(c: &mut Connection, now: &TimeCtx, i: &TodayItem, kind: DoseKind) -> Result<String, AppError> {
    ledger::record(
        c,
        now.clone(),
        RecordDose { medication_id: i.medication_id.clone(), kind, occurrence_id: Some(i.occurrence_id.clone()), occurred_local: None, note: None, second_occurrence: None },
    )
}

fn count(c: &Connection, sql: &str) -> i64 {
    c.query_row(sql, [], |r| r.get(0)).unwrap()
}

/// Every occurrence row, in a stable order, as text (to compare "before" and "after").
fn occurrence_dump(c: &Connection) -> Vec<String> {
    let mut stmt = c
        .prepare(
            "SELECT id || '|' || schedule_version_id || '|' || ifnull(slot_id,'') || '|' || local_date || '|' || local_time || '|' || ifnull(zone_id,'') || '|' ||
                    offset_seconds || '|' || fold || '|' || adjusted || '|' || scheduled_at_utc || '|' || ifnull(tzdb_version,'') || '|' || generated_at_utc || '|' || state
             FROM scheduled_occurrence ORDER BY id",
        )
        .unwrap();
    stmt.query_map([], |r| r.get::<_, String>(0)).unwrap().map(|r| r.unwrap()).collect()
}

fn dates_of(c: &Connection, schedule_id: &str) -> Vec<(String, String, bool, i64, String)> {
    let mut stmt = c
        .prepare("SELECT local_date, local_time, adjusted, scheduled_at_utc, state FROM scheduled_occurrence WHERE schedule_id=?1 ORDER BY scheduled_at_utc")
        .unwrap();
    stmt.query_map([schedule_id], |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? == 1, r.get(3)?, r.get(4)?))).unwrap().map(|r| r.unwrap()).collect()
}

// ---- generation across DST, through the database ----

#[test]
fn new_york_spring_forward_materializes_one_adjusted_occurrence_and_is_idempotent() {
    let mut c = storage::open_memory().unwrap();
    let now = at(NY, "2026-03-01T08:00");
    let m = med(&mut c, &now);
    let sid = make(&mut c, &now, &m, home(NY, &["02:30"], &[]));
    let added = schedule::generate_range(&c, &now, "2026-03-07", "2026-03-10").unwrap();
    assert_eq!(added, 4, "one per date: 4 dates -> 4 occurrences (spec fixture shape)");
    let rows = dates_of(&c, &sid);
    assert_eq!(rows.iter().map(|r| r.0.as_str()).collect::<Vec<_>>(), ["2026-03-07", "2026-03-08", "2026-03-09", "2026-03-10"]);
    let gap = &rows[1];
    assert!(gap.2, "the 03-08 occurrence is marked adjusted");
    assert_eq!(gap.3, utc("2026-03-08T07:30"), "Phase 1 gap rule: moved forward by the gap");
    assert!(!rows[0].2 && !rows[2].2 && !rows[3].2);
    assert_eq!(rows[2].3, utc("2026-03-09T06:30"), "next date's 02:30 is EDT");
    // Idempotent: nothing new on a second run, rows byte-identical.
    let before = occurrence_dump(&c);
    assert_eq!(schedule::generate_range(&c, &now, "2026-03-07", "2026-03-10").unwrap(), 0);
    assert_eq!(schedule::generate_range(&c, &now, "2026-03-08", "2026-03-09").unwrap(), 0);
    assert_eq!(before, occurrence_dump(&c));
    // Each row records the zone, offset and the tz database version used.
    let (zone, off, tzdb): (String, i32, String) = c
        .query_row("SELECT zone_id, offset_seconds, tzdb_version FROM scheduled_occurrence WHERE local_date='2026-03-08'", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap();
    assert_eq!((zone.as_str(), off), (NY, -4 * 3600));
    assert!(!tzdb.is_empty());
}

#[test]
fn new_york_fall_back_materializes_exactly_one_earlier_occurrence() {
    let mut c = storage::open_memory().unwrap();
    let now = at(NY, "2026-10-01T08:00");
    let m = med(&mut c, &now);
    let sid = make(&mut c, &now, &m, home(NY, &["01:30"], &[]));
    schedule::generate_range(&c, &now, "2026-10-31", "2026-11-02").unwrap();
    let rows = dates_of(&c, &sid);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[1].0, "2026-11-01");
    assert_eq!(rows[1].3, utc("2026-11-01T05:30"), "earlier (EDT) instant");
    assert!(!rows[1].2);
    assert_eq!(count(&c, "SELECT count(*) FROM scheduled_occurrence WHERE local_date='2026-11-01'"), 1);
}

#[test]
fn southern_hemisphere_schedule_follows_its_own_dst_calendar() {
    let mut c = storage::open_memory().unwrap();
    let now = at(SYD, "2026-01-01T08:00");
    let m = med(&mut c, &now);
    let sid = make(&mut c, &now, &m, home(SYD, &["02:30"], &[]));
    schedule::generate_range(&c, &now, "2026-04-04", "2026-04-06").unwrap(); // autumn fall-back 04-05
    schedule::generate_range(&c, &now, "2026-10-03", "2026-10-05").unwrap(); // spring gap 10-04
    let rows = dates_of(&c, &sid);
    assert_eq!(rows.len(), 6);
    let apr5 = rows.iter().find(|r| r.0 == "2026-04-05").unwrap();
    assert_eq!(apr5.3, utc("2026-04-04T15:30"), "earlier AEDT (+11) occurrence, once");
    assert!(!apr5.2);
    let oct4 = rows.iter().find(|r| r.0 == "2026-10-04").unwrap();
    assert!(oct4.2, "02:30 does not exist on 2026-10-04 in Sydney");
    assert_eq!(oct4.3, utc("2026-10-03T16:30"), "03:30 AEDT");
    // A device in another zone doesn't change a home-zone schedule's instants.
    let travelling = at("Pacific/Auckland", "2026-04-04T08:00");
    assert_eq!(schedule::generate_range(&c, &travelling, "2026-04-04", "2026-04-06").unwrap(), 0);
}

#[test]
fn weekday_slots_only_generate_on_selected_days() {
    let mut c = storage::open_memory().unwrap();
    let now = at(NY, "2026-03-01T08:00");
    let m = med(&mut c, &now);
    let sid = make(&mut c, &now, &m, home(NY, &["08:00"], &[1, 3])); // Mon, Wed
    schedule::generate_range(&c, &now, "2026-03-02", "2026-03-08").unwrap();
    assert_eq!(dates_of(&c, &sid).iter().map(|r| r.0.clone()).collect::<Vec<_>>(), ["2026-03-02", "2026-03-04"]);
}

// ---- derived status ----

#[test]
fn status_is_derived_never_stored() {
    let mut c = storage::open_memory().unwrap();
    // No column on the occurrence can hold "due"/"taken"/"skipped": only the row's own lifecycle.
    let cols: Vec<String> = c.prepare("PRAGMA table_info(scheduled_occurrence)").unwrap().query_map([], |r| r.get::<_, String>(1)).unwrap().map(|r| r.unwrap()).collect();
    assert!(!cols.iter().any(|n| n.contains("due") || n == "outcome" || n == "taken"), "{cols:?}");
    let states: String = c.query_row("SELECT sql FROM sqlite_master WHERE name='scheduled_occurrence'", [], |r| r.get(0)).unwrap();
    assert!(states.contains("'active','superseded','cancelled'") && !states.contains("'due'") && !states.contains("'taken'"));

    let t0 = at(NY, "2026-03-01T08:00");
    let m = med(&mut c, &t0);
    make(&mut c, &t0, &m, home(NY, &["08:00", "20:00"], &[]));

    let now = at(NY, "2026-03-10T09:00");
    let v = items(&c, &now);
    let status = |v: &[TodayItem], time: &str| v.iter().find(|i| i.local_time == time && i.local_date == "2026-03-10").unwrap().status.clone();
    assert_eq!(status(&v, "08:00"), TodayStatus::Due);
    assert_eq!(status(&v, "20:00"), TodayStatus::Upcoming);
    // Yesterday's unrecorded doses are "earlier, not logged": neutral, not "missed"/"overdue".
    let earlier: Vec<_> = v.iter().filter(|i| i.status == TodayStatus::EarlierNotLogged).collect();
    assert_eq!(earlier.len(), 2);
    assert!(earlier.iter().all(|i| i.local_date == "2026-03-09"));
    assert_eq!(serde_json::to_string(&TodayStatus::EarlierNotLogged).unwrap(), "\"earlier_not_logged\"");

    // Taken -> Taken; Undo -> Due again; Skip -> Skipped; all from the same stored occurrence.
    let due = v.iter().find(|i| i.status == TodayStatus::Due).unwrap().clone();
    let e = act(&mut c, &now, &due, DoseKind::Take).unwrap();
    assert_eq!(status(&items(&c, &now), "08:00"), TodayStatus::Taken);
    ledger::undo(&mut c, now.clone(), &e).unwrap();
    assert_eq!(status(&items(&c, &now), "08:00"), TodayStatus::Due);
    act(&mut c, &now, &due, DoseKind::Skip).unwrap();
    assert_eq!(status(&items(&c, &now), "08:00"), TodayStatus::Skipped);
    // The time passing changes Upcoming -> Due without any write.
    let rows_before = occurrence_dump(&c);
    let later = at(NY, "2026-03-10T20:00");
    assert_eq!(status(&items(&c, &later), "20:00"), TodayStatus::Due);
    assert_eq!(occurrence_dump(&c).len(), rows_before.len(), "no rows rewritten or added by the clock moving");
    // Next day: a recorded dose from yesterday is not on Today; the unrecorded one is.
    let next_day = at(NY, "2026-03-11T09:00");
    let v = items(&c, &next_day);
    assert!(v.iter().filter(|i| i.local_date == "2026-03-10").all(|i| i.status == TodayStatus::EarlierNotLogged && i.local_time == "20:00"), "{v:?}");
    assert_eq!(v.iter().filter(|i| i.local_date == "2026-03-10").count(), 1);
}

#[test]
fn earlier_day_cards_cannot_be_acted_on_from_today() {
    let mut c = storage::open_memory().unwrap();
    let t0 = at(NY, "2026-03-01T08:00");
    let m = med(&mut c, &t0);
    make(&mut c, &t0, &m, home(NY, &["08:00"], &[]));
    let now = at(NY, "2026-03-10T09:00");
    let earlier = items(&c, &now).into_iter().find(|i| i.status == TodayStatus::EarlierNotLogged).unwrap();
    assert!(matches!(act(&mut c, &now, &earlier, DoseKind::Take), Err(AppError::StaleDate(_))));
    assert_eq!(count(&c, "SELECT count(*) FROM dose_event"), 0);
}

#[test]
fn a_new_schedule_does_not_invent_doses_before_it_started() {
    let mut c = storage::open_memory().unwrap();
    let now = at(NY, "2026-03-10T14:00");
    let m = med(&mut c, &now);
    make(&mut c, &now, &m, home(NY, &["08:00", "20:00"], &[]));
    let v = items(&c, &now);
    assert_eq!(v.len(), 1, "{v:?}");
    assert_eq!((v[0].local_time.as_str(), v[0].status.clone()), ("20:00", TodayStatus::Upcoming));
}

#[test]
fn home_zone_cards_show_zone_context_and_the_device_time() {
    let mut c = storage::open_memory().unwrap();
    let t0 = at("Asia/Tokyo", "2026-03-01T08:00");
    let m = med(&mut c, &t0);
    make(&mut c, &t0, &m, home("Asia/Tokyo", &["08:00"], &[]));
    // The device is in New York; it is 2026-03-09 20:30 EDT = 2026-03-10 09:30 in Tokyo.
    let now = at(NY, "2026-03-09T20:30");
    let v = items(&c, &now);
    let i = v.iter().find(|i| i.local_date == "2026-03-10").expect("Tokyo's today, not New York's");
    assert_eq!(i.status, TodayStatus::Due);
    assert_eq!(i.zone_id.as_deref(), Some("Asia/Tokyo"));
    assert_eq!(i.zone_abbreviation.as_deref(), Some("JST"));
    assert_eq!(i.device_time.as_deref(), Some("19:00"), "08:00 JST = 19:00 EDT the day before");
    assert_eq!(i.scheduled_at_utc_us, utc("2026-03-09T23:00"));
    // Taken works against it (its own day is current in its own zone).
    act(&mut c, &now, i, DoseKind::Take).unwrap();
}

// ---- edit / pause / resume / end ----

#[test]
fn editing_a_schedule_never_rewrites_history() {
    let mut c = storage::open_memory().unwrap();
    let t0 = at(NY, "2026-03-01T08:00");
    let m = med(&mut c, &t0);
    let sid = make(&mut c, &t0, &m, home(NY, &["08:00", "20:00"], &[]));
    let now = at(NY, "2026-03-10T12:00");
    let v = items(&c, &now);
    let morning = v.iter().find(|i| i.local_date == "2026-03-10" && i.local_time == "08:00").unwrap().clone();
    let evening = v.iter().find(|i| i.local_date == "2026-03-10" && i.local_time == "20:00").unwrap().clone();
    let event = act(&mut c, &now, &morning, DoseKind::Take).unwrap();
    let before = occurrence_dump(&c);
    let v1_before: (i64, Option<i64>, String) =
        c.query_row("SELECT effective_from_utc, effective_until_utc, state FROM schedule_version WHERE schedule_id=?1 AND version_number=1", [&sid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
    assert_eq!(v1_before.1, None);

    schedule::edit(&mut c, &now, EditSchedule { schedule_id: sid.clone(), definition: home(NY, &["09:00", "21:00"], &[]) }).unwrap();

    // Version 1 is closed at the edit instant and otherwise unchanged; version 2 starts there.
    let (from1, until1): (i64, Option<i64>) =
        c.query_row("SELECT effective_from_utc, effective_until_utc FROM schedule_version WHERE schedule_id=?1 AND version_number=1", [&sid], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
    assert_eq!((from1, until1), (v1_before.0, Some(now.now_utc_us)));
    let (from2, until2, reason): (i64, Option<i64>, String) =
        c.query_row("SELECT effective_from_utc, effective_until_utc, change_reason FROM schedule_version WHERE schedule_id=?1 AND version_number=2", [&sid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
    assert_eq!((from2, until2, reason.as_str()), (now.now_utc_us, None, "edit"));

    // Every occurrence that existed is byte-identical, except tonight's unrecorded 20:00 which is
    // marked superseded (never deleted, never edited).
    let after = occurrence_dump(&c);
    assert_eq!(after.len(), before.len());
    let changed: Vec<_> = before.iter().zip(&after).filter(|(a, b)| a != b).collect();
    assert_eq!(changed.len(), 1, "{changed:?}");
    assert!(changed[0].0.starts_with(&evening.occurrence_id) && changed[0].1.ends_with("|superseded"));
    // The recorded dose and its link are untouched.
    assert_eq!(count(&c, "SELECT count(*) FROM dose_event WHERE undone=0 AND current_revision=0"), 1);
    assert_eq!(count(&c, "SELECT count(*) FROM dose_event_occurrence"), 1);
    let h = &ledger::history(&c, 10).unwrap()[0];
    assert_eq!((h.event_id.as_str(), h.slot_time.as_deref()), (event.as_str(), Some("08:00")));

    // Today: this morning's recorded 08:00 stays Taken; the old 20:00 is gone; the new 21:00 is next.
    let v = items(&c, &now);
    let today: Vec<_> = v.iter().filter(|i| i.local_date == "2026-03-10").map(|i| (i.local_time.clone(), i.status.clone())).collect();
    assert_eq!(today, [("08:00".into(), TodayStatus::Taken), ("21:00".into(), TodayStatus::Upcoming)]);
    // Tomorrow comes only from version 2.
    schedule::generate_range(&c, &now, "2026-03-11", "2026-03-11").unwrap();
    let tomorrow: Vec<String> = dates_of(&c, &sid).into_iter().filter(|r| r.0 == "2026-03-11").map(|r| r.1).collect();
    assert_eq!(tomorrow, ["09:00", "21:00"]);

    // The past can't be edited even with direct SQL.
    assert!(c.execute("UPDATE scheduled_occurrence SET scheduled_at_utc = 1", []).is_err());
    assert!(c.execute("UPDATE schedule_version SET interval_seconds = 5 WHERE version_number = 1", []).is_err());
    assert!(c.execute("UPDATE schedule_version SET effective_until_utc = 5 WHERE version_number = 1", []).is_err(), "a closed version can't be re-closed");
    assert!(c.execute("UPDATE scheduled_occurrence SET state='active' WHERE state='superseded'", []).is_err());
    // Versions can't be made to overlap.
    assert!(c
        .execute(
            "INSERT INTO schedule_version (id, schedule_id, medication_version_id, version_number, effective_from_utc, state, mode, zone_id, recurrence, change_reason, created_at_utc)
             SELECT 'x', schedule_id, medication_version_id, 9, effective_from_utc, 'active', 'home_zone', zone_id, 'weekly_slots', 'edit', 1 FROM schedule_version WHERE version_number=2",
            [],
        )
        .is_err());
}

#[test]
fn a_superseded_card_cannot_be_logged_or_restored() {
    let mut c = storage::open_memory().unwrap();
    let t0 = at(NY, "2026-03-01T08:00");
    let m = med(&mut c, &t0);
    let sid = make(&mut c, &t0, &m, home(NY, &["20:00"], &[]));
    let now = at(NY, "2026-03-10T12:00");
    let stale = items(&c, &now).into_iter().find(|i| i.local_date == "2026-03-10").unwrap();
    schedule::edit(&mut c, &now, EditSchedule { schedule_id: sid, definition: home(NY, &["21:00"], &[]) }).unwrap();
    let err = act(&mut c, &now, &stale, DoseKind::Take).unwrap_err();
    assert!(matches!(err, AppError::StaleDate(_)), "{err:?}");
    assert_eq!(count(&c, "SELECT count(*) FROM dose_event"), 0);
}

#[test]
fn a_recorded_dose_survives_an_edit_that_changes_nothing_about_it() {
    // Edit at 07:00; the 08:00 slot is unchanged, today's recorded-early dose stays, and there is
    // no second 08:00 card from the new version.
    let mut c = storage::open_memory().unwrap();
    let t0 = at(NY, "2026-03-01T08:00");
    let m = med(&mut c, &t0);
    let sid = make(&mut c, &t0, &m, home(NY, &["08:00", "20:00"], &[]));
    let early = at(NY, "2026-03-10T07:00");
    let card = items(&c, &early).into_iter().find(|i| i.local_date == "2026-03-10" && i.local_time == "08:00").unwrap();
    act(&mut c, &early, &card, DoseKind::Take).unwrap();
    schedule::edit(&mut c, &early, EditSchedule { schedule_id: sid, definition: home(NY, &["08:00", "21:00"], &[]) }).unwrap();
    let v: Vec<_> = items(&c, &early).into_iter().filter(|i| i.local_date == "2026-03-10").map(|i| (i.local_time, i.status)).collect();
    assert_eq!(v, [("08:00".into(), TodayStatus::Taken), ("21:00".into(), TodayStatus::Upcoming)], "no duplicate 08:00");
}

#[test]
fn pause_resume_and_end_never_backfill() {
    let mut c = storage::open_memory().unwrap();
    let t0 = at(NY, "2026-03-01T08:00");
    let m = med(&mut c, &t0);
    let sid = make(&mut c, &t0, &m, home(NY, &["08:00", "20:00"], &[]));
    schedule::generate_range(&c, &t0, "2026-03-09", "2026-03-09").unwrap();

    let paused_at = at(NY, "2026-03-10T12:00");
    schedule::pause(&mut c, &paused_at, &sid).unwrap();
    // Paused: nothing is generated for the paused period, now or later.
    let during = at(NY, "2026-03-11T09:00");
    // (Only the 08:00 that fell inside version 1's range, before the pause, is added.)
    assert_eq!(schedule::generate_range(&c, &during, "2026-03-10", "2026-03-12").unwrap(), 1);
    assert!(items(&c, &during).iter().all(|i| i.local_date != "2026-03-11"));
    assert_eq!(
        count(&c, &format!("SELECT count(*) FROM scheduled_occurrence WHERE state='active' AND scheduled_at_utc >= {}", paused_at.now_utc_us)),
        0,
        "future occurrences were superseded, none remain active"
    );
    let s = schedule::summary_for_medication(&c, &during, &m).unwrap().unwrap();
    assert_eq!((s.state, s.next.len()), (crate::domain::schedule::ScheduleState::Paused, 0));
    assert!(schedule::pause(&mut c, &during, &sid).is_err(), "already paused");

    let resumed_at = at(NY, "2026-03-12T12:00");
    schedule::resume(&mut c, &resumed_at, &sid).unwrap();
    schedule::generate_range(&c, &resumed_at, "2026-03-08", "2026-03-14").unwrap();
    let dates: Vec<(String, String)> = dates_of(&c, &sid).into_iter().filter(|r| r.4 == "active").map(|r| (r.0, r.1)).collect();
    // Before the pause (up to its instant), nothing between pause and resume, then from resume on.
    assert!(dates.contains(&("2026-03-10".into(), "08:00".into())));
    assert!(!dates.iter().any(|d| d.0 == "2026-03-11"), "{dates:?}");
    assert!(!dates.contains(&("2026-03-12".into(), "08:00".into())), "08:00 on the resume day is before the resume instant");
    assert!(dates.contains(&("2026-03-12".into(), "20:00".into())));
    assert!(dates.contains(&("2026-03-13".into(), "08:00".into())));

    // End: a closing version, no occurrences afterwards, no backfill, cannot be edited or resumed.
    let ended_at = at(NY, "2026-03-13T12:00");
    schedule::end(&mut c, &ended_at, &sid).unwrap();
    schedule::generate_range(&c, &ended_at, "2026-03-13", "2026-03-20").unwrap();
    let last = dates_of(&c, &sid).into_iter().filter(|r| r.4 == "active").last().unwrap();
    assert_eq!((last.0.as_str(), last.1.as_str()), ("2026-03-13", "08:00"));
    assert!(schedule::edit(&mut c, &ended_at, EditSchedule { schedule_id: sid.clone(), definition: home(NY, &["10:00"], &[]) }).is_err());
    assert!(schedule::resume(&mut c, &ended_at, &sid).is_err());
    let reasons: Vec<String> = c.prepare("SELECT change_reason FROM schedule_version ORDER BY version_number").unwrap().query_map([], |r| r.get(0)).unwrap().map(|r| r.unwrap()).collect();
    assert_eq!(reasons, ["create", "pause", "resume", "end"]);
    // Ranges chain without gaps or overlaps.
    let ranges: Vec<(i64, Option<i64>)> = c.prepare("SELECT effective_from_utc, effective_until_utc FROM schedule_version ORDER BY version_number").unwrap().query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap().map(|r| r.unwrap()).collect();
    assert!(ranges.windows(2).all(|w| w[0].1 == Some(w[1].0)) && ranges.last().unwrap().1.is_none());
}

#[test]
fn a_new_schedule_can_follow_an_ended_one_but_not_overlap_an_open_one() {
    let mut c = storage::open_memory().unwrap();
    let t0 = at(NY, "2026-03-01T08:00");
    let m = med(&mut c, &t0);
    let sid = make(&mut c, &t0, &m, home(NY, &["08:00"], &[]));
    assert!(schedule::create(&mut c, &t0, NewSchedule { medication_id: m.clone(), definition: home(NY, &["09:00"], &[]) }).is_err());
    let later = at(NY, "2026-03-05T08:00");
    schedule::end(&mut c, &later, &sid).unwrap();
    make(&mut c, &later, &m, home(NY, &["09:00"], &[]));
    let s = schedule::summary_for_medication(&c, &later, &m).unwrap().unwrap();
    assert_eq!(s.slots[0].time, "09:00", "the open schedule is the one shown");
}

#[test]
fn schedule_changes_validate_their_input_and_the_clock() {
    let mut c = storage::open_memory().unwrap();
    let t0 = at(NY, "2026-03-05T08:00");
    let m = med(&mut c, &t0);
    let create = |c: &mut Connection, d: ScheduleInput| schedule::create(c, &t0, NewSchedule { medication_id: m.clone(), definition: d });
    assert!(matches!(create(&mut c, home(NY, &[], &[])), Err(AppError::Validation(_))), "needs a time");
    assert!(create(&mut c, home(NY, &["8am"], &[])).is_err());
    assert!(create(&mut c, home(NY, &["08:00", "08:00"], &[])).is_err());
    assert!(create(&mut c, home("Nowhere/Land", &["08:00"], &[])).is_err());
    assert!(create(&mut c, ScheduleInput { zone_id: None, ..home(NY, &["08:00"], &[]) }).is_err(), "home_zone needs an explicit zone");
    assert!(create(&mut c, home(NY, &["08:00"], &[0])).is_err() && create(&mut c, home(NY, &["08:00"], &[8])).is_err());
    assert!(create(&mut c, every(0, "2026-03-05T08:00")).is_err() && create(&mut c, every(24 * 31, "2026-03-05T08:00")).is_err());
    assert!(create(&mut c, ScheduleInput { slots: vec![SlotInput { time: "08:00".into(), weekdays: vec![] }], ..every(8, "2026-03-05T08:00") }).is_err());
    assert!(create(&mut c, every(8, "not a time")).is_err());
    assert!(schedule::create(&mut c, &t0, NewSchedule { medication_id: "missing".into(), definition: home(NY, &["08:00"], &[]) }).is_err());
    assert_eq!(count(&c, "SELECT count(*) FROM schedule"), 0, "nothing written for rejected input");
    let sid = create(&mut c, home(NY, &["08:00"], &[])).unwrap();
    // An unchanged edit is refused rather than creating an empty version.
    assert!(schedule::edit(&mut c, &at(NY, "2026-03-06T08:00"), EditSchedule { schedule_id: sid.clone(), definition: home(NY, &["08:00"], &[]) }).is_err());
    // The device clock reading earlier than the last change is refused (it would overlap).
    assert!(schedule::edit(&mut c, &at(NY, "2026-03-01T08:00"), EditSchedule { schedule_id: sid.clone(), definition: home(NY, &["09:00"], &[]) }).is_err());
    assert!(schedule::resume(&mut c, &at(NY, "2026-03-06T08:00"), &sid).is_err(), "not paused");
    assert!(matches!(schedule::pause(&mut c, &t0, "nope"), Err(AppError::NotFound(_))));
    assert_eq!(count(&c, "SELECT count(*) FROM schedule_version"), 1);
    // The CHECK refuses device_local until it is implemented.
    assert!(c.execute("UPDATE schedule_version SET mode='device_local'", []).is_err());
}

// ---- elapsed interval ----

#[test]
fn elapsed_interval_keeps_its_cadence_after_a_late_dose_and_can_be_reanchored() {
    let mut c = storage::open_memory().unwrap();
    let t0 = at(NY, "2026-03-10T07:00");
    let m = med(&mut c, &t0);
    let sid = make(&mut c, &t0, &m, every(8, "2026-03-10T08:00")); // 08:00, 16:00, 00:00 ...
    let late = at(NY, "2026-03-10T10:30");
    let v = items(&c, &late);
    let times: Vec<_> = v.iter().map(|i| (i.local_time.as_str(), i.status.clone())).collect();
    assert_eq!(times, [("08:00", TodayStatus::Due), ("16:00", TodayStatus::Upcoming)]);
    let first = v[0].clone();
    // A late dose (10:30 for the 08:00 occurrence) is recorded as such...
    let e = act(&mut c, &late, &first, DoseKind::Take).unwrap();
    let h = &ledger::history(&c, 5).unwrap()[0];
    assert_eq!((h.event_id.as_str(), h.current_local.as_str()), (e.as_str(), "2026-03-10T10:30"));
    // ...and the fixed cadence is NOT shifted: the next occurrence is still 16:00 (anchor + n * interval).
    let v = items(&c, &late);
    assert_eq!(v.iter().map(|i| (i.local_time.as_str(), i.status.clone())).collect::<Vec<_>>(), [("08:00", TodayStatus::Taken), ("16:00", TodayStatus::Upcoming)]);
    assert_eq!(v[1].scheduled_at_utc_us - v[0].scheduled_at_utc_us, 8 * 3_600_000_000);
    assert_eq!(v[0].mode, ScheduleMode::ElapsedInterval);
    assert!(v[0].zone_id.as_deref() == Some(NY), "displayed in the device zone");

    // The user can re-anchor from the actual dose time: a new version, history intact.
    let before = occurrence_dump(&c);
    // (The anchor is the first dose of the new cadence: 8 hours after the dose actually taken.)
    schedule::edit(&mut c, &late, EditSchedule { schedule_id: sid.clone(), definition: every(8, "2026-03-10T18:30") }).unwrap();
    let after = occurrence_dump(&c);
    let changed: Vec<_> = before.iter().zip(&after).filter(|(a, b)| a != b).collect();
    assert_eq!(changed.len(), 1, "only the unrecorded 16:00 is superseded: {changed:?}");
    let v = items(&c, &late);
    assert_eq!(v.iter().map(|i| (i.local_time.as_str(), i.status.clone())).collect::<Vec<_>>(), [("08:00", TodayStatus::Taken), ("18:30", TodayStatus::Upcoming)]);
    // Editing the interval alone keeps the existing anchor.
    schedule::edit(&mut c, &late, EditSchedule { schedule_id: sid.clone(), definition: ScheduleInput { anchor_local: None, ..every(6, "x") } }).unwrap();
    let anchor: i64 = c.query_row("SELECT anchor_utc FROM schedule_version WHERE version_number=3", [], |r| r.get(0)).unwrap();
    assert_eq!(anchor, utc("2026-03-10T22:30"), "18:30 EDT");
}

#[test]
fn elapsed_interval_ignores_dst_and_travel() {
    let mut c = storage::open_memory().unwrap();
    let t0 = at(NY, "2026-03-07T07:00");
    let m = med(&mut c, &t0);
    let sid = make(&mut c, &t0, &m, every(12, "2026-03-07T20:00"));
    schedule::generate_range(&c, &t0, "2026-03-07", "2026-03-09").unwrap();
    let rows = dates_of(&c, &sid);
    for w in rows.windows(2) {
        assert_eq!(w[1].3 - w[0].3, 12 * 3_600_000_000, "exactly 12 elapsed hours, across the spring-forward night");
    }
    // Same schedule, viewed from Sydney: same UTC instants (cadence is absolute), different wall times.
    let syd = at(SYD, "2026-03-08T12:00");
    let n0 = items(&c, &syd);
    assert!(!n0.is_empty());
    assert!(n0.iter().all(|i| i.zone_id.as_deref() == Some(SYD)));
    let utcs: Vec<i64> = rows.iter().map(|r| r.3).collect();
    assert!(n0.iter().all(|i| utcs.contains(&i.scheduled_at_utc_us)));
}

// ---- next due and preview ----

#[test]
fn next_due_is_the_next_unrecorded_occurrence_even_on_a_later_day() {
    let mut c = storage::open_memory().unwrap();
    let t0 = at(NY, "2026-03-01T08:00");
    let m = med(&mut c, &t0);
    make(&mut c, &t0, &m, home(NY, &["08:00", "20:00"], &[]));
    let now = at(NY, "2026-03-10T09:00");
    let n = schedule::next_due(&c, &now).unwrap().unwrap();
    assert_eq!((n.occurrence.local_date.as_str(), n.occurrence.local_time.as_str(), n.name.as_str()), ("2026-03-10", "20:00", "Metformin"));
    assert_eq!(n.occurrence.zone_abbreviation.as_deref(), Some("EDT"));
    // Record tonight's dose early: the next one is tomorrow morning.
    let evening = items(&c, &now).into_iter().find(|i| i.local_time == "20:00" && i.local_date == "2026-03-10").unwrap();
    act(&mut c, &now, &evening, DoseKind::Take).unwrap();
    let n = schedule::next_due(&c, &now).unwrap().unwrap();
    assert_eq!((n.occurrence.local_date.as_str(), n.occurrence.local_time.as_str()), ("2026-03-11", "08:00"));
    // Nothing scheduled -> None; paused -> None.
    let mut c2 = storage::open_memory().unwrap();
    assert!(schedule::next_due(&c2, &now).unwrap().is_none());
    let m2 = med(&mut c2, &t0);
    let sid = make(&mut c2, &t0, &m2, home(NY, &["08:00"], &[]));
    schedule::pause(&mut c2, &now, &sid).unwrap();
    assert!(schedule::next_due(&c2, &now).unwrap().is_none());
}

#[test]
fn preview_shows_seven_occurrences_with_zone_abbreviation_and_marks_adjusted_ones() {
    let now = at(NY, "2026-03-05T12:00");
    let p = schedule::preview(&now, &home(NY, &["02:30"], &[])).unwrap();
    assert_eq!(p.len(), 7);
    assert_eq!(p[0].local_date, "2026-03-06");
    assert_eq!(p[0].zone_abbreviation.as_deref(), Some("EST"));
    let gap = p.iter().find(|o| o.local_date == "2026-03-08").unwrap();
    assert!(gap.adjusted && gap.local_time == "03:30" && gap.zone_abbreviation.as_deref() == Some("EDT"), "{gap:?}");
    assert_eq!(p.iter().filter(|o| o.adjusted).count(), 1);
    // A home zone different from the device's is previewed in the home zone.
    let tokyo = schedule::preview(&now, &home("Asia/Tokyo", &["08:00"], &[])).unwrap();
    assert_eq!(tokyo[0].zone_abbreviation.as_deref(), Some("JST"));
    // Elapsed preview, and invalid input is rejected without side effects.
    assert_eq!(schedule::preview(&now, &every(8, "2026-03-05T20:00")).unwrap().len(), 7);
    assert!(schedule::preview(&now, &home(NY, &[], &[])).is_err());
}

#[test]
fn schedule_summary_lists_the_definition_and_next_seven() {
    let mut c = storage::open_memory().unwrap();
    let t0 = at(NY, "2026-03-01T08:00");
    let m = med(&mut c, &t0);
    assert!(medication::list(&c, &t0).unwrap()[0].schedule.is_none(), "as needed");
    make(&mut c, &t0, &m, home(NY, &["20:00", "08:00"], &[1, 5]));
    let s = medication::list(&c, &t0).unwrap().remove(0).schedule.unwrap();
    assert_eq!((s.mode, s.version_number, s.zone_id.as_deref()), (ScheduleMode::HomeZone, 1, Some(NY)));
    assert_eq!(s.slots.iter().map(|x| (x.time.as_str(), x.weekdays.clone())).collect::<Vec<_>>(), [("08:00", vec![1, 5]), ("20:00", vec![1, 5])]);
    assert_eq!(s.next.len(), 7);
    assert_eq!(serde_json::to_value(&s).unwrap()["state"], "active");
}

// ---- lifecycle with the medication ----

#[test]
fn deleting_a_medication_removes_its_schedules_occurrences_and_slots() {
    let mut c = storage::open_memory().unwrap();
    let t0 = at(NY, "2026-03-01T08:00");
    let m = med(&mut c, &t0);
    let sid = make(&mut c, &t0, &m, home(NY, &["08:00"], &[]));
    let now = at(NY, "2026-03-10T09:00");
    let card = items(&c, &now).into_iter().find(|i| i.status == TodayStatus::Due).unwrap();
    act(&mut c, &now, &card, DoseKind::Take).unwrap();
    schedule::edit(&mut c, &now, EditSchedule { schedule_id: sid, definition: home(NY, &["09:00"], &[]) }).unwrap();
    medication::delete(&mut c, &m).unwrap();
    for t in ["schedule", "schedule_version", "schedule_slot", "scheduled_occurrence", "dose_event", "dose_event_occurrence"] {
        assert_eq!(count(&c, &format!("SELECT count(*) FROM {t}")), 0, "{t}");
    }
}

#[test]
fn archiving_a_medication_hides_its_cards_and_keeps_its_schedule() {
    let mut c = storage::open_memory().unwrap();
    let t0 = at(NY, "2026-03-01T08:00");
    let m = med(&mut c, &t0);
    make(&mut c, &t0, &m, home(NY, &["08:00"], &[]));
    let now = at(NY, "2026-03-10T09:00");
    medication::archive(&c, now.clone(), &m).unwrap();
    assert!(items(&c, &now).is_empty());
    assert!(schedule::next_due(&c, &now).unwrap().is_none());
    medication::unarchive(&c, now.clone(), &m).unwrap();
    assert!(!items(&c, &now).is_empty());
}

#[test]
fn creating_a_medication_with_times_makes_a_device_zone_home_schedule() {
    let mut c = storage::open_memory().unwrap();
    let now = at(NY, "2026-03-10T09:00");
    let m = medication::create(&mut c, now.clone(), NewMedication { name: "A".into(), strength: None, form: None, times: vec!["20:00".into(), "08:00".into()] }).unwrap();
    let s = schedule::summary_for_medication(&c, &now, &m).unwrap().unwrap();
    assert_eq!((s.mode, s.zone_id.as_deref()), (ScheduleMode::HomeZone, Some(NY)));
    assert_eq!(s.slots.iter().map(|x| x.time.as_str()).collect::<Vec<_>>(), ["08:00", "20:00"]);
    // Bad times are rejected before anything is written.
    assert!(medication::create(&mut c, now.clone(), NewMedication { name: "B".into(), strength: None, form: None, times: vec!["25:00".into()] }).is_err());
    assert_eq!(count(&c, "SELECT count(*) FROM user_medication"), 1);
}

// ---- migration 003 ----

fn make_v2_fixture(path: &std::path::Path) -> (i64, i64) {
    let c = Connection::open(path).unwrap();
    c.execute_batch(migrations::MIGRATIONS[0]).unwrap();
    c.execute_batch(migrations::MIGRATIONS[1]).unwrap();
    let created = utc("2026-03-01T14:00");
    let due = utc("2026-03-10T14:00"); // 08:00 at UTC-6
    c.execute_batch(&format!(
        "PRAGMA user_version = 2;
         INSERT INTO user_medication (id, current_version_id, status, created_at_utc, updated_at_utc) VALUES ('m1', NULL, 'active', {created}, {created});
         INSERT INTO medication_version (id, medication_id, version_number, effective_from_utc, name, strength, form, source_kind, created_at_utc)
           VALUES ('v1', 'm1', 1, {created}, 'Metformin', '500 mg', 'tablet', 'manual', {created});
         UPDATE user_medication SET current_version_id = 'v1';
         INSERT INTO schedule_slot (id, medication_id, medication_version_id, local_time, created_at_utc) VALUES
           ('s1', 'm1', 'v1', '08:00', {created}), ('s2', 'm1', 'v1', '20:00', {created});
         INSERT INTO user_medication (id, current_version_id, status, created_at_utc, updated_at_utc) VALUES ('m2', NULL, 'active', {created}, {created});
         INSERT INTO medication_version (id, medication_id, version_number, effective_from_utc, name, source_kind, created_at_utc)
           VALUES ('v2', 'm2', 1, {created}, 'As needed thing', 'manual', {created});
         UPDATE user_medication SET current_version_id = 'v2' WHERE id = 'm2';
         INSERT INTO scheduled_occurrence (id, slot_id, local_date, local_time, offset_seconds, due_at_utc, zone_id) VALUES ('o1', 's1', '2026-03-10', '08:00', -21600, {due}, NULL);
         INSERT INTO dose_event (id, medication_id, medication_version_id, kind, occurred_at_utc, entered_at_utc, original_local, original_offset_seconds, occurred_offset_seconds, note, current_revision, undone, deleted)
           VALUES ('e-take', 'm1', 'v1', 'take', {due}, {due}, '2026-03-10T08:00', -21600, -21600, 'with food', 0, 0, 0),
                  ('e-undone', 'm1', 'v1', 'skip', {due}, {due}, '2026-03-10T08:00', -21600, -21600, NULL, 1, 1, 0);
         INSERT INTO dose_event_occurrence (dose_event_id, occurrence_id, linked_at_utc) VALUES ('e-take', 'o1', {due}), ('e-undone', 'o1', {due});"
    ))
    .unwrap();
    (created, due)
}

#[test]
fn v2_database_upgrades_to_schedules_keeping_every_row_and_link() {
    let dir = std::env::temp_dir().join(format!("medtracker-m3-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user-data.sqlite");
    let (created, due) = make_v2_fixture(&path);

    let opened = storage::open_report(&path).unwrap();
    let c = opened.conn;
    assert_eq!(migrations::user_version(&c).unwrap(), 3);
    assert!(opened.snapshot.as_ref().is_some_and(|s| s.exists()), "a verified pre-migration snapshot was made");
    assert_eq!(count(&c, "SELECT count(*) FROM pragma_foreign_key_check"), 0);
    assert_eq!(count(&c, "SELECT count(*) FROM pragma_integrity_check WHERE integrity_check <> 'ok'"), 0);

    // One schedule + version-1 per scheduled medication; the as-needed one has none.
    assert_eq!(count(&c, "SELECT count(*) FROM schedule"), 1);
    let (mode, zone, state, reason, from, until): (String, Option<String>, String, String, i64, Option<i64>) = c
        .query_row(
            "SELECT mode, zone_id, state, change_reason, effective_from_utc, effective_until_utc FROM schedule_version",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .unwrap();
    assert_eq!((mode.as_str(), zone, state.as_str(), reason.as_str(), until), ("home_zone", None, "active", "migrated", None));
    assert_eq!(from, created.min(due));
    // Slots keep their ids and times; the occurrence keeps its id, instant and its dose links.
    let slots: Vec<(String, String)> = c.prepare("SELECT id, local_time FROM schedule_slot ORDER BY id").unwrap().query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap().map(|r| r.unwrap()).collect();
    assert_eq!(slots, [("s1".to_string(), "08:00".to_string()), ("s2".to_string(), "20:00".to_string())]);
    let (slot, at_utc, off, st): (String, i64, i32, String) =
        c.query_row("SELECT slot_id, scheduled_at_utc, offset_seconds, state FROM scheduled_occurrence WHERE id='o1'", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).unwrap();
    assert_eq!((slot.as_str(), at_utc, off, st.as_str()), ("s1", due, -21600, "active"));
    assert_eq!(count(&c, "SELECT count(*) FROM dose_event_occurrence"), 2);
    assert_eq!(count(&c, "SELECT count(*) FROM dose_event"), 2);
    // Medication snapshots untouched.
    assert_eq!(count(&c, "SELECT count(*) FROM medication_version WHERE name='Metformin'"), 1);

    // The migrated data drives Today: the logged 08:00 card is Taken (with its original event).
    let now = TimeCtx::fixed(utc("2026-03-10T15:00"), -6 * 3600);
    let v = items(&c, &now);
    let morning = v.iter().find(|i| i.local_date == "2026-03-10" && i.local_time == "08:00").unwrap();
    assert_eq!((morning.status.clone(), morning.event_ids.clone()), (TodayStatus::Taken, vec!["e-take".to_string()]));
    let evening = v.iter().find(|i| i.local_date == "2026-03-10" && i.local_time == "20:00").unwrap();
    assert_eq!(evening.status, TodayStatus::Upcoming);
    assert_eq!(count(&c, "SELECT count(*) FROM scheduled_occurrence WHERE id='o1'"), 1, "the migrated occurrence is reused, not duplicated");

    // A migrated, zone-less schedule adopts the device zone once; after that it is fixed.
    assert_eq!(schedule::adopt_device_zone(&c, "Not/AZone").unwrap(), 0);
    assert_eq!(schedule::adopt_device_zone(&c, "America/Chicago").unwrap(), 1);
    assert_eq!(schedule::adopt_device_zone(&c, "America/Chicago").unwrap(), 0);
    assert!(c.execute("UPDATE schedule_version SET zone_id='Asia/Tokyo'", []).is_err(), "a stored zone can't be changed");
    // History still works and shows the intended slot time for the linked event.
    let h = ledger::history(&c, 10).unwrap();
    assert!(h.iter().any(|e| e.event_id == "e-take" && e.slot_time.as_deref() == Some("08:00")));
    drop(c);
    // The next launch removes the snapshot.
    drop(storage::open(&path).unwrap());
    assert!(std::fs::read_dir(&dir).unwrap().all(|e| !e.unwrap().file_name().to_string_lossy().contains(".bak")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn migration_003_rolls_back_cleanly_if_a_later_step_fails() {
    let dir = std::env::temp_dir().join(format!("medtracker-m3f-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user-data.sqlite");
    make_v2_fixture(&path);
    let bad: Vec<&str> = vec![migrations::MIGRATIONS[0], migrations::MIGRATIONS[1], migrations::MIGRATIONS[2], "SELECT * FROM nope;"];
    assert!(matches!(storage::open_with(&path, &bad), Err(AppError::MigrationFailed(_))));
    let c = Connection::open(&path).unwrap();
    assert_eq!(migrations::user_version(&c).unwrap(), 2, "still the v2 database");
    assert_eq!(count(&c, "SELECT count(*) FROM schedule_slot"), 2);
    assert_eq!(count(&c, "SELECT count(*) FROM scheduled_occurrence"), 1);
    assert_eq!(count(&c, "SELECT count(*) FROM dose_event_occurrence"), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn input_dtos_have_the_wire_shape_the_ui_sends() {
    let i: ScheduleInput = serde_json::from_str(r#"{"mode":"home_zone","zone_id":"America/New_York","slots":[{"time":"08:00","weekdays":[1,2]},{"time":"20:00"}]}"#).unwrap();
    assert_eq!(i.mode, ScheduleMode::HomeZone);
    assert_eq!(i.slots[1].weekdays, Vec::<u8>::new());
    let e: ScheduleInput = serde_json::from_str(r#"{"mode":"elapsed_interval","zone_id":null,"interval_seconds":28800,"anchor_local":"2026-03-10T08:00"}"#).unwrap();
    assert_eq!(e.mode, ScheduleMode::ElapsedInterval);
    assert!(serde_json::from_str::<ScheduleInput>(r#"{"mode":"device_local"}"#).is_err(), "device_local is not accepted");
}
