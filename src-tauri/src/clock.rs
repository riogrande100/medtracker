use crate::error::{invalid, AppResult};
use chrono::{DateTime, Datelike, Local, NaiveDateTime, Offset, Timelike};

/// The device clock reading used for one operation. Passed in so rules are deterministic.
#[derive(Debug, Clone, Copy)]
pub struct TimeCtx {
    pub now_utc_us: i64,
    pub offset_seconds: i32,
}

impl TimeCtx {
    pub fn system() -> Self {
        let now = Local::now();
        TimeCtx {
            now_utc_us: now.timestamp_micros(),
            offset_seconds: now.offset().fix().local_minus_utc(),
        }
    }

    fn local_now(&self) -> NaiveDateTime {
        local_from_utc(self.now_utc_us, self.offset_seconds)
    }

    pub fn today(&self) -> String {
        self.local_now().format("%Y-%m-%d").to_string()
    }

    pub fn minutes_now(&self) -> u32 {
        let t = self.local_now();
        t.hour() * 60 + t.minute()
    }

    /// Resolve a user-entered "YYYY-MM-DDTHH:MM" using this context's offset.
    pub fn resolve_local(&self, s: &str) -> AppResult<i64> {
        let n = parse_local(s)?;
        Ok(utc_from_local(n, self.offset_seconds))
    }
}

pub fn local_from_utc(utc_us: i64, offset: i32) -> NaiveDateTime {
    DateTime::from_timestamp_micros(utc_us)
        .unwrap_or_default()
        .naive_utc()
        + chrono::Duration::seconds(offset as i64)
}

pub fn utc_from_local(n: NaiveDateTime, offset: i32) -> i64 {
    (n - chrono::Duration::seconds(offset as i64)).and_utc().timestamp_micros()
}

pub fn fmt_local(n: NaiveDateTime) -> String {
    n.format("%Y-%m-%dT%H:%M").to_string()
}

pub fn parse_local(s: &str) -> AppResult<NaiveDateTime> {
    match NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M") {
        Ok(n) if n.year() >= 1970 => Ok(n),
        _ => invalid("Enter a valid date and time."),
    }
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
