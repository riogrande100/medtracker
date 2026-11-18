# MedTracker Current State (baseline)

**Baseline date:** 2026-11-18 · **Branch:** `main` @ `ec55cfa` · **Author scope:** planning/documentation only.

This is a baseline report, not a plan. Facts below come from the repository tree, [handoff-2026-11-18-paused.md](handoff-2026-11-18-paused.md), and `spikes/HANDOFF.md`. Anything not observed by an artifact is labeled **unverified**. Product direction is in [product-spec.md](product-spec.md#authoritative-product-direction); this file records only what exists.

## 1. What exists on `main`

| Item | State |
|---|---|
| Planning documents | [product-spec.md](product-spec.md) (merged, authoritative), [backlog.md](backlog.md), [registers.md](registers.md), [ux-journeys.md](ux-journeys.md), [release-evidence.md](release-evidence.md), this file, and `docs/planning-draft/` (history only). Evidence in the spec was checked 2026-11-14 and not re-verified. |
| Phase 1 vertical slice | **Exists on `main`** (integrated from the 92-file branch `riogrande100-medtracker-phase1-slice`): Tauri 2, React/TypeScript/Vite, locally bundled Geist fonts, Tailwind and Radix; a Rust command/application/storage boundary; SQLite migrations (`src-tauri/src/storage/sql/001_initial.sql`); immutable medication snapshots and manual medication entry; a local dose ledger with Taken, explicit Skip, backdating, correction revisions, history, undo, medication archive and deletion; accessible Today and History surfaces; a native-reminder adapter **stub**; an empty runtime network allowlist baseline and local-only UI copy. |
| Host test results (per the handoff, not re-run for this document) | `npm ci`, `npm run typecheck`, `npm run build` passed; `npm test` 8/8 UI tests; `cargo test` 10/10 Rust tests. `npm ci` reported six high-severity dependency audit findings and one install-script warning; **not triaged**. |
| Phase 0 spike scaffolding | `spikes/` holds a probe crate (`tz`, `reminder`, `health`, `evidence` modules), 58 scenario definitions (all **PENDING**), and notes; as of the paused handoff `cargo test` for it had never been run and its tests directory was empty ([spikes/HANDOFF.md](../spikes/HANDOFF.md)); another worker is extending it concurrently, so re-read `spikes/` before relying on this row. |
| Mobile/desktop status | [MOBILE.md](../MOBILE.md): behavior verified **on the macOS host only**. `tauri ios init` succeeded; nothing was built or run on a simulator or device. Android init is blocked (no SDK/NDK). Windows not attempted. |
| CI | None (no `.github/`). |
| Releases, signing, store accounts | None known. |
| `product-owner-prompt.md`, `start-prompt.md` | Rewritten to the current direction (Phases 0–4, WS-1…WS-9 workstreams, no authentication/subscription/Linux-first/mockup gate). |

## 2. Known review findings (being fixed; not yet verified fixed)

An independent architecture review of the slice found the following. A fix is in progress in the Phase 1 worktree and has **not** been integrated or re-tested here. Each is carried into [backlog.md](backlog.md#phase-1--the-today-core-loop) as acceptance items.

| # | Severity | Finding | Ticket |
|---|---|---|---|
| 1 | High | Backdated/corrected wall times use the offset from the current day, not the entered date, producing wrong instants and stored offsets across DST. Needs IANA rules for the entered date and DST tests. | B1-04, B1-05 |
| 2 | Medium | Today is not refreshed at midnight, on resume/focus, or when an item becomes due; Rust does not reject stale slot-linked dates. | B1-06 |
| 3 | Medium | No pre-migration snapshot, no `integrity_check`, no recoverable UI state for migration or newer-database failures. | B1-02 |
| 4 | Medium | Undo is intentionally soft/reversible, but per-event privacy deletion (redact notes and revision payloads, record a tombstone) is missing. | B1-05 |

Other gaps against the backlog observed in an earlier read and not yet audited: no `schedule`/`schedule_version` tables (slots attach to a medication); changing slots deletes `scheduled_occurrence` rows, which conflicts with PR-07; time context is a fixed OS offset without an IANA zone (see [MOBILE.md](../MOBILE.md)); the undo shape differs from D-10 (`voided` status). Per backlog governance rule 5, none of this code counts toward a ticket until audited.

## 3. Phase 0 findings so far (documentation and host only)

| Finding | Status |
|---|---|
| `chrono-tz` 0.10.4 bundles tzdb 2025b, stale against IANA 2026f (released 2026-10-27); `jiff` 1.x with bundled tzdb is recommended (D-04a). | Library finding from the host spike; **not a device claim**. Spec updated. |
| Tauri notification plugin: `Schedule` is mobile-only; desktop notifications are immediate; Android falls back to inexact alarms when exact-alarm access is missing; iOS `Schedule.at` is an elapsed-time trigger. | Read from documentation and source; **no device run**. |
| HealthKit and Android/Google health: medication-dose support exists only on recent OS versions or experimental APIs, or is unestablished; the Google Health API is a cloud OAuth API. | Documentation only; feasibility remains B0-09, optional and non-blocking (PR-28). |

Spike recommendations are provisional and carry no device evidence: iOS/Android reminders DEFER, Windows/macOS plugin scheduling NO-GO, `device_local` DEFER, health import NO-GO for MVP.

## 4. Host toolchain (macOS arm64 planning host, observed earlier)

| Tool | Observed |
|---|---|
| macOS / Xcode | macOS 27.2.1 arm64; Xcode 27.2 |
| Rust | 1.99.1 with `aarch64-apple-darwin`, `aarch64-apple-ios`, `aarch64-apple-ios-sim`, `x86_64-apple-ios` (no Android or Windows targets) |
| Node / npm | v26.12.0 / 11.20.0 |
| Android | no `adb`, no SDK, no Java runtime |
| Windows host | none available |
| Doc tooling | `markdownlint-cli2` not installed (run via `npx`); a local link/anchor script is used instead |

Consequence: macOS and an iOS simulator are plausible on this host. **Android and Windows are blocked** until a human supplies a toolchain/host, and no physical device was observed.

## 5. Resolved contradictions (latest product direction wins)

| # | Conflict | Resolution |
|---|---|---|
| 1 | iOS-first vs four primary targets | Four primary targets, none first; release order from per-platform evidence (PR-16, PR-24, D-05). |
| 2 | Linux-first production target and CI | Linux non-primary, gates nothing (D-08, D-26). |
| 3 | Authentication, registration, subscription, "backup and sync" in the old PO prompt | Removed (PR-22); local backup/restore only. |
| 4 | No engineering before mockup approval | Design review is parallel; usability is a Phase 3 gate (PR-27, D-07). |
| 5 | Phases 1–9 (PO prompt) vs Phases 0–4 | Phases 0–4 only; PO phases become WS-1…WS-9 ([mapping](product-spec.md#workstream-mapping)). |
| 6 | Spec Phase 1 lacked schedule and Today | Minimal schedule + occurrences in Phase 1 (B1-04); versioning in Phase 2 (D-11). |
| 7 | Delete-only vs accidental-action undo | Undo = `voided` with audit; delete is the separate privacy action (PR-21, PR-06, D-10). |
| 8 | Fully offline vs network-enabled sibling revision | Merged: allowlist model; web access only for reviewed features; allowlist currently empty (PR-23, D-27); connectivity failure never blocks local logging (PR-01). |
| 9 | Health integration as `PR-19` in the sibling revision, colliding with the Today core loop `PR-19` | Today keeps PR-19; health integration is **PR-28**, optional and feasibility-gated (B0-09, DF-08, D-28). |
| 10 | `chrono-tz` as the time engine | `jiff` 1.x with bundled tzdb (D-04a). |
| 11 | "Do not generate application code yet" (start prompt) | Applied to the original planning task only; implementation follows [backlog.md](backlog.md). |

## 6. Gate status at baseline

All of the following are **NOT PASSED — evidence-dependent** (no artifact exists): GT-LIC, GT-UX, GT-SIGN, GT-STORE, GT-LEGAL, GT-A11Y, GT-REC, GT-NET, GT-HEALTH, GT-DEV-IOS, GT-DEV-AND, GT-DEV-WIN, GT-DEV-MAC, and phase gates G0–G4. No device, store, or signing evidence exists. Host-side unit tests do not satisfy any of these. Definitions: [registers.md](registers.md). Per-platform cells: [release-evidence.md](release-evidence.md).

## 7. Known limitations of this baseline

- External facts (Tauri plugin capabilities, OS notification behavior, NLM licensing, crate versions) are carried from the 2026-11-14 spec evidence and the 2026-11-18 spike notes and were **not re-verified** here.
- Test counts come from the handoff; they were not re-run for this document.
- Paused sibling sessions (Phase 2 MVP, spikes) may hold uncommitted work not reflected here.
- No stakeholder, device, store, or legal input has been collected.
