# Sources

All fetched or checked 2026-11-18 (per `spikes/HANDOFF.md`). Documentation and source reading only; none of this is device verification. Items marked "not fetched" are explicitly unverified.

| Topic | URL | Date | Used for |
|---|---|---|---|
| IANA tz releases | https://www.iana.org/time-zones | 2026-11-18 | Latest `2026f`, released 2026-10-27; Cuba does not fall back 2026-11-01 |
| Tauri notification plugin docs | https://v2.tauri.app/plugin/notification/ | 2026-11-18 | `Schedule` is mobile-only; desktop notifications are immediate |
| Tauri notification JS reference | https://v2.tauri.app/reference/javascript/notification/ | 2026-11-18 | Schedule types |
| Tauri plugins-workspace source (`v2` branch, plugin 2.8.1, Rust >= 1.90; not a release tag) | github.com/tauri-apps/plugins-workspace, `plugins/notification` | 2026-11-18 | Android AlarmManager use, inexact fallback, manifest permissions, boot receiver; iOS trigger types; desktop `notify-rust` |
| Android alarms | https://developer.android.com/develop/background-work/services/alarms | 2026-11-18 | Exact alarm permission, cancellation on revoke or shutdown |
| Android 14 exact alarm change | https://developer.android.com/about/versions/14/changes/schedule-exact-alarms | 2026-11-18 | `SCHEDULE_EXACT_ALARM` denied by default (Android 13+ targets) |
| Windows scheduled app notifications | https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/app-notifications-scheduled | 2026-11-18 | 5-minute delivery window, dropped if PC stays off |
| Apple HealthKit docs (`.md` pages: HKMedicationDoseEvent, "Authorizing access to health data", "Protecting user privacy") | developer.apple.com/documentation/healthkit | 2026-11-18 | iOS 26+ dose events and statuses, denied-vs-empty indistinguishable, privacy policy |
| Health Connect data types / Medical Records (Jetpack 1.1.0-beta02, experimental) | developer.android.com health-connect docs | 2026-11-18 | No medication dose type; FHIR Medication* resources experimental |
| Google Fit / Google Health API / Fitbit Web API deprecation notices | developer.android.com and developers.google.com pages | 2026-11-18 | Fit deprecated, no new sign-ups since 2024-05-01; Fitbit Web API off 2026-11-02; Health API is cloud OAuth |
| Play Console health app categories | support.google.com/googleplay/android-developer | 2026-11-18 | "Medication and pain management" category |
| crates.io chrono-tz 0.10.4 | https://crates.io/crates/chrono-tz | 2026-11-18 | Latest release 2025-07-11, bundles tzdb 2025b |
| crates.io jiff 1.0.1 / jiff-tzdb 1.0.0 | https://crates.io/crates/jiff-tzdb | 2026-11-18 | jiff-tzdb 1.0.0 (2026-11-15) bundles 2026f |
| Apple UserNotifications docs | developer.apple.com/documentation/usernotifications | 2026-11-18 | Fetched pages do not state a pending-request cap or time-zone behavior (absence of statement) |
| macOS native scheduling | not fetched | n/a | Rely on spec row until the macOS spike runs |

Host observation (this repo's `cargo test`, not a web source): on 2026-11-18, `chrono-tz` 0.10.4 reports tzdb `2025b`; jiff bundled reports `2026f`; they disagree for `America/Havana` and `Africa/Casablanca` in 2026-2027 (see `probes/tests/tz.rs`).
