//! Pure schedule domain: validated definitions and occurrence generation. No database access
//! and no clock reads, so every rule here is deterministic and unit-testable.
//!
//! Modes
//! * `HomeZone`: fixed wall-clock times (optionally on selected weekdays) in ONE chosen IANA
//!   zone. Travel never changes the zone; DST is resolved by `Zone::resolve` for each date:
//!   a time inside a spring-forward gap moves forward by the gap (marked `adjusted`), a time
//!   that occurs twice uses the earlier instant (one occurrence, never two).
//! * `ElapsedInterval`: `anchor + n * interval` in absolute time (DST never adds or removes
//!   elapsed time).
//! `device_local` is deliberately absent (deferred until its device spike passes).

use crate::clock::{fmt_date, local_from_utc, parse_hhmm, Zone};
use crate::error::{invalid, AppResult};
use jiff::{civil::Date, civil::Weekday, Span};
use serde::{Deserialize, Serialize};

pub const MAX_SLOTS: usize = 24;
pub const MIN_INTERVAL_SECONDS: i64 = 15 * 60;
pub const MAX_INTERVAL_SECONDS: i64 = 30 * 86_400;
/// Safety bound on generated rows per call.
const MAX_CANDIDATES: usize = 20_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleMode {
    HomeZone,
    ElapsedInterval,
}

impl ScheduleMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScheduleMode::HomeZone => "home_zone",
            ScheduleMode::ElapsedInterval => "elapsed_interval",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "home_zone" => Some(ScheduleMode::HomeZone),
            "elapsed_interval" => Some(ScheduleMode::ElapsedInterval),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleState {
    Active,
    Paused,
    Ended,
}

impl ScheduleState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScheduleState::Active => "active",
            ScheduleState::Paused => "paused",
            ScheduleState::Ended => "ended",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "active" => Some(ScheduleState::Active),
            "paused" => Some(ScheduleState::Paused),
            "ended" => Some(ScheduleState::Ended),
            _ => None,
        }
    }
}

/// One wall-clock time of day and the weekdays it runs on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    pub minutes: u32,
    /// Bit 0 = Monday ... bit 6 = Sunday; 0 = every day.
    pub weekday_mask: u8,
}

impl Slot {
    pub fn time(&self) -> String {
        format!("{:02}:{:02}", self.minutes / 60, self.minutes % 60)
    }

    /// Stable identity across schedule versions: the wall time itself.
    pub fn key(&self) -> String {
        self.time()
    }

    pub fn runs_on(&self, d: Weekday) -> bool {
        self.weekday_mask == 0 || self.weekday_mask & (1 << (d.to_monday_zero_offset() as u8)) != 0
    }
}

/// ISO weekday numbers (1 = Monday ... 7 = Sunday) -> mask. Empty = every day.
pub fn mask_from_weekdays(days: &[u8]) -> AppResult<u8> {
    let mut mask = 0u8;
    for d in days {
        if !(1..=7).contains(d) {
            return invalid("Weekdays must be between 1 (Monday) and 7 (Sunday).");
        }
        mask |= 1 << (d - 1);
    }
    // All seven days is the same as "every day".
    Ok(if mask == 0b111_1111 { 0 } else { mask })
}

pub fn weekdays_from_mask(mask: u8) -> Vec<u8> {
    (1..=7u8).filter(|d| mask & (1 << (d - 1)) != 0).collect()
}

/// A validated schedule definition (what one `schedule_version` stores).
#[derive(Debug, Clone, PartialEq)]
pub struct Definition {
    pub mode: ScheduleMode,
    /// `HomeZone`: the chosen IANA zone; `None` only for migrated legacy rows that have not
    /// adopted one yet (resolved with the device zone). Always `None` for `ElapsedInterval`.
    pub zone_id: Option<String>,
    pub slots: Vec<Slot>,
    pub interval_seconds: Option<i64>,
    pub anchor_utc_us: Option<i64>,
}

impl Definition {
    /// A home-zone definition from "HH:MM" strings (every day). Used for the simple
    /// "show on Today every day" path of adding a medication.
    pub fn daily(zone_id: Option<&str>, times: &[String]) -> AppResult<Definition> {
        let slots = times.iter().map(|t| Ok(Slot { minutes: parse_hhmm(t)?, weekday_mask: 0 })).collect::<AppResult<Vec<_>>>()?;
        Definition { mode: ScheduleMode::HomeZone, zone_id: zone_id.map(str::to_string), slots, interval_seconds: None, anchor_utc_us: None }.validated()
    }

    /// Checks every rule; returns the definition with slots sorted by time.
    pub fn validated(mut self) -> AppResult<Definition> {
        match self.mode {
            ScheduleMode::HomeZone => {
                if self.slots.is_empty() {
                    return invalid("Add at least one time of day.");
                }
                if self.slots.len() > MAX_SLOTS {
                    return invalid("A schedule can have at most 24 times.");
                }
                if self.interval_seconds.is_some() || self.anchor_utc_us.is_some() {
                    return invalid("A fixed-time schedule has no interval.");
                }
                if let Some(z) = &self.zone_id {
                    if Zone::iana(z).is_none() {
                        return invalid("That time zone isn't recognised. Choose one from the list.");
                    }
                }
                self.slots.sort_by_key(|s| s.minutes);
                if self.slots.windows(2).any(|w| w[0].minutes == w[1].minutes) {
                    return invalid("Each time can only be listed once.");
                }
                if self.slots.iter().any(|s| s.minutes >= 24 * 60 || s.weekday_mask > 0b111_1111) {
                    return invalid("Times must look like 08:00.");
                }
            }
            ScheduleMode::ElapsedInterval => {
                if !self.slots.is_empty() || self.zone_id.is_some() {
                    return invalid("An every-N-hours schedule has no fixed times or zone.");
                }
                let i = self.interval_seconds.ok_or_else(|| crate::error::AppError::Validation("Enter how long between doses.".into()))?;
                if !(MIN_INTERVAL_SECONDS..=MAX_INTERVAL_SECONDS).contains(&i) {
                    return invalid("The time between doses must be between 15 minutes and 30 days.");
                }
                let a = self.anchor_utc_us.ok_or_else(|| crate::error::AppError::Validation("Choose when the first dose is.".into()))?;
                // 1970-01-01 .. 2200-01-01 in microseconds.
                if !(0..=7_258_118_400_000_000).contains(&a) {
                    return invalid("Choose a valid start time.");
                }
            }
        }
        Ok(self)
    }
}

/// One expected occurrence, before it is stored.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub slot_key: Option<String>,
    pub seq: Option<i64>,
    /// Intended local date and wall time (the slot's own time).
    pub local_date: String,
    pub intended_time: String,
    pub scheduled_at_utc_us: i64,
    /// Offset in force at `scheduled_at_utc_us`.
    pub offset_seconds: i32,
    /// True when the intended wall time did not exist (clock change) and was moved.
    pub adjusted: bool,
    pub fold: u8,
}

pub fn add_days(d: Date, n: i64) -> AppResult<Date> {
    d.checked_add(Span::new().days(n)).or_else(|_| invalid("Date out of range."))
}

/// All occurrences of `def` whose resolved instant lies in `[lo, hi)`, in time order, resolved
/// with the IANA rules of `zone` for each date (never a single current offset). Pure and
/// repeatable: the same inputs always give the same output.
pub fn generate(def: &Definition, zone: &Zone, lo: i64, hi: i64) -> AppResult<Vec<Candidate>> {
    if lo >= hi {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    match def.mode {
        ScheduleMode::HomeZone => {
            // One day of slack each side: an instant near the window edge can belong to a local
            // date that starts before/after it.
            let first = add_days(zone.local_date_at(lo), -1)?;
            let last = add_days(zone.local_date_at(hi - 1), 1)?;
            let mut d = first;
            while d <= last {
                for slot in &def.slots {
                    if !slot.runs_on(d.weekday()) {
                        continue;
                    }
                    let want = d.at((slot.minutes / 60) as i8, (slot.minutes % 60) as i8, 0, 0);
                    let r = zone.resolve(want)?;
                    // The whole civil date does not exist (e.g. Pacific/Apia skipped 2011-12-30):
                    // moving forward would land on the next date, which generates its own
                    // occurrence. Never backfill or double up; the date is simply skipped.
                    if r.local.date() != want.date() {
                        continue;
                    }
                    if r.utc_us >= lo && r.utc_us < hi {
                        out.push(Candidate {
                            slot_key: Some(slot.key()),
                            seq: None,
                            local_date: fmt_date(want),
                            intended_time: slot.time(),
                            scheduled_at_utc_us: r.utc_us,
                            offset_seconds: r.offset_seconds,
                            adjusted: r.local != want,
                            fold: 0,
                        });
                    }
                }
                if out.len() > MAX_CANDIDATES {
                    return invalid("That range is too large.");
                }
                d = add_days(d, 1)?;
            }
            out.sort_by_key(|c| (c.scheduled_at_utc_us, c.slot_key.clone()));
        }
        ScheduleMode::ElapsedInterval => {
            let (Some(i), Some(a)) = (def.interval_seconds, def.anchor_utc_us) else { return Ok(out) };
            let step = i * 1_000_000;
            let mut n = if lo <= a { 0 } else { (lo - a + step - 1) / step };
            loop {
                let t = a + n * step;
                if t >= hi {
                    break;
                }
                let off = zone.offset_at_utc(t);
                let local = local_from_utc(t, off);
                out.push(Candidate {
                    slot_key: None,
                    seq: Some(n),
                    local_date: fmt_date(local),
                    intended_time: format!("{:02}:{:02}", local.hour(), local.minute()),
                    scheduled_at_utc_us: t,
                    offset_seconds: off,
                    adjusted: false,
                    fold: 0,
                });
                if out.len() > MAX_CANDIDATES {
                    return invalid("That range is too large.");
                }
                n += 1;
            }
        }
    }
    Ok(out)
}

/// The next `count` occurrences at or after `from` and before `until` (if any), searching in
/// bounded chunks so a sparse weekday schedule is still found.
pub fn next_occurrences(def: &Definition, zone: &Zone, from: i64, until: Option<i64>, count: usize) -> AppResult<Vec<Candidate>> {
    const CHUNK: i64 = 14 * 86_400 * 1_000_000;
    const MAX_CHUNKS: i64 = 30;
    let mut out: Vec<Candidate> = Vec::new();
    let mut lo = from;
    for _ in 0..MAX_CHUNKS {
        let mut hi = lo + CHUNK;
        if let Some(u) = until {
            hi = hi.min(u);
        }
        if lo >= hi {
            break;
        }
        out.extend(generate(def, zone, lo, hi)?);
        if out.len() >= count {
            break;
        }
        lo = hi;
    }
    out.truncate(count);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::{fmt_local, parse_local};

    fn z(n: &str) -> Zone {
        Zone::iana(n).unwrap()
    }
    fn utc(s: &str) -> i64 {
        Zone::fixed(0).resolve(parse_local(s).unwrap()).unwrap().utc_us
    }
    fn daily(zone: &str, times: &[&str]) -> Definition {
        Definition::daily(Some(zone), &times.iter().map(|t| t.to_string()).collect::<Vec<_>>()).unwrap()
    }
    fn day(zone: &Zone, def: &Definition, date: &str) -> Vec<Candidate> {
        let lo = zone.resolve(parse_local(&format!("{date}T00:00")).unwrap()).unwrap().utc_us;
        generate(def, zone, lo, lo + 86_400_000_000).unwrap()
    }

    #[test]
    fn weekday_masks_round_trip_and_validate() {
        assert_eq!(mask_from_weekdays(&[]).unwrap(), 0);
        assert_eq!(mask_from_weekdays(&[1, 3]).unwrap(), 0b101);
        assert_eq!(mask_from_weekdays(&[1, 2, 3, 4, 5, 6, 7]).unwrap(), 0, "all days means every day");
        assert_eq!(weekdays_from_mask(0b100_0001), vec![1, 7]);
        assert!(mask_from_weekdays(&[0]).is_err() && mask_from_weekdays(&[8]).is_err());
    }

    #[test]
    fn validation_rejects_bad_definitions() {
        let ok = |d: Definition| d.validated();
        assert!(Definition::daily(Some("America/New_York"), &[]).is_err(), "needs a time");
        assert!(Definition::daily(Some("America/New_York"), &["08:00".into(), "08:00".into()]).is_err());
        assert!(Definition::daily(Some("America/New_York"), &["25:00".into()]).is_err());
        assert!(Definition::daily(Some("Mars/Olympus_Mons"), &["08:00".into()]).is_err(), "unknown zone");
        let elapsed = |i: Option<i64>, a: Option<i64>| Definition { mode: ScheduleMode::ElapsedInterval, zone_id: None, slots: vec![], interval_seconds: i, anchor_utc_us: a };
        assert!(ok(elapsed(Some(8 * 3600), Some(utc("2026-01-01T08:00")))).is_ok());
        assert!(ok(elapsed(Some(60), Some(0))).is_err(), "under 15 minutes");
        assert!(ok(elapsed(Some(31 * 86_400), Some(0))).is_err(), "over 30 days");
        assert!(ok(elapsed(None, Some(0))).is_err() && ok(elapsed(Some(3600), None)).is_err());
        assert!(ok(elapsed(Some(3600), Some(-1))).is_err());
        let mut with_zone = elapsed(Some(3600), Some(0));
        with_zone.zone_id = Some("UTC".into());
        assert!(ok(with_zone).is_err());
    }

    #[test]
    fn home_zone_resolves_each_date_with_that_dates_rules() {
        let ny = z("America/New_York");
        let def = daily("America/New_York", &["08:00"]);
        let winter = &day(&ny, &def, "2026-01-14")[0];
        assert_eq!((winter.scheduled_at_utc_us, winter.offset_seconds), (utc("2026-01-14T13:00"), -5 * 3600));
        let summer = &day(&ny, &def, "2026-07-14")[0];
        assert_eq!((summer.scheduled_at_utc_us, summer.offset_seconds), (utc("2026-07-14T12:00"), -4 * 3600));
        assert!(!winter.adjusted && !summer.adjusted);
    }

    #[test]
    fn spring_forward_gap_gives_exactly_one_adjusted_occurrence() {
        let ny = z("America/New_York");
        let def = daily("America/New_York", &["02:30"]);
        let gap = day(&ny, &def, "2026-03-08");
        assert_eq!(gap.len(), 1);
        // Phase 1 rule (Zone::resolve): move forward by the gap => 03:30 EDT.
        assert_eq!(gap[0].scheduled_at_utc_us, utc("2026-03-08T07:30"));
        assert_eq!(gap[0].offset_seconds, -4 * 3600);
        assert!(gap[0].adjusted);
        assert_eq!(gap[0].local_date, "2026-03-08");
        assert_eq!(gap[0].intended_time, "02:30", "the intent is kept; only the instant moves");
        // Neighbouring days are untouched.
        assert_eq!(day(&ny, &def, "2026-03-07")[0].scheduled_at_utc_us, utc("2026-03-07T07:30"));
        assert_eq!(day(&ny, &def, "2026-03-09")[0].scheduled_at_utc_us, utc("2026-03-09T06:30"));
        assert!(!day(&ny, &def, "2026-03-09")[0].adjusted);
    }

    #[test]
    fn fall_back_overlap_gives_exactly_one_earlier_occurrence() {
        let ny = z("America/New_York");
        let def = daily("America/New_York", &["01:30"]);
        let fold = day(&ny, &def, "2026-11-01");
        assert_eq!(fold.len(), 1, "one occurrence, not two");
        assert_eq!(fold[0].scheduled_at_utc_us, utc("2026-11-01T05:30"));
        assert_eq!(fold[0].offset_seconds, -4 * 3600, "earlier offset");
        assert!(!fold[0].adjusted);
        // A two-day window still has exactly one per date.
        let lo = ny.day_start(parse_local("2026-11-01T00:00").unwrap().date()).unwrap();
        assert_eq!(generate(&def, &ny, lo, lo + 2 * 86_400_000_000).unwrap().len(), 2);
    }

    #[test]
    fn southern_hemisphere_dst_runs_the_other_way() {
        let syd = z("Australia/Sydney");
        let def = daily("Australia/Sydney", &["02:30", "09:00"]);
        // 2026-10-04 spring forward: 02:30 is skipped (gap), 09:00 is AEDT.
        let spring = day(&syd, &def, "2026-10-04");
        assert_eq!(spring.len(), 2);
        assert!(spring[0].adjusted);
        assert_eq!(fmt_local(local_from_utc(spring[0].scheduled_at_utc_us, spring[0].offset_seconds)), "2026-10-04T03:30");
        assert_eq!(spring[1].offset_seconds, 11 * 3600);
        // 2026-04-05 fall back: 02:30 happens twice, earlier (AEDT +11) is used, once.
        let autumn = day(&syd, &def, "2026-04-05");
        assert_eq!(autumn.len(), 2);
        assert_eq!(autumn[0].scheduled_at_utc_us, utc("2026-04-04T15:30"));
        assert_eq!(autumn[0].offset_seconds, 11 * 3600);
        assert!(!autumn[0].adjusted);
        // July is winter (+10).
        assert_eq!(day(&syd, &def, "2026-07-15")[1].scheduled_at_utc_us, utc("2026-07-14T23:00"));
    }

    #[test]
    fn weekday_selection_limits_dates() {
        let ny = z("America/New_York");
        // Mon + Wed only.
        let def = Definition {
            mode: ScheduleMode::HomeZone,
            zone_id: Some("America/New_York".into()),
            slots: vec![Slot { minutes: 8 * 60, weekday_mask: 0b101 }],
            interval_seconds: None,
            anchor_utc_us: None,
        };
        let lo = ny.day_start(parse_local("2026-03-02T00:00").unwrap().date()).unwrap(); // a Monday
        let dates: Vec<String> = generate(&def, &ny, lo, lo + 7 * 86_400_000_000).unwrap().into_iter().map(|c| c.local_date).collect();
        assert_eq!(dates, ["2026-03-02", "2026-03-04"]);
    }

    #[test]
    fn home_zone_is_independent_of_the_viewing_device_zone() {
        // The same schedule gives the same UTC instants wherever the device is.
        let def = daily("America/Los_Angeles", &["08:00"]);
        let la = z("America/Los_Angeles");
        let c = &day(&la, &def, "2026-01-15")[0];
        assert_eq!(c.scheduled_at_utc_us, utc("2026-01-15T16:00"));
        // Spec example: LA 08:00 on 2026-01-15 is 11:00 EST in New York.
        let ny = z("America/New_York");
        assert_eq!(fmt_local(local_from_utc(c.scheduled_at_utc_us, ny.offset_at_utc(c.scheduled_at_utc_us))), "2026-01-15T11:00");
    }

    #[test]
    fn generation_is_deterministic_and_window_edges_are_half_open() {
        let ny = z("America/New_York");
        let def = daily("America/New_York", &["08:00", "20:00"]);
        let lo = utc("2026-03-10T12:00"); // exactly the 08:00 EDT instant
        let a = generate(&def, &ny, lo, lo + 86_400_000_000).unwrap();
        assert_eq!(a, generate(&def, &ny, lo, lo + 86_400_000_000).unwrap());
        assert_eq!(a[0].scheduled_at_utc_us, lo, "lo is inclusive");
        assert_eq!(a.len(), 2);
        let b = generate(&def, &ny, lo + 1, lo + 86_400_000_000).unwrap();
        assert_eq!(b.len(), 1, "08:00 at lo is excluded, and tomorrow's 08:00 sits exactly on the exclusive end");
        assert!(generate(&def, &ny, lo, lo).unwrap().is_empty());
    }

    #[test]
    fn date_line_gives_at_most_one_occurrence_per_local_date() {
        // Pacific/Apia skipped 2011-12-30 entirely: no occurrence is invented for it.
        let apia = z("Pacific/Apia");
        let def = daily("Pacific/Apia", &["09:00"]);
        let lo = utc("2011-12-28T00:00");
        let dates: Vec<String> = generate(&def, &apia, lo, lo + 5 * 86_400_000_000).unwrap().into_iter().map(|c| c.local_date).collect();
        assert!(!dates.contains(&"2011-12-30".to_string()), "{dates:?}");
        let mut uniq = dates.clone();
        uniq.dedup();
        assert_eq!(uniq, dates);
        assert!(dates.contains(&"2011-12-29".to_string()) && dates.contains(&"2011-12-31".to_string()), "{dates:?}");
    }

    #[test]
    fn elapsed_interval_is_absolute_time_across_dst() {
        let ny = z("America/New_York");
        // Every 12h from 2026-03-07 20:00 EST. The night of spring-forward is 11 wall-clock hours.
        let anchor = utc("2026-03-08T01:00");
        let def = Definition { mode: ScheduleMode::ElapsedInterval, zone_id: None, slots: vec![], interval_seconds: Some(12 * 3600), anchor_utc_us: Some(anchor) };
        let c = generate(&def, &ny, anchor, anchor + 3 * 12 * 3_600_000_000).unwrap();
        assert_eq!(c.iter().map(|c| c.seq.unwrap()).collect::<Vec<_>>(), [0, 1, 2]);
        assert_eq!(c[1].scheduled_at_utc_us - c[0].scheduled_at_utc_us, 12 * 3_600_000_000, "DST adds/removes no elapsed time");
        assert_eq!(c[0].offset_seconds, -5 * 3600);
        assert_eq!(c[2].offset_seconds, -4 * 3600, "displayed wall time shifts, cadence does not");
        // Window starting mid-way lands on the same grid (n counted from the anchor).
        let mid = generate(&def, &ny, anchor + 1, anchor + 3 * 12 * 3_600_000_000).unwrap();
        assert_eq!(mid.iter().map(|c| c.seq.unwrap()).collect::<Vec<_>>(), [1, 2]);
        // Nothing before the anchor.
        assert!(generate(&def, &ny, anchor - 86_400_000_000, anchor).unwrap().is_empty());
    }

    #[test]
    fn next_occurrences_finds_sparse_weekday_slots() {
        let ny = z("America/New_York");
        let def = Definition {
            mode: ScheduleMode::HomeZone,
            zone_id: Some("America/New_York".into()),
            slots: vec![Slot { minutes: 9 * 60, weekday_mask: 1 << 6 }], // Sundays
            interval_seconds: None,
            anchor_utc_us: None,
        };
        let n = next_occurrences(&def, &ny, utc("2026-03-02T00:00"), None, 7).unwrap();
        assert_eq!(n.len(), 7);
        assert_eq!(n[0].local_date, "2026-03-08");
        assert_eq!(n[1].local_date, "2026-03-15");
        assert!(next_occurrences(&def, &ny, utc("2026-03-02T00:00"), Some(utc("2026-03-04T00:00")), 7).unwrap().is_empty());
    }
}
