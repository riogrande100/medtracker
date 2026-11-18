# Phase 0 spike handoff (work stopped early to save cost; uncommitted)

Branch `riogrande100-spike-medtracker-platforms`. Research dates: 2026-11-18. Only `spikes/` was touched.
**No physical-device evidence exists. Every device scenario is PENDING. Nothing here proves reminders or health import work.**

## State of the work

| Item | State |
|---|---|
| `spikes/probes/` crate (`tz`, `reminder`, `health`, `evidence` modules; bins `tz_report`, `evidence_check`) | Written. `cargo build` passed. `tz_report` and `evidence_check` each ran once (58 scenarios, all PENDING, exit 0). |
| `spikes/scenarios/scenarios.json` | Written: 58 scenarios, pass/fail criteria, platform x area matrix. Not reviewed. |
| Unit/integration tests (`spikes/probes/tests/`) | **Not written. Directory is empty. `cargo test` has never been run.** `tz::resolve` and `plan_device_local` are therefore unverified. |
| `spikes/evidence/templates/` | **Empty.** One JSON template per scenario family is still needed (schema is in `src/evidence.rs`). |
| `spikes/README.md`, `RESULTS.md` (matrix and recommendations), `timezone/README.md`, `sources.md` | **Not written.** `scenarios.json` references `spikes/timezone/README.md`, which does not exist; `SAMPLE_ZONES` in `src/tz.rs` is the list it should describe. |
| Tauri/native device apps | None built. No emulator or device runs. |

Run when resuming: `cd spikes/probes && cargo test && cargo run -q --bin evidence_check && cargo run -q --bin tz_report`.
`tz_report --strict` currently exits 1 (see chrono-tz below).

## Host evidence collected (not device evidence)

- `tz_report` on this Mac (macOS, aarch64): device zone `America/Chicago`, resolvable by both engines; system zoneinfo `2026f`.
- IANA latest `2026f`, released 2026-10-27 (Cuba does not fall back 2026-11-01). Source: https://www.iana.org/time-zones
- **chrono-tz 0.10.4 (latest on crates.io, last release 2025-07-11) bundles tzdb `2025b`: stale.** jiff 1.0.1 with jiff-tzdb 1.0.0 (2026-11-15) bundles `2026f`. `docs/product-spec.md` names chrono-tz as an example engine. This is a real finding for the time-engine decision (not edited in the spec).
- Tauri OS plugin source has no time-zone API, so device zone needs native code or another crate per platform.

## Documentation findings (capability only, not device-verified)

Tauri notification plugin 2.8.1 (needs Rust >= 1.90). Read from the `v2` branch source, not a release tag.
- Docs: `Schedule` is mobile-only; desktop notifications are immediate. Source: https://v2.tauri.app/plugin/notification/ and https://v2.tauri.app/reference/javascript/notification/
- Android source: uses AlarmManager. When `canScheduleExactAlarms()` is false it falls back to inexact `set` (or `setAndAllowWhileIdle` if `allowWhileIdle`). The plugin manifest declares `RECEIVE_BOOT_COMPLETED`, `WAKE_LOCK` and `POST_NOTIFICATIONS`, but **not** `SCHEDULE_EXACT_ALARM` or `USE_EXACT_ALARM`. A boot receiver reschedules saved requests; past-due ones are re-fired about 15 s after boot. `pending()` reads the plugin's own SharedPreferences, not AlarmManager, so it cannot detect OS-cancelled alarms. Repeating `at` uses inexact `setRepeating`.
- iOS source: `Schedule.at` becomes a `UNTimeIntervalNotificationTrigger` (elapsed time from now, not a wall-clock trigger). `Schedule.interval` becomes a repeating `UNCalendarNotificationTrigger`. Behavior across time-zone or clock changes is not documented.
- Desktop source: `notify-rust`, immediate only. macOS and Windows cannot remove a shown notification. Windows plugin notifications only work for installed apps.
- Android alarms: `SCHEDULE_EXACT_ALARM` is denied by default for new installs targeting Android 13+. Revocation cancels future exact alarms. Alarms are cancelled at shutdown unless rebuilt. `USE_EXACT_ALARM` is for alarm/calendar apps under Play policy; whether a medication reminder qualifies is **not established**. Sources: https://developer.android.com/develop/background-work/services/alarms and https://developer.android.com/about/versions/14/changes/schedule-exact-alarms
- Windows: scheduled toasts have a 5-minute delivery window and are dropped if the PC stays off beyond it. Source: https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/app-notifications-scheduled
- macOS: no Apple page for native scheduling was fetched for this handoff; rely on the existing spec row until the macOS spike runs.
- Apple UserNotifications docs fetched do not state a pending-request cap or time-zone-change behavior.

HealthKit (Apple docs, fetched as `.md` pages):
- `HKMedicationDoseEvent` is iOS 26+. `LogStatus` has taken, skipped, snoozed, notInteracted, notificationNotSent, notLogged; `scheduleType` is asNeeded or schedule. Pipeline statuses are not intake.
- An app cannot tell denied from full access (denied reads look like no data); only limited-window access is detectable. Locked devices may block background reads. A privacy policy is required, and HealthKit data may not go to ads or brokers. See "Authorizing access to health data" and "Protecting user privacy".
- I did not verify whether third-party apps can write medication dose events.

Health Connect and Google:
- The documented data-type categories (activity, body, cycle, nutrition, sleep, vitals, wellness) do not list medication dose records.
- Medical Records exposes FHIR `Medication`, `MedicationRequest` and `MedicationStatement` through an experimental API (`ExperimentalPersonalHealthRecordApi`, Jetpack 1.1.0-beta02, needs the Android 16 SDK). Play policy for it is "still being developed". A statement or request is not evidence of intake.
- Play Console lists "Medication and pain management" under health-app categories.
- Google Fit APIs: deprecated in 2026, no new sign-ups since 2024-05-01. The Google Health API is a cloud OAuth API (account plus network), which conflicts with product constraints. The legacy Fitbit Web API was turned off 2026-11-02.

## Provisional recommendations (docs and host only; no device evidence)

| Area | Recommendation | Re-entry criteria |
|---|---|---|
| iOS reminders | **DEFER** to device spike | All `required_for_go` REM-IOS-* scenarios PASS on a physical iPhone. |
| Android reminders | **DEFER** | REM-AND-* PASS on 2 phones from different manufacturers, and a decision on exact-alarm handling (approximate or omit). |
| Windows / macOS scheduled reminders via the Tauri plugin | **NO-GO** (docs establish immediate-only) | Native scheduling spike passes REM-WIN-* / REM-MAC-*. |
| `device_local` schedule mode | **DEFER** | TZD-* and TZE-* PASS per platform; otherwise ship `home_zone` and `elapsed_interval` only. |
| tz engine | **Do not pin chrono-tz 0.10.4 as-is** (2025b) | Re-test jiff or an updated chrono-tz; record tzdb version per occurrence. |
| HealthKit / Health Connect import | **NO-GO for MVP** (spec: no automatic import) | A product requirement plus HLT-* PASS. Imports may only appear as unverified external claims and never create or change a dose. |
| Google Fit / Google Health API | **NO-GO** (network and accounts) | A product constraint change. |

Catalog licensing, backup/recovery and signing: evidence-template items only, no legal claims. Not started in `spikes/`.

## Exact next action

Write `spikes/probes/tests/` first, in this order:
1. DST tests from the spec examples. New York spring-forward 02:30 on 2026-03-08 -> `2026-03-08T07:00:00Z`, adjusted. Fall-back 01:30 on 2026-11-01 -> earlier fold `05:30Z`, one occurrence. Add Lord Howe (30-minute DST), Apia 2011-12-30 (`CivilDateMissing`) and Havana 2026-11-01.
2. A cross-engine test asserting that chrono-tz and jiff agree, with the known 2025b vs 2026f divergence made explicit.
3. Reminder-model tests (permission denied, unavailable, exact-alarm lost, requests removed, no "missed" wording, notification events write zero doses).
4. Health tests and evidence-judging tests using synthetic fixtures (synthetic must never PASS).

Then run `cargo test`, write the evidence templates, `README.md` and `RESULTS.md`, and re-run `evidence_check`. Temporary source downloads in `/tmp/probe-docs` and `/tmp/tauri-plugins-probe` are disposable; the URLs above are the record.
