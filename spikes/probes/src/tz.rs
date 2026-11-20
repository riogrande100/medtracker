//! Time-zone probes: DST gap/fold resolution, schedule-mode semantics, device-zone discovery.
//!
//! The resolver is written once against a tiny `offset_at(zone, utc)` primitive so that two
//! independent tzdb implementations (chrono-tz, jiff) can be compared on identical policy code.
//! The policy implemented here is the one *proposed* in docs/product-spec.md section 4; it is a
//! software-behavior choice, not medication guidance.

use std::collections::BTreeSet;

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, NaiveTime, Offset, TimeZone};
use chrono_tz::Tz;

/// Newest IANA tz release observed on https://www.iana.org/time-zones when this spike was
/// written (checked 2026-11-18). Update together with the evidence register.
pub const IANA_LATEST_AS_CHECKED: &str = "2026f";
pub const IANA_LATEST_RELEASED: &str = "2026-10-27";
pub const IANA_LATEST_CHECKED_ON: &str = "2026-11-18";

/// Zones every platform must resolve (device scenarios TZD-*-01 reuse this list). Chosen for
/// unusual offsets, DST rules, or history: half/45-minute offsets, 30-minute DST, negative DST,
/// a skipped civil date, a suppressed fall-back, and a fixed-offset Etc zone.
pub const SAMPLE_ZONES: &[&str] = &[
    "UTC",
    "America/Los_Angeles",
    "America/New_York",
    "America/Havana",
    "America/St_Johns",
    "America/Sao_Paulo",
    "Europe/London",
    "Europe/Dublin",
    "Africa/Casablanca",
    "Africa/Cairo",
    "Asia/Kolkata",
    "Asia/Kathmandu",
    "Asia/Tehran",
    "Asia/Tokyo",
    "Australia/Lord_Howe",
    "Australia/Sydney",
    "Pacific/Auckland",
    "Pacific/Chatham",
    "Pacific/Apia",
    "Pacific/Kiritimati",
    "Pacific/Honolulu",
    "Etc/GMT+5",
];

/// Legacy identifiers some platforms still report; they must resolve or be explicitly mapped.
pub const LEGACY_ALIASES: &[&str] =
    &["Asia/Calcutta", "Asia/Katmandu", "Europe/Kiev", "America/Buenos_Aires", "US/Pacific", "Asia/Saigon"];

/// Minimal tzdb primitive. Everything else is shared policy code.
pub trait TzEngine {
    fn name(&self) -> &'static str;
    fn tzdb_version(&self) -> String;
    /// UTC offset in seconds in effect at the UTC instant `utc` (unix seconds).
    fn offset_at(&self, zone: &str, utc: i64) -> Result<i32, String>;
}

pub struct ChronoTzEngine;

impl TzEngine for ChronoTzEngine {
    fn name(&self) -> &'static str {
        "chrono-tz"
    }
    fn tzdb_version(&self) -> String {
        chrono_tz::IANA_TZDB_VERSION.to_string()
    }
    fn offset_at(&self, zone: &str, utc: i64) -> Result<i32, String> {
        let tz: Tz = zone.parse().map_err(|e| format!("{zone}: {e}"))?;
        let dt = DateTime::from_timestamp(utc, 0).ok_or("timestamp out of range")?;
        Ok(tz.offset_from_utc_datetime(&dt.naive_utc()).fix().local_minus_utc())
    }
}

/// jiff using only its embedded database, never the host's /usr/share/zoneinfo, so results do
/// not depend on the machine running the test.
pub struct JiffBundledEngine {
    db: jiff::tz::TimeZoneDatabase,
}

impl JiffBundledEngine {
    pub fn new() -> Self {
        Self { db: jiff::tz::TimeZoneDatabase::bundled() }
    }
}

impl Default for JiffBundledEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TzEngine for JiffBundledEngine {
    fn name(&self) -> &'static str {
        "jiff(bundled)"
    }
    fn tzdb_version(&self) -> String {
        jiff_tzdb::VERSION.to_string()
    }
    fn offset_at(&self, zone: &str, utc: i64) -> Result<i32, String> {
        let tz = self.db.get(zone).map_err(|e| format!("{zone}: {e}"))?;
        let ts = jiff::Timestamp::from_second(utc).map_err(|e| e.to_string())?;
        Ok(tz.to_offset(ts).seconds())
    }
}

/// Outcome of mapping one wall-clock slot (zone + civil date + time) to an instant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    Exact { utc: i64, offset_s: i32 },
    /// Wall time does not exist. Moved to the first valid instant after the gap.
    AdjustedForGap { utc: i64, offset_s: i32, shifted_by_s: i64, date_changed: bool },
    /// Wall time occurs twice. The earlier instant is used; the later one is never scheduled.
    FoldEarlier { utc: i64, offset_s: i32, later_utc: i64, later_offset_s: i32 },
    /// No wall time on this civil date exists in the zone (for example Pacific/Apia 2011-12-30).
    CivilDateMissing,
}

impl Resolution {
    pub fn utc(&self) -> Option<i64> {
        match self {
            Resolution::Exact { utc, .. }
            | Resolution::AdjustedForGap { utc, .. }
            | Resolution::FoldEarlier { utc, .. } => Some(*utc),
            Resolution::CivilDateMissing => None,
        }
    }
}

fn wall_secs(local: NaiveDateTime) -> i64 {
    local.and_utc().timestamp()
}

/// All UTC instants whose local wall time in `zone` equals `local`.
fn candidates(engine: &dyn TzEngine, zone: &str, local: NaiveDateTime) -> Result<Vec<(i64, i32)>, String> {
    let w = wall_secs(local);
    let mut offsets = BTreeSet::new();
    // Sample offsets for 2 days around the wall time; every real transition is isolated enough.
    let mut t = w - 2 * 86_400;
    while t <= w + 2 * 86_400 {
        offsets.insert(engine.offset_at(zone, t)?);
        t += 3_600;
    }
    let mut out = Vec::new();
    for o in offsets {
        let u = w - i64::from(o);
        if engine.offset_at(zone, u)? == o {
            out.push((u, o));
        }
    }
    out.sort();
    Ok(out)
}

/// First instant at which the offset differs from the offset at `lo`, searching `(lo, hi]`.
fn first_change(engine: &dyn TzEngine, zone: &str, mut lo: i64, mut hi: i64) -> Result<(i64, i32), String> {
    let before = engine.offset_at(zone, lo)?;
    if engine.offset_at(zone, hi)? == before {
        return Err("no offset change in search window".into());
    }
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if engine.offset_at(zone, mid)? == before {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok((hi, engine.offset_at(zone, hi)?))
}

/// Resolve a wall-clock slot under the spec's proposed policy: gap -> first valid instant after
/// the gap; fold -> earlier instant; wholly missing civil date -> `CivilDateMissing`.
pub fn resolve(engine: &dyn TzEngine, zone: &str, date: NaiveDate, time: NaiveTime) -> Result<Resolution, String> {
    let local = date.and_time(time);
    let c = candidates(engine, zone, local)?;
    match c.as_slice() {
        [(utc, offset_s)] => Ok(Resolution::Exact { utc: *utc, offset_s: *offset_s }),
        [(utc, offset_s), (later_utc, later_offset_s)] => Ok(Resolution::FoldEarlier {
            utc: *utc,
            offset_s: *offset_s,
            later_utc: *later_utc,
            later_offset_s: *later_offset_s,
        }),
        [] => {
            let w = wall_secs(local);
            let (t, o_after) = first_change(engine, zone, w - 100_000, w + 100_000)?;
            let adjusted_local = DateTime::from_timestamp(t + i64::from(o_after), 0)
                .ok_or("range")?
                .naive_utc();
            let day_start = date.and_time(NaiveTime::MIN);
            if adjusted_local.date() > date && candidates(engine, zone, day_start)?.is_empty() {
                return Ok(Resolution::CivilDateMissing);
            }
            Ok(Resolution::AdjustedForGap {
                utc: t,
                offset_s: o_after,
                shifted_by_s: wall_secs(adjusted_local) - w,
                date_changed: adjusted_local.date() != date,
            })
        }
        more => Err(format!("unexpected {} candidates for {local} in {zone}", more.len())),
    }
}

/// `elapsed_interval` mode: occurrence n is anchor + n * interval, independent of any zone.
pub fn elapsed_occurrence(anchor_utc: i64, interval_s: i64, n: i64) -> i64 {
    anchor_utc + n * interval_s
}

/// Seconds-since-epoch of a calendar instant, for readable test fixtures.
pub fn utc(y: i32, m: u32, d: u32, hh: u32, mm: u32, ss: u32) -> i64 {
    NaiveDate::from_ymd_opt(y, m, d)
        .and_then(|d| d.and_hms_opt(hh, mm, ss))
        .expect("valid fixture date")
        .and_utc()
        .timestamp()
}

pub fn local_date_at(engine: &dyn TzEngine, zone: &str, utc_s: i64) -> Result<NaiveDate, String> {
    let off = engine.offset_at(zone, utc_s)?;
    Ok(DateTime::from_timestamp(utc_s + i64::from(off), 0).ok_or("range")?.date_naive())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedOccurrence {
    pub zone: String,
    pub local_date: NaiveDate,
    pub resolution: Resolution,
}

/// `device_local` planner. `zone_timeline` is the device zone history/forecast as
/// `(effective_from_utc, iana_zone)`, sorted. Occurrences are only planned from `from_utc`
/// forward; skipped civil dates are not backfilled and a civil date that repeats after westward
/// travel yields at most one occurrence (`consumed` is the persisted slot-date set).
pub fn plan_device_local(
    engine: &dyn TzEngine,
    zone_timeline: &[(i64, &str)],
    time: NaiveTime,
    from_utc: i64,
    count: usize,
    consumed: &mut BTreeSet<NaiveDate>,
) -> Result<Vec<PlannedOccurrence>, String> {
    let zone_at = |t: i64| -> &str {
        zone_timeline.iter().rev().find(|(from, _)| *from <= t).map(|(_, z)| *z).unwrap_or(zone_timeline[0].1)
    };
    let next_change_after = |t: i64| zone_timeline.iter().map(|(f, _)| *f).find(|f| *f > t);

    let mut out = Vec::new();
    let mut cursor = from_utc;
    let mut zone = zone_at(cursor).to_string();
    let mut date = local_date_at(engine, &zone, cursor)?;
    let mut guard = 0;
    while out.len() < count {
        guard += 1;
        if guard > 10_000 {
            return Err("planner did not converge".into());
        }
        if consumed.contains(&date) {
            date = date.succ_opt().ok_or("date range")?;
            continue;
        }
        let resolution = resolve(engine, &zone, date, time)?;
        let Some(u) = resolution.utc() else {
            date = date.succ_opt().ok_or("date range")?;
            continue;
        };
        if u < cursor {
            date = date.succ_opt().ok_or("date range")?;
            continue;
        }
        if let Some(change) = next_change_after(cursor) {
            if change <= u {
                cursor = change;
                zone = zone_at(cursor).to_string();
                date = local_date_at(engine, &zone, cursor)?;
                continue;
            }
        }
        consumed.insert(date);
        out.push(PlannedOccurrence { zone: zone.clone(), local_date: date, resolution });
        cursor = u;
        date = date.succ_opt().ok_or("date range")?;
    }
    Ok(out)
}

/// Add a whole number of civil days without any zone arithmetic (for fixtures).
pub fn add_days(d: NaiveDate, n: i64) -> NaiveDate {
    d + Duration::days(n)
}

/// What the host reports as its current IANA zone, and whether each engine can resolve it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DeviceZoneReport {
    pub iana_time_zone_crate: Result<String, String>,
    pub jiff_system_iana_name: Option<String>,
    pub resolvable_by_chrono_tz: Option<bool>,
    pub resolvable_by_jiff_bundled: Option<bool>,
}

/// Version string of the OS-provided zoneinfo, where one exists as files. Mobile OSes expose no
/// such directory to apps, which is one reason the app must bundle its own tzdb.
pub fn host_system_tzdb_version() -> Option<String> {
    for dir in ["/usr/share/zoneinfo", "/usr/share/lib/zoneinfo", "/usr/lib/zoneinfo"] {
        if let Ok(s) = std::fs::read_to_string(format!("{dir}/+VERSION")) {
            return Some(s.trim().to_string());
        }
        if let Ok(s) = std::fs::read_to_string(format!("{dir}/tzdata.zi")) {
            if let Some(l) = s.lines().next() {
                return Some(l.trim_start_matches("# version ").to_string());
            }
        }
    }
    None
}

pub fn host_zone_report() -> DeviceZoneReport {
    let via_crate = iana_time_zone::get_timezone().map_err(|e| e.to_string());
    let jiff_name = jiff::tz::TimeZone::system().iana_name().map(str::to_string);
    let chosen = via_crate.as_ref().ok().cloned().or_else(|| jiff_name.clone());
    let jiff_db = jiff::tz::TimeZoneDatabase::bundled();
    DeviceZoneReport {
        resolvable_by_chrono_tz: chosen.as_ref().map(|z| z.parse::<Tz>().is_ok()),
        resolvable_by_jiff_bundled: chosen.as_ref().map(|z| jiff_db.get(z).is_ok()),
        iana_time_zone_crate: via_crate,
        jiff_system_iana_name: jiff_name,
    }
}
