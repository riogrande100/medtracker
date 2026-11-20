# Phase 0 spikes

Throwaway host-side probes and a device-acceptance harness for MedTracker. **No physical-device evidence exists yet; every device scenario is PENDING.** Nothing in this directory proves reminders, time-zone handling or health import on iOS, Android, Windows or macOS.

## Layout
- `probes/` Rust crate (not application code): `tz`, `reminder`, `health`, `evidence` modules; bins `tz_report`, `evidence_check`; tests in `probes/tests/`.
- `scenarios/scenarios.json` 58 scenarios with machine-checked pass criteria.
- `evidence/templates/` one JSON template per scenario family (14). Templates are skipped by `evidence_check`.
- `timezone/README.md` sample zone set. `sources.md`, `RESULTS.md`, `HANDOFF.md`.

## Run host-side checks
```
cd spikes/probes
cargo test                      # tz resolution, planner, reminder model, health model, evidence rules
cargo run -q --bin evidence_check   # acceptance board; exit 0 = no failures (PENDING is not failure)
cargo run -q --bin tz_report        # bundled tzdb versions, sample zones, host zone
cargo run -q --bin tz_report -- --strict   # exit 1 while any engine's tzdb is older than IANA latest
```
`evidence_check` options: `--json`, `--evidence-dir DIR`, `--require-go PLATFORM:AREA` (exit 3 if the gate is not met). Exit 1 means a scenario failed; 2 unreadable files.
`tz_report --strict` currently exits 1 because chrono-tz 0.10.4 bundles tzdb 2025b while IANA latest is 2026f.

## Record device evidence
1. Pick a scenario ID from `scenarios/scenarios.json` and read its `pass` text and criteria.
2. Copy the matching family template from `evidence/templates/` (for example `REM-IOS.template.json`) to `evidence/<scenario_id>-<device_id>.json`.
3. Set `scenario_id`; fill `device` (id, manufacturer, model, os_version), `app.build_mode` (release builds only: `release-dev-signed` or `release-distribution-signed`), `started_at_utc` (RFC 3339), `operator`, `physical_device: true`.
4. Timed scenarios: one entry per trial in `trials` with `target_utc` and `observed_utc` (null if never delivered); set `duplicate`/`stale` where seen. Checklists: set each check to true/false in `checks`. Measurements: fill `numbers`.
5. Delete every key starting with `_`. Leave nothing null that you actually tested.
6. Run `cargo run -q --bin evidence_check`. Only a valid file from a real physical device can move a scenario off PENDING. Files marked `synthetic`, emulator/simulator runs, debug builds and unfilled fields are rejected.
7. Scenarios with `min_distinct_devices: 2` (all Android reminder/time-zone ones) need passing files from two different device ids, ideally two manufacturers.
8. Keep raw logs/screenshots alongside, referenced from `notes`. Do not commit personal data.

`spikes/**/target` is gitignored.
