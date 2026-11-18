//! Plain CSV / JSON export of the dose-event ledger.
//!
//! Where files go: `commands::export_ledger` writes into the platform's Downloads folder
//! (falling back to Documents), because a user-visible location needs no picker plugin and no
//! extra capability, and the app stays offline-only. If that folder cannot be written to, the
//! export goes to `<app data>/exports`. The file name is returned so the UI can say exactly where
//! it went. Names are unique (millisecond + random suffix) and never overwrite. The content is
//! written to a hidden temp file in the same folder, flushed, then renamed, so a failed write
//! leaves no partial plaintext; the file is owner-only (0600) on Unix.
//!
//! Format: `schema_version` 1. Every export is PLAIN, unencrypted text. Notes are included only
//! when the caller asks; otherwise the note column/field does not exist. Undone entries and
//! deleted tombstones are included only with `include_history`, labelled by `status`; tombstones
//! never carry a note. Revision payloads (which can contain old notes) are never exported.
use crate::clock::{fmt_date, fmt_local, local_from_utc, parse_local};
use crate::error::AppResult;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Csv,
    Json,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExportOptions {
    /// Inclusive local dates; both absent = the whole ledger.
    pub from: Option<String>,
    pub to: Option<String>,
    pub format: Format,
    #[serde(default)]
    pub include_notes: bool,
    #[serde(default)]
    pub include_history: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ExportRow {
    pub event_id: String,
    /// "active", "undone" (voided) or "deleted" (details erased).
    pub status: String,
    pub medication_id: String,
    pub medication_name: String,
    pub medication_strength: Option<String>,
    pub kind: String,
    pub occurred_at_utc: String,
    pub occurred_local: String,
    pub offset_seconds: i32,
    pub zone_id: Option<String>,
    pub entered_at_utc: String,
    pub original_local: String,
    pub original_offset_seconds: i32,
    pub occurrence_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

pub struct Built {
    pub extension: &'static str,
    pub content: String,
    pub rows: usize,
}

fn rfc3339(us: i64) -> String {
    jiff::Timestamp::from_microsecond(us).map(|t| t.to_string()).unwrap_or_default()
}

pub fn rows(conn: &Connection, o: &ExportOptions) -> AppResult<Vec<ExportRow>> {
    let mut sql = String::from(
        "SELECT e.id, e.undone, e.deleted, e.medication_id, v.name, v.strength, e.kind, e.occurred_at_utc,
                e.occurred_offset_seconds, e.original_zone_id, e.entered_at_utc, e.original_local,
                e.original_offset_seconds, e.note,
                (SELECT l.occurrence_id FROM dose_event_occurrence l WHERE l.dose_event_id = e.id ORDER BY l.occurrence_id LIMIT 1)
         FROM dose_event e JOIN medication_version v ON v.id = e.medication_version_id",
    );
    if !o.include_history {
        sql.push_str(" WHERE e.undone = 0 AND e.deleted = 0");
    }
    sql.push_str(" ORDER BY e.occurred_at_utc, e.entered_at_utc, e.id");
    let (from, to) = (o.from.as_deref(), o.to.as_deref());
    if let (Some(f), Some(t)) = (from, to) {
        super::query::check_range(f, t)?;
    }
    if let Some(f) = from {
        parse_local(&format!("{f}T00:00"))?;
    }
    if let Some(t) = to {
        parse_local(&format!("{t}T00:00"))?;
    }
    let mut s = conn.prepare(&sql)?;
    let mut out = vec![];
    let all = s.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?, r.get::<_, String>(3)?, r.get::<_, String>(4)?,
            r.get::<_, Option<String>>(5)?, r.get::<_, String>(6)?, r.get::<_, i64>(7)?, r.get::<_, i32>(8)?,
            r.get::<_, Option<String>>(9)?, r.get::<_, i64>(10)?, r.get::<_, String>(11)?, r.get::<_, i32>(12)?,
            r.get::<_, Option<String>>(13)?, r.get::<_, Option<String>>(14)?,
        ))
    })?;
    for r in all {
        let (id, undone, deleted, med, name, strength, kind, at, off, zone, entered, orig, orig_off, note, occ) = r?;
        let local = local_from_utc(at, off);
        let date = fmt_date(local);
        if from.is_some_and(|f| date.as_str() < f) || to.is_some_and(|t| date.as_str() > t) {
            continue;
        }
        let status = if deleted == 1 { "deleted" } else if undone == 1 { "undone" } else { "active" };
        out.push(ExportRow {
            event_id: id,
            status: status.into(),
            medication_id: med,
            medication_name: name,
            medication_strength: strength,
            kind,
            occurred_at_utc: rfc3339(at),
            occurred_local: fmt_local(local),
            offset_seconds: off,
            zone_id: zone,
            entered_at_utc: rfc3339(entered),
            original_local: orig,
            original_offset_seconds: orig_off,
            occurrence_id: occ,
            // Opt-in, and a tombstone has no note by construction (also enforced here).
            note: if o.include_notes && deleted == 0 { Some(note.unwrap_or_default()) } else { None },
        });
    }
    Ok(out)
}

/// One RFC 4180 field. Text that a spreadsheet could read as a formula (starts with = + - @, or a
/// tab/CR) is prefixed with an apostrophe first, so the cell shows as text. Applied to text only,
/// never to the numeric offset columns.
pub fn csv_text(s: &str) -> String {
    let guarded = match s.chars().next() {
        Some('=' | '+' | '-' | '@' | '\t' | '\r') => format!("'{s}"),
        _ => s.to_string(),
    };
    csv_raw(&guarded)
}

fn csv_raw(s: &str) -> String {
    if s.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub fn to_csv(rows: &[ExportRow], include_notes: bool) -> String {
    let mut header = vec![
        "schema_version", "event_id", "status", "medication_id", "medication_name", "medication_strength", "kind",
        "occurred_at_utc", "occurred_local", "offset_seconds", "zone_id", "entered_at_utc", "original_local",
        "original_offset_seconds", "occurrence_id",
    ];
    if include_notes {
        header.push("note");
    }
    let mut out = header.join(",");
    out.push_str("\r\n");
    for r in rows {
        let opt = |o: &Option<String>| o.as_deref().map(csv_text).unwrap_or_default();
        let mut f = vec![
            SCHEMA_VERSION.to_string(), csv_text(&r.event_id), csv_text(&r.status), csv_text(&r.medication_id),
            csv_text(&r.medication_name), opt(&r.medication_strength), csv_text(&r.kind), csv_text(&r.occurred_at_utc),
            csv_text(&r.occurred_local), r.offset_seconds.to_string(), opt(&r.zone_id), csv_text(&r.entered_at_utc),
            csv_text(&r.original_local), r.original_offset_seconds.to_string(), opt(&r.occurrence_id),
        ];
        if include_notes {
            f.push(opt(&r.note));
        }
        out.push_str(&f.join(","));
        out.push_str("\r\n");
    }
    out
}

pub fn to_json(rows: &[ExportRow], o: &ExportOptions, now_utc_us: i64) -> String {
    let count = |s: &str| rows.iter().filter(|r| r.status == s).count();
    let v = serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "kind": "medtracker_ledger_export",
        "exported_at_utc": rfc3339(now_utc_us),
        "plain_text_warning": "This file is not encrypted. Anyone who can open it can read it.",
        "range": { "from": o.from, "to": o.to },
        "includes_notes": o.include_notes,
        "includes_history": o.include_history,
        "definitions": super::query::definitions(),
        "entry_counts": { "active": count("active"), "undone": count("undone"), "deleted": count("deleted") },
        "events": rows,
    });
    serde_json::to_string_pretty(&v).unwrap_or_default() + "\n"
}

pub fn build(conn: &Connection, o: &ExportOptions, now_utc_us: i64) -> AppResult<Built> {
    let r = rows(conn, o)?;
    Ok(match o.format {
        Format::Csv => Built { extension: "csv", content: to_csv(&r, o.include_notes), rows: r.len() },
        Format::Json => Built { extension: "json", content: to_json(&r, o, now_utc_us), rows: r.len() },
    })
}

/// "medtracker-export-20260310-091000-123-a1b2c3.csv": UTC date and time, the millisecond, and a
/// random suffix, so two exports in the same second never share a name.
pub fn file_name(now_utc_us: i64, ext: &str) -> String {
    crate::fsutil::unique_name("medtracker-export", now_utc_us, ext)
}

/// Saves `content` as `name` in the first of `dirs` that accepts it (a folder is created if
/// missing). The file is written to a hidden temp file, flushed, owner-only (Unix), then renamed
/// into place, so a failed write never leaves a partial export; if the preferred folder cannot be
/// written to, the next one is tried.
pub fn save(dirs: &[std::path::PathBuf], name: &str, content: &[u8]) -> AppResult<std::path::PathBuf> {
    let mut last = None;
    for dir in dirs {
        let r = std::fs::create_dir_all(dir).and_then(|_| crate::fsutil::save_atomic(dir, name, content));
        match r {
            Ok(p) => return Ok(p),
            Err(e) => last = Some(e),
        }
    }
    Err(crate::error::AppError::Storage(format!(
        "Couldn't save the export{}.",
        last.map(|e| format!(" ({e})")).unwrap_or_default()
    )))
}
