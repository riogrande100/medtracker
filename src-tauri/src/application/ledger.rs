use crate::clock::{fmt_local, local_from_utc, parse_hhmm, parse_local, utc_from_local, TimeCtx};
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

pub fn today(conn: &Connection, ctx: TimeCtx) -> AppResult<Vec<TodayItem>> {
    let date = ctx.today();
    let now_min = ctx.minutes_now();
    let mut stmt = conn.prepare(
        "SELECT s.id, m.id, v.name, v.strength, v.form, s.local_time
         FROM schedule_slot s
         JOIN user_medication m ON m.id = s.medication_id AND m.status = 'active'
         JOIN medication_version v ON v.id = m.current_version_id
         ORDER BY s.local_time, v.name",
    )?;
    let slots = stmt
        .query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get::<_, String>(5)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut out = Vec::new();
    for (slot_id, medication_id, name, strength, form, local_time) in slots {
        let mut q = conn.prepare(
            "SELECT e.id, e.kind FROM dose_event e
             JOIN dose_event_occurrence l ON l.dose_event_id = e.id
             JOIN scheduled_occurrence o ON o.id = l.occurrence_id
             WHERE o.slot_id = ?1 AND o.local_date = ?2 AND e.deleted = 0 ORDER BY e.occurred_at_utc",
        )?;
        let events: Vec<(String, String)> =
            q.query_map(params![slot_id, date], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
        let takes = events.iter().any(|e| e.1 == "take");
        let skips = events.iter().any(|e| e.1 == "skip");
        let status = match (takes, skips) {
            (true, true) => TodayStatus::Conflict,
            (true, false) => TodayStatus::Taken,
            (false, true) => TodayStatus::Skipped,
            _ if parse_hhmm(&local_time)? <= now_min => TodayStatus::Due,
            _ => TodayStatus::Upcoming,
        };
        out.push(TodayItem {
            slot_id,
            medication_id,
            name,
            strength,
            form,
            local_date: date.clone(),
            local_time,
            status,
            event_ids: events.into_iter().map(|e| e.0).collect(),
        });
    }
    Ok(out)
}

pub fn record(conn: &mut Connection, ctx: TimeCtx, input: RecordDose) -> AppResult<String> {
    let note = clean_note(input.note)?;
    let occurred = match &input.occurred_local {
        Some(s) => ctx.resolve_local(s)?,
        None => ctx.now_utc_us,
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

    let mut occurrence_id = None;
    if let Some(slot_id) = &input.slot_id {
        let date = input.local_date.as_deref().ok_or_else(|| AppError::Validation("Missing slot date.".into()))?;
        let slot_time: String = tx
            .query_row(
                "SELECT local_time FROM schedule_slot WHERE id=?1 AND medication_id=?2",
                params![slot_id, input.medication_id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound("Scheduled time not found.".into()))?;
        let due_local = parse_local(&format!("{date}T{slot_time}"))?;
        let due_utc = utc_from_local(due_local, ctx.offset_seconds);
        tx.execute(
            "INSERT OR IGNORE INTO scheduled_occurrence (id, slot_id, local_date, local_time, offset_seconds, due_at_utc)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![Uuid::new_v4().to_string(), slot_id, date, slot_time, ctx.offset_seconds, due_utc],
        )?;
        let occ: String = tx.query_row(
            "SELECT id FROM scheduled_occurrence WHERE slot_id=?1 AND local_date=?2",
            params![slot_id, date],
            |r| r.get(0),
        )?;
        let dup: i64 = tx.query_row(
            "SELECT count(*) FROM dose_event e JOIN dose_event_occurrence l ON l.dose_event_id=e.id
             WHERE l.occurrence_id=?1 AND e.deleted=0 AND e.kind=?2",
            params![occ, input.kind.as_str()],
            |r| r.get(0),
        )?;
        if dup > 0 {
            return invalid("Already recorded for this time. Undo it first to change it.");
        }
        occurrence_id = Some(occ);
    }

    let id = Uuid::new_v4().to_string();
    let original_local = fmt_local(local_from_utc(occurred, ctx.offset_seconds));
    tx.execute(
        "INSERT INTO dose_event (id, medication_id, medication_version_id, kind, occurred_at_utc, entered_at_utc,
           original_local, original_offset_seconds, note) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![id, input.medication_id, version, input.kind.as_str(), occurred, ctx.now_utc_us, original_local, ctx.offset_seconds, note],
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

fn snapshot(kind: &str, local: &str, note: &Option<String>) -> String {
    serde_json::json!({ "kind": kind, "occurred_local": local, "note": note }).to_string()
}

pub fn correct(conn: &mut Connection, ctx: TimeCtx, input: CorrectDose) -> AppResult<()> {
    let tx = conn.transaction()?;
    let (kind, occurred, offset, note, rev, deleted): (String, i64, i32, Option<String>, i64, i64) = tx
        .query_row(
            "SELECT kind, occurred_at_utc, original_offset_seconds, note, current_revision, deleted FROM dose_event WHERE id=?1",
            [&input.event_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound("Entry not found.".into()))?;
    if deleted == 1 {
        return invalid("This entry was undone and can't be edited.");
    }
    let before = snapshot(&kind, &fmt_local(local_from_utc(occurred, offset)), &note);
    let new_kind = input.kind.map(|k| k.as_str().to_string()).unwrap_or(kind);
    let new_occurred = match &input.occurred_local {
        Some(s) => utc_from_local(parse_local(s)?, offset),
        None => occurred,
    };
    if new_occurred > ctx.now_utc_us + 60_000_000 {
        return invalid("A dose can't be recorded in the future.");
    }
    let new_note = match input.note {
        Some(n) => clean_note(Some(n))?,
        None => note,
    };
    let after = snapshot(&new_kind, &fmt_local(local_from_utc(new_occurred, offset)), &new_note);
    if before == after {
        return invalid("Nothing was changed.");
    }
    tx.execute(
        "UPDATE dose_event SET kind=?1, occurred_at_utc=?2, note=?3, current_revision=?4 WHERE id=?5",
        params![new_kind, new_occurred, new_note, rev + 1, input.event_id],
    )?;
    tx.execute(
        "INSERT INTO dose_event_revision (id, dose_event_id, revision_number, changed_at_utc, reason, before_value, after_value)
         VALUES (?1,?2,?3,?4,'correction',?5,?6)",
        params![Uuid::new_v4().to_string(), input.event_id, rev + 1, ctx.now_utc_us, before, after],
    )?;
    tx.commit()?;
    Ok(())
}

/// Undo hides the entry from Today and totals; the tombstone revision remains in History.
pub fn undo(conn: &mut Connection, ctx: TimeCtx, event_id: &str) -> AppResult<()> {
    let tx = conn.transaction()?;
    let (rev, deleted): (i64, i64) = tx
        .query_row("SELECT current_revision, deleted FROM dose_event WHERE id=?1", [event_id], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .optional()?
        .ok_or_else(|| AppError::NotFound("Entry not found.".into()))?;
    if deleted == 1 {
        return invalid("This entry was already undone.");
    }
    tx.execute("UPDATE dose_event SET deleted=1, current_revision=?1 WHERE id=?2", params![rev + 1, event_id])?;
    tx.execute(
        "INSERT INTO dose_event_revision (id, dose_event_id, revision_number, changed_at_utc, reason)
         VALUES (?1,?2,?3,?4,'undo')",
        params![Uuid::new_v4().to_string(), event_id, rev + 1, ctx.now_utc_us],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn history(conn: &Connection, limit: u32) -> AppResult<Vec<HistoryEntry>> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.medication_id, v.name, v.strength, e.kind, e.deleted, e.occurred_at_utc, e.entered_at_utc,
                e.original_local, e.original_offset_seconds, e.note,
                (SELECT o.local_time FROM dose_event_occurrence l JOIN scheduled_occurrence o ON o.id=l.occurrence_id
                 WHERE l.dose_event_id=e.id LIMIT 1)
         FROM dose_event e JOIN medication_version v ON v.id = e.medication_version_id
         ORDER BY e.occurred_at_utc DESC, e.entered_at_utc DESC LIMIT ?1",
    )?;
    let mut rows = stmt
        .query_map([limit.min(500)], |r| {
            let offset: i32 = r.get(9)?;
            let occurred: i64 = r.get(6)?;
            Ok(HistoryEntry {
                event_id: r.get(0)?,
                medication_id: r.get(1)?,
                name: r.get(2)?,
                strength: r.get(3)?,
                kind: DoseKind::parse(&r.get::<_, String>(4)?).unwrap_or(DoseKind::Take),
                undone: r.get::<_, i64>(5)? == 1,
                occurred_at_utc_us: occurred,
                entered_at_utc_us: r.get(7)?,
                original_local: r.get(8)?,
                original_offset_seconds: offset,
                current_local: fmt_local(local_from_utc(occurred, offset)),
                note: r.get(10)?,
                slot_time: r.get(11)?,
                revisions: vec![],
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
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
    Ok(rows)
}
