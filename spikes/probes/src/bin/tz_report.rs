//! Host-side time-zone report. Run on a desktop device to collect zone-discovery evidence
//! (TZD-WIN-01, TZD-MAC-01) and anywhere to see how current each bundled tzdb is.
//!
//! Usage: tz_report [--strict]
//! `--strict` exits 1 if any engine's bundled tzdb is older than the newest IANA release recorded
//! in `tz::IANA_LATEST_AS_CHECKED`, if the host zone is unresolvable, or a sample zone is missing.
//! The output is host evidence only. It is not iOS/Android evidence and not a Tauri app run.

use std::env;
use std::process;

use medtracker_phase0_probes::tz::{self, ChronoTzEngine, JiffBundledEngine, TzEngine};

fn main() {
    let strict = env::args().any(|a| a == "--strict");
    let engines: Vec<Box<dyn TzEngine>> = vec![Box::new(ChronoTzEngine), Box::new(JiffBundledEngine::new())];
    let now = chrono::Utc::now().timestamp();

    let mut problems: Vec<String> = Vec::new();
    let engine_rows: Vec<_> = engines
        .iter()
        .map(|e| {
            let v = e.tzdb_version();
            let current = v.starts_with(tz::IANA_LATEST_AS_CHECKED);
            if !current {
                problems.push(format!("{} bundles tzdb {v}, older than IANA {}", e.name(), tz::IANA_LATEST_AS_CHECKED));
            }
            serde_json::json!({"engine": e.name(), "bundled_tzdb": v, "is_latest_iana_as_checked": current})
        })
        .collect();

    let zones: Vec<_> = tz::SAMPLE_ZONES
        .iter()
        .chain(tz::LEGACY_ALIASES.iter())
        .map(|z| {
            let offsets: Vec<_> = engines.iter().map(|e| e.offset_at(z, now)).collect();
            if offsets.iter().any(|o| o.is_err()) {
                problems.push(format!("sample zone {z} unresolvable"));
            }
            serde_json::json!({"zone": z, "offset_now_s": offsets.iter().map(|o| o.as_ref().ok().copied()).collect::<Vec<_>>()})
        })
        .collect();

    let host = tz::host_zone_report();
    if host.resolvable_by_chrono_tz != Some(true) || host.resolvable_by_jiff_bundled != Some(true) {
        problems.push("host zone not resolvable by both bundled engines".into());
    }

    let report = serde_json::json!({
        "kind": "host-side evidence; not device acceptance",
        "generated_at_utc": chrono::Utc::now().to_rfc3339(),
        "host": {"os": env::consts::OS, "arch": env::consts::ARCH, "system_zoneinfo_version": tz::host_system_tzdb_version()},
        "iana_latest_as_checked": {"version": tz::IANA_LATEST_AS_CHECKED, "released": tz::IANA_LATEST_RELEASED, "checked_on": tz::IANA_LATEST_CHECKED_ON},
        "device_zone": host,
        "engines": engine_rows,
        "sample_zones": zones,
        "problems": problems,
    });
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    if strict && !problems.is_empty() {
        process::exit(1);
    }
}
