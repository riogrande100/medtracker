use std::collections::BTreeSet;

use chrono::{NaiveDate, NaiveTime};
use medtracker_phase0_probes::tz::{self, ChronoTzEngine, JiffBundledEngine, Resolution, TzEngine, plan_device_local, resolve, utc};

fn d(y: i32, m: u32, dd: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, dd).unwrap()
}
fn t(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap()
}

/// Run an assertion against every engine; used where the two tzdb snapshots must agree.
fn engines() -> Vec<Box<dyn TzEngine>> {
    vec![Box::new(ChronoTzEngine), Box::new(JiffBundledEngine::new())]
}

#[test]
fn dst_gap_new_york_spring_forward() {
    for e in engines() {
        let r = resolve(e.as_ref(), "America/New_York", d(2026, 3, 8), t(2, 30)).unwrap();
        match r {
            Resolution::AdjustedForGap { utc: u, offset_s, shifted_by_s, date_changed } => {
                assert_eq!(u, utc(2026, 3, 8, 7, 0, 0), "{}", e.name());
                assert_eq!(offset_s, -4 * 3600);
                assert_eq!(shifted_by_s, 1800);
                assert!(!date_changed);
            }
            other => panic!("{}: {other:?}", e.name()),
        }
    }
}

#[test]
fn dst_fold_new_york_fall_back_is_one_earlier_occurrence() {
    for e in engines() {
        let r = resolve(e.as_ref(), "America/New_York", d(2026, 11, 1), t(1, 30)).unwrap();
        assert_eq!(
            r,
            Resolution::FoldEarlier {
                utc: utc(2026, 11, 1, 5, 30, 0),
                offset_s: -4 * 3600,
                later_utc: utc(2026, 11, 1, 6, 30, 0),
                later_offset_s: -5 * 3600
            },
            "{}",
            e.name()
        );
    }
}

#[test]
fn ordinary_time_is_exact() {
    let r = resolve(&JiffBundledEngine::new(), "America/New_York", d(2026, 7, 1), t(8, 0)).unwrap();
    assert_eq!(r, Resolution::Exact { utc: utc(2026, 7, 1, 12, 0, 0), offset_s: -4 * 3600 });
}

#[test]
fn lord_howe_thirty_minute_dst() {
    // DST starts 2026-10-04 02:00 +10:30 -> 02:30 +11:00 (gap 02:00-02:30). Ends 2026-04-05 02:00 +11 -> 01:30 +10:30.
    for e in engines() {
        let gap = resolve(e.as_ref(), "Australia/Lord_Howe", d(2026, 10, 4), t(2, 15)).unwrap();
        match gap {
            Resolution::AdjustedForGap { utc: u, offset_s, shifted_by_s, .. } => {
                assert_eq!(offset_s, 11 * 3600, "{}", e.name());
                assert_eq!(shifted_by_s, 900);
                assert_eq!(u, utc(2026, 10, 3, 15, 30, 0));
            }
            other => panic!("{}: {other:?}", e.name()),
        }
        let fold = resolve(e.as_ref(), "Australia/Lord_Howe", d(2026, 4, 5), t(1, 45)).unwrap();
        match fold {
            Resolution::FoldEarlier { offset_s, later_offset_s, utc: u, later_utc } => {
                assert_eq!((offset_s, later_offset_s), (11 * 3600, 10 * 3600 + 1800), "{}", e.name());
                assert_eq!(later_utc - u, 1800);
            }
            other => panic!("{}: {other:?}", e.name()),
        }
    }
}

#[test]
fn southern_hemisphere_sydney_inverted_dst() {
    for e in engines() {
        // January is DST (+11) and July is standard (+10) in Sydney.
        let jan = resolve(e.as_ref(), "Australia/Sydney", d(2026, 1, 15), t(8, 0)).unwrap();
        let jul = resolve(e.as_ref(), "Australia/Sydney", d(2026, 7, 15), t(8, 0)).unwrap();
        assert_eq!(jan, Resolution::Exact { utc: utc(2026, 1, 14, 21, 0, 0), offset_s: 11 * 3600 }, "{}", e.name());
        assert_eq!(jul, Resolution::Exact { utc: utc(2026, 7, 14, 22, 0, 0), offset_s: 10 * 3600 }, "{}", e.name());
        // Spring forward 2026-10-04 02:00 -> 03:00; fall back 2026-04-05 03:00 -> 02:00.
        assert!(matches!(
            resolve(e.as_ref(), "Australia/Sydney", d(2026, 10, 4), t(2, 30)).unwrap(),
            Resolution::AdjustedForGap { offset_s, .. } if offset_s == 11 * 3600
        ));
        assert!(matches!(
            resolve(e.as_ref(), "Australia/Sydney", d(2026, 4, 5), t(2, 30)).unwrap(),
            Resolution::FoldEarlier { offset_s, .. } if offset_s == 11 * 3600
        ));
    }
}

#[test]
fn apia_skipped_civil_date() {
    for e in engines() {
        assert_eq!(resolve(e.as_ref(), "Pacific/Apia", d(2011, 12, 30), t(8, 0)).unwrap(), Resolution::CivilDateMissing, "{}", e.name());
        assert!(matches!(resolve(e.as_ref(), "Pacific/Apia", d(2011, 12, 29), t(8, 0)).unwrap(), Resolution::Exact { .. }));
        assert!(matches!(resolve(e.as_ref(), "Pacific/Apia", d(2011, 12, 31), t(8, 0)).unwrap(), Resolution::Exact { .. }));
    }
}

#[test]
fn fixed_offset_and_unusual_offsets() {
    let e = JiffBundledEngine::new();
    assert_eq!(resolve(&e, "Asia/Kathmandu", d(2026, 6, 1), t(9, 0)).unwrap(), Resolution::Exact { utc: utc(2026, 6, 1, 3, 15, 0), offset_s: 20_700 });
    assert_eq!(resolve(&e, "Etc/GMT+5", d(2026, 6, 1), t(9, 0)).unwrap(), Resolution::Exact { utc: utc(2026, 6, 1, 14, 0, 0), offset_s: -5 * 3600 });
}

#[test]
fn unknown_zone_is_an_error_not_a_guess() {
    for e in engines() {
        assert!(resolve(e.as_ref(), "Mars/Olympus_Mons", d(2026, 1, 1), t(8, 0)).is_err());
    }
}

#[test]
fn legacy_aliases_resolve_in_both_engines() {
    for e in engines() {
        for z in tz::LEGACY_ALIASES {
            assert!(e.offset_at(z, utc(2026, 6, 1, 0, 0, 0)).is_ok(), "{} cannot resolve {z}", e.name());
        }
    }
}

#[test]
fn all_sample_zones_resolve_in_both_engines() {
    for e in engines() {
        for z in tz::SAMPLE_ZONES {
            assert!(e.offset_at(z, utc(2026, 6, 1, 0, 0, 0)).is_ok(), "{} cannot resolve {z}", e.name());
        }
    }
}

#[test]
fn elapsed_interval_ignores_zones_and_dst() {
    let anchor = utc(2026, 3, 7, 12, 0, 0);
    // 8-hour interval keeps exactly 8 elapsed hours across the NY spring-forward.
    assert_eq!(tz::elapsed_occurrence(anchor, 8 * 3600, 5) - tz::elapsed_occurrence(anchor, 8 * 3600, 4), 8 * 3600);
}

#[test]
fn home_zone_dose_keeps_wall_clock_across_dst() {
    let e = JiffBundledEngine::new();
    let a = resolve(&e, "America/New_York", d(2026, 3, 7), t(8, 0)).unwrap().utc().unwrap();
    let b = resolve(&e, "America/New_York", d(2026, 3, 8), t(8, 0)).unwrap().utc().unwrap();
    assert_eq!(b - a, 23 * 3600);
}

// ---- cross-engine comparison and the tzdb staleness finding ----

#[test]
fn bundled_versions_are_recorded_and_differ() {
    let c = ChronoTzEngine.tzdb_version();
    let j = JiffBundledEngine::new().tzdb_version();
    eprintln!("chrono-tz bundles {c}; jiff bundles {j}; IANA latest as checked {}", tz::IANA_LATEST_AS_CHECKED);
    assert_eq!(c, "2025b", "chrono-tz pin changed; update RESULTS.md");
    assert_eq!(j, tz::IANA_LATEST_AS_CHECKED, "jiff-tzdb no longer matches latest IANA; update constants");
    assert_ne!(c, j);
}

#[test]
fn engines_agree_on_most_2026_zones_but_diverge_on_havana_and_casablanca() {
    // Compare every sample zone at 12-hour steps over 2026-01..2027-01 and collect disagreements.
    let (c, j) = (ChronoTzEngine, JiffBundledEngine::new());
    let mut diffs: BTreeSet<String> = BTreeSet::new();
    let start = utc(2026, 1, 1, 0, 0, 0);
    for z in tz::SAMPLE_ZONES {
        for i in 0..(365 * 2 + 2) {
            let ts = start + i * 43_200;
            if c.offset_at(z, ts).unwrap() != j.offset_at(z, ts).unwrap() {
                diffs.insert((*z).to_string());
            }
        }
    }
    eprintln!("zones where chrono-tz(2025b) and jiff(2026f) disagree in 2026: {diffs:?}");
    // Observed disagreements (2025b vs 2026f). Havana: the 2026-11-01 fall-back is cancelled in
    // 2026f (IANA, see sources.md). Africa/Casablanca: offsets differ from 2026-09-20 onward; the
    // cause was NOT investigated against the tz release notes, only observed here.
    assert_eq!(diffs, BTreeSet::from(["Africa/Casablanca".to_string(), "America/Havana".to_string()]));
}

#[test]
fn havana_2026_11_01_stale_tzdb_changes_the_answer() {
    let (c, j) = (ChronoTzEngine, JiffBundledEngine::new());
    let (ts_before, ts_after) = (utc(2026, 11, 1, 3, 0, 0), utc(2026, 11, 2, 12, 0, 0));
    // 2025b (stale): Cuba falls back to -05:00. 2026f: Cuba stays on -04:00 (per HANDOFF/IANA).
    assert_eq!(c.offset_at("America/Havana", ts_after).unwrap(), -5 * 3600);
    assert_eq!(j.offset_at("America/Havana", ts_after).unwrap(), -4 * 3600);
    assert_eq!(c.offset_at("America/Havana", ts_before).unwrap(), -4 * 3600);

    // 00:30 on the transition day is an ambiguous fold under 2025b only.
    let rc = resolve(&c, "America/Havana", d(2026, 11, 1), t(0, 30)).unwrap();
    let rj = resolve(&j, "America/Havana", d(2026, 11, 1), t(0, 30)).unwrap();
    eprintln!("Havana 2026-11-01 00:30  chrono-tz(2025b)={rc:?}\n                         jiff(2026f)={rj:?}");
    assert_ne!(rc, rj);
    // A 08:00 dose on 2026-11-02 lands 1 hour apart depending on which tzdb is used.
    let uc = resolve(&c, "America/Havana", d(2026, 11, 2), t(8, 0)).unwrap().utc().unwrap();
    let uj = resolve(&j, "America/Havana", d(2026, 11, 2), t(8, 0)).unwrap().utc().unwrap();
    assert_eq!(uc - uj, 3600);
}

#[test]
fn engines_agree_on_dst_policy_for_non_stale_zones() {
    for z in ["America/New_York", "Europe/London", "Australia/Lord_Howe", "Pacific/Apia", "Asia/Kolkata"] {
        for (date, time) in [(d(2026, 3, 8), t(2, 30)), (d(2026, 11, 1), t(1, 30)), (d(2026, 3, 29), t(1, 30)), (d(2026, 10, 25), t(1, 30)), (d(2011, 12, 30), t(8, 0))] {
            assert_eq!(resolve(&ChronoTzEngine, z, date, time).unwrap(), resolve(&JiffBundledEngine::new(), z, date, time).unwrap(), "{z} {date} {time}");
        }
    }
}

// ---- device_local planner ----

fn plan(timeline: &[(i64, &str)], time: NaiveTime, from: i64, n: usize, consumed: &mut BTreeSet<NaiveDate>) -> Vec<tz::PlannedOccurrence> {
    plan_device_local(&JiffBundledEngine::new(), timeline, time, from, n, consumed).unwrap()
}

#[test]
fn device_local_single_zone_through_dst_gap_and_fold() {
    let tl = [(0, "America/New_York")];
    let mut c = BTreeSet::new();
    let p = plan(&tl, t(2, 30), utc(2026, 3, 7, 0, 0, 0), 3, &mut c);
    let dates: Vec<_> = p.iter().map(|o| o.local_date).collect();
    assert_eq!(dates, vec![d(2026, 3, 7), d(2026, 3, 8), d(2026, 3, 9)]);
    assert!(matches!(p[1].resolution, Resolution::AdjustedForGap { .. }));
    assert_eq!(p[1].resolution.utc(), Some(utc(2026, 3, 8, 7, 0, 0)));

    let mut c = BTreeSet::new();
    let p = plan(&tl, t(1, 30), utc(2026, 10, 31, 12, 0, 0), 3, &mut c);
    assert_eq!(p.len(), 3);
    // 10-31 01:30 EDT (05:30Z) is already past `from`, so the first planned slot is the fold day.
    assert_eq!(p[0].local_date, d(2026, 11, 1));
    assert_eq!(p[0].resolution.utc(), Some(utc(2026, 11, 1, 5, 30, 0)));
}

#[test]
fn device_local_never_schedules_the_fold_twice() {
    let tl = [(0, "America/New_York")];
    let mut c = BTreeSet::new();
    let p = plan(&tl, t(1, 30), utc(2026, 11, 1, 4, 0, 0), 4, &mut c);
    let dates: BTreeSet<_> = p.iter().map(|o| o.local_date).collect();
    assert_eq!(dates.len(), p.len(), "one occurrence per civil date");
    assert_eq!(p[0].resolution.utc(), Some(utc(2026, 11, 1, 5, 30, 0)));
}

#[test]
fn device_local_apia_skipped_date_is_not_backfilled() {
    let tl = [(0, "Pacific/Apia")];
    let mut c = BTreeSet::new();
    let p = plan(&tl, t(8, 0), utc(2011, 12, 29, 0, 0, 0), 3, &mut c);
    let dates: Vec<_> = p.iter().map(|o| o.local_date).collect();
    assert!(!dates.contains(&d(2011, 12, 30)));
    assert!(dates.contains(&d(2011, 12, 31)));
}

#[test]
fn device_local_southern_hemisphere_dst_start() {
    let tl = [(0, "Australia/Sydney")];
    let mut c = BTreeSet::new();
    let p = plan(&tl, t(2, 30), utc(2026, 10, 3, 0, 0, 0), 3, &mut c);
    assert_eq!(p[0].local_date, d(2026, 10, 4));
    assert!(matches!(p[0].resolution, Resolution::AdjustedForGap { .. }));
}

#[test]
fn zone_change_westward_does_not_repeat_the_same_civil_date() {
    // Dose for 2026-06-01 08:00 New York was already taken (slot consumed). The traveller lands
    // in Los Angeles at 13:00Z (06:00 local, same civil date). 08:00 LA on 06-01 must not fire.
    let tl = [(0, "America/New_York"), (utc(2026, 6, 1, 13, 0, 0), "America/Los_Angeles")];
    let mut consumed = BTreeSet::from([d(2026, 6, 1)]);
    let p = plan(&tl, t(8, 0), utc(2026, 6, 1, 13, 0, 0), 2, &mut consumed);
    assert_eq!(p[0].local_date, d(2026, 6, 2));
    assert_eq!(p[0].zone, "America/Los_Angeles");
    assert_eq!(p[0].resolution.utc(), Some(utc(2026, 6, 2, 15, 0, 0)));
    assert_eq!(p[1].resolution.utc(), Some(utc(2026, 6, 3, 15, 0, 0)));
}

#[test]
fn zone_change_westward_unconsumed_slot_still_fires_once() {
    // Not yet taken: after landing in LA at 06:00, 08:00 LA on the same date is still ahead.
    let tl = [(0, "America/New_York"), (utc(2026, 6, 1, 13, 0, 0), "America/Los_Angeles")];
    let mut consumed = BTreeSet::new();
    let p = plan(&tl, t(8, 0), utc(2026, 6, 1, 10, 0, 0), 3, &mut consumed);
    // NY 08:00 on 06-01 was 12:00Z (before the move) and fires in NY; after landing the 06-01 slot is consumed.
    assert_eq!(p[0].zone, "America/New_York");
    assert_eq!(p[0].resolution.utc(), Some(utc(2026, 6, 1, 12, 0, 0)));
    assert_eq!(p[1].local_date, d(2026, 6, 2));
    assert_eq!(p[1].zone, "America/Los_Angeles");
    let u: Vec<_> = p.iter().map(|o| o.resolution.utc().unwrap()).collect();
    assert!(u.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn zone_change_eastward_skips_forward_without_backfill() {
    // New York -> London at 2026-06-01 22:00Z (23:00 BST). 08:00 on 06-02 in London = 07:00Z.
    let tl = [(0, "America/New_York"), (utc(2026, 6, 1, 22, 0, 0), "Europe/London")];
    let mut c = BTreeSet::new();
    let p = plan(&tl, t(8, 0), utc(2026, 6, 1, 13, 0, 0), 3, &mut c);
    let u: Vec<_> = p.iter().map(|o| o.resolution.utc().unwrap()).collect();
    assert!(u.windows(2).all(|w| w[0] < w[1]), "{u:?}");
    let dates: BTreeSet<_> = p.iter().map(|o| o.local_date).collect();
    assert_eq!(dates.len(), 3);
    assert!(p.iter().any(|o| o.zone == "Europe/London" && o.resolution.utc() == Some(utc(2026, 6, 2, 7, 0, 0))));
}

#[test]
fn zone_timeline_on_stale_chrono_tz_shifts_a_havana_dose() {
    // Same planner, two engines: the only difference is the tzdb snapshot.
    let tl = [(0, "America/Havana")];
    let (mut c1, mut c2) = (BTreeSet::new(), BTreeSet::new());
    let from = utc(2026, 11, 1, 12, 0, 0);
    let a = plan_device_local(&ChronoTzEngine, &tl, t(8, 0), from, 2, &mut c1).unwrap();
    let b = plan_device_local(&JiffBundledEngine::new(), &tl, t(8, 0), from, 2, &mut c2).unwrap();
    assert_ne!(a[1].resolution.utc(), b[1].resolution.utc());
}

#[test]
fn casablanca_offset_differs_between_snapshots_late_2026() {
    let ts = utc(2026, 12, 1, 12, 0, 0);
    let (c, j) = (ChronoTzEngine.offset_at("Africa/Casablanca", ts).unwrap(), JiffBundledEngine::new().offset_at("Africa/Casablanca", ts).unwrap());
    eprintln!("Casablanca 2026-12-01: chrono-tz(2025b)={c}s jiff(2026f)={j}s");
    assert_eq!((c, j), (3600, 0));
}
