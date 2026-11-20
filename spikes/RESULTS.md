# Phase 0 results (research date 2026-11-18)

**Scope note (2026-11-18, owner decision D-29): the first release is mobile only (iOS + Android); the Windows and macOS rows/columns below are DEFERRED (not removed) and are retained as evidence for desktop re-entry (REM-WIN/REM-MAC spikes). Health-platform rows are NOT PURSUED (owner decision D-30, 2026-11-18); their evidence is kept as a record only.** **Nothing below is device-verified.** No physical iOS, Android, Windows or macOS run exists. Every device scenario is PENDING (`cargo run -q --bin evidence_check`: 58 scenarios, 0 pass, 0 fail). "Docs/source" means read from documentation or plugin source; "host test" means `cargo test` on a developer Mac against the probe model, not a device. Sources: `sources.md`.

## Host-side results (verified on a Mac, not on devices)
- `cargo test`: DST gap, DST fold, Lord Howe 30-minute DST, Sydney (southern hemisphere), Apia skipped civil date, zone change westward/eastward in the `device_local` planner, reminder-degradation model, health-import model, evidence-judging rules, templates. These test the probe policy code (proposed in docs/product-spec.md), not OS behavior.
- tzdb currency: chrono-tz 0.10.4 bundles **2025b**; jiff-tzdb 1.0.0 bundles **2026f** (IANA latest, released 2026-10-27).
- Cross-engine differences observed 2026-2027 in the sample zones: **America/Havana** (2025b still falls back 2026-11-01; 2026f does not; an 08:00 dose on 2026-11-02 lands 1 hour apart) and **Africa/Casablanca** (offset differs from 2026-09-20; cause not investigated). All other sample zones and all DST policy tests agree between engines.
- Host zone on this Mac resolved in both engines (`America/Chicago`; system zoneinfo 2026f).
- Tauri OS plugin source has no time-zone API: device zone needs native code or another crate per platform.

## Capability matrix (status of evidence)
Legend: DOC = documented/source-read only; HOST = host test only; PENDING = no evidence.

| Area | iOS | Android | Windows (DEFERRED, D-29) | macOS (DEFERRED, D-29) |
|---|---|---|---|---|
| Scheduled reminders | DOC: plugin maps `at` to elapsed-time trigger, `interval` to calendar trigger; PENDING device | DOC: AlarmManager, inexact fallback without exact-alarm access; boot reschedule; PENDING device | DOC: plugin is immediate-only; native scheduled toast has 5-min window; PENDING | DOC: plugin immediate-only; native scheduling not researched; PENDING |
| Time zone (device zone + change events) | HOST model only; PENDING device | HOST model only; PENDING device (2 phones) | HOST model only; Windows-to-IANA mapping PENDING | HOST: zone resolved on this Mac, not recorded as acceptance; PENDING |
| Health import (NOT PURSUED, D-30) | DOC: dose events iOS 26+, denied indistinguishable from empty. SIM (iPhone 17, iOS 27.2 simulator, 2026-11-18; not device evidence): per-object medication sheet, medication list read and dose-event read (fake data) observed; third-party dose write refused by the OS (uncaught exception); denial indistinguishable from empty; revoke/locked/background-delivery not run. Physical device HLT-IOS-01..04 PENDING; see healthkit-ios/evidence/sim-results.json | DOC: no dose type in Health Connect; Medical Records experimental; PENDING | n/a | n/a |

## Recommendations
| Area | Recommendation | Re-entry criteria |
|---|---|---|
| iOS reminders | **DEFER** to device spike | All `required_for_go` REM-IOS-* PASS on a physical iPhone |
| Android reminders | **DEFER** | REM-AND-* required scenarios PASS on 2 phones from different manufacturers, plus a decision on exact-alarm handling (approximate or omit; `USE_EXACT_ALARM` eligibility for medication reminders is not established) |
| Windows reminders via Tauri plugin (DEFERRED, D-29) | **NO-GO** (docs: immediate-only) | Native scheduling spike passes REM-WIN-02..05 |
| macOS reminders via Tauri plugin (DEFERRED, D-29) | **NO-GO** (docs: immediate-only) | Native scheduling spike passes REM-MAC-02..06 |
| `device_local` schedule mode | **DEFER** | TZD-* and TZE-* PASS per platform; otherwise ship `home_zone` and `elapsed_interval` only |
| Time-zone engine | **Do not pin chrono-tz 0.10.4 as-is** (2025b, stale; confirmed by host tests). Prefer a current tzdb (jiff 2026f) or updated chrono-tz | Re-test with chosen engine; record tzdb version per occurrence; `tz_report --strict` exits 0 |
| HealthKit (iOS) medication import (B0-09 spike, simulator only) | **NOT PURSUED (D-30)**; spike closed. Simulator only: medication-name and dose-event reads worked on fake data, third-party write refused, denied indistinguishable from empty; no HLT-IOS PASS | Not applicable. Re-entry only under the D-30 criterion (explicit owner request, unmet catalog need, all HLT-* PASS on physical devices) |
| Health Connect import | **NOT PURSUED (D-30)** | Not applicable; re-entry only under the D-30 criterion |
| Google Fit / Google Health API | **NO-GO** (accounts + network conflict with constraints); not pursued (D-30) | Product constraint change; not applicable under D-30 |

Host tests did not contradict any recommendation carried over from HANDOFF.md. They strengthen the tz-engine one.

## Blockers needing physical devices
See the final report; scenario IDs are in `scenarios/scenarios.json`. Summary: iPhone (REM-IOS-00..11, TZD-IOS-01/02, TZE-IOS-01; HLT-IOS-01..04 not pursued (D-30)); two Android phones from different manufacturers (REM-AND-00..14, TZD-AND-01/02, TZE-AND-01; HLT-AND-01..03 not pursued (D-30)); Windows PC (REM-WIN-02..05 required, TZD-WIN-01, TZE-WIN-01); Mac (REM-MAC-02..06 required, TZD-MAC-01, TZE-MAC-01).

## Not covered
Catalog licensing, backup/recovery and signing: evidence-template items only, no claims made. Medical-safety content is out of scope.
