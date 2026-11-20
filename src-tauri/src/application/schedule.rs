//! Schedules, schedule versions, occurrence materialization and the derived Today view.
//!
//! Rules this module upholds:
//! * A `schedule_version` is immutable. Edit / pause / resume / end close the current version at
//!   the change instant and open the next one there; ranges never overlap.
//! * Occurrences are generated per version, idempotently (`INSERT OR IGNORE` on the identity
//!   index), only inside the version's effective range, and never for a paused/ended version, so
//!   nothing is back-filled across a pause or after an end.
//! * Occurrences are snapshots. A change may mark FUTURE, unrecorded ones `superseded`; it never
//!   edits or deletes past ones, and never touches an occurrence that has an active dose event.
//! * Status (Upcoming / Due / Taken / Skipped / Conflict / EarlierNotLogged) is derived on read
//!   from occurrence + dose events + clock. No "due" flag is ever stored.

use crate::clock::{fmt_date, local_from_utc, parse_hhmm, parse_local, tzdb_version, TimeCtx, Zone};
use crate::domain::schedule::{
    self as dom, add_days, mask_from_weekdays, weekdays_from_mask, Candidate, Definition, ScheduleMode, ScheduleState, Slot,
};
use crate::dto::*;
use crate::error::{invalid, AppError, AppResult};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

const DAY_US: i64 = 86_400_000_000;
/// How far back Today looks for earlier, unrecorded occurrences (whole local days).
const LOOKBACK_DAYS: i64 = 1;
/// How far ahead "next due" searches.
const NEXT_DUE_DAYS: i64 = 8;

/// A stored schedule version with its slots.
#[derive(Debug, Clone)]
pub struct VersionRow {
    pub id: String,
    pub schedule_id: String,
    pub medication_id: String,
    pub version_number: i64,
    pub from_utc: i64,
    pub until_utc: Option<i64>,
    pub state: ScheduleState,
    pub def: Definition,
    /// (slot_key, slot_id)
    pub slot_ids: Vec<(String, String)>,
}

impl VersionRow {
    /// The zone this version is read in. A home-zone version without a stored zone (migrated
    /// from Phase 1, not yet adopted) or whose zone id is no longer in the bundled database falls
    /// back to the device zone rather than failing; `elapsed_interval` has no zone of its own.
    pub fn zone(&self, ctx: &TimeCtx) -> Zone {
        match (self.def.mode, &self.def.zone_id) {
            (ScheduleMode::HomeZone, Some(id)) => Zone::iana(id).unwrap_or_else(|| ctx.zone.clone()),
            _ => ctx.zone.clone(),
        }
    }
}

fn load_version(conn: &Connection, id: &str) -> AppResult<VersionRow> {
    let row = conn
        .query_row(
            "SELECT v.id, v.schedule_id, s.medication_id, v.version_number, v.effective_from_utc, v.effective_until_utc,
                    v.state, v.mode, v.zone_id, v.interval_seconds, v.anchor_utc
             FROM schedule_version v JOIN schedule s ON s.id = v.schedule_id WHERE v.id = ?1",
            [id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, Option<i64>>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, String>(7)?,
                    r.get::<_, Option<String>>(8)?,
                    r.get::<_, Option<i64>>(9)?,
                    r.get::<_, Option<i64>>(10)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound("Schedule not found.".into()))?;
    let (id, schedule_id, medication_id, version_number, from_utc, until_utc, state, mode, zone_id, interval, anchor) = row;
    let mut stmt = conn.prepare("SELECT id, slot_key, local_time, weekday_mask FROM schedule_slot WHERE schedule_version_id = ?1 ORDER BY local_time")?;
    let mut slots = Vec::new();
    let mut slot_ids = Vec::new();
    for r in stmt.query_map([&id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, i64>(3)?)))? {
        let (sid, key, time, mask) = r?;
        slots.push(Slot { minutes: parse_hhmm(&time)?, weekday_mask: mask as u8 });
        slot_ids.push((key, sid));
    }
    let mode = ScheduleMode::parse(&mode).ok_or_else(|| AppError::Storage("Unknown schedule mode.".into()))?;
    let state = ScheduleState::parse(&state).ok_or_else(|| AppError::Storage("Unknown schedule state.".into()))?;
    Ok(VersionRow {
        id,
        schedule_id,
        medication_id,
        version_number,
        from_utc,
        until_utc,
        state,
        def: Definition { mode, zone_id, slots, interval_seconds: interval, anchor_utc_us: anchor },
        slot_ids,
    })
}

/// Active-state versions of active medications that are still in force after `since`.
fn load_active_versions(conn: &Connection, since: i64) -> AppResult<Vec<VersionRow>> {
    let mut stmt = conn.prepare(
        "SELECT v.id FROM schedule_version v
         JOIN schedule s ON s.id = v.schedule_id
         JOIN user_medication m ON m.id = s.medication_id AND m.status = 'active'
         WHERE v.state = 'active' AND (v.effective_until_utc IS NULL OR v.effective_until_utc > ?1)
         ORDER BY v.effective_from_utc, v.id",
    )?;
    let ids: Vec<String> = stmt.query_map([since], |r| r.get(0))?.collect::<Result<_, _>>()?;
    ids.iter().map(|id| load_version(conn, id)).collect()
}

// ---------------------------------------------------------------------------------------------
// Definitions from input
// ---------------------------------------------------------------------------------------------

/// Turns user input into a validated domain definition. `previous` supplies the current anchor
/// when an elapsed-interval edit doesn't restate it.
pub fn definition_from_input(input: &ScheduleInput, ctx: &TimeCtx, previous: Option<&Definition>) -> AppResult<Definition> {
    match input.mode {
        ScheduleMode::HomeZone => {
            let zone = input.zone_id.clone().filter(|z| !z.trim().is_empty());
            if zone.is_none() {
                return invalid("Choose a time zone for these times.");
            }
            let slots = input
                .slots
                .iter()
                .map(|s| Ok(Slot { minutes: parse_hhmm(&s.time)?, weekday_mask: mask_from_weekdays(&s.weekdays)? }))
                .collect::<AppResult<Vec<_>>>()?;
            Definition { mode: ScheduleMode::HomeZone, zone_id: zone, slots, interval_seconds: input.interval_seconds, anchor_utc_us: None }.validated()
        }
        ScheduleMode::ElapsedInterval => {
            let anchor = match &input.anchor_local {
                Some(s) => ctx.resolve_local_choice(s, false)?.utc_us,
                None => previous
                    .filter(|p| p.mode == ScheduleMode::ElapsedInterval)
                    .and_then(|p| p.anchor_utc_us)
                    .unwrap_or(ctx.now_utc_us),
            };
            Definition { mode: ScheduleMode::ElapsedInterval, zone_id: input.zone_id.clone(), slots: input.slots.iter().map(|_| Slot { minutes: 0, weekday_mask: 0 }).collect(), interval_seconds: input.interval_seconds, anchor_utc_us: Some(anchor) }
                .validated()
        }
    }
}

fn preview_of(c: &Candidate, zone: &Zone) -> PreviewOccurrence {
    let local = local_from_utc(c.scheduled_at_utc_us, c.offset_seconds);
    PreviewOccurrence {
        scheduled_at_utc_us: c.scheduled_at_utc_us,
        local_date: fmt_date(local),
        local_time: format!("{:02}:{:02}", local.hour(), local.minute()),
        offset_seconds: c.offset_seconds,
        zone_id: zone.id().map(str::to_string),
        zone_abbreviation: zone.abbreviation_at(c.scheduled_at_utc_us),
        adjusted: c.adjusted,
    }
}

/// The next seven occurrences of a not-yet-saved definition (spec: the creation flow previews at
/// least the next seven dates). Read-only.
pub fn preview(ctx: &TimeCtx, input: &ScheduleInput) -> AppResult<Vec<PreviewOccurrence>> {
    let def = definition_from_input(input, ctx, None)?;
    let zone = match (&def.mode, &def.zone_id) {
        (ScheduleMode::HomeZone, Some(id)) => Zone::iana(id).unwrap_or_else(|| ctx.zone.clone()),
        _ => ctx.zone.clone(),
    };
    Ok(dom::next_occurrences(&def, &zone, ctx.now_utc_us, None, 7)?.iter().map(|c| preview_of(c, &zone)).collect())
}

// ---------------------------------------------------------------------------------------------
// Create / edit / pause / resume / end
// ---------------------------------------------------------------------------------------------

fn insert_version(
    tx: &Connection,
    ctx: &TimeCtx,
    schedule_id: &str,
    medication_version_id: &str,
    number: i64,
    from_utc: i64,
    state: ScheduleState,
    def: &Definition,
    reason: &str,
) -> AppResult<String> {
    let id = Uuid::new_v4().to_string();
    let recurrence = if def.mode == ScheduleMode::ElapsedInterval { "fixed_interval" } else { "weekly_slots" };
    tx.execute(
        "INSERT INTO schedule_version (id, schedule_id, medication_version_id, version_number, effective_from_utc, state, mode,
           zone_id, recurrence, interval_seconds, anchor_utc, tzdb_version, change_reason, created_at_utc)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        params![
            id,
            schedule_id,
            medication_version_id,
            number,
            from_utc,
            state.as_str(),
            def.mode.as_str(),
            def.zone_id,
            recurrence,
            def.interval_seconds,
            def.anchor_utc_us,
            tzdb_version(),
            reason,
            ctx.now_utc_us
        ],
    )?;
    for s in &def.slots {
        if def.mode == ScheduleMode::ElapsedInterval {
            break;
        }
        tx.execute(
            "INSERT INTO schedule_slot (id, schedule_version_id, slot_key, local_time, weekday_mask, created_at_utc) VALUES (?1,?2,?3,?4,?5,?6)",
            params![Uuid::new_v4().to_string(), id, s.key(), s.time(), s.weekday_mask, ctx.now_utc_us],
        )?;
    }
    Ok(id)
}

fn current_medication_version(tx: &Connection, medication_id: &str) -> AppResult<String> {
    tx.query_row("SELECT current_version_id FROM user_medication WHERE id = ?1", [medication_id], |r| r.get::<_, Option<String>>(0))
        .optional()?
        .flatten()
        .ok_or_else(|| AppError::NotFound("Medication not found.".into()))
}

/// Creates a schedule and its first version inside the caller's transaction (used when a
/// medication is created together with its first schedule).
pub fn create_in_tx(tx: &Connection, ctx: &TimeCtx, medication_id: &str, def: &Definition) -> AppResult<String> {
    let med_version = current_medication_version(tx, medication_id)?;
    let open: i64 = tx.query_row("SELECT count(*) FROM schedule WHERE medication_id = ?1 AND status <> 'ended'", [medication_id], |r| r.get(0))?;
    if open > 0 {
        return invalid("This medication already has a schedule. Edit it instead.");
    }
    let schedule_id = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO schedule (id, medication_id, status, archived, created_at_utc) VALUES (?1,?2,'active',0,?3)",
        params![schedule_id, medication_id, ctx.now_utc_us],
    )?;
    let vid = insert_version(tx, ctx, &schedule_id, &med_version, 1, ctx.now_utc_us, ScheduleState::Active, def, "create")?;
    tx.execute("UPDATE schedule SET current_version_id = ?1 WHERE id = ?2", params![vid, schedule_id])?;
    Ok(schedule_id)
}

pub fn create(conn: &mut Connection, ctx: &TimeCtx, input: NewSchedule) -> AppResult<String> {
    let def = definition_from_input(&input.definition, ctx, None)?;
    let tx = conn.transaction()?;
    let id = create_in_tx(&tx, ctx, &input.medication_id, &def)?;
    tx.commit()?;
    Ok(id)
}

enum Change {
    Edit(Definition),
    Pause,
    Resume,
    End,
}

/// Closes the current version at `now` and opens the next one there, in ONE transaction, then
/// supersedes future unrecorded occurrences of the old version. Nothing in the past is touched.
fn transition(conn: &mut Connection, ctx: &TimeCtx, schedule_id: &str, change: Change) -> AppResult<()> {
    let tx = conn.transaction()?;
    let (status, cur_id, med_id): (String, Option<String>, String) = tx
        .query_row("SELECT status, current_version_id, medication_id FROM schedule WHERE id = ?1", [schedule_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .optional()?
        .ok_or_else(|| AppError::NotFound("Schedule not found.".into()))?;
    let cur = load_version(&tx, cur_id.as_deref().unwrap_or_default())?;
    let state = ScheduleState::parse(&status).unwrap_or(ScheduleState::Ended);
    let now = ctx.now_utc_us;
    if now < cur.from_utc {
        return invalid("The device clock is earlier than this schedule's last change. Check the date and time, then try again.");
    }
    let (new_state, def, reason) = match (change, state) {
        (_, ScheduleState::Ended) => return invalid("This schedule has ended. Start a new schedule instead."),
        (Change::Edit(def), s) => {
            if def == cur.def {
                return invalid("Nothing was changed.");
            }
            (s, def, "edit")
        }
        (Change::Pause, ScheduleState::Active) => (ScheduleState::Paused, cur.def.clone(), "pause"),
        (Change::Pause, _) => return invalid("This schedule is already paused."),
        (Change::Resume, ScheduleState::Paused) => (ScheduleState::Active, cur.def.clone(), "resume"),
        (Change::Resume, _) => return invalid("This schedule isn't paused."),
        (Change::End, _) => (ScheduleState::Ended, cur.def.clone(), "end"),
    };
    let med_version = current_medication_version(&tx, &med_id)?;
    tx.execute("UPDATE schedule_version SET effective_until_utc = ?1 WHERE id = ?2", params![now, cur.id])?;
    let vid = insert_version(&tx, ctx, schedule_id, &med_version, cur.version_number + 1, now, new_state, &def, reason)?;
    tx.execute("UPDATE schedule SET current_version_id = ?1, status = ?2 WHERE id = ?3", params![vid, new_state.as_str(), schedule_id])?;
    // Future, unrecorded occurrences of the old version are replaced. Anything already recorded
    // against (an active take/skip), and everything before `now`, stays exactly as it was.
    tx.execute(
        "UPDATE scheduled_occurrence SET state = 'superseded', superseded_at_utc = ?1
         WHERE schedule_id = ?2 AND state = 'active' AND scheduled_at_utc >= ?1
           AND NOT EXISTS (SELECT 1 FROM dose_event_occurrence l JOIN dose_event e ON e.id = l.dose_event_id
                           WHERE l.occurrence_id = scheduled_occurrence.id AND e.undone = 0)",
        params![now, schedule_id],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn edit(conn: &mut Connection, ctx: &TimeCtx, input: EditSchedule) -> AppResult<()> {
    let prev = {
        let cur: Option<String> = conn.query_row("SELECT current_version_id FROM schedule WHERE id = ?1", [&input.schedule_id], |r| r.get(0)).optional()?.flatten();
        let cur = cur.ok_or_else(|| AppError::NotFound("Schedule not found.".into()))?;
        load_version(conn, &cur)?
    };
    let def = definition_from_input(&input.definition, ctx, Some(&prev.def))?;
    transition(conn, ctx, &input.schedule_id, Change::Edit(def))
}

pub fn pause(conn: &mut Connection, ctx: &TimeCtx, schedule_id: &str) -> AppResult<()> {
    transition(conn, ctx, schedule_id, Change::Pause)
}

pub fn resume(conn: &mut Connection, ctx: &TimeCtx, schedule_id: &str) -> AppResult<()> {
    transition(conn, ctx, schedule_id, Change::Resume)
}

pub fn end(conn: &mut Connection, ctx: &TimeCtx, schedule_id: &str) -> AppResult<()> {
    transition(conn, ctx, schedule_id, Change::End)
}

/// A migrated Phase 1 schedule has no zone ("resolve in the device zone"). Gives those versions
/// the device's IANA zone once, so they stop depending on where the device later travels.
pub fn adopt_device_zone(conn: &Connection, zone_id: &str) -> AppResult<usize> {
    if Zone::iana(zone_id).is_none() {
        return Ok(0);
    }
    Ok(conn.execute("UPDATE schedule_version SET zone_id = ?1 WHERE mode = 'home_zone' AND zone_id IS NULL", [zone_id])?)
}

// ---------------------------------------------------------------------------------------------
// Occurrence generation
// ---------------------------------------------------------------------------------------------

/// Inserts the occurrences of `v` that fall in `[lo, hi)`, clipped to the version's effective
/// range. Idempotent: running it again inserts nothing. Returns how many rows were added.
fn materialize(tx: &Connection, ctx: &TimeCtx, v: &VersionRow, lo: i64, hi: i64) -> AppResult<usize> {
    if v.state != ScheduleState::Active {
        return Ok(0);
    }
    let lo = lo.max(v.from_utc);
    let hi = hi.min(v.until_utc.unwrap_or(i64::MAX));
    if lo >= hi {
        return Ok(0);
    }
    let zone = v.zone(ctx);
    let mut added = 0;
    for c in dom::generate(&v.def, &zone, lo, hi)? {
        // A surviving occurrence of an earlier version for the same slot and date (or instant)
        // already represents this one; a second would show the same dose twice.
        let taken: i64 = tx.query_row(
            "SELECT count(*) FROM scheduled_occurrence
             WHERE schedule_id = ?1 AND state = 'active' AND schedule_version_id <> ?2
               AND (scheduled_at_utc = ?3 OR (slot_key IS NOT NULL AND slot_key = ?4 AND local_date = ?5))",
            params![v.schedule_id, v.id, c.scheduled_at_utc_us, c.slot_key, c.local_date],
            |r| r.get(0),
        )?;
        if taken > 0 {
            continue;
        }
        let slot_id = c.slot_key.as_ref().and_then(|k| v.slot_ids.iter().find(|(key, _)| key == k)).map(|(_, id)| id.clone());
        added += tx.execute(
            "INSERT OR IGNORE INTO scheduled_occurrence
               (id, schedule_id, schedule_version_id, slot_id, slot_key, seq, local_date, local_time, zone_id, offset_seconds,
                fold, adjusted, scheduled_at_utc, tzdb_version, generated_at_utc, state)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,'active')",
            params![
                Uuid::new_v4().to_string(),
                v.schedule_id,
                v.id,
                slot_id,
                c.slot_key,
                c.seq,
                c.local_date,
                c.intended_time,
                zone.id(),
                c.offset_seconds,
                c.fold,
                c.adjusted as i64,
                c.scheduled_at_utc_us,
                tzdb_version(),
                ctx.now_utc_us
            ],
        )?;
    }
    Ok(added)
}

/// Materializes every active schedule's occurrences for the inclusive local date range
/// `from..=to` (each schedule's dates are read in its own zone; elapsed-interval schedules use
/// the device zone). Idempotent. Returns the number of rows added.
pub fn generate_range(conn: &Connection, ctx: &TimeCtx, from: &str, to: &str) -> AppResult<usize> {
    let d0 = parse_local(&format!("{from}T00:00"))?.date();
    let d1 = parse_local(&format!("{to}T00:00"))?.date();
    if d1 < d0 || add_days(d0, 400)? < d1 {
        return invalid("Choose a date range of up to 400 days.");
    }
    let versions = load_active_versions(conn, i64::MIN)?;
    let tx = conn.unchecked_transaction()?;
    let mut added = 0;
    for v in &versions {
        let zone = v.zone(ctx);
        added += materialize(&tx, ctx, v, zone.day_start(d0)?, zone.day_start(add_days(d1, 1)?)?)?;
    }
    tx.commit()?;
    Ok(added)
}

/// Today window of a version: (lookback start, start of today, end of today) in its own zone.
fn today_window(v: &VersionRow, ctx: &TimeCtx) -> AppResult<(i64, i64, i64)> {
    let zone = v.zone(ctx);
    let today = zone.local_date_at(ctx.now_utc_us);
    Ok((zone.day_start(add_days(today, -LOOKBACK_DAYS)?)?, zone.day_start(today)?, zone.day_start(add_days(today, 1)?)?))
}

// ---------------------------------------------------------------------------------------------
// Today (derived)
// ---------------------------------------------------------------------------------------------

fn hhmm(utc_us: i64, offset: i32) -> String {
    let l = local_from_utc(utc_us, offset);
    format!("{:02}:{:02}", l.hour(), l.minute())
}

pub fn today(conn: &Connection, ctx: TimeCtx) -> AppResult<Vec<TodayItem>> {
    let now = ctx.now_utc_us;
    let versions = load_active_versions(conn, now - (LOOKBACK_DAYS + 2) * DAY_US)?;
    let tx = conn.unchecked_transaction()?;
    for v in &versions {
        let (lo, _, hi) = today_window(v, &ctx)?;
        materialize(&tx, &ctx, v, lo, hi)?;
    }
    tx.commit()?;

    let mut items = Vec::new();
    for v in &versions {
        let (lo, today_start, hi) = today_window(v, &ctx)?;
        let (name, strength, form): (String, Option<String>, Option<String>) = conn.query_row(
            "SELECT mv.name, mv.strength, mv.form FROM user_medication m JOIN medication_version mv ON mv.id = m.current_version_id WHERE m.id = ?1",
            [&v.medication_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        let mut stmt = conn.prepare(
            "SELECT id, scheduled_at_utc, zone_id, offset_seconds, adjusted FROM scheduled_occurrence
             WHERE schedule_version_id = ?1 AND state = 'active' AND scheduled_at_utc >= ?2 AND scheduled_at_utc < ?3
             ORDER BY scheduled_at_utc",
        )?;
        let occs: Vec<(String, i64, Option<String>, i32, bool)> = stmt
            .query_map(params![v.id, lo, hi], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get::<_, i64>(4)? == 1)))?
            .collect::<Result<_, _>>()?;
        for (occ_id, at, zone_id, offset, adjusted) in occs {
            let mut q = conn.prepare(
                "SELECT e.id, e.kind FROM dose_event e JOIN dose_event_occurrence l ON l.dose_event_id = e.id
                 WHERE l.occurrence_id = ?1 AND e.undone = 0 ORDER BY e.occurred_at_utc, e.id",
            )?;
            let events: Vec<(String, String)> = q.query_map([&occ_id], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
            let takes = events.iter().any(|e| e.1 == "take");
            let skips = events.iter().any(|e| e.1 == "skip");
            let same_day = at >= today_start;
            let status = match (takes, skips) {
                (true, true) => TodayStatus::Conflict,
                (true, false) | (false, true) if !same_day => continue, // recorded on an earlier day: not on Today
                (true, false) => TodayStatus::Taken,
                (false, true) => TodayStatus::Skipped,
                _ if !same_day => TodayStatus::EarlierNotLogged,
                _ if at <= now => TodayStatus::Due,
                _ => TodayStatus::Upcoming,
            };
            let device_offset = ctx.zone.offset_at_utc(at);
            // An every-N-hours schedule has no zone of its own: its cards read in the device's
            // current zone (travel changes the wall time, never the cadence). Home-zone cards use
            // the zone and offset snapshotted on the occurrence.
            let (zone_id, offset) = if v.def.mode == ScheduleMode::ElapsedInterval { (ctx.zone.id().map(str::to_string), device_offset) } else { (zone_id, offset) };
            let zone = Zone::from_stored(zone_id.as_deref(), offset);
            let local = local_from_utc(at, offset);
            items.push(TodayItem {
                occurrence_id: occ_id,
                schedule_id: v.schedule_id.clone(),
                medication_id: v.medication_id.clone(),
                name: name.clone(),
                strength: strength.clone(),
                form: form.clone(),
                mode: v.def.mode,
                scheduled_at_utc_us: at,
                local_date: fmt_date(local),
                local_time: hhmm(at, offset),
                zone_id: zone_id.clone(),
                zone_abbreviation: zone.abbreviation_at(at),
                offset_seconds: offset,
                adjusted,
                device_time: (device_offset != offset).then(|| hhmm(at, device_offset)),
                status,
                event_ids: events.into_iter().map(|e| e.0).collect(),
            });
        }
    }
    items.sort_by(|a, b| (a.scheduled_at_utc_us, &a.name, &a.occurrence_id).cmp(&(b.scheduled_at_utc_us, &b.name, &b.occurrence_id)));
    Ok(items)
}

/// Checks that an occurrence may be acted on now, for the explicit Taken/Skip link: it exists,
/// belongs to this medication, is still current (not superseded by a schedule change), and its
/// local day (in its own schedule zone) is today. Otherwise the caller's Today is stale.
pub fn check_linkable(tx: &Connection, ctx: &TimeCtx, occurrence_id: &str, medication_id: &str) -> AppResult<()> {
    let row: Option<(String, i64, String, String)> = tx
        .query_row(
            "SELECT o.state, o.scheduled_at_utc, s.medication_id, o.schedule_version_id
             FROM scheduled_occurrence o JOIN schedule s ON s.id = o.schedule_id WHERE o.id = ?1",
            [occurrence_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    let (state, at, med, version_id) = row.ok_or_else(|| AppError::NotFound("Scheduled time not found.".into()))?;
    if med != medication_id {
        return invalid("That scheduled time belongs to a different medication.");
    }
    if state != "active" {
        return Err(AppError::StaleDate("This schedule was changed, so Today was out of date. It has been refreshed; please check it and try again.".into()));
    }
    let v = load_version(tx, &version_id)?;
    let (_, today_start, today_end) = today_window(&v, ctx)?;
    if at < today_start || at >= today_end {
        return Err(AppError::StaleDate("The day has changed, so Today was out of date. It has been refreshed; please check it and try again.".into()));
    }
    Ok(())
}

/// True when the occurrence was replaced by a schedule change (so restoring a dose onto it would
/// bring back a card that no longer exists).
pub fn is_superseded(conn: &Connection, event_id: &str) -> AppResult<bool> {
    let n: i64 = conn.query_row(
        "SELECT count(*) FROM dose_event_occurrence l JOIN scheduled_occurrence o ON o.id = l.occurrence_id
         WHERE l.dose_event_id = ?1 AND o.state <> 'active'",
        [event_id],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

/// The next occurrence after now that hasn't been recorded, searched up to eight days ahead.
/// Computed, not stored: it can fall on a day Today hasn't materialized yet.
pub fn next_due(conn: &Connection, ctx: &TimeCtx) -> AppResult<Option<NextDue>> {
    let now = ctx.now_utc_us;
    let mut best: Option<(i64, NextDue)> = None;
    for v in load_active_versions(conn, now)? {
        let zone = v.zone(ctx);
        let lo = (now + 1).max(v.from_utc);
        let hi = (now + NEXT_DUE_DAYS * DAY_US).min(v.until_utc.unwrap_or(i64::MAX));
        for c in dom::next_occurrences(&v.def, &zone, lo, Some(hi), 64)? {
            let recorded: i64 = conn.query_row(
                "SELECT count(*) FROM scheduled_occurrence o
                 JOIN dose_event_occurrence l ON l.occurrence_id = o.id JOIN dose_event e ON e.id = l.dose_event_id
                 WHERE o.schedule_id = ?1 AND o.state = 'active' AND e.undone = 0
                   AND (o.scheduled_at_utc = ?2 OR (o.slot_key IS NOT NULL AND o.slot_key = ?3 AND o.local_date = ?4))",
                params![v.schedule_id, c.scheduled_at_utc_us, c.slot_key, c.local_date],
                |r| r.get(0),
            )?;
            if recorded > 0 {
                continue;
            }
            if best.as_ref().is_none_or(|(t, _)| c.scheduled_at_utc_us < *t) {
                let (name, strength): (String, Option<String>) = conn.query_row(
                    "SELECT mv.name, mv.strength FROM user_medication m JOIN medication_version mv ON mv.id = m.current_version_id WHERE m.id = ?1",
                    [&v.medication_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )?;
                best = Some((c.scheduled_at_utc_us, NextDue { medication_id: v.medication_id.clone(), name, strength, occurrence: preview_of(&c, &zone) }));
            }
            break; // candidates are in time order: the first unrecorded one is this version's best
        }
    }
    Ok(best.map(|b| b.1))
}

// ---------------------------------------------------------------------------------------------
// Summaries for the Medications screen
// ---------------------------------------------------------------------------------------------

/// The medication's most recent schedule (an open one if any), with its next seven occurrences.
pub fn summary_for_medication(conn: &Connection, ctx: &TimeCtx, medication_id: &str) -> AppResult<Option<ScheduleSummary>> {
    let row: Option<(String, String)> = conn
        .query_row(
            "SELECT id, current_version_id FROM schedule WHERE medication_id = ?1 AND current_version_id IS NOT NULL
             ORDER BY (status = 'ended'), created_at_utc DESC, id LIMIT 1",
            [medication_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((schedule_id, version_id)) = row else { return Ok(None) };
    let v = load_version(conn, &version_id)?;
    let zone = v.zone(ctx);
    let next = if v.state == ScheduleState::Active {
        dom::next_occurrences(&v.def, &zone, ctx.now_utc_us.max(v.from_utc), None, 7)?.iter().map(|c| preview_of(c, &zone)).collect()
    } else {
        vec![]
    };
    Ok(Some(ScheduleSummary {
        id: schedule_id,
        state: v.state,
        version_number: v.version_number,
        mode: v.def.mode,
        zone_id: v.def.zone_id.clone(),
        slots: v.def.slots.iter().map(|s| SlotDto { time: s.time(), weekdays: weekdays_from_mask(s.weekday_mask) }).collect(),
        interval_seconds: v.def.interval_seconds,
        anchor_utc_us: v.def.anchor_utc_us,
        effective_from_utc_us: v.from_utc,
        next,
    }))
}
