use crate::application::schedule;
use crate::clock::{fmt_date, local_from_utc, parse_local, TimeCtx, Zone};
use crate::error::{invalid, AppResult};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

const DAY_US: i64 = 86_400_000_000;

#[derive(Debug, Default, Clone, Serialize, PartialEq)]
pub struct Counts {
    /// Scheduled occurrences in range (taken + skipped + not_logged + pending + conflicts).
    pub occurrences: u32,
    pub taken: u32,
    pub skipped: u32,
    /// Earlier local day, nothing recorded. Descriptive only.
    pub not_logged: u32,
    /// Today or later, nothing recorded yet.
    pub pending: u32,
    /// Active take and skip both linked; excluded from taken/skipped.
    pub conflicts: u32,
    pub unscheduled_taken: u32,
    pub unscheduled_skipped: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct MedicationRow {
    pub medication_id: String,
    pub name: String,
    pub strength: Option<String>,
    #[serde(flatten)]
    pub counts: Counts,
}

#[derive(Debug, Clone, Serialize)]
pub struct DayRow {
    pub date: String,
    #[serde(flatten)]
    pub counts: Counts,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub from: String,
    pub to: String,
    /// Which local day an occurrence / event is grouped under, stated so the table is self-describing.
    pub grouping: String,
    pub reporting_zone: Option<String>,
    pub totals: Counts,
    pub by_medication: Vec<MedicationRow>,
    pub by_day: Vec<DayRow>,
    /// Undone entries in range: excluded from every number above.
    pub voided_entries: u32,
    /// Privacy-deleted entries in range: details erased, never counted as outcomes.
    pub deleted_entries: u32,
    pub definitions: Vec<String>,
}

pub fn definitions() -> Vec<String> {
    vec![
        "Counts only; this is not a score and says nothing about whether any dose was needed or appropriate.".into(),
        "Scheduled occurrences are the active scheduled times whose own schedule-zone date is in the range.".into(),
        "Taken or skipped means an active entry is linked to that occurrence. Several takes on one occurrence count once.".into(),
        "Earlier, not logged: nothing was recorded for a scheduled time on an earlier day. It does not mean a dose was or was not taken.".into(),
        "Later, not yet logged: the scheduled time is today or later.".into(),
        "Conflicting: both a take and a skip are recorded for one scheduled time; it is excluded from taken and skipped until you correct one.".into(),
        "Unscheduled entries are not linked to a scheduled time and have no denominator.".into(),
        "Undone entries are excluded from all counts and shown only as a separate number. Deleted entries have their details erased and are never counted as outcomes.".into(),
    ]
}

fn parse_date(s: &str) -> AppResult<jiff::civil::Date> {
    Ok(parse_local(&format!("{s}T00:00"))?.date())
}

/// Validated inclusive range (max 400 days, matching occurrence generation).
pub fn check_range(from: &str, to: &str) -> AppResult<(String, String, i64, i64)> {
    let d0 = parse_date(from)?;
    let d1 = parse_date(to)?;
    if d1 < d0 {
        return invalid("The end date must not be before the start date.");
    }
    let days = (d1 - d0).get_days() as i64;
    if days > 400 {
        return invalid("Choose a date range of up to 400 days.");
    }
    // Event days are recorded with their own offsets (at most +-14h), so pad a day each side.
    let z = Zone::fixed(0);
    let lo = z.day_start(d0)? - DAY_US;
    let hi = z.day_start(d1)? + 2 * DAY_US;
    Ok((fmt_date(d0.at(0, 0, 0, 0)), fmt_date(d1.at(0, 0, 0, 0)), lo, hi))
}

fn bump(c: &mut Counts, o: &OccState, pending: bool) {
    c.occurrences += 1;
    match (o.take, o.skip) {
        (true, true) => c.conflicts += 1,
        (true, false) => c.taken += 1,
        (false, true) => c.skipped += 1,
        (false, false) if pending => c.pending += 1,
        (false, false) => c.not_logged += 1,
    }
}

#[derive(Default, Clone, Copy)]
struct OccState {
    take: bool,
    skip: bool,
}

pub fn report(conn: &Connection, ctx: &TimeCtx, from: &str, to: &str) -> AppResult<Report> {
    let (from, to, lo, hi) = check_range(from, to)?;
    let today = fmt_date(local_from_utc(ctx.now_utc_us, ctx.zone.offset_at_utc(ctx.now_utc_us)));

    // Make sure derived occurrences exist for the part of the range that is not in the future.
    // Idempotent; creates only schedule-derived rows, never entries.
    let upto = if to.as_str() < today.as_str() { to.clone() } else { today.clone() };
    if from.as_str() <= upto.as_str() {
        schedule::generate_range(conn, ctx, &from, &upto)?;
    }

    let mut names: HashMap<String, (String, Option<String>)> = HashMap::new();
    {
        let mut s = conn.prepare(
            "SELECT m.id, v.name, v.strength FROM user_medication m JOIN medication_version v ON v.id = m.current_version_id",
        )?;
        for r in s.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?)))? {
            let (id, n, st) = r?;
            names.insert(id, (n, st));
        }
    }

    // Occurrence outcomes.
    let mut occ: HashMap<String, (String, String, OccState)> = HashMap::new(); // id -> (date, med, state)
    {
        let mut s = conn.prepare(
            "SELECT o.id, o.local_date, s.medication_id FROM scheduled_occurrence o
             JOIN schedule s ON s.id = o.schedule_id
             WHERE o.state = 'active' AND o.local_date BETWEEN ?1 AND ?2",
        )?;
        for r in s.query_map([&from, &to], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))? {
            let (id, d, m) = r?;
            occ.insert(id, (d, m, OccState::default()));
        }
    }
    {
        let mut s = conn.prepare(
            "SELECT l.occurrence_id, e.kind FROM dose_event_occurrence l
             JOIN dose_event e ON e.id = l.dose_event_id
             JOIN scheduled_occurrence o ON o.id = l.occurrence_id
             WHERE e.undone = 0 AND e.deleted = 0 AND o.local_date BETWEEN ?1 AND ?2",
        )?;
        for r in s.query_map([&from, &to], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
            let (id, kind) = r?;
            if let Some((_, _, st)) = occ.get_mut(&id) {
                if kind == "take" {
                    st.take = true
                } else {
                    st.skip = true
                }
            }
        }
    }

    let mut totals = Counts::default();
    let mut by_med: BTreeMap<String, Counts> = BTreeMap::new();
    let mut by_day: BTreeMap<String, Counts> = BTreeMap::new();
    for (date, med, st) in occ.values() {
        let pending = date.as_str() >= today.as_str();
        bump(&mut totals, st, pending);
        bump(by_med.entry(med.clone()).or_default(), st, pending);
        bump(by_day.entry(date.clone()).or_default(), st, pending);
    }

    // Events: unscheduled, voided and deleted, by the local day recorded with the event.
    let mut voided = 0;
    let mut deleted = 0;
    {
        let mut s = conn.prepare(
            "SELECT e.medication_id, e.kind, e.undone, e.deleted, e.occurred_at_utc, e.occurred_offset_seconds,
                    EXISTS (SELECT 1 FROM dose_event_occurrence l WHERE l.dose_event_id = e.id)
             FROM dose_event e WHERE e.occurred_at_utc BETWEEN ?1 AND ?2",
        )?;
        let rows = s.query_map([lo, hi], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?, r.get::<_, i64>(4)?, r.get::<_, i32>(5)?, r.get::<_, i64>(6)?))
        })?;
        for r in rows {
            let (med, kind, undone, del, at, off, linked) = r?;
            let date = fmt_date(local_from_utc(at, off));
            if date.as_str() < from.as_str() || date.as_str() > to.as_str() {
                continue;
            }
            if del == 1 {
                deleted += 1;
            } else if undone == 1 {
                voided += 1;
            } else if linked == 0 {
                for c in [&mut totals, by_med.entry(med).or_default(), by_day.entry(date).or_default()] {
                    if kind == "take" {
                        c.unscheduled_taken += 1
                    } else {
                        c.unscheduled_skipped += 1
                    }
                }
            }
        }
    }

    let mut meds: Vec<MedicationRow> = by_med
        .into_iter()
        .map(|(id, counts)| {
            let (name, strength) = names.get(&id).cloned().unwrap_or(("Unknown medication".into(), None));
            MedicationRow { medication_id: id, name, strength, counts }
        })
        .collect();
    meds.sort_by(|a, b| (a.name.to_lowercase(), &a.medication_id).cmp(&(b.name.to_lowercase(), &b.medication_id)));

    Ok(Report {
        grouping: "Scheduled times are grouped by their own schedule-local date; unscheduled entries by the local date recorded with the entry.".into(),
        reporting_zone: ctx.zone.id().map(str::to_string),
        from,
        to,
        totals,
        by_medication: meds,
        by_day: by_day.into_iter().map(|(date, counts)| DayRow { date, counts }).collect(),
        voided_entries: voided,
        deleted_entries: deleted,
        definitions: definitions(),
    })
}
