# MedTracker execution handoff

Paused on 2026-11-18 at the user's request to control token cost. No final release claim should be made from this state.

## Integrated baseline

The active checkout is on `main`. A 92-file Phase 1 vertical slice from branch
`riogrande100-medtracker-phase1-slice` is staged and uncommitted in the current
checkout. The unrelated untracked `.DS_Store` remains untouched.

The slice includes:

- Tauri 2, React, TypeScript, Vite, locally bundled Geist fonts, Tailwind, and Radix.
- A Rust command/application/storage boundary and SQLite migrations.
- Immutable medication snapshots and manual medication entry.
- A local dose ledger with Taken, explicit Skip, backdating, correction revisions,
  history, undo, medication archive, and medication deletion.
- Accessible Today and History surfaces plus a native-reminder adapter stub.
- An explicit no-runtime-network baseline and local-only UI copy.

Independent evidence on the integrated files:

| Command | Result |
|---|---|
| `npm ci` | Passed; npm reported six high-severity dependency audit findings and one package install-script warning. Audit findings were not yet triaged. |
| `npm run typecheck` | Passed. |
| `npm test` | Passed: 8/8 UI tests. |
| `npm run build` | Passed; Geist assets were bundled. |
| `cargo test` from `src-tauri` | Passed: 10/10 Rust tests. |

Phase 1 worker commit `9533022` was used only as an integration source. Its files
are uncommitted in the active checkout.

## Independent architecture review findings

Fix these before building later work on Phase 1:

1. **High:** Backdated/corrected wall times use the offset from the current day,
   not the offset for the entered date. This produces wrong instants and stored
   offsets across DST. Resolve with IANA rules for the specified date; preserve
   the actual offset and add DST tests.
2. **Medium:** Today is not refreshed at midnight, resume/focus, or when an item
   becomes due. Reject stale slot-linked dates in Rust and refresh in the UI.
3. **Medium:** Upgrades lack a pre-migration snapshot, `integrity_check`, and a
   recoverable UI state for migration/newer-database failures.
4. **Medium:** Undo is intentionally soft/reversible but privacy deletion is
   missing. Add a separate per-event deletion action that redacts sensitive
   notes and revision payloads and records a tombstone.

The Phase 1 worker was asked to fix these. Inspect branch
`riogrande100-medtracker-phase1-slice` for any uncommitted follow-up work before
integrating again.

## Other active work at pause

| Session | Branch | Scope | Status at pause |
|---|---|---|---|
| Reconcile MedTracker planning | `riogrande100-reconcile-medtracker-planning` | Reconcile specifications and create backlog/current-state/deferred evidence docs | In progress; `docs/product-spec.md` had an uncommitted edit. |
| Spike MedTracker platforms | `riogrande100-spike-medtracker-platforms` | Reminder, IANA, iOS/Android/Windows/macOS, HealthKit, Health Connect evidence | In progress; no diff was visible when paused. |
| Complete MedTracker MVP | `riogrande100-medtracker-phase2-mvp` | Schedules, DST, reports, export, encrypted backup/restore, settings, reminders seam | Started from Phase 1 commit; no diff was visible when paused. |
| Build MedTracker vertical slice | `riogrande100-medtracker-phase1-slice` | Phase 1 review fixes | Processing the four findings above when paused. |

Each session was instructed to stop and leave a local handoff without further
tool work. Collect those results before doing anything else.

## Acceptance gates not yet satisfied

Do not infer or claim the following without evidence:

- Intended-user usability of Today, Taken/Skip, and Undo on a primary phone.
- Physical-device reminder behavior for iOS or Android.
- Windows or macOS desktop notification behavior.
- Android SDK/NDK build. Initialization failed because `ANDROID_HOME`/SDK/NDK
  was unavailable.
- Signed iOS, Android, Windows, or macOS release artifacts.
- Store distribution, permissions, signing, and package review.
- HealthKit, Health Connect, or Google health go/no-go and native review.
- Catalog redistribution approval/provenance.
- Encrypted backup/restore migration and corruption recovery.
- Full WCAG 2.2 AA, screen-reader, and real-device accessibility checks.
- Full offline network-isolation validation on installed packages.

## Resume sequence

1. Ask the four paused sessions for their saved handoffs; do not rerun work
   already present in their worktrees.
2. Integrate and reconcile the three planning documents, backlog, and deferred
   register.
3. Integrate the Phase 1 review fixes and rerun Rust/UI/typecheck/build checks.
4. Integrate the Phase 2 MVP work after checking for overlap, especially
   `clock.rs`, `ledger.rs`, migrations, DTOs, and the UI API.
5. Review schedules/time correctness, security/privacy, backup/recovery, native
   health, and release readiness separately.
6. Add only evidence-backed platform adapters. Keep logging/history operational
   when notifications or health permissions are denied, removed, or unsupported.
7. Establish CI with a least-cost host baseline, then collect signed artifacts
   and physical-device evidence separately for iOS, Android, Windows, and macOS.

The immediate resume command should be: collect paused child handoffs, then
integrate the Phase 1 review fixes and run the narrow test suite before adding
new scope.
