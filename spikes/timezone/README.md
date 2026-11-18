# Time-zone sample zone set

Referenced by `spikes/scenarios/scenarios.json` (TZD-* scenarios). The authoritative list is `SAMPLE_ZONES` in `spikes/probes/src/tz.rs`; keep both in sync. Legacy aliases are `LEGACY_ALIASES` in the same file.

Every platform must resolve every zone below to an IANA identifier that the bundled tzdb knows (zero unresolvable). Windows reports Windows zone names and needs an explicit mapping.

| Zone | Why it is in the set |
|---|---|
| UTC, Etc/GMT+5 | Fixed offsets; note the POSIX-inverted sign of Etc/GMT+5 (UTC-5) |
| America/Los_Angeles, America/New_York | Common DST rules; spec spring-forward and fall-back examples |
| America/Havana | DST rule changed between tzdb 2025b and 2026f (2026-11-01 fall-back cancelled in 2026f) |
| America/St_Johns | 30-minute standard offset (-03:30) |
| America/Sao_Paulo | Southern hemisphere, no DST since 2019 (history) |
| Europe/London, Europe/Dublin | Dublin models negative DST in tzdb source |
| Africa/Casablanca | Ramadan-driven offset changes; differs between tzdb 2025b and 2026f from 2026-09-20 (observed, cause not investigated) |
| Africa/Cairo | DST reintroduced 2023 |
| Asia/Kolkata, Asia/Kathmandu | +05:30 and +05:45 offsets |
| Asia/Tehran | +03:30, DST abolished 2022 |
| Asia/Tokyo | No DST |
| Australia/Lord_Howe | 30-minute DST shift |
| Australia/Sydney | Southern-hemisphere DST (inverted seasons) |
| Pacific/Auckland, Pacific/Chatham | +12/+13 and +12:45/+13:45 |
| Pacific/Apia | Skipped civil date 2011-12-30 |
| Pacific/Kiritimati | +14, furthest ahead |
| Pacific/Honolulu | No DST, -10 |

Legacy aliases that must resolve or be explicitly mapped: Asia/Calcutta, Asia/Katmandu, Europe/Kiev, America/Buenos_Aires, US/Pacific, Asia/Saigon.

## Scenarios using this list
- TZD-*-01: device-reported zone maps to an IANA id in this set; run `cargo run -q --bin tz_report` on desktop devices, and an equivalent native probe on mobile.
- TZD-*-02 (iOS, Android): OS offsets at sampled instants agree with the bundled tzdb.
- TZE-*: zone-change events (running, backgrounded, cold start).

## Resolution policy under test (proposed in docs/product-spec.md, software behavior only)
Gap: first valid instant after the gap. Fold: earlier instant, one occurrence. Wholly missing civil date: no occurrence. Record the tzdb version used for each occurrence.
