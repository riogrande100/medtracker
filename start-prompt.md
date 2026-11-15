Act as a senior product architect, Rust/Tauri engineer, and UX designer. Create an implementation-ready product specification and phased engineering roadmap for the application described below.

This is a planning task. Do not generate application code yet. Make concrete recommendations, explain important tradeoffs, and distinguish verified framework capabilities from assumptions that need a technical spike.

## Product concept

An offline-first medication tracking application for mobile and desktop, built with Tauri v2.

The app lets users:

- Find a medication in a bundled catalog or enter one manually.
- Create and edit medication schedules.
- Record doses, skipped doses, notes, and relevant user-defined observations.
- View clear charts, trends, and historical reports.
- Export their records for personal use or sharing.

The motivation is to make medication history more transparent and flexible than existing tracking experiences, especially when users travel or change time zones.

This is a logging, scheduling, and reporting tool—not a medical-advice tool. It must not recommend doses, suggest treatment changes, diagnose conditions, assess drug interactions, or imply that recorded patterns establish clinical improvement.

## Constraints and preferred stack

- Tauri v2.
- Rust application/domain logic.
- A web-based frontend; recommend a practical UI stack and chart library.
- SQLite as the preferred primary database.
- Consider DuckDB only if a specific demonstrated requirement justifies it.
- All personal data stays on the user's device.
- No accounts, backend, cloud sync, advertising, telemetry, or runtime medication API.
- No remote fonts, scripts, chart assets, or other runtime network dependencies.
- Bundle the medication catalog with the application.
- Update the catalog only through app releases.
- Permit manual medication entry.
- Mobile and desktop are both product goals, but do not assume every platform must ship in the first release.
- Recommend a platform rollout order and explain it.
- Favor a small, maintainable architecture suitable for an initial solo-developer implementation.
- Do not invent app-size, performance, or delivery-time guarantees.

Clarify the boundary between “works completely offline” and distribution through app stores or user-initiated application updates. Do not assume an in-app updater is required.

## Core product principles

1. Historical records must not silently change meaning when the device time zone changes.
2. A reminder is not evidence that a dose was taken.
3. “Not logged” and “explicitly skipped” are different states.
4. Users must be able to correct mistakes without losing the ability to understand what changed.
5. Changes to a medication catalog must not rewrite personal medication history.
6. Charts must be descriptive, with clear definitions and visible reporting context.
7. Data portability and recovery are first-class requirements.
8. Complexity must be justified by a concrete user need.

## Time and scheduling requirements

Treat time handling as a core domain, not a formatting detail.

Separate:

- The actual instant the user says a medication was taken.
- The instant the record was entered.
- The original local-time and time-zone context.
- The scheduled occurrence, if any.
- The schedule version that applied.

Evaluate a model that preserves UTC instants, IANA time-zone identifiers, and the original UTC offset where appropriate.

Explicitly distinguish schedule semantics:

- A fixed local clock time that follows the user's current time zone.
- A fixed local clock time in a selected home time zone.
- A fixed elapsed interval anchored to a specified instant.

Do not assume these schedules are interchangeable.

Define proposed behavior for:

- Travel between time zones.
- Daylight-saving gaps and repeated times.
- International Date Line crossings.
- Manual device-clock changes.
- Backdated entries.
- Pausing, resuming, editing, and ending schedules.
- Schedule changes after future notifications have already been queued.
- Medication discontinuation and later resumption.
- As-needed medication logging without a recurring schedule.
- Multiple events that might relate to one scheduled occurrence.
- Changes to time-zone rules or the bundled time-zone database.

Never automatically advise a user how to adjust medication use during travel. Define software behavior and user-controlled schedule changes only.

Explain which historical values remain immutable and which are calculated for display.

Support viewing history in:

- Original recorded local time.
- Current-device local time.
- A selected reporting time zone.

Define how chart day boundaries and schedule-based reporting work in each case.

## Medication catalog requirements

Evaluate RxTerms/RxNorm-derived data as a starting point for a U.S.-focused release.

Verify, rather than assume:

- Download availability.
- Licensing and redistribution conditions.
- Coverage and limitations.
- Useful identifiers and fields.
- Source release/version information.

Propose:

- A reproducible build-time catalog ingestion pipeline.
- A searchable bundled catalog.
- Separation of catalog data from personal records.
- Preservation of a medication's historical name, strength, form, and units.
- Handling of retired, renamed, merged, or missing catalog entries.
- Manual entries and user-defined display names.
- Clear distinction between product strength and the amount the user records taking.

Do not expand the MVP into a complete drug-information or prescribing system.

## Notification requirements

Verify current Tauri v2 notification capabilities separately for:

- iOS.
- Android.
- Windows.
- macOS.
- Linux.

Do not assume feature parity or that desktop scheduling works like mobile scheduling.

Identify:

- What the official plugin supports.
- What requires native code or another integration.
- Relevant permission and operating-system constraints.
- What happens while the app is closed, terminated, or the device is asleep.
- Restart and reboot behavior.
- Notification scheduling limits where relevant.
- Whether reliable behavior requires platform-specific code.

Never use a foreground JavaScript timer as the sole reminder mechanism.

Separate the schedule engine and dose ledger from notification delivery. Distinguish delivery requests, actual delivery where observable, notification interaction, and explicit dose logging.

Create an early feasibility spike to test reminder behavior before significant UI investment. If a platform cannot satisfy the intended behavior, state the limitation and propose a narrower release scope rather than promising a workaround.

## Privacy, storage, and recovery

Translate “data stays on the device” into explicit requirements.

Address:

- Application database location.
- Database encryption options and tradeoffs.
- Key management and OS secure storage.
- OS backups and cloud-backup inclusion or exclusion.
- Sensitive notification previews.
- Diagnostic logs and crash reporting.
- Temporary files and exported files.
- User-controlled encrypted backups.
- Restore validation and incompatible backup versions.
- Database migrations and recovery from failed upgrades.
- Full data deletion and its practical limitations.
- Separate device histories without automatic synchronization.

Do not claim that local storage, encryption, or the absence of medical advice automatically establishes legal or regulatory compliance.

Briefly flag regulatory, privacy, accessibility, and app-store questions requiring later review. Do not turn this into a legal memorandum.

## Required deliverables

Organize the output into the following sections.

### 1. Executive recommendation

State the recommended architecture, initial platform scope, major tradeoffs, and highest-risk assumptions.

### 2. Product specification

Include:

- Problem statement.
- Target users and primary use cases.
- Product principles.
- MVP scope.
- Explicit non-goals.
- Post-MVP capabilities.
- Functional and nonfunctional requirements.

Use stable requirement IDs so the roadmap and tests can reference them.

### 3. User journeys and information architecture

Describe:

- Onboarding without an account.
- Adding a catalog or manual medication.
- Creating a schedule and selecting time-zone behavior.
- Logging now and backdating a dose.
- Skipping, correcting, or deleting a record.
- Traveling and changing schedule behavior.
- Reviewing history and charts.
- Exporting, backing up, and restoring data.
- Managing notification permissions.

Recommend the minimum necessary screens and navigation structure.

### 4. Domain model and time semantics

Define the entities, states, relationships, invariants, and timestamp semantics.

Explain schedule versioning, scheduled occurrences, dose-event linkage, and correction history. Recommend the simplest model that preserves accuracy; do not assume full event sourcing is necessary.

Provide worked examples of travel and daylight-saving behavior using explicit dates, offsets, and time-zone identifiers. Clearly distinguish examples from medical guidance.

### 5. Data model

Propose a conceptual SQLite schema covering:

- User medications.
- Catalog references and historical snapshots.
- Schedule definitions and versions.
- Scheduled occurrences, if persisted.
- Dose events.
- Notes or observations.
- Correction history.
- Notification bookkeeping.
- Preferences.
- Catalog and schema versions.

Specify:

- Keys and important constraints.
- Timestamp and numeric representations.
- Units and precision.
- Indexes.
- Referential integrity.
- Deletion/archive behavior.
- Transaction boundaries.
- Migration strategy.
- Whether personal and catalog data should use separate database files.

Explain how each major design choice supports a requirement.

### 6. Technical architecture

Recommend:

- Frontend stack.
- Charting approach.
- Rust modules and responsibilities.
- Typed frontend-to-Rust command boundaries.
- Database access strategy.
- Notification integration boundaries.
- Catalog build pipeline.
- Packaging and release approach.
- Dependency-selection criteria.

Keep the architecture local and modular. Do not introduce servers, microservices, or speculative infrastructure.

### 7. Reporting and chart definitions

Define useful initial charts and exact metric meanings.

Address:

- Scheduled occurrences versus recorded outcomes.
- Explicit skips versus missing records.
- Elapsed time between recorded doses.
- As-needed medications.
- Partial or multiple logged amounts.
- Schedule edits.
- Reporting-zone day boundaries.
- Missing data.
- User-entered observations without unsupported causal claims.

Do not present a generic adherence score without defining its denominator, exclusions, and limitations.

### 8. Notification capability matrix and feasibility spike

Provide a sourced platform matrix.

Define a small prototype, test scenarios, evidence to collect, pass/fail criteria, and decisions triggered by failure.

Make this an early roadmap gate.

### 9. Security, privacy, and recovery design

Provide a proportionate threat model and concrete controls. Distinguish protection against a lost device, casual local access, malicious local software, and intentionally exported data.

State what the app can and cannot guarantee.

### 10. Test and acceptance plan

Include:

- Unit tests.
- Database integration tests.
- Migration and restore tests.
- Time-zone and daylight-saving tests.
- Notification/device tests.
- Accessibility tests.
- Offline/network-isolation tests.
- Cross-platform UI smoke tests.
- Release-build size and performance measurements.

Give specific Given/When/Then acceptance criteria for the most important requirements.

Include a proposed representative test dataset and explain how expected results are established.

### 11. Phased roadmap

Break the work into:

- Phase 0: feasibility and risk reduction.
- Phase 1: smallest useful vertical slice.
- Phase 2: MVP completion.
- Phase 3: hardening and first release.
- Phase 4: additional platforms and improvements.

For every phase include:

- Objective.
- Deliverables.
- Requirement IDs.
- Dependencies.
- Major tasks.
- Acceptance criteria.
- Risks and mitigations.
- Exit gate.
- What is deliberately deferred.

Use relative effort estimates, not invented calendar promises. Identify the critical path.

### 12. Initial implementation backlog

Produce a prioritized backlog of concrete engineering tickets.

Each ticket should include:

- ID and title.
- Purpose.
- Scope.
- Dependencies.
- Acceptance criteria.
- Relative effort.
- Associated product requirements.

Make the first tickets actionable without beginning implementation.

### 13. Decisions and open questions

Separate:

- Recommended decisions.
- Decisions dependent on a prototype.
- Decisions requiring product input.
- Decisions requiring specialist review.

End with no more than five high-impact questions whose answers would materially change the plan.

## Evidence and planning standards

- Research current official documentation where available.
- Cite sources for framework capabilities, operating-system constraints, and medication-data licensing or distribution claims.
- Do not cite sources for your own proposed product decisions as if those sources mandate them.
- Clearly label facts, recommendations, assumptions, and unresolved risks.
- When documentation does not establish a capability, say so.
- Do not assume all platforms support identical behavior.
- Prefer a useful, testable MVP over a large feature list.
- Avoid generic boilerplate and unnecessary architectural patterns.
- Proceed with explicitly stated provisional assumptions instead of blocking the whole specification on unanswered questions.
- Keep the specification and roadmap internally consistent.
- Do not generate application code yet.
