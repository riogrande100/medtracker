use crate::error::{invalid, AppResult};
use chrono::{Local, Offset};
use jiff::{
    civil::DateTime,
    tz::{AmbiguousOffset, Offset as JOffset, TimeZone},
    Timestamp,
};

/// A time zone whose UTC offset is looked up for the *specific instant or date* being
/// resolved, using IANA rules from jiff's bundled tz database (never the OS database, so
/// results are identical on every platform). `Fixed` is only a fallback for when the device's
/// IANA zone id is unknown or no longer recognised.
#[derive(Debug, Clone)]
pub struct Zone {
    tz: TimeZone,
    id: Option<String>,
}

/// A wall-clock time resolved to an instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resolved {
    pub utc_us: i64,
    /// The offset actually in force at `utc_us` (not "today's" offset).
    pub offset_seconds: i32,
    /// The wall-clock reading of `utc_us` at `offset_seconds`. Equals the entered time
    /// except when the entered time fell in a spring-forward gap (see `Zone::resolve`).
    pub local: DateTime,
}

impl Zone {
    pub fn iana(name: &str) -> Option<Zone> {
        TimeZone::get(name).ok().map(|tz| Zone { tz, id: Some(name.to_string()) })
    }

    pub fn fixed(offset_seconds: i32) -> Zone {
        let off = JOffset::from_seconds(offset_seconds).unwrap_or(JOffset::UTC);
        Zone { tz: TimeZone::fixed(off), id: None }
    }

    /// The device's zone. Falls back to the device's current fixed offset when the IANA id
    /// can't be read or isn't in the bundled database.
    pub fn detect() -> Zone {
        Zone::from_name_or_fixed(iana_time_zone::get_timezone().ok().as_deref(), Local::now().offset().fix().local_minus_utc())
    }

    pub fn from_name_or_fixed(name: Option<&str>, fallback_offset: i32) -> Zone {
        name.and_then(Zone::iana).unwrap_or_else(|| Zone::fixed(fallback_offset))
    }

    /// Rebuild the zone stored on an event; `fallback_offset` is the offset stored with it.
    pub fn from_stored(id: Option<&str>, fallback_offset: i32) -> Zone {
        Zone::from_name_or_fixed(id, fallback_offset)
    }

    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// The offset in force at an instant.
    pub fn offset_at_utc(&self, utc_us: i64) -> i32 {
        match Timestamp::from_microsecond(utc_us) {
            Ok(ts) => self.tz.to_offset(ts).seconds(),
            Err(_) => 0,
        }
    }

    /// Resolve a wall-clock time using the offset in force on THAT date.
    ///
    /// DST rules (deterministic, matching RFC 5545 / JS Temporal "compatible"):
    /// * Overlap (fall-back; the wall time occurs twice): the EARLIER instant is used,
    ///   i.e. the offset before the clock change (e.g. 01:30 on 2026-11-01 in New York is
    ///   01:30 EDT, UTC-4). The later repetition can't be selected by wall time alone.
    /// * Gap (spring-forward; the wall time never occurs): the time is moved FORWARD by the
    ///   length of the gap, i.e. interpreted with the offset before the change (02:30 on
    ///   2026-03-08 in New York becomes 03:30 EDT, UTC-4). The returned `local`/`offset_seconds`
    ///   describe that real instant, so stored values always agree with each other.
    pub fn resolve(&self, dt: DateTime) -> AppResult<Resolved> {
        self.resolve_choice(dt, false)
    }

    /// Like `resolve`, but when the wall time occurs twice (fall-back overlap) and
    /// `second_occurrence` is true, picks the LATER instant (the offset after the clock change).
    /// The flag is ignored for wall times that are not repeated.
    pub fn resolve_choice(&self, dt: DateTime, second_occurrence: bool) -> AppResult<Resolved> {
        let amb = self.tz.to_ambiguous_zoned_datetime(dt);
        let pick = if second_occurrence && matches!(amb.offset(), AmbiguousOffset::Fold { .. }) { amb.later() } else { amb.compatible() };
        match pick {
            Ok(z) => Ok(Resolved {
                utc_us: z.timestamp().as_microsecond(),
                offset_seconds: z.offset().seconds(),
                local: z.datetime(),
            }),
            Err(_) => invalid("Enter a valid date and time."),
        }
    }
}

/// The device clock reading used for one operation. Passed in so rules are deterministic.
#[derive(Debug, Clone)]
pub struct TimeCtx {
    pub now_utc_us: i64,
    pub zone: Zone,
}

impl TimeCtx {
    pub fn system() -> Self {
        TimeCtx { now_utc_us: Timestamp::now().as_microsecond(), zone: Zone::detect() }
    }

    #[cfg(test)]
    pub fn fixed(now_utc_us: i64, offset_seconds: i32) -> Self {
        TimeCtx { now_utc_us, zone: Zone::fixed(offset_seconds) }
    }

    pub fn offset_now(&self) -> i32 {
        self.zone.offset_at_utc(self.now_utc_us)
    }

    fn local_now(&self) -> DateTime {
        local_from_utc(self.now_utc_us, self.offset_now())
    }

    pub fn today(&self) -> String {
        fmt_date(self.local_now())
    }

    pub fn minutes_now(&self) -> u32 {
        let t = self.local_now();
        t.hour() as u32 * 60 + t.minute() as u32
    }

    /// Resolve a user-entered "YYYY-MM-DDTHH:MM" in this context's zone, optionally choosing the
    /// second occurrence of a repeated (fall-back) hour.
    pub fn resolve_local_choice(&self, s: &str, second_occurrence: bool) -> AppResult<Resolved> {
        self.zone.resolve_choice(parse_local(s)?, second_occurrence)
    }
}

pub fn local_from_utc(utc_us: i64, offset: i32) -> DateTime {
    let ts = Timestamp::from_microsecond(utc_us).unwrap_or(Timestamp::UNIX_EPOCH);
    JOffset::from_seconds(offset).unwrap_or(JOffset::UTC).to_datetime(ts)
}

pub fn fmt_local(n: DateTime) -> String {
    format!("{:04}-{:02}-{:02}T{:02}:{:02}", n.year(), n.month(), n.day(), n.hour(), n.minute())
}

pub fn fmt_date(n: DateTime) -> String {
    format!("{:04}-{:02}-{:02}", n.year(), n.month(), n.day())
}

/// Strictly parse "YYYY-MM-DDTHH:MM" (year 1970..=9999).
pub fn parse_local(s: &str) -> AppResult<DateTime> {
    let b = s.as_bytes();
    let shape = b.len() == 16
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b'T'
        && b[13] == b':'
        && b.iter().enumerate().all(|(i, c)| matches!(i, 4 | 7 | 10 | 13) || c.is_ascii_digit());
    let num = |a: usize, z: usize| s[a..z].parse::<i32>().ok();
    if shape {
        if let (Some(y), Some(mo), Some(d), Some(h), Some(mi)) = (num(0, 4), num(5, 7), num(8, 10), num(11, 13), num(14, 16)) {
            if y >= 1970 {
                if let Ok(dt) = DateTime::new(y as i16, mo as i8, d as i8, h as i8, mi as i8, 0, 0) {
                    return Ok(dt);
                }
            }
        }
    }
    invalid("Enter a valid date and time.")
}

pub fn parse_hhmm(s: &str) -> AppResult<u32> {
    let ok = s.len() == 5 && s.as_bytes()[2] == b':';
    let h = s.get(0..2).and_then(|x| x.parse::<u32>().ok());
    let m = s.get(3..5).and_then(|x| x.parse::<u32>().ok());
    match (ok, h, m) {
        (true, Some(h), Some(m)) if h < 24 && m < 60 => Ok(h * 60 + m),
        _ => invalid("Times must look like 08:00."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn z(name: &str) -> Zone {
        Zone::iana(name).expect("zone in bundled tzdb")
    }
    fn at(zone: &Zone, s: &str) -> Resolved {
        zone.resolve(parse_local(s).unwrap()).unwrap()
    }
    fn utc(s: &str) -> i64 {
        // "YYYY-MM-DDTHH:MM" interpreted as UTC
        Zone::fixed(0).resolve(parse_local(s).unwrap()).unwrap().utc_us
    }

    #[test]
    fn zone_ids_resolve_from_the_bundled_database() {
        assert!(Zone::iana("America/Argentina/Buenos_Aires").is_some());
        assert!(Zone::iana("Mars/Olympus_Mons").is_none());
    }

    #[test]
    fn backdated_time_uses_offset_in_force_on_the_entered_date_not_today() {
        let ny = z("America/New_York");
        // "Now" is mid-summer (EDT, -4h); the user backdates into winter (EST, -5h).
        let winter = at(&ny, "2026-01-15T09:00");
        assert_eq!(winter.offset_seconds, -5 * 3600);
        assert_eq!(winter.utc_us, utc("2026-01-15T14:00"));
        // And into summer from a winter "now".
        let summer = at(&ny, "2026-07-15T09:00");
        assert_eq!(summer.offset_seconds, -4 * 3600);
        assert_eq!(summer.utc_us, utc("2026-07-15T13:00"));
        // Same zone, one fixed-offset context would have got one of these wrong.
        assert_ne!(winter.offset_seconds, summer.offset_seconds);
    }

    #[test]
    fn us_spring_forward_gap_moves_forward_by_the_gap() {
        let ny = z("America/New_York");
        // 2026-03-08 02:00 -> 03:00 in New York: 02:30 does not exist.
        let r = at(&ny, "2026-03-08T02:30");
        assert_eq!(r.offset_seconds, -4 * 3600, "uses the offset before the change, landing after the gap");
        assert_eq!(fmt_local(r.local), "2026-03-08T03:30");
        assert_eq!(r.utc_us, utc("2026-03-08T07:30"));
        // Self-consistency: stored (utc, offset) reproduces the stored local reading.
        assert_eq!(fmt_local(local_from_utc(r.utc_us, r.offset_seconds)), "2026-03-08T03:30");
        // Neighbours are unaffected.
        assert_eq!(at(&ny, "2026-03-08T01:59").offset_seconds, -5 * 3600);
        assert_eq!(at(&ny, "2026-03-08T03:00").offset_seconds, -4 * 3600);
    }

    #[test]
    fn us_fall_back_overlap_picks_the_earlier_instant() {
        let ny = z("America/New_York");
        // 2026-11-01 02:00 EDT -> 01:00 EST: 01:30 happens twice.
        let r = at(&ny, "2026-11-01T01:30");
        assert_eq!(r.offset_seconds, -4 * 3600, "first (EDT) occurrence");
        assert_eq!(r.utc_us, utc("2026-11-01T05:30"));
        assert_eq!(fmt_local(r.local), "2026-11-01T01:30");
        // The second occurrence is exactly one hour later and is only reachable via the instant.
        assert_eq!(local_from_utc(r.utc_us + 3_600_000_000, -5 * 3600), r.local);
        assert_eq!(at(&ny, "2026-11-01T00:59").offset_seconds, -4 * 3600);
        assert_eq!(at(&ny, "2026-11-01T02:00").offset_seconds, -5 * 3600);
        // Explicitly choosing the second occurrence gives the later instant, same wall reading.
        let second = ny.resolve_choice(parse_local("2026-11-01T01:30").unwrap(), true).unwrap();
        assert_eq!(second.offset_seconds, -5 * 3600);
        assert_eq!(second.utc_us, r.utc_us + 3_600_000_000);
        assert_eq!(second.local, r.local);
        // The choice is ignored when the time is not repeated.
        assert_eq!(ny.resolve_choice(parse_local("2026-11-01T03:30").unwrap(), true).unwrap().offset_seconds, -5 * 3600);
        assert_eq!(ny.resolve_choice(parse_local("2026-07-01T03:30").unwrap(), true).unwrap().offset_seconds, -4 * 3600);
    }

    #[test]
    fn southern_hemisphere_dst_runs_the_other_way() {
        let syd = z("Australia/Sydney");
        // January is summer (AEDT +11), July is winter (AEST +10).
        assert_eq!(at(&syd, "2026-01-15T09:00").offset_seconds, 11 * 3600);
        assert_eq!(at(&syd, "2026-07-15T09:00").offset_seconds, 10 * 3600);
        assert_eq!(at(&syd, "2026-07-15T09:00").utc_us, utc("2026-07-14T23:00"));
        // Spring-forward is in October (2026-10-04 02:00 -> 03:00): gap at 02:30.
        let gap = at(&syd, "2026-10-04T02:30");
        assert_eq!(gap.offset_seconds, 11 * 3600);
        assert_eq!(fmt_local(gap.local), "2026-10-04T03:30");
        // Fall-back is in April (2026-04-05 03:00 AEDT -> 02:00 AEST): 02:30 happens twice.
        let fold = at(&syd, "2026-04-05T02:30");
        assert_eq!(fold.offset_seconds, 11 * 3600, "earlier (AEDT) occurrence");
        assert_eq!(fold.utc_us, utc("2026-04-04T15:30"));
    }

    #[test]
    fn unknown_zone_id_falls_back_to_the_stored_fixed_offset() {
        let zone = Zone::from_stored(Some("Mars/Olympus_Mons"), -6 * 3600);
        assert!(zone.id().is_none());
        // Offset is constant regardless of date: no DST guessing without rules.
        assert_eq!(at(&zone, "2026-01-15T09:00").offset_seconds, -6 * 3600);
        assert_eq!(at(&zone, "2026-07-15T09:00").offset_seconds, -6 * 3600);
        assert_eq!(at(&zone, "2026-07-15T09:00").utc_us, utc("2026-07-15T15:00"));
        // Missing id behaves the same; a known id is used when present.
        assert!(Zone::from_stored(None, 3600).id().is_none());
        assert_eq!(Zone::from_stored(Some("Europe/Paris"), 0).id(), Some("Europe/Paris"));
        assert_eq!(Zone::from_name_or_fixed(None, 7200).offset_at_utc(0), 7200);
    }

    #[test]
    fn today_uses_the_offset_at_now() {
        // 2026-03-08 06:59 UTC is 01:59 EST; one minute later it is 03:00 EDT.
        let ny = z("America/New_York");
        let before = TimeCtx { now_utc_us: utc("2026-03-08T06:59"), zone: ny.clone() };
        let after = TimeCtx { now_utc_us: utc("2026-03-08T07:00"), zone: ny };
        assert_eq!(before.minutes_now(), 1 * 60 + 59);
        assert_eq!(after.minutes_now(), 3 * 60);
        assert_eq!(after.today(), "2026-03-08");
    }

    #[test]
    fn parse_local_is_strict() {
        assert!(parse_local("2026-03-08T02:30").is_ok());
        for bad in ["", "2026-3-8T2:30", "2026-02-30T10:00", "2026-03-08 02:30", "2026-03-08T24:00", "1969-12-31T23:59", "+026-03-08T02:30"] {
            assert!(parse_local(bad).is_err(), "{bad}");
        }
    }
}
