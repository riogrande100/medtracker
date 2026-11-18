# MedTracker Current State (baseline)

**Baseline date:** 2026-11-18 · **Branch:** `riogrande100-reconcile-medtracker-planning` (from `main` @ `8f8d43a`) · **Author scope:** planning/documentation only.

This is a baseline report, not a plan. Facts below were observed in this worktree or via read-only git inspection of sibling branches. Anything not observed is labeled **unverified**.

## 1. What exists on `main`

| Item | State |
|---|---|
| `README.md` | Two lines: title and "A simple private medication tracker." |
| `LICENSE` | Apache License 2.0 text. |
| `.gitignore` | Rust-oriented (Cargo, rustfmt, mutants, ICE dumps, RustRover); no Node/Tauri-generated entries. |
| `docs/product-spec.md` | ~86 KB planning spec, evidence dated 2026-11-14, PR-01–PR-18, Phases 0–4, backlog B0-01…B3-01. Originally iOS-first and fully offline. |
| `start-prompt.md` | The original planning prompt (spec-generation brief). |
| `product-owner-prompt.md` | Product-owner brief with Phases 1–9, Linux-first CI, authentication/subscription/sync flows, and a mockup-approval gate. |
| Application code | **None** on `main`: no `Cargo.toml`, `package.json`, `src/`, or `src-tauri/`. |
| CI | **None**: no `.github/` directory. |
| Tests / builds | **None** on `main`. |
| Releases, signing, store accounts | **None known.** |

## 2. Parallel work in sibling worktrees (read-only observation)

`git worktree list` shows other sessions working from the same repository. Nothing below was modified, merged, or accepted by this branch.

| Branch | Observation | Implication |
|---|---|---|
| `riogrande100-medtracker-phase1-slice` (1 commit, 92 files) | Tauri 2 + React/Vite/Tailwind app with Rust `application/ledger.rs`, `application/medication.rs`, `storage/sql/001_initial.sql`, Today/History/Medications views, Vitest and Rust tests. CSP is `default-src 'self'; connect-src ipc: http://ipc.localhost …`; capability file grants `core:default` only (comment: "runtime network allowlist is empty"). Depends on locally bundled `@fontsource-variable/geist`. Its own `MOBILE.md` states Phase 1 was verified **on the macOS host only**: iOS `tauri ios init` succeeded but nothing was built or run on a simulator/device; Android init **blocked** (no SDK/NDK); Windows not attempted. Notifications are a stub. | A **candidate starting point** for B1-01…B1-08, not accepted evidence. Quick read found gaps against this backlog: no `schedule`/`schedule_version` tables (slots attach to a medication), `medication.rs` deletes `scheduled_occurrence` rows when slots change (conflicts with PR-07), time context is a fixed OS offset without IANA zone/DST engine (its notes say so), `undo` exists but with a different shape than D-10 (`voided` status). Audit under backlog governance rule 5. |
| `riogrande100-user-friendly-medtracker-platforms` (also on `origin`, 1 commit) | Revises the three governing docs toward four primary targets and a Today/Taken/Skip/Undo flow, but **also** permits runtime network access (catalog retrieval, updates, optional sync later, Android Health Connect integration), relaxes but still keeps an approval gate on "production feature work", and re-uses requirement ID `PR-19` for health integrations. | Conflicts with the latest request (empty runtime network allowlist, no sync, no mockup gate on Phase 1) and with this branch's `PR-19`. **Do not merge as-is**; its Today/Undo framing is already incorporated here, and its optional health integration is carried as deferred feature DF-08 in [registers.md](registers.md). |
| `riogrande100-spike-medtracker-platforms`, `copilot-prewarm-scaling-robot` | Same commit as `main`; no work yet. | None. |

## 3. Host toolchain (this macOS arm64 machine)

| Tool | Observed |
|---|---|
| macOS | 27.2.1, arm64 |
| Xcode | 27.2 (build 27B52), command-line tools path set |
| Rust | `rustc`/`cargo` 1.99.1; installed targets: `aarch64-apple-darwin`, `aarch64-apple-ios`, `aarch64-apple-ios-sim`, `x86_64-apple-ios` (no Android or Windows targets listed) |
| Node / npm | v26.12.0 / 11.20.0 (npm registry reachable at check time) |
| `cargo-tauri` CLI | not found (the `phase1-slice` branch uses `@tauri-apps/cli` via npm instead) |
| Android | no `adb`, no `~/Library/Android/sdk`, no Java runtime found |
| Windows host | none available to this session |
| Doc tooling | `markdownlint`, `markdownlint-cli2`, `lychee`, `mmdc` not installed (can be run via `npx`; see Validation in the delivery report) |

Consequence: on this host, macOS and iOS-simulator builds are plausible; **Android and Windows are blocked** until a human supplies a toolchain/host. Physical iOS/Android devices were not observed. No evidence for any gate exists.

## 4. Contradictions found in the source documents (and resolution)

| # | Conflict | Sources | Resolution (latest user request wins) |
|---|---|---|---|
| 1 | iOS as the first release candidate vs four primary targets | spec §1/§2/§13 vs request | Four primary targets, none ranked first; per-platform evidence decides release order (PR-16, PR-24, D-05). |
| 2 | Linux-first production target, Linux-only CI | PO prompt Phase 6 | Linux is non-primary and gates nothing; CI cost policy is target-neutral (D-08, D-26). |
| 3 | Authentication, registration, login, subscription, "backup and sync" in the flow | PO prompt Phase 1 | Removed; excluded scope PR-22; local backup/restore only. |
| 4 | No UI/engineering before mockup approval | PO prompt Phases 1, 4 | Design review is parallel and non-blocking for Phase 1; usability evidence is a Phase 3 gate (PR-27, D-07). |
| 5 | Phases 1–9 (PO prompt) vs Phases 0–4 (spec/start prompt) | both | Phases 0–4 are the delivery phases; PO "phases" become workstreams WS-1…WS-9 mapped to them. |
| 6 | Spec Phase 1 had no schedule and no Today; primary experience needs due occurrences | spec §11 vs request | Minimal schedule + occurrence engine moved into Phase 1 (B1-04); full versioning stays Phase 2 (D-11). |
| 7 | Spec had delete-with-tombstone but no accidental-action undo | spec §3/§4 | Undo = void with audit (PR-21, D-10); delete remains the separate privacy action. |
| 8 | Fully offline, no runtime network vs sibling branch allowing network integrations | spec PR-01/PR-17 vs sibling docs | Allowlist empty by default with a change process (PR-23); health integration deferred, non-blocking (DF-08). |
| 9 | Spec "Android, then desktop" and desktop notifications "later"; PO prompt Windows/macOS "local verification only" | spec, PO prompt | All four have equal status; each gets its own matrix; desktop reminders not promised until a spike (B0-06). |
| 10 | Start prompt: "Do not generate application code yet" | start prompt | Applied to the planning task; implementation follows [backlog.md](backlog.md). |

## 5. Gate status at baseline

All of the following are **NOT PASSED — evidence-dependent** (no artifact exists): GT-LIC (catalog licensing), GT-UX (stakeholder usability), GT-SIGN (signing), GT-STORE (store), GT-LEGAL, GT-A11Y, GT-REC, GT-NET, GT-DEV-IOS, GT-DEV-AND, GT-DEV-WIN, GT-DEV-MAC, and phase gates G0–G4. Definitions: [registers.md](registers.md). Per-platform cells: [release-evidence.md](release-evidence.md).

## 6. Known limitations of this baseline

- External facts (Tauri plugin capabilities, OS notification behavior, NLM licensing) are carried from the spec's 2026-11-14 evidence and were **not re-verified** here; Phase 0 tickets re-verify them.
- Sibling-branch observations come from a short read and diff, not from running their tests.
- No stakeholder, device, store, or legal input has been collected.
