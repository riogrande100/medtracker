Act as a senior product architect, Rust/Tauri engineer, and UX designer. Create or revise an implementation-ready product specification and phased engineering roadmap for the application described below.

**Scope amendment (2026-11-19, owner decision D-29):** the first release is mobile only (iOS + Android). Windows and macOS desktop are deferred, not removed; the original definition of done (four signed builds) is amended accordingly for the first release. Where this brief lists four targets, read it as the long-term intent; `docs/product-spec.md` governs.

**Status of this brief:** the merged specification in `docs/product-spec.md` is authoritative and supersedes this brief wherever they differ; read `docs/README.md` and `docs/current-state.md` first. A Phase 1 vertical slice already exists in the repository and is not accepted evidence. Reuse the stable requirement IDs (PR-xx), decision IDs (D-xx), gate names (G0-G4, GT-*), and ticket IDs (Bx-xx) already defined in `docs/`; do not invent new ones that collide.

This brief is for the planning task: do not generate application code when asked only to plan or revise documents. Implementation work follows `docs/backlog.md`. Make concrete recommendations, explain important tradeoffs, and distinguish verified framework capabilities from assumptions that need a technical spike.

## Product concept

A user-friendly medication tracking application for iOS and Android first (Windows and macOS deferred, D-29), built with Tauri 2, a Rust core, and a React/TypeScript/Vite frontend. The local SQLite ledger is the source of truth.

The app lets users:

- Find a medication in a bundled catalog or enter one manually.
- Create and edit medication schedules.
- Record doses, skipped doses, notes, and relevant user-defined observations.
- View clear charts, trends, and historical reports.
- Export their records for personal use or sharing.

The primary motivation is to make seeing a due medication and recording that it was taken easy for a non-technical user, especially the product owner's wife. Preserve transparent history and flexible time-zone behavior without allowing setup and reporting complexity to obstruct routine use.

This is a logging, scheduling, and reporting tool—not a medical-advice tool. It must not recommend doses, suggest treatment changes, diagnose conditions, assess drug interactions, or imply that recorded patterns establish clinical improvement.

## Constraints and preferred stack

- Tauri 2.
- Rust application/domain logic.
- A React/TypeScript/Vite frontend; recommend a practical chart library.
- A Geist-inspired visual language with locally bundled fonts and assets.
- SQLite as the preferred primary database.
- Consider DuckDB only if a specific demonstrated requirement justifies it.
- Personal records use a local SQLite primary database. Web access is permitted only for explicit, reviewed features through a documented runtime network allowlist (currently empty) and must never block local logging, correction, or history during an outage or integration failure.
- No Medtracker account, authentication, registration, subscription, advertising, analytics, telemetry, arbitrary cloud sync, dose recommendations, interaction checking, clinical claims, or undisclosed data transfer. A backend must be justified by a concrete integration need.
- Bundle fonts, scripts, styles, and charts locally; restrict network calls to reviewed, user-visible features.
- Bundle the medication catalog with the application.
- Update the catalog only through app releases.
- Permit manual medication entry.
- iOS and Android are the first-release targets; Windows and macOS are deferred (D-29). No target is first by assumption (not iOS-first, not Linux-first). Recommend staged validation and release order from per-platform evidence without treating their notification capabilities as equivalent.
- HealthKit and Android Health Connect / Google health integrations are NOT PURSUED (owner decision D-30, 2026-11-19; the app is standalone). An external platform's acceptance never proves a dose was taken. Faster medication entry comes from a bundled offline catalog (GT-LIC-gated) with manual entry always available.
- Model policy for delegated work: Haiku 5.5 for mechanical and documentation tasks, Sonnet 5.5 for standard implementation, and Sonnet 5.5 with high reasoning for DST/time-zone, backup, security, and architecture.
- Favor a small, maintainable architecture suitable for an initial solo-developer implementation.
- Do not invent app-size, performance, or delivery-time guarantees.

Do not require the product to be completely offline, but keep the MVP's runtime allowlist empty. Document the network allowlist, consent boundaries, failure behavior, and distinction between local logging and online integrations. Do not assume an in-app updater is required.

## Core product principles

1. Historical records must not silently change meaning when the device time zone changes.
2. A reminder is not evidence that a dose was taken.
3. “Not logged” and “explicitly skipped” are different states.
4. Users must be able to correct mistakes without losing the ability to understand what changed.
5. Changes to a medication catalog must not rewrite personal medication history.
6. Charts must be descriptive, with clear definitions and visible reporting context.
7. Data portability and recovery are first-class requirements.
8. Complexity must be justified by a concrete user need.
9. Routine dose logging should be understandable and completable from Today with minimal navigation.

## Time and scheduling requirements

Treat time handling as a core domain, not a formatting detail.

Separate:

- The actual instant the user says a medication was taken.
- The instant the record was entered.
- The original local-time and time-zone context.
- The scheduled occurrence, if any.
- The schedule version that applied.

Evaluate a model that preserves UTC instants, IANA time-zone identifiers, and the original UTC offset where appropriate. Prefer a time-zone engine whose bundled tzdb is current (a Phase 0 spike found `chrono-tz` 0.10.4 bundles stale tzdb 2025b; `jiff` 1.x is the recommended engine) and record the tzdb version per occurrence.

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

Evaluate RxNorm-derived data (and confirm, rather than assume, RxTerms availability) as a starting point for a U.S.-focused release. No catalog may be bundled before the catalog licensing gate (GT-LIC) passes.

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
- Windows (deferred, D-29).
- macOS (deferred, D-29).

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
- Network resilience, allowlist, consent, and timeout tests (health-integration tests not applicable, D-30).
- Cross-platform UI smoke tests.
- Release-build size and performance measurements.

Give specific Given/When/Then acceptance criteria for the most important requirements.

Include a proposed representative test dataset and explain how expected results are established.

### 11. Phased roadmap

Break the work into:

- Phase 0: feasibility and risk reduction.
- Phase 1: smallest useful vertical slice.
- Phase 2: complete MVP.
- Phase 3: native integrations, hardening, and first release.
- Phase 4: cross-platform parity and improvements.

These are the only delivery phases. Product-owner work areas are workstreams (WS-1 to WS-9) mapped into Phases 0-4 in `docs/product-spec.md`. Evidence-dependent gates (devices, usability, licensing, signing, stores) stay NOT PASSED until an artifact exists.

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
- Do not generate application code when the task is documentation or planning only.
