use crate::clock::{parse_hhmm, TimeCtx};
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
    let mut times = Vec::new();
    for t in &input.times {
        parse_hhmm(t)?;
        if times.contains(t) {
            return invalid("Each time can only be listed once.");
        }
        times.push(t.clone());
    }
    times.sort();
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
    for t in &times {
        tx.execute(
            "INSERT INTO schedule_slot (id, medication_id, medication_version_id, local_time, created_at_utc) VALUES (?1,?2,?3,?4,?5)",
            params![Uuid::new_v4().to_string(), med_id, ver_id, t, now],
        )?;
    }
    tx.commit()?;
    Ok(med_id)
}

pub fn list(conn: &Connection) -> AppResult<Vec<MedicationSummary>> {
    let mut stmt = conn.prepare(
        "SELECT m.id, v.name, v.strength, v.form, m.status FROM user_medication m
         JOIN medication_version v ON v.id = m.current_version_id ORDER BY m.status, v.name",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(MedicationSummary {
            id: r.get(0)?,
            name: r.get(1)?,
            strength: r.get(2)?,
            form: r.get(3)?,
            archived: r.get::<_, String>(4)? == "archived",
            times: vec![],
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        let mut m = row?;
        let mut t = conn.prepare("SELECT local_time FROM schedule_slot WHERE medication_id=?1 ORDER BY local_time")?;
        m.times = t.query_map([&m.id], |r| r.get(0))?.collect::<Result<_, _>>()?;
        out.push(m);
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
        "DELETE FROM scheduled_occurrence WHERE slot_id IN (SELECT id FROM schedule_slot WHERE medication_id=?1)",
        "DELETE FROM schedule_slot WHERE medication_id=?1",
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
