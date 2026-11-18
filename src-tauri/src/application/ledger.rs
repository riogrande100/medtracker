use crate::application::schedule;
use crate::clock::{fmt_local, local_from_utc, parse_local, TimeCtx, Zone};
use crate::dto::*;
use crate::error::{invalid, AppError, AppResult};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

fn clean_note(n: Option<String>) -> AppResult<Option<String>> {
    let n = n.map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    if n.as_deref().map_or(0, |v| v.chars().count()) > 500 {
        return invalid("Notes must be 500 characters or fewer.");
    }
    Ok(n)
}

/// Today's occurrences with derived status. See `schedule::today`.
pub fn today(conn: &Connection, ctx: TimeCtx) -> AppResult<Vec<TodayItem>> {
    crate::application::schedule::today(conn, ctx)
}

pub fn record(conn: &mut Connection, ctx: TimeCtx, input: RecordDose) -> AppResult<String> {
    let note = clean_note(input.note)?;
    // Backdated wall times use the offset in force on the ENTERED date (see Zone::resolve for
    // the DST gap/overlap rule); "now" uses the offset in force now.
    let (occurred, offset, local) = match &input.occurred_local {
        Some(s) => {
            let r = ctx.resolve_local_choice(s, input.second_occurrence.unwrap_or(false))?;
            (r.utc_us, r.offset_seconds, r.local)
        }
        None => {
            let off = ctx.offset_now();
            (ctx.now_utc_us, off, local_from_utc(ctx.now_utc_us, off))
        }
    };
    // One minute of tolerance for the minute-precision "now" shown in the form.
    if occurred > ctx.now_utc_us + 60_000_000 {
        return invalid("A dose can't be recorded in the future.");
    }
    let tx = conn.transaction()?;
    let version: Option<String> = tx
        .query_row(
            "SELECT current_version_id FROM user_medication WHERE id=?1 AND status='active'",
            [&input.medication_id],
            |r| r.get(0),
        )
        .optional()?;
    let version = version.ok_or_else(|| AppError::NotFound("Medication not found or archived.".into()))?;

    // Taken/Skip on a card is the explicit link to that occurrence. It must still be current and
    // belong to today in its schedule's zone; Today can go stale across midnight, device resume,
    // a time-zone change or a schedule edit, and logging against a stale card is refused.
    let mut occurrence_id = None;
    if let Some(occ) = &input.occurrence_id {
        schedule::check_linkable(&tx, &ctx, occ, &input.medication_id)?;
        let dup: i64 = tx.query_row(
            "SELECT count(*) FROM dose_event e JOIN dose_event_occurrence l ON l.dose_event_id=e.id
             WHERE l.occurrence_id=?1 AND e.undone=0 AND e.kind=?2",
            params![occ, input.kind.as_str()],
            |r| r.get(0),
        )?;
        if dup > 0 {
            return invalid("Already recorded for this time. Undo it first to change it.");
        }
        occurrence_id = Some(occ.clone());
    }

    let id = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO dose_event (id, medication_id, medication_version_id, kind, occurred_at_utc, entered_at_utc,
           original_local, original_offset_seconds, occurred_offset_seconds, original_zone_id, note)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8,?9,?10)",
        params![id, input.medication_id, version, input.kind.as_str(), occurred, ctx.now_utc_us, fmt_local(local), offset, ctx.zone.id(), note],
    )?;
    if let Some(occ) = occurrence_id {
        tx.execute(
            "INSERT INTO dose_event_occurrence (dose_event_id, occurrence_id, linked_at_utc) VALUES (?1,?2,?3)",
            params![id, occ, ctx.now_utc_us],
        )?;
    }
    tx.commit()?;
    Ok(id)
}

fn snapshot(kind: &str, local: &str, offset: i32, note: &Option<String>) -> String {
    // The offset disambiguates a repeated (fall-back) hour, where two instants share one wall time.
    serde_json::json!({ "kind": kind, "occurred_local": local, "offset_seconds": offset, "note": note }).to_string()
}

pub fn correct(conn: &mut Connection, ctx: TimeCtx, input: CorrectDose) -> AppResult<()> {
    let tx = conn.transaction()?;
    let (kind, occurred, offset, zone_id, note, rev, undone, deleted): (String, i64, i32, Option<String>, Option<String>, i64, i64, i64) = tx
        .query_row(
            "SELECT kind, occurred_at_utc, occurred_offset_seconds, original_zone_id, note, current_revision, undone, deleted
             FROM dose_event WHERE id=?1",
            [&input.event_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound("Entry not found.".into()))?;
    if deleted == 1 {
        return invalid("This entry was deleted and can't be edited.");
    }
    if undone == 1 {
        return invalid("This entry was undone. Restore it before editing.");
    }
    let current_local = local_from_utc(occurred, offset);
    let before = snapshot(&kind, &fmt_local(current_local), offset, &note);
    let new_kind = input.kind.map(|k| k.as_str().to_string()).unwrap_or(kind);
    // The corrected wall time is read in the event's own zone, using the offset in force on the
    // NEW date (a correction can move an entry across a DST change).
    // If the submitted wall time is the entry's own stored wall time (and no explicit choice of
    // occurrence is given) the stored instant is kept: in a repeated hour the wall time alone
    // can't tell the two instants apart, and re-resolving would silently move a second-occurrence
    // entry an hour earlier.
    let (new_occurred, new_offset) = match &input.occurred_local {
        Some(s) => {
            let wall = parse_local(s)?;
            if wall == current_local && input.second_occurrence.is_none() {
                (occurred, offset)
            } else {
                let r = Zone::from_stored(zone_id.as_deref(), offset).resolve_choice(wall, input.second_occurrence.unwrap_or(false))?;
                (r.utc_us, r.offset_seconds)
            }
        }
        None => (occurred, offset),
    };
    if new_occurred > ctx.now_utc_us + 60_000_000 {
        return invalid("A dose can't be recorded in the future.");
    }
    let new_note = match input.note {
        Some(n) => clean_note(Some(n))?,
        None => note,
    };
    let after = snapshot(&new_kind, &fmt_local(local_from_utc(new_occurred, new_offset)), new_offset, &new_note);
    if before == after {
        return invalid("Nothing was changed.");
    }
    tx.execute(
        "UPDATE dose_event SET kind=?1, occurred_at_utc=?2, occurred_offset_seconds=?3, note=?4, current_revision=?5 WHERE id=?6",
        params![new_kind, new_occurred, new_offset, new_note, rev + 1, input.event_id],
    )?;
    tx.execute(
        "INSERT INTO dose_event_revision (id, dose_event_id, revision_number, changed_at_utc, reason, before_value, after_value)
         VALUES (?1,?2,?3,?4,'correction',?5,?6)",
        params![Uuid::new_v4().to_string(), input.event_id, rev + 1, ctx.now_utc_us, before, after],
    )?;
    tx.commit()?;
    Ok(())
}

fn event_state(tx: &rusqlite::Transaction, event_id: &str) -> AppResult<(i64, bool, bool)> {
    let (rev, undone, deleted): (i64, i64, i64) = tx
        .query_row("SELECT current_revision, undone, deleted FROM dose_event WHERE id=?1", [event_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .optional()?
        .ok_or_else(|| AppError::NotFound("Entry not found.".into()))?;
    Ok((rev, undone == 1, deleted == 1))
}

fn add_revision(tx: &rusqlite::Transaction, ctx: &TimeCtx, event_id: &str, rev: i64, reason: &str) -> AppResult<()> {
    tx.execute(
        "INSERT INTO dose_event_revision (id, dose_event_id, revision_number, changed_at_utc, reason)
         VALUES (?1,?2,?3,?4,?5)",
        params![Uuid::new_v4().to_string(), event_id, rev, ctx.now_utc_us, reason],
    )?;
    Ok(())
}

/// Undo is SOFT and REVERSIBLE: the entry leaves Today and totals but everything (including its
/// note and change history) is kept, and `restore` brings it back. For permanent removal of
/// sensitive content use `delete_entry`, which is a separate, explicit action.
pub fn undo(conn: &mut Connection, ctx: TimeCtx, event_id: &str) -> AppResult<()> {
    let tx = conn.transaction()?;
    let (rev, undone, deleted) = event_state(&tx, event_id)?;
    if deleted {
        return invalid("This entry was deleted.");
    }
    if undone {
        return invalid("This entry was already undone.");
    }
    tx.execute("UPDATE dose_event SET undone=1, current_revision=?1 WHERE id=?2", params![rev + 1, event_id])?;
    add_revision(&tx, &ctx, event_id, rev + 1, "undo")?;
    tx.commit()?;
    Ok(())
}

/// Reverses an undo. Refused if the same slot has since been logged with the same outcome.
pub fn restore(conn: &mut Connection, ctx: TimeCtx, event_id: &str) -> AppResult<()> {
    let tx = conn.transaction()?;
    let (rev, undone, deleted) = event_state(&tx, event_id)?;
    if deleted {
        return invalid("This entry was deleted and can't be restored.");
    }
    if !undone {
        return invalid("This entry isn't undone.");
    }
    if schedule::is_superseded(&tx, event_id)? {
        return invalid("The schedule has changed since this was recorded, so it can't be restored to Today. It stays in History.");
    }
    let clash: i64 = tx.query_row(
        "SELECT count(*) FROM dose_event_occurrence mine
           JOIN dose_event_occurrence other ON other.occurrence_id = mine.occurrence_id AND other.dose_event_id <> mine.dose_event_id
           JOIN dose_event o ON o.id = other.dose_event_id
           JOIN dose_event me ON me.id = mine.dose_event_id
         WHERE mine.dose_event_id=?1 AND o.undone=0 AND o.kind = me.kind",
        [event_id],
        |r| r.get(0),
    )?;
    if clash > 0 {
        return invalid("That time has been recorded again since. Undo the newer entry first.");
    }
    tx.execute("UPDATE dose_event SET undone=0, current_revision=?1 WHERE id=?2", params![rev + 1, event_id])?;
    add_revision(&tx, &ctx, event_id, rev + 1, "restore")?;
    tx.commit()?;
    Ok(())
}

/// PRIVACY DELETION. Unlike undo this is permanent and cannot be reversed:
/// * the note is erased and every earlier revision's before/after payload is erased (they carry
///   copies of the note and times);
/// * the row stays as a tombstone (id, medication link, kind, timestamps, offsets) so history
///   remains an honest ledger, with a final revision of reason 'delete' and no payload. So what
///   REMAINS is: that an entry existed, which medication, when, taken vs skipped, and that it
///   was changed/deleted. What is ERASED is the note and the before/after contents of changes;
/// * the tombstone never counts as a dose (it is also marked undone).
/// `secure_delete` (see storage::configure) zeroes the freed bytes in the database file.
pub fn delete_entry(conn: &mut Connection, ctx: TimeCtx, event_id: &str) -> AppResult<()> {
    let tx = conn.transaction()?;
    let (rev, _undone, deleted) = event_state(&tx, event_id)?;
    if deleted {
        return invalid("This entry was already deleted.");
    }
    tx.execute("UPDATE dose_event_revision SET before_value=NULL, after_value=NULL WHERE dose_event_id=?1", [event_id])?;
    tx.execute(
        "UPDATE dose_event SET note=NULL, deleted=1, undone=1, current_revision=?1 WHERE id=?2",
        params![rev + 1, event_id],
    )?;
    add_revision(&tx, &ctx, event_id, rev + 1, "delete")?;
    tx.commit()?;
    Ok(())
}

/// Largest page a caller may request.
pub const MAX_HISTORY_PAGE: u32 = 1000;
pub const DEFAULT_HISTORY_PAGE: u32 = 100;

/// First page of History (newest first). See `history_page`.
#[cfg(test)]
pub fn history(conn: &Connection, limit: u32) -> AppResult<Vec<HistoryEntry>> {
    Ok(history_page(conn, Some(limit), None)?.entries)
}

/// One page of History, newest first by (occurred, entered, id). `before` is the previous page's
/// `next_cursor`; the result's `next_cursor` is set exactly when older entries remain, so every
/// entry (and so every deletable note) is reachable.
pub fn history_page(conn: &Connection, limit: Option<u32>, before: Option<HistoryCursor>) -> AppResult<HistoryPage> {
    let limit = limit.unwrap_or(DEFAULT_HISTORY_PAGE).clamp(1, MAX_HISTORY_PAGE);
    let (b_occ, b_ent, b_id) = match &before {
        Some(c) => (Some(c.occurred_at_utc_us), Some(c.entered_at_utc_us), Some(c.event_id.clone())),
        None => (None, None, None),
    };
    let mut stmt = conn.prepare(
        "SELECT e.id, e.medication_id, v.name, v.strength, e.kind, e.undone, e.deleted, e.occurred_at_utc, e.entered_at_utc,
                e.original_local, e.original_offset_seconds, e.occurred_offset_seconds, e.original_zone_id, e.note,
                (SELECT o.local_time FROM dose_event_occurrence l JOIN scheduled_occurrence o ON o.id=l.occurrence_id
                 WHERE l.dose_event_id=e.id LIMIT 1)
         FROM dose_event e JOIN medication_version v ON v.id = e.medication_version_id
         WHERE (?2 IS NULL OR (e.occurred_at_utc, e.entered_at_utc, e.id) < (?2, ?3, ?4))
         ORDER BY e.occurred_at_utc DESC, e.entered_at_utc DESC, e.id DESC LIMIT ?1",
    )?;
    // One extra row tells us whether an older page exists.
    let mut rows = stmt
        .query_map(params![limit + 1, b_occ, b_ent, b_id], |r| {
            let occurred: i64 = r.get(7)?;
            let current_offset: i32 = r.get(11)?;
            let deleted = r.get::<_, i64>(6)? == 1;
            Ok(HistoryEntry {
                event_id: r.get(0)?,
                medication_id: r.get(1)?,
                name: r.get(2)?,
                strength: r.get(3)?,
                kind: DoseKind::parse(&r.get::<_, String>(4)?).unwrap_or(DoseKind::Take),
                undone: !deleted && r.get::<_, i64>(5)? == 1,
                deleted,
                occurred_at_utc_us: occurred,
                entered_at_utc_us: r.get(8)?,
                original_local: r.get(9)?,
                original_offset_seconds: r.get(10)?,
                original_zone_id: r.get(12)?,
                current_local: fmt_local(local_from_utc(occurred, current_offset)),
                current_offset_seconds: current_offset,
                note: r.get(13)?,
                slot_time: r.get(14)?,
                revisions: vec![],
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let more = rows.len() > limit as usize;
    rows.truncate(limit as usize);
    let next_cursor = if more {
        rows.last().map(|e| HistoryCursor {
            occurred_at_utc_us: e.occurred_at_utc_us,
            entered_at_utc_us: e.entered_at_utc_us,
            event_id: e.event_id.clone(),
        })
    } else {
        None
    };
    for e in &mut rows {
        let mut q = conn.prepare(
            "SELECT revision_number, changed_at_utc, reason, before_value, after_value FROM dose_event_revision
             WHERE dose_event_id=?1 ORDER BY revision_number",
        )?;
        e.revisions = q
            .query_map([&e.event_id], |r| {
                Ok(Revision {
                    revision_number: r.get(0)?,
                    changed_at_utc_us: r.get(1)?,
                    reason: r.get(2)?,
                    before: r.get(3)?,
                    after: r.get(4)?,
                })
            })?
            .collect::<Result<_, _>>()?;
    }
    Ok(HistoryPage { entries: rows, next_cursor })
}
