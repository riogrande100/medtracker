use crate::application::schedule;
use crate::clock::TimeCtx;
use crate::domain::schedule::Definition;
use crate::dto::{MedicationSummary, NewMedication};
use crate::error::{invalid, AppError, AppResult};
use rusqlite::{params, Connection};
use uuid::Uuid;

fn clean(s: Option<String>) -> Option<String> {
    s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

pub fn create(conn: &mut Connection, ctx: TimeCtx, input: NewMedication) -> AppResult<String> {
    let name = input.name.trim().to_string();
    if name.is_empty() || name.len() > 120 {
        return invalid("Enter a medication name (up to 120 characters).");
    }
    let strength = clean(input.strength);
    let form = clean(input.form);
    if strength.as_deref().map_or(0, str::len) > 80 || form.as_deref().map_or(0, str::len) > 80 {
        return invalid("Strength and form must be 80 characters or fewer.");
    }
    // "Show on Today every day": a home-zone schedule in the device's current zone (validated
    // up front, so bad times are rejected before anything is written).
    let daily = if input.times.is_empty() { None } else { Some(Definition::daily(ctx.zone.id(), &input.times)?) };
    let (med_id, ver_id) = (Uuid::new_v4().to_string(), Uuid::new_v4().to_string());
    let now = ctx.now_utc_us;
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO user_medication (id, status, created_at_utc, updated_at_utc) VALUES (?1,'active',?2,?2)",
        params![med_id, now],
    )?;
    tx.execute(
        "INSERT INTO medication_version (id, medication_id, version_number, effective_from_utc, name, strength, form, source_kind, created_at_utc)
         VALUES (?1,?2,1,?3,?4,?5,?6,'manual',?3)",
        params![ver_id, med_id, now, name, strength, form],
    )?;
    tx.execute("UPDATE user_medication SET current_version_id=?1 WHERE id=?2", params![ver_id, med_id])?;
    if let Some(def) = &daily {
        schedule::create_in_tx(&tx, &ctx, &med_id, def)?;
    }
    tx.commit()?;
    Ok(med_id)
}

pub fn list(conn: &Connection, ctx: &TimeCtx) -> AppResult<Vec<MedicationSummary>> {
    let mut stmt = conn.prepare(
        "SELECT m.id, v.name, v.strength, v.form, m.status FROM user_medication m
         JOIN medication_version v ON v.id = m.current_version_id ORDER BY m.status, v.name",
    )?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get::<_, String>(4)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut out = Vec::new();
    for (id, name, strength, form, status) in rows {
        let schedule = schedule::summary_for_medication(conn, ctx, &id)?;
        out.push(MedicationSummary { id, name, strength, form, archived: status == "archived", schedule });
    }
    Ok(out)
}

/// Archive keeps all history; the medication leaves Today.
pub fn archive(conn: &Connection, ctx: TimeCtx, id: &str) -> AppResult<()> {
    let n = conn.execute(
        "UPDATE user_medication SET status='archived', updated_at_utc=?1 WHERE id=?2",
        params![ctx.now_utc_us, id],
    )?;
    if n == 0 {
        return Err(AppError::NotFound("Medication not found.".into()));
    }
    Ok(())
}

pub fn unarchive(conn: &Connection, ctx: TimeCtx, id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE user_medication SET status='active', updated_at_utc=?1 WHERE id=?2",
        params![ctx.now_utc_us, id],
    )?;
    Ok(())
}

/// Permanently removes the medication and everything recorded for it.
pub fn delete(conn: &mut Connection, id: &str) -> AppResult<()> {
    let tx = conn.transaction()?;
    let steps = [
        "DELETE FROM dose_event_revision WHERE dose_event_id IN (SELECT id FROM dose_event WHERE medication_id=?1)",
        "DELETE FROM dose_event_occurrence WHERE dose_event_id IN (SELECT id FROM dose_event WHERE medication_id=?1)",
        "DELETE FROM dose_event WHERE medication_id=?1",
        "DELETE FROM scheduled_occurrence WHERE schedule_id IN (SELECT id FROM schedule WHERE medication_id=?1)",
        "DELETE FROM schedule_slot WHERE schedule_version_id IN
           (SELECT v.id FROM schedule_version v JOIN schedule s ON s.id=v.schedule_id WHERE s.medication_id=?1)",
        "DELETE FROM schedule_version WHERE schedule_id IN (SELECT id FROM schedule WHERE medication_id=?1)",
        "DELETE FROM schedule WHERE medication_id=?1",
        "UPDATE user_medication SET current_version_id=NULL WHERE id=?1",
    ];
    for sql in &steps {
        tx.execute(sql, [id])?;
    }
    tx.execute("DELETE FROM medication_version WHERE medication_id=?1", [id])?;
    let n = tx.execute("DELETE FROM user_medication WHERE id=?1", [id])?;
    if n == 0 {
        return Err(AppError::NotFound("Medication not found.".into()));
    }
    tx.commit()?;
    Ok(())
}
