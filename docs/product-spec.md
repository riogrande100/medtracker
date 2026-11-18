# Medtracker Product Specification and Engineering Roadmap

**Status:** Proposed; reconciled 2026-11-18 against the latest product direction (see [Authoritative product direction](#authoritative-product-direction)). This document supersedes the earlier "Amendment A" draft and the user-friendly cross-platform revision, both merged here (the draft is kept at [planning-draft/product-spec.amendment-a.md](planning-draft/product-spec.amendment-a.md) for history only). A Phase 1 vertical slice exists in the repository but is not accepted evidence (see [current-state.md](current-state.md)).

**Evidence checked:** 2026-11-14 (external facts were not re-verified during reconciliation; Phase 0 tickets re-verify them)
**Scope assumption:** U.S.-focused personal medication companion; one local data set per device; **iOS, Android, Windows, and macOS are the four primary targets**. No target is ranked ahead of another by assumption; each ships only when its own evidence matrix ([release-evidence.md](release-evidence.md)) is complete. Linux is not a primary target and gates nothing.

**Companion documents:** [README.md](README.md) (index) · [current-state.md](current-state.md) (baseline) · [backlog.md](backlog.md) (tickets; source of truth for ticket detail) · [registers.md](registers.md) (capabilities, deferred/excluded features, decisions, gates) · [ux-journeys.md](ux-journeys.md) (personas, journeys, Mermaid flows) · [release-evidence.md](release-evidence.md) (per-platform evidence matrices).

## Authoritative product direction

This section overrides any conflicting text in this document, `start-prompt.md`, or `product-owner-prompt.md`. If a residual conflict is found, this section wins.

1. **Primary experience:** a non-technical user opens **Today**, identifies the medication that is due, records **Taken** or **Skip**, and can **Undo** an accidental action (PR-19–PR-21).
2. **Stack:** Tauri 2, a Rust core (domain, time, storage), and a React/TypeScript/Vite frontend, built once as a shared codebase.
3. **Four primary targets:** iOS, Android, Windows, macOS (PR-24). No target is ranked first (not iOS-first, not Linux-first); per-platform release readiness is evidence-driven. Linux is non-primary and gates nothing.
4. **Local-first:** the local SQLite ledger is the source of truth. Connectivity loss or an integration failure must never block local logging, correction, or history (PR-01).
5. **Excluded:** accounts, authentication, subscriptions, arbitrary cloud sync, ads, analytics, telemetry, dose recommendations, interaction checking, and clinical claims (PR-22).
6. **Network:** web access is allowed only for reviewed features, through an explicit runtime network allowlist (PR-23, §9). The allowlist is **currently empty**; the MVP ships with no runtime network feature.
7. **Health platforms:** HealthKit (iOS) and Android/Google health APIs are optional, separately authorized feasibility work that must not block the MVP; an external platform's acceptance of a record never proves that a dose was taken (PR-28).
8. **UI:** Geist-inspired visual language with locally bundled fonts and assets (PR-17, D-14).
9. **No mockup-approval gate blocks Phase 1 engineering.** Design review runs in parallel; stakeholder usability evidence is a Phase 3 release gate, not a Phase 1 start condition.
10. **Phases 0–4 are the only delivery phases.** The nine "Phase 1–9" headings formerly in `product-owner-prompt.md` are re-expressed as workstreams WS-1–WS-9 mapped onto Phases 0–4 ([Workstream mapping](#workstream-mapping)).
11. **Model policy:** Haiku 5.5 for mechanical/documentation work, Sonnet 5.5 for standard implementation, and Sonnet 5.5 with high reasoning for DST/time-zone, backup, security, and architecture work (governance rule in [backlog.md](backlog.md#governance-rules)).
12. **Gates that depend on physical devices, stakeholder usability, licensing, signing, or stores are _NOT PASSED — evidence-dependent_** until an evidence artifact exists. Nothing in this repository currently constitutes such evidence.

## Evidence labels

- **Verified** means the statement is supported by a cited official source.
- **Recommendation** means a product or engineering decision proposed here, not a framework mandate.
- **Spike** means public documentation does not settle a material implementation or reliability question.
- **Assumption** means a provisional product choice that should be revisited if user research contradicts it.

## 1. Executive recommendation

Build a local-first Tauri v2 app with a TypeScript/React frontend, a small Rust domain/application layer, and a private SQLite database. Keep the medication catalog in a separately versioned, read-only bundled SQLite file. Use SQLite for the ledger, schedule calculations, filters, and charts; do not add DuckDB unless representative workloads demonstrate a need that SQLite cannot meet.

The local SQLite ledger is the source of truth for what the user recorded; the schedule engine is the source of truth for expected occurrences. Treat native notifications as best-effort delivery requests only. A reminder, notification tap, or notification dismissal must never write a taken-dose event. Target iOS, Android, Windows, and macOS from one shared codebase. The Today core loop (open → see what is due → Taken/Skip → Undo) depends only on the schedule engine and ledger, so it is identical on all four targets and never depends on notification delivery. Do not promise reminder reliability on any target until the Tauri plugin and native OS behavior pass that target's Phase 0 spike. iOS and Android need physical-device runs, with a separate Android exact-alarm and reboot assessment. Desktop reminders are not promised in the MVP: the official Tauri notification plugin schedules only on mobile and shows desktop notifications immediately; platform-native Windows/macOS scheduling requires different integrations that remain a spike.[^tauri-notifications]

Adopt immutable schedule versions and persisted occurrence snapshots. Store UTC instants, IANA zone identifiers, original local wall time, original offset, entry time, schedule-version reference, and time-zone-database version. Keep corrections understandable through a compact revision history rather than full event sourcing. Show history in original, current-device, or selected reporting-zone time without rewriting stored facts.

For the catalog, investigate the current RxNorm Current Prescribable Content monthly release first. NLM lists it as downloadable without a UMLS license, while the full release requires one; however, NLM's broader redistribution terms and the source-vocabulary restrictions still require a release-specific review before bundling. RxTerms is not a verified dependency for the MVP: its current downloadable format and redistribution terms were not established from the current NLM release pages. Do not ship a catalog until provenance and redistribution are approved; this is the **catalog licensing gate (GT-LIC, NOT PASSED — evidence-dependent)** defined in §6. Manual entry is a complete fallback and Phase 1 does not depend on the catalog.[^rxnorm-files][^rxnorm-terms][^umls-license]

The highest risks are:

1. Whether the chosen catalog subset can lawfully be redistributed in a packaged app, including in its derived searchable form.
2. Whether reminders are sufficiently dependable under app termination, device sleep/reboot, permissions, travel, and time-zone changes, evaluated per target (iOS, Android, Windows, macOS).
3. Whether a non-technical user can complete the Today → Taken/Skip → Undo loop unaided (stakeholder-usability gate GT-UX, NOT PASSED — evidence-dependent).
4. Whether a bundled IANA time-zone database and the device's reported current zone can be kept consistent across targets. A Phase 0 spike found that `chrono-tz` 0.10.4 bundles tzdb 2025b, which is stale against the latest IANA release (2026f); `jiff` 1.x with its bundled tzdb is the recommended engine (D-04a, §6 Time engine).
5. Whether users expect encrypted app databases, OS-backup exclusion, or simple per-device storage; each changes recovery and key-management work.

Medtracker is local-first, not a networked service. The core loop (Today, Taken/Skip/Undo, logging, correction, history) depends only on the local ledger and schedule engine and must keep working when connectivity is absent or an integration fails. Web access is allowed only for explicitly reviewed features, each listed in the runtime network allowlist (§9) with a purpose, data fields, and consent boundary. **The allowlist is currently empty:** the MVP makes no runtime network request, and there is no runtime medication API, account, cloud, telemetry, remote font, or remote script. Obtaining the installer or an app-store update is a separate distribution flow, not an app network feature. Optional health-platform integration (HealthKit, Android/Google health) is separate feasibility work (PR-28, B0-09) that cannot block the MVP and, if ever adopted, needs its own review before it ships.

## 2. Product specification

### Problem statement

People who want a personal medication history need a trustworthy way to record what they say they took, skipped, or observed, and to review that history across dates and time zones. Existing reminder behavior can be easy to confuse with evidence of a dose. Medtracker should make the distinction explicit and keep a user's historical records stable when a schedule, catalog entry, clock, or time zone changes.

### Target users and primary use cases

- **Primary:** a non-technical person managing their own medications who needs to open the app, see what is due, record Taken or Skip in as few steps as possible, and undo an accidental tap.
- A person maintaining their own medication log, including while traveling.
- A person who wants to record doses, explicit skips, notes, or observations and later inspect or export the raw history.
- A person who wants reminders but understands that notifications can be denied, delayed, suppressed, or missed.

**Not in the initial scope:** multi-person/caregiver records, shared accounts, clinician workflows, administrator roles (there are no accounts to administer), pharmacy or prescription management, or health-platform import/export (optional, separately authorized; PR-28). Caregiver, family, and provider personas are documented in [ux-journeys.md](ux-journeys.md) as deferred, not designed for.

### Product principles

1. A recorded event is a user's statement, not an independently verified clinical fact.
2. A missing log is “not logged,” not “missed” or “skipped.”
3. A reminder is not a dose record.
4. Corrections retain a readable audit trail unless the user chooses the separate privacy-oriented delete action.
5. Catalog updates never overwrite a user's medication snapshot.
6. Charts describe recorded data and expose their time zone, date range, denominator, and exclusions.
7. Local data, export, backup, restore, and deletion are first-class product surfaces.
8. No feature should infer safe dosage, treatment effectiveness, or clinical causation.

### MVP scope

- One local profile per installation, with no account, authentication, subscription, or automatic synchronization.
- **Today** as the launch screen: shows what is due now, what is upcoming today, and what is already recorded; offers one-step **Taken** and **Skip** on each due occurrence, an equally prominent **Undo** of the latest action, and **Log as needed** for as-needed medications (PR-19–PR-21).
- Add a medication from an approved bundled U.S. catalog or enter one manually.
- Store a historical medication snapshot: selected display name, strength, form, units, optional catalog identifier/release, and optional user display name.
- Create, pause, resume, edit, and discontinue versioned schedules. MVP recurrence choices: selected weekdays at one or more wall-clock times, plus fixed elapsed intervals anchored to a user-confirmed instant. The minimum schedule needed for Today (daily/weekday wall-clock slots, version 1) lands in Phase 1; edit/pause/resume versioning and elapsed intervals complete in Phase 2.
- Log a dose now or backdated; explicitly skip a scheduled occurrence; add a note or user-defined observation; correct, undo, or delete an event (undo and correction keep an audit trail; delete is the separate privacy action).
- Review a chronological ledger and descriptive scheduled-occurrence summary.
- Export a user-selected CSV/JSON report; create and restore a versioned, encrypted backup after the recovery spike.
- Show notification permission state and a generic, privacy-preserving reminder where the target platform passes the feasibility gate.
- The core loop, history, reports, export, backup, and restore work with no network connectivity; the MVP makes no runtime network request (empty allowlist).
- Responsive touch-first UI (Geist-inspired, locally bundled fonts) that adapts to phone, tablet, and desktop windows on all four primary targets; localization-ready date, number, and unit display.
- User-selectable accent color (PR-26, lowest priority; never encodes state by color alone).

### Explicit non-goals

- Dose recommendations, dose calculations, interaction checking, diagnoses, treatment changes, contraindication screening, adherence coaching, risk alerts, or clinical-improvement claims.
- Prescription creation/refill, medication identification by image, barcode scanning, pill recognition, or comprehensive drug information.
- Accounts, authentication/login/registration, subscriptions or payments, backend, cloud or arbitrary synchronization, advertising, analytics, telemetry, crash upload, remote medication API, or automatic cross-device restore (PR-22). Backup/restore is a local, user-initiated file operation, not sync.
- Blocking external integrations: no feature or release gate may depend on a third-party service or health platform being reachable at runtime, and no integration may be the source of truth for a dose.
- Linux as a primary target, Linux-first development order, or Linux-only CI gating.
- A generic scheduling engine, arbitrary RRULE editor, or user-programmable workflows.
- Guaranteeing notification delivery, including during force-stop, reboot, OS focus modes, permission denial, or battery restrictions.
- Clinical records, clinician access, or claims of regulatory or privacy compliance.

### Post-MVP capabilities

- Optional HealthKit / Android-Google health integration, only after B0-09 feasibility, separate authorization, and review (PR-28; DF-08).
- Any primary target not yet release-ready at first release is completed in Phase 4 as an independent device history; no platform is assumed to ship first.
- Linux, only if separately requested (non-primary).
- Additional chart filters and user-defined observation visualizations.
- Optional encrypted user-controlled import/export workflows and more detailed audit browsing.
- Additional schedule patterns only where user research demonstrates need.
- Platform-specific notification enhancements only after separate validation.

### Functional and nonfunctional requirements

| ID | Requirement |
|---|---|
| PR-01 | Core local logging, correction, history, schedule calculation, charts, export, backup, and restore shall never require network access, and shall remain fully usable when connectivity is lost or an integration fails. Every network destination shall have a documented purpose and consent boundary (PR-23); no personal data shall be sent to an unapproved service or used for advertising or behavioral analytics. |
| PR-02 | The app shall support catalog selection and manual medication entry. Immutable medication versions shall preserve historical names, strength, form, units, and catalog provenance across user edits, catalog releases, or item retirement. |
| PR-03 | A user shall be able to create, edit, pause, resume, and end a schedule using one of the explicitly distinct schedule semantics in §4. |
| PR-04 | A dose event shall store occurrence time separately from entry time and shall preserve the original local-time, IANA-zone, and UTC-offset context when available. |
| PR-05 | Taken, explicitly skipped, corrected/deleted, and not-logged states shall remain distinguishable. A notification action shall never count as a dose. |
| PR-06 | Corrections shall retain a readable change record; privacy deletion shall be an explicit, separately explained action. |
| PR-07 | Schedule versions and materialized occurrence snapshots shall prevent later schedule edits or time-zone changes from silently rewriting history. |
| PR-08 | Every schedule occurrence shall be reportable as upcoming, logged, explicitly skipped, not logged, or conflicting entries; no state shall be inferred from notification delivery. |
| PR-09 | History and charts shall identify the reporting time zone and define their date boundaries and denominators. |
| PR-10 | The user shall be able to export records and create, validate, and restore a portable backup without a service account. |
| PR-11 | Personal records shall live in the platform's per-app data area, separate from the installed application bundle and the read-only catalog. |
| PR-12 | App and catalog schema versions shall be explicit. A failed migration shall not leave the only user database unusable. |
| PR-13 | Notifications shall be optional and permission-aware, and shall be scheduled by a native/platform integration, never by a foreground JavaScript timer alone. |
| PR-14 | The initial architecture shall remain one application with local modules and one primary database; DuckDB, services, and speculative infrastructure are excluded absent measured need. |
| PR-15 | The interface shall support keyboard, touch, screen readers, text scaling, and non-color state cues; every chart shall have an equivalent textual/table representation. |
| PR-16 | Release order among the four primary targets shall be decided only from collected per-platform evidence (build, physical-device, notification, signing, store). No platform shall be implied to have feature parity with another, and none is ranked first by assumption. |
| PR-17 | UI fonts (including Geist assets), scripts, styles, charts, and catalog data shall be locally bundled. The app shall not load remote fonts, scripts, stylesheets, or other runtime code; any approved API response shall be typed, bounded, and validated. |
| PR-18 | The app shall not claim legal, privacy, security, or regulatory compliance solely because records are local or encrypted. |
| PR-19 | **Today core loop (Must).** The app shall open to Today. Today shall group the current local day's occurrences as **Due now** (scheduled instant has passed, no active take/skip linked), **Upcoming**, and **Recorded**, and shall show earlier unlogged occurrences as **Earlier, not logged** (never "missed"). The medication name, strength, and scheduled local time shall be legible on each card. With data present, Today shall be usable without opening any other screen. |
| PR-20 | **One-step Taken/Skip (Must).** Each Due now or Upcoming occurrence card shall offer **Taken** and **Skip** directly. A single tap records an event with `occurred_at` = now, linked to that occurrence by that explicit user action; a secondary "at a different time" path covers backdating. As-needed medications offer **Log as needed**. No notification, tap, or dismissal records an event (PR-05). |
| PR-21 | **Undo (Must).** After any Taken/Skip/Log-as-needed action Today shall present an immediately reachable **Undo**, and the same action shall remain available per event from History. Undo voids the event, returns the occurrence to its prior state, and keeps a readable audit entry; it is distinct from privacy delete (PR-06). Changing Taken↔Skip shall be one explicit action, never a silent overwrite. |
| PR-22 | **Excluded scope (Must).** The product shall contain no account, login, registration, subscription/payment, cloud or arbitrary sync, advertising, analytics, telemetry, crash upload, dose recommendation, interaction checking, or clinical claim (diagnosis, dosing advice, efficacy). No release gate shall depend on an external service being reachable at runtime. |
| PR-23 | **Network allowlist (Must).** Web access is permitted only for reviewed features listed in the runtime network allowlist (§9), which is **currently empty**. Any addition requires a decision-register entry, security review, minimum-scope origin, CSP/capability change, updated egress test, and privacy-disclosure update. A connectivity failure on an allowlisted feature shall never block local logging, correction, or history (PR-01). |
| PR-24 | **Four primary targets (Must).** iOS, Android, Windows, and macOS are the primary targets. Each has its own evidence matrix in [release-evidence.md](release-evidence.md); a matrix cell is "passed" only with a linked evidence artifact. Linux is non-primary and gates no release. |
| PR-25 | **Catalog licensing gate (Must for catalog distribution).** No catalog data or derivative shall be bundled in a distributed build until GT-LIC passes (§6). Manual entry shall remain fully functional regardless. |
| PR-26 | **Accent color (Could).** Settings shall let the user pick an accent from a bundled preset list. Accent shall apply to primary CTAs, focus, and selected controls only; destructive actions stay visually distinct; every preset shall meet WCAG 2.2 AA contrast in light and dark; state is never conveyed by color alone. |
| PR-27 | **Usability evidence (Must for release).** Release of a target requires stakeholder usability evidence that representative non-technical participants complete Today → identify due → Taken/Skip → Undo unaided (GT-UX, NOT PASSED — evidence-dependent). The protocol and success threshold are set in B1-10 and approved by the product owner before testing; no threshold is invented here. |
| PR-28 | **Optional health integrations (Could, deferred).** HealthKit and Android/Google health integrations shall be optional, separately authorized, and feasibility-gated (B0-09); they shall never block the MVP, be the source of truth for a dose, or be required for any core flow. Imported data shall retain source/provenance and duplicate status and become a ledger event only after explicit user review; exports shall be explicit; a receiving platform's acceptance shall never be treated as proof that a dose was taken. Failure, revocation, or unavailability shall leave local logging and history fully usable. |

Priority key used in the backlog: **Must** (release-blocking), **Should**, **Could**. Process requirements (review separation, model-cost tiers, CI cost) are governance rules in [backlog.md](backlog.md#governance-rules), not product requirements.

## 3. User journeys and information architecture

### Minimum navigation

Use a responsive shell with four primary destinations:

1. **Today** — the launch screen and primary experience. Sections: Due now, Upcoming, Recorded, Earlier not logged; each occurrence card carries Taken and Skip; a persistent Undo for the latest action; Log as needed; a clear permission/reminder status. Full flows and states are in [ux-journeys.md](ux-journeys.md).
2. **History** — event timeline and filters by medication, event type, range, and reporting zone.
3. **Reports** — descriptive charts, metric definitions, date range, and zone selector.
4. **Medications** — catalog/manual add, medication details, schedules, pause/resume/discontinue.

Settings is a secondary destination reachable from the navigation/header: time display defaults, notification permissions and privacy, (only if PR-28 proceeds) health-integration permissions and status, export/backup/restore, data deletion, catalog/version details, and about/disclaimer. On narrow screens, use a bottom navigation bar; on desktop, use a compact side rail. Keep add/log flows as modal sheets on mobile and dialogs or panels on desktop, with full-screen alternatives for accessibility.

### Journeys

- **First launch:** explain local-first storage and no account (there is no sign-in); disclose that system notifications and OS backups are separate and that the app currently makes no network requests; request no permission until a user enables reminders; offer catalog/manual add. Onboarding is skippable and ends on Today.
- **Today core loop (primary journey):** launch → Today → identify the Due now card → tap **Taken** or **Skip** → see the card move to Recorded with an Undo → optionally Undo. Step-level detail, empty/error states, and Mermaid flows: [ux-journeys.md](ux-journeys.md).
- **Add medication:** search the bundled catalog by normalized/display name, brand, ingredient, strength, or form; show source/release and the distinction between product strength and amount entered per event. If not found, choose manual entry. Store a snapshot and optional custom display name.
- **Create a schedule:** choose “follows this device's current time zone,” “fixed in a selected home time zone,” or “fixed elapsed interval anchored at [instant].” Preview upcoming instants with zone abbreviations/offsets and explain DST policies. The preview is descriptive; it never tells the user to change medication use.
- **Log now:** select the medication, enter an optional amount and unit plus note/observation, confirm the event time in the current zone, and save. No reminder is required.
- **Backdate:** choose date/time and an explicit zone or fixed offset; show the resolved UTC instant and the entered local context before save. `entered_at` remains the current system-reported instant.
- **Skip/correct/undo/delete:** an explicit Skip action creates a skip event tied to an occurrence when selected. **Undo** voids the latest (or any chosen) event, restores the occurrence's prior state, and keeps an audit entry. Correct creates a revision. Delete is distinct, warns that deleted values will not be available in the event audit, and offers a separate permanent data wipe in Settings.
- **Travel:** show active zone mode, detected current IANA zone when available, and upcoming instants in both schedule and current-device zones. Ask the user to choose a schedule version/mode change; never advise how to alter medication timing.
- **Review charts:** show the range, reporting zone, scheduled-occurrence denominator, explicit skips, logged occurrences, missing logs, and conflicting entries. Provide table/export access and metric definitions.
- **Export/backup/restore:** let the user choose destination and format; warn before a plain export leaves the app sandbox. For encrypted backup, collect a passphrase or use a user-controlled key; verify a restore in a staging file before replacing live data.
- **Manage permission:** show app-level state and a link to OS settings when denied. Distinguish “reminder request queued” from “notification permission granted” and “dose logged.” Permission denial never prevents manual logging.

## 4. Domain model and time semantics

### Core entities and invariants

- **UserMedication:** a user's active or archived medication identity with a stable local ID and pointer to its current immutable `MedicationVersion`. Catalog reference is convenience, not the source of historical display.
- **MedicationVersion:** immutable selected name, strength, form, unit, optional catalog identifier/release, and optional user display name effective from an instant. Editing any of those fields creates a new version; prior events and schedule versions retain their original reference.
- **Schedule:** user intent for one medication; may be absent for as-needed medication.
- **ScheduleVersion:** immutable schedule definition with effective bounds, recurrence semantics, zone policy, recurrence parameters, and DST policy. Editing, pausing, resuming, ending, or changing time-zone behavior creates a new version.
- **ScheduledOccurrence:** one materialized expected reminder/logging opportunity under a particular schedule version. It captures intended local date/time, resolved UTC instant, zone, offset, fold, and tzdb version. It is not proof of a dose.
- **DoseEvent:** one user-entered take or skip statement. It stores `occurred_at` and `entered_at` separately and may link to zero or more scheduled occurrences. One occurrence can link to multiple events.
- **DoseEventRevision:** compact before/after record of edits, links, and **undo (void)** actions; not a full event-sourcing log.
- **Observation:** user-defined note/value associated with a medication, dose event, or timestamp. Display it as a user report, never as an inferred effect.
- **NotificationRequest:** bookkeeping for an attempt to ask an OS to display a reminder. It is not a dose event and delivery is `unknown` unless a supported OS callback provides evidence.

Invariants:

1. No scheduled occurrence automatically creates a dose or skip event.
2. No notification delivery, tap, dismissal, or action writes a take event.
3. No event with no linked occurrence is forced into a schedule.
4. Corrections update the current event in one transaction and append a revision in that same transaction.
5. Historical medication and schedule snapshots are not rehydrated from the current catalog or schedule; events reference the medication version in effect when logged.
6. A report never labels an unlogged occurrence “missed” or “nonadherent.”
7. If an occurrence has both active take and skip entries, it is “conflicting entries—review,” not silently resolved.
8. **Undo voids, it does not erase.** An undone event has status `voided`, is excluded from every outcome, chart, and Today state as if absent, and keeps its values in a revision row so the user can still see what changed. Only the separate privacy delete removes values (PR-06, PR-21). A voided event is not restored to active by anything except an explicit user "Redo/restore" in History.
9. **Due is derived, never stored as a fact.** An occurrence is *Due now* on Today when its `scheduled_at_utc` ≤ now, it is `active`, the current local day (in the schedule's applicable zone) contains it, and it has no active linked take/skip. Earlier-day unlogged occurrences show as *Earlier, not logged*. No due window or grace period is invented; if one is wanted it is a product decision (D-12).
10. **Tapping Taken/Skip on an occurrence card is the explicit user link** to that occurrence. It is a ledger action performed in the app, not a notification interaction.

### Timestamp semantics

Capture these values separately:

- `occurred_at_utc`: UTC instant the user says the event happened. For a new “now” entry, initialize from the device wall clock, but make clear it is user/system-reported, not independently verified.
- `entered_at_utc`: UTC instant the record was created on this installation, as reported by the system clock.
- `original_local_datetime`: local date/time as entered or resolved at entry, preserved as text.
- `original_zone_id`: IANA zone when available (for example `America/Los_Angeles`); store the entered fixed offset separately.
- `original_offset_seconds`: signed offset used to resolve the event at entry.
- `schedule_version_id` and occurrence link(s): only when the user explicitly relates the event to an occurrence.
- `tzdb_version`: version used to resolve an occurrence. Pin a bundled IANA database implementation (decision D-04a: `jiff` 1.x with its bundled tzdb) and record the tzdb version it reports.[^iana-tzdb][^time-engine]

`occurred_at_utc`, entry time, original local value, original zone, offset, and version are facts/snapshots and are immutable after initial entry; a correction creates a revision and a new current value. The date/time shown in a selected reporting zone is calculated from the UTC instant and that zone's currently bundled rules. Label it as a display conversion. Do not recompute the preserved original local string. A future timezone-rule update may change future calculations but not stored historical instants or snapshots.

### Schedule semantics

| Mode | Definition | Travel behavior |
|---|---|---|
| `device_local` | User-selected wall-clock time(s) and weekdays in the device's current IANA zone. | On a detected zone change, future occurrences are calculated in the new current zone after the app wakes/reconciles. Already materialized instants remain snapshots; stale native requests are canceled/replaced where possible. If the app is closed during travel, the app cannot promise immediate re-planning. |
| `home_zone` | User-selected wall-clock time(s) and weekdays in one explicitly selected IANA zone. | Travel does not change the schedule zone. Display both home-zone and current-device times. |
| `elapsed_interval` | A fixed duration (stored as integer seconds) from an explicit UTC anchor instant; each next occurrence is `anchor + n × interval`. | Travel changes its displayed local wall time but not its UTC cadence. DST does not add or remove elapsed time. |

These are not interchangeable. The creation flow must label the mode and preview at least the next seven dates/instants before saving. MVP recurrence is daily/selected weekdays and bounded fixed intervals; do not silently reinterpret one mode as another.

### Resolution and lifecycle policies

- **Travel/time-zone change:** preserve all past occurrence snapshots. For `device_local`, detect zone changes on launch/resume and native time-zone-change events where available; cancel queued requests for stale future occurrences; generate new ones. Until reconciliation, a previously queued OS notification can be stale. Show that limitation rather than claiming instant travel awareness.
- **DST gap:** proposed deterministic policy for a wall-clock recurrence: resolve a nonexistent wall time to the first valid instant after the gap and show the actual time/offset in the preview and history as “adjusted for clock change.” No hidden second occurrence.
- **DST repeated time:** select the earlier offset/fold by default and create one occurrence, not two. Preserve the fold/offset in the snapshot. This is a product default to test, not a clinical recommendation.
- **International Date Line:** create at most one occurrence per schedule-version local calendar date and slot key; do not backfill civil dates skipped during travel or create a second occurrence for a repeated local date after flying west. Report the zone/date used. Users can manually backdate a record.
- **Manual device-clock change:** use the system-reported clock for new entry and record entry context; it cannot establish true time. On a detectable jump, show a non-alarming reconciliation notice and recalculate future notifications after confirmation. Never rewrite existing UTC instants. A manually backdated event uses the user's explicit zone/offset and is labeled as entered.
- **Pause/resume/edit/end:** new immutable versions with effective instants; do not generate occurrences during a paused period or retroactively fill them. Resuming starts from an explicit effective instant. A schedule edit affects future occurrences only by default.
- **Queued notifications after schedule change:** persist request IDs/version IDs; cancel requests for future old versions before enqueuing new ones. Record cancellation failures and avoid duplicate request IDs. If a stale notification appears, it can open the schedule but must not create an event.
- **Discontinue/resume medication:** archive the medication by default and end its active schedule version. Later resumption is an explicit new schedule version referencing the existing medication snapshot; never delete prior events.
- **As-needed medication:** allow medication records and unscheduled dose events without recurring schedule or denominator.
- **Several events for one occurrence:** allow multiple take events (e.g., separately logged amounts) or a skip and multiple related records. Count an occurrence once in aggregate charts; if both take and skip records remain active, mark it conflicting until the user corrects/deletes one.
- **Time-zone database change:** store tzdb version per generated occurrence. Recalculate only future, not-yet-fired occurrences under new rules, record supersession, and require explicit zone selection if an identifier disappears. Keep old occurrence instants and original event contexts unchanged.

### Worked examples (software behavior only; not medication guidance)

**Travel, local clock versus home zone versus elapsed interval.** Assume an occurrence at `2026-01-14 08:00 America/Los_Angeles`, offset `-08:00`, is `2026-01-14T16:00:00Z`.

- A `device_local` slot at 08:00 calculated in New York on `2026-01-15` is `2026-01-15 08:00 America/New_York`, offset `-05:00`, or `2026-01-15T13:00:00Z`.
- A `home_zone` slot at 08:00 Los Angeles on that date remains `2026-01-15T16:00:00Z`, displayed in New York as 11:00 EST.
- A 24-hour elapsed interval anchored at `2026-01-14T16:00:00Z` is `2026-01-15T16:00:00Z`, displayed in New York as 11:00 EST.

These are timestamp illustrations only. The app does not evaluate whether any interval or local time is appropriate.

**New York spring-forward.** On `2026-03-08`, the New York clock jumps from `01:59:59 -05:00` to `03:00:00 -04:00`. A wall-clock slot of 02:30 does not exist. Under the gap policy (decision D-29: shift forward by the gap length, the same rule as backdated entries), its one occurrence resolves to `2026-03-08 03:30 America/New_York (-04:00) = 2026-03-08T07:30:00Z`, marked adjusted. The next date's 02:30 is `2026-03-09T06:30:00Z`.

**New York fall-back.** On `2026-11-01`, 01:30 occurs once at `-04:00` and again at `-05:00`. The proposed policy creates one occurrence at the earlier fold, `2026-11-01T05:30:00Z`; the second local 01:30 is not a second occurrence. Show the offset so the user can distinguish the instant.

## 5. Data model

Use two SQLite files: `user-data.sqlite` in the per-app data directory and a bundled read-only `catalog.sqlite` resource. This keeps personal data out of release/catalog migrations and lets a catalog update replace its own version without rewriting the user ledger. Personal medication rows copy the necessary catalog labels and identifiers.

### Conceptual user database

All IDs are random UUIDs stored as text. UTC instants are signed integer Unix microseconds; local dates are ISO `YYYY-MM-DD`; local time is `HH:MM[:SS]`; IANA zones and offsets are text/integer respectively. Do not use SQLite `REAL` for dose quantities.

| Table | Important fields and constraints |
|---|---|
| `app_meta` | Singleton `schema_version`, `catalog_release_seen`, `created_at_utc`, backup-format compatibility metadata. Migration version is also mirrored in SQLite `user_version`. |
| `user_medication` | `id PK`; `current_version_id FK`; `status CHECK(active, archived, deleted)`; create/update/archive instants. |
| `medication_version` | `id PK`, `medication_id FK`, `version_number`, `effective_from_utc`, required historical display name, optional custom display name, nullable catalog RXCUI/release, name/strength/form/product-unit snapshots, source kind `catalog/manual`, create instant. Unique `(medication_id, version_number)`. Immutable after creation. |
| `schedule` | `id PK`, `medication_id FK`, create instant, current version ID, archive flag. Medication deletion is restricted while linked records exist; archive is default. |
| `schedule_version` | `id PK`, `schedule_id FK`, `medication_version_id FK`, monotonic `version_number`, `effective_from_utc`, nullable `effective_until_utc`, `mode CHECK(device_local, home_zone, elapsed_interval)`, `zone_id`, recurrence kind, interval seconds/anchor, DST gap/fold policy, `tzdb_version`, change reason. Unique `(schedule_id, version_number)`; non-overlapping effective ranges validated in transaction. |
| `schedule_slot` | `id PK`, `schedule_version_id FK`, local wall time, weekday mask, stable slot key. Empty weekday mask means all days; no implicit timezone. |
| `scheduled_occurrence` | `id PK`, schedule/version/slot FKs, local date/time, zone ID, offset seconds, fold, resolved UTC instant, tzdb version, generation instant, state `active/superseded/cancelled`. Unique `(schedule_version_id, slot_id, local_date, fold)`; superseding never deletes the original snapshot. |
| `dose_event` | `id PK`, `medication_id FK`, `medication_version_id FK`, `kind CHECK(take, skip)`, `occurred_at_utc`, `entered_at_utc`, original local datetime, zone ID, offset seconds, optional amount mantissa/scale/unit, optional note, current revision number, `status CHECK(active, voided, deleted)` (replaces the former boolean deleted flag; `voided` = undone with audit retained, `deleted` = privacy tombstone). `take` may have no amount; `skip` has no amount. |
| `dose_event_occurrence` | `(dose_event_id, occurrence_id) PK`, linked-at instant, link source `user/system-import`; indexes both FKs. Application rules disallow implicit links and identify conflicting take/skip links. |
| `dose_event_revision` | `id PK`, event ID, revision number, change instant, actor `user`, `kind CHECK(create, correct, void, restore, relink, delete)`, reason, changed-field list, before/after values for corrections and voids. Enforce unique `(dose_event_id, revision_number)`. A delete tombstone records deletion without retaining deleted values. |
| `observation` | `id PK`, medication FK, medication-version FK, optional event FK, occurred/entered UTC instants, original zone/local/offset context, user-defined label, optional exact value/unit, text, revision/deletion fields. No predefined clinical interpretation. |
| `notification_request` | `id PK`, occurrence FK, schedule-version FK, platform request ID, requested/due UTC instants, state `queued/cancel-requested/cancelled/accepted/observed/unknown/error`, last error code, interaction timestamp/action. Never a take/skip row. |
| `user_preference` | Key/value for reporting zone mode, privacy-safe preview, locale, and reminder setting. OS permission remains OS-authoritative and is refreshed, not trusted as permanently cached. |

The read-only catalog file has `catalog_meta(release_id, release_date, source, license/attribution manifest hash)`, `catalog_drug(rxcui, tty, normalized_name, ingredient, strength, dose_form, brand, active_status, source_release)`, and `catalog_synonym(rxcui, term, source, term_type)`, plus an FTS5 index. Import only fields/term types approved by the source license. Do not include NDC, package, classification, or source vocabularies unless a concrete search need and license review justify them.

### Numeric precision, keys, and indexing

- Store amount as integer `mantissa` plus decimal `scale` (e.g. `125` and `2` means 1.25) and a user-visible unit string. Constrain scale to a documented small range, e.g. 0–6. Do not convert amount into active ingredient mass or calculate a dose from catalog strength.
- Product strength stays a historical string/structured snapshot such as `500 mg per tablet`; amount entered stays a separate quantity/unit such as `0.5 tablet`.
- Use `CHECK` constraints for enums, ranges, nullable/required combinations; `UNIQUE` for revision numbers and occurrence identity; `ON DELETE RESTRICT` for records that must be archived rather than cascaded. Explicit privacy wipe uses a transaction and purpose-built deletion path.
- Enable `PRAGMA foreign_keys=ON` for every connection; SQLite does not enforce them unless enabled by the application.[^sqlite-fk]
- Index dose events by `(medication_id, occurred_at_utc)`, `(occurred_at_utc)`, occurrence links by occurrence ID, occurrences by `(schedule_version_id, scheduled_at_utc)`, notification state by `(state, due_at_utc)`, and catalog search terms through FTS5.

### Transactions and lifecycle

- Create/edit a medication and its schedule version in one transaction.
- Save event plus event revision and occurrence links atomically.
- Save schedule version, superseded future occurrences, cancellation requests, and new occurrence generation transactionally; call native notification APIs outside the DB transaction and persist outcomes afterward.
- Never hold a write transaction while awaiting the OS or rendering UI.
- Archive discontinued medications by default. Keep historical events and medication snapshots. An explicit delete warns that associated schedule, event, observation, audit, and notification data will be removed.
- For full wipe, close DB, cancel notifications, checkpoint and close SQLite, remove database and WAL/SHM/sidecars, remove key material, and explain that OS backups, exported copies, flash wear-leveling, and prior filesystem snapshots are outside app control. SQLite deletion is not a forensic-erasure guarantee.

### Migrations and recovery

Use numbered, embedded SQL migrations applied in a transaction with `user_version`; run `foreign_key_check` and `integrity_check` after upgrades. Before a schema-changing upgrade, take a consistent snapshot using SQLite's online backup API or `VACUUM INTO`, not an ad hoc copy of a live WAL database.[^sqlite-backup][^sqlite-wal] If upgrade fails, keep the original and restore from the validated snapshot. Do not auto-downgrade a newer database; give an explicit recovery/export path.

Catalog DB migration is independent: release build generates and validates a new catalog file, and the app swaps only the catalog resource on application update. Do not migrate user data to update catalog labels.

Personal records and catalog should not be in the same DB. The catalog is shipped in the app bundle, treated read-only, and may be replaced by an app release; personal data lives in a writable per-app location and survives app updates. Separate files also reduce accidental catalog-to-personal-data joins and simplify deletion/export.

## 6. Technical architecture

### Frontend and charts

- **Recommendation:** React + TypeScript + Vite, using Tauri v2 APIs and locally bundled CSS/components. This is a familiar, small web stack with explicit TypeScript types and responsive layouts; keep the frontend thin and do not move time or schedule rules into JavaScript.
- **Charts:** Chart.js for the initial bar/line views, bundled locally and configured without remote resources. Supply accessible headings, definitions, screen-reader summaries, and an equivalent data table; do not make canvas the only representation. If a prototype shows poor touch/accessibility or unnecessarily large bundles, use simple inline SVG rather than adopting a larger visualization framework.
- No remote fonts, scripts, chart packages loaded from CDNs, web analytics, or runtime remote styles.
- **Design system (Recommendation):** a small bundled component set and design tokens (spacing, type scale, color roles, focus ring) inspired by the Vercel Geist principles named in the product-owner brief: high readability, generous whitespace, focus-driven layouts, reduced visual noise, fast interaction feedback. Any Geist asset (for example the font) must be bundled locally, and its license verified when adopted (D-14); the design language is a reference, not a dependency that needs the network. Colors are semantic roles so that the accent preset (PR-26) changes only the accent role, never the destructive, success, or warning roles. Primary actions on Today should use large touch targets (recommend at least 44 px; WCAG 2.2 AA's minimum target size is lower, so this is a product recommendation, not a conformance requirement).

### Rust modules and boundaries

Keep one Tauri application. Suggested modules:

```text
domain/          medications, schedules, events, occurrence/outcome rules
time/            IANA zone resolution, DST policy, reporting conversion
application/     validated use cases, transaction boundaries, and the Today read model (derives Due now / Upcoming / Recorded / Earlier-not-logged; PR-19)
storage/         SQLite repositories and migrations
catalog/         read-only search and bundled release metadata
notifications/   platform adapter, queue reconciliation, permission state
export/          CSV/JSON serialization and destination handoff
backup/          snapshot, encryption envelope, validate/restore
commands/         typed Tauri command DTOs and error mapping
```

Tauri commands accept typed input DTOs, validate in Rust, invoke application use cases, and return typed DTOs/errors. Do not expose raw SQL or generic “execute query” commands to the frontend. Return explicit failures (validation, permission denied, storage full, migration failure, backup invalid, notification unsupported) rather than success-shaped defaults.

### Database access

Use `rusqlite` initially with a small repository layer and explicit migrations. A single serialized writer is appropriate for one person's local log; run potentially blocking DB work away from the UI thread, enable foreign keys on connection setup, and keep transactions short. Use SQLite FTS5 for bundled catalog search. SQLite's WAL mode may improve concurrent reads, but it creates sidecar/checkpoint considerations; choose journal mode after mobile/desktop testing, not by default assumption.[^sqlite-wal]

### Time engine

**Decision (D-04a, Recommendation; supersedes the earlier `chrono-tz` suggestion):** use `jiff` 1.x with its bundled IANA time-zone database for deterministic recurrence calculation, an explicit gap/fold resolver, and recording of the tzdb version. This is a library decision, not a device claim.

**Finding that drove the decision:** the Phase 0 spike found that `chrono-tz` 0.10.4 bundles tzdb **2025b**, which is stale (IANA's latest release is **2026f**). A stale bundled database yields wrong offsets for zones whose rules changed after 2025b, silently wrong for DST-sensitive occurrences. `jiff` 1.x with bundled tzdb is recommended instead. Whether `jiff`'s bundled tzdb is current must be re-checked when pinning and the version recorded per occurrence (`tzdb_version`); pinning a crate never substitutes for that check. This finding comes from a repository spike and published crate metadata, not from any device run.[^time-engine]

The library does not identify the device's current zone by itself; a native Tauri bridge/prototype must establish that mapping on each target. Test each target's local-zone identifier, app-resume events, and transitions before deciding whether `device_local` mode can be promised (B0-04). Wall times entered for a date must be resolved with the IANA rules for that date, never with the current-day offset (a defect found in the Phase 1 slice review; see [current-state.md](current-state.md)).

### Notifications

Define `ReminderScheduler` domain port with operations to request/cancel/reconcile an occurrence, query permission status, and capture supported interactions. A native adapter receives immutable occurrence IDs and UTC instants; it cannot write dose events. The schedule engine and ledger remain usable when the adapter is unavailable or permission is denied.

Use the official Tauri notification plugin as the first prototype because it exposes immediate and scheduled mobile notifications. Its current docs state that `Schedule` is mobile-only and desktop notifications are immediate, so desktop reminders need a different integration.[^tauri-notifications] For the first release, cap the queued horizon to a short rolling window chosen by measured per-platform limits; the database schedule remains authoritative. Never enqueue unbounded reminders.

### Catalog pipeline

1. Pin a named monthly NLM release and keep its download/release note, source terms, checksum, release identifier, and ingestion manifest in version control or release metadata.
2. Download only during the maintainer's build process, not at runtime. Verify published checksum where provided.
3. Parse the RRF files deterministically; retain only whitelisted sources/term types and fields needed for display/search.
4. Generate a read-only SQLite DB with a source/release manifest and FTS index. Preserve exact normalized name, brand/ingredient/strength/form labels and source RXCUI, but do not treat them as prescribing data.
5. Run deterministic-output, row-count, duplicate-ID, label, and license/attribution checks; inspect a release-to-release diff for retired/merged identifiers.
6. Bundle the generated catalog in app releases. Catalog changes create no writes to `user_medication`; users may see “catalog entry no longer current” while historical snapshots remain.

**Verified source shape and coverage:** NLM provides downloadable monthly ZIP releases; the RRF files are UTF-8, pipe-delimited text. RxNorm concepts use RXCUIs, individual source/name atoms use RXAUIs, and normalized drug names can express ingredient, strength, and dose form. RxNorm covers U.S. prescription and many OTC drugs, including generic/branded clinical drugs and packs, but is not a complete list of supplements, devices, foods, or every medication-like product. Keep the release ID and source/term type with each catalog row; use RXCUI as a reference, not a timeless substitute for the user's text snapshot.[^rxnorm-overview][^rxnorm-files]

RxNorm says RXCUIs may be merged, split, archived, or retired, while RxNorm-normalized concepts generally persist and history files describe changes. Preserve source release and user snapshot rather than silently remapping a personal medication.[^rxnorm-faq]

#### Catalog licensing gate (GT-LIC — NOT PASSED, evidence-dependent)

No catalog file, fixture derived from real RxNorm rows, or catalog-derived search index may be committed to the repository or shipped in any distributed build until **all** of the following have a written, release-specific artifact (B0-03):

1. Exact source and release identified (for example a named Current Prescribable Content monthly release) with download URL, date, and checksum recorded.
2. Source terms of service and any source-vocabulary restrictions read against the exact fields and term types to be kept; a field/TTY whitelist approved.
3. Redistribution of the **derived bundled SQLite output** in a packaged app, in each target's store, approved by a named human reviewer (specialist/legal review is not an agent decision).
4. Required attribution and "currentness" language drafted and placed in the app's About screen and release notices.
5. Update policy stated: catalog changes only via app releases (no runtime download).

Until GT-LIC passes: the pipeline (B2-01) is built and tested only on **synthetic** fixtures; the catalog search UI is behind a build flag and absent from distributed builds; manual entry is the only add path in any release. RxTerms is not a dependency (its availability and terms were not established).

### Network allowlist (runtime) — currently empty

See §9 for the normative table. The architecture implication: until an entry is approved, no HTTP client, updater, or remote-capable Tauri plugin is added to the application; CSP and Tauri capabilities grant no remote origin; an automated egress check runs in the test suite (B2-12). If an entry is later approved, its network code lives behind an explicit integration command with visible pending/denied/disconnected/retry states, is never on the Today render path, and cannot write a dose event directly.

### Optional health integrations (PR-28; feasibility only)

HealthKit (iOS) and Android Health Connect / Google health APIs, if ever adopted, are optional adapters behind their own port, never the primary database. They require separate user authorization, and a feasibility spike (B0-09) must first establish supported data types, medication-related read/write limits, permissions, OS/version requirements, whether the adapter is on-device (platform API) or needs the network, and whether Tauri needs a native bridge. Imported candidates carry source, timestamp, source record ID, and a user-reviewed duplicate decision before becoming a local event; exported records are only requests, and platform acceptance never proves a dose was taken. No adapter is in the MVP, in the Phase 1 slice, or on any critical path.

### Packaging and release

Build platform artifacts from pinned Rust/Node dependencies, include a software bill of materials and catalog provenance (once GT-LIC passes), and test the actual installed release package on each target (not only a dev build). Keep signing keys outside the repository. App-store delivery, desktop installer download, and optional updater checks are distinct from offline app operation. Tauri's updater is an optional signed-update mechanism with configured endpoints, not a requirement; **the MVP ships without an updater** (D-09) so the allowlist stays empty.[^tauri-updater]

Per-target distribution mechanisms are **candidates to verify in Phase 3**, not decisions: iOS and Android via their respective stores/TestFlight-style beta channels; Windows and macOS via store or direct-download packages with the signing/notarization steps each vendor requires. Signing, notarization, developer-program enrollment, and store review are human-owned, cost-bearing gates (GT-SIGN, GT-STORE: NOT PASSED — evidence-dependent). See [release-evidence.md](release-evidence.md).

**CI cost policy (Recommendation, reversible — D-08):** target-neutral logic (Rust domain/time/storage tests, TypeScript tests, markdown/link checks) runs on the cheapest available runner on every change; this is a convenience for pure logic and does **not** make Linux a target or a gate. Per-target builds (iOS, Android, Windows, macOS) run on demand/pre-release, path-filtered where possible, because mobile and macOS runners are the expensive ones; a target's release evidence must come from a real build and install of that target regardless of where logic tests ran.

### Dependency-selection criteria

Prefer maintained, permissively licensed crates/plugins with iOS, Android, Windows, and macOS support, local bundling, explicit permission behavior, typed APIs, auditable source, and platform-specific tests. Pin versions and lockfiles. Review transitive network behavior and notices. No DuckDB for MVP: SQLite covers the ledger and aggregations without a second engine or larger analytical surface. Add DuckDB only if measured exports or reports exceed SQLite performance/complexity requirements.

## 7. Reporting and chart definitions

Each report shows metric definition, date range, reporting zone, schedule mode(s), excluded/conflicting rows, and whether the value is a count or percentage. No generic adherence score.

1. **Scheduled occurrence outcomes (stacked daily bars):** denominator is all active generated occurrences in the selected schedule versions whose scheduled instants fall in the selected range. Categories are logged, explicitly skipped, not logged, and conflicting. Multiple linked take events count once in the occurrence denominator; they remain separate in the event ledger. A schedule paused/ended before an occurrence contributes no occurrence. An unlogged state is not a missed dose.
2. **Dose-event timeline:** each active take/skip event once at `occurred_at_utc`, with amount/unit only when entered. Supports notes and correction indicators. No dose is inferred from notification activity.
3. **Elapsed time between recorded events:** difference between consecutive active take-event UTC instants for the same medication, optionally excluding events whose occurred time is corrected/uncertain only when user explicitly applies that filter. State that it is descriptive elapsed time, not an appropriate interval recommendation. For multiple entries tied to one scheduled occurrence, keep them distinct and do not combine without an explicit user-defined compatible-unit action.
4. **As-needed medication activity:** event counts/amounts by day or week, without scheduled denominator or adherence percent.
5. **Observation series:** display only user-chosen labels and values; no correlation coefficient or causal language in MVP. If a later descriptive overlay is added, label it as co-occurrence only and provide a non-causality explanation.

**Missing data:** show not logged as its own category. Never fill missing events with zero dose or “missed.” **Partial/multiple amounts:** show each event entry as entered; do not sum incompatible units or infer active ingredient quantity. **Schedule edits:** use the schedule version that generated each occurrence; show version boundaries in the report. **Skip and take conflict:** exclude from logged/skipped totals and expose in a review count until resolved. **Voided (undone) events:** excluded from every chart, total, and elapsed-interval calculation; visible only in the audit view and in exports that explicitly include revision history.

**Date boundaries:** For event charts, group each UTC event by the selected reporting zone's local calendar day. “Original recorded local time” groups using the preserved original local date, not a recalculation. “Current-device local time” converts UTC instants using the current device zone and can change when viewed elsewhere. “Selected time zone” converts UTC instants using a chosen IANA zone and current bundled rules. Label each view. For occurrence charts, provide two explicit grouping options: by each occurrence's stored schedule-local date, or by its scheduled UTC instant projected into the chosen reporting zone. The default is schedule-local date, with the selected zone and schedule mode visible.

Export the table underlying each chart with occurrence IDs, event IDs, schedule-version IDs, original timestamps, reporting zone, metric definition, and filter parameters, subject to privacy choices.

## 8. Notification capability matrix and feasibility spike

### Capability matrix

The table distinguishes the official Tauri plugin's documented behavior from the underlying platform's native APIs. It does not promise identical delivery. Permissions, OS focus settings, battery policy, device sleep, and user settings can suppress or delay display.

| Platform | Official Tauri v2 notification plugin | Native scheduling facts / constraints | MVP stance |
|---|---|---|---|
| iOS | Supports immediate and scheduled notifications; scheduling is mobile-only. Plugin provides permission checks/request and actions. | Apple's UserNotifications framework accepts local calendar/time-interval triggers; the OS handles display when the app is backgrounded/not running, and pending requests can be canceled. User authorization/settings still apply. The plugin docs do not establish reboot, queue-cap, time-zone-travel, or precise-delivery guarantees. | First reminder spike (physical device). Release reminders only with permission/privacy UX and measured reconciliation behavior. No claim of guaranteed delivery. Primary target; core loop ships regardless of the spike outcome. |
| Android | Supports immediate and scheduled mobile notifications. Plugin includes schedule helpers; exact alarm behavior and reboot restoration are not established by its overview. | Android alarms can operate outside app lifetime and while asleep. Exact alarms on Android 12+ require declared special access; Android 13+ notifications need runtime `POST_NOTIFICATIONS` permission. Battery-saving policies can delay inexact alarms. AlarmManager state should not be assumed to survive reboot; verify rescheduling and force-stop behavior on devices.[^android-alarms][^android-permission] | Phase 0 feasibility on a physical device (parallel with iOS), then release per its own evidence matrix. Require explicit permission handling; if exact-alarm access is unavailable, state the weaker behavior or omit reminders. Primary target. |
| Windows | Plugin supports notifications on Windows only for installed apps, but its schedule API is mobile-only, so plugin notifications are immediate. | Windows App SDK supports scheduled app notifications independently of Tauri. Microsoft's docs specify a five-minute delivery window and say a notification is dropped if the computer is off beyond five minutes; a background-task alternative is suggested for stronger delivery but needs a separate integration.[^windows-notifications][^windows-scheduled] | No reminder promise in MVP. Prototype WinRT/Windows App SDK integration before adding scheduled reminders. |
| macOS | Plugin supports immediate notification; plugin scheduling is mobile-only. | Apple UserNotifications supports local scheduled requests. A native Swift/Objective-C bridge, permission UX, bundle identity/signing behavior, app-not-running behavior, sleep/wake, and restart semantics need a macOS spike. | Primary target. In-app Today/Undo only for MVP; no scheduled-reminder promise; native-scheduling spike (B0-06) decides whether any reminder is added later; no timer-based fallback. |
| Linux | Plugin supports immediate notification; plugin scheduling is mobile-only. | Freedesktop notifications describe passive desktop notifications, not a portable scheduler. GNOME documents app-associated notification persistence when the app has a valid desktop file and D-Bus activation, but that is a GNOME/GApplication integration requirement, not proof of a portable Tauri scheduler.[^freedesktop-notifications][^gnome-notifications] | Non-primary; not evaluated and gates nothing. Immediate UI feedback only if ever built; any future scheduled reminders need a target-desktop-specific integration that passes its own spike. |

Sources: [Tauri Notification plugin](https://v2.tauri.app/plugin/notification/); [Apple local notifications](https://developer.apple.com/documentation/usernotifications/scheduling-a-notification-locally-from-your-app.md); [Android alarms](https://developer.android.com/develop/background-work/services/alarms); [Android notification permission](https://developer.android.com/develop/ui/compose/notifications/notification-permission); [Windows app notifications](https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/); [Windows scheduling](https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/app-notifications-scheduled); [freedesktop spec](https://specifications.freedesktop.org/notification/latest/); [GNOME notification guidance](https://developer.gnome.org/documentation/tutorials/notifications.html).

### Notification bookkeeping contract

1. `scheduled` means the domain engine calculated an occurrence.
2. `request_accepted` means the native API accepted a request, if observable.
3. `delivered` is recorded only where the OS provides an explicit reliable callback; otherwise delivery remains unknown.
4. `opened/actioned` records an interaction, not a dose.
5. `taken/skipped` requires an explicit user ledger action in the app.

Use a generic lock-screen message by default (for example, “Medication reminder”). Offer detail previews only through an explicit privacy setting. The default action opens the relevant medication/occurrence. Do not provide “Taken” as a one-tap notification action in MVP, because interaction must not be conflated with a complete ledger event.

### Phase 0 prototype

Build a throwaway vertical prototype before investing in polished UI:

- Tauri v2 minimal app using the official notification plugin on iOS and Android (B0-02); Windows and macOS get a separate immediate-notification plus native-scheduling feasibility spike (B0-06). Record plugin versions and signed-build configuration.
- One native one-shot request, one daily wall-clock request, one cancellation/reschedule after schedule-version change, and an occurrence-ID payload that opens the app without logging a dose.
- Test iOS and Android on physical devices; include locked screen, app foreground/background/terminated, reboot, permission denied/re-enabled, battery saver/Doze, time-zone change while app closed, DST gap/repeated time, clock jump, and schedule edit with old request pending.
- Capture OS/version/device, build/signing mode, permission state, scheduled UTC and local components, API return/error, pending-request list where exposed, manual notification-center observation and timestamp, interaction callback, reboot/resume reconciliation, and duplicates/stale requests.
- On Android separately test exact-alarm permission missing/granted, notification permission missing/granted, Doze, and reboot. Do not assume a plugin option maps to an exact alarm.

**Pass for a platform:** in a controlled physical-device run with permissions allowed and no Focus/Do-Not-Disturb suppression, all planned scenarios produce no duplicate/stale schedule after reconciliation; at least 19 of 20 timed deliveries in each ordinary locked/background scenario appear within five minutes of target; the app correctly reports unsupported/denied state; and boot/time-zone-change behavior is documented. This is a prototype threshold, not a user-facing delivery guarantee.

**Fail or uncertain:** if the platform cannot reschedule after reboot/zone change, has unexplained stale requests, or cannot meet the threshold, remove scheduled reminders from that release or narrow them to a clearly labeled best-effort feature with user consent. Do not replace native scheduling with a foreground JavaScript timer. Desktop (Windows, macOS) reminders require a separate native-scheduling spike (B0-06) before any commitment.

**Physical-device gate (GT-DEV-IOS, GT-DEV-AND — NOT PASSED, evidence-dependent):** the thresholds above are evaluated only from a controlled physical-device run recorded in an evidence artifact (`docs/evidence/`, created by the spike tickets). Simulators/emulators may support development but never satisfy this gate. As of this document no such run exists.

## 9. Security, privacy, and recovery design

### Proportionate threat model

| Threat | Control and limit |
|---|---|
| Lost device | Private app directory; rely on OS device encryption and lock-screen security; exclude database from automatic cloud backup (the product has no cloud feature); generic notification previews. This does not protect an unlocked or compromised device. |
| Casual local access | Use OS app sandbox/data directory, least-privilege Tauri capabilities, no raw SQL command, no personal values in routine logs, optional OS-backed key storage if DB encryption is adopted. Desktop users with access to their own account can often access app files. |
| Malicious local software/rooted device | Do not claim protection. A local attacker with process/filesystem privileges may read memory, export plaintext, or capture keys. Encryption at rest raises the bar for offline file theft but cannot protect data while the app is unlocked. |
| Intentionally exported data | User chooses destination and format; plain CSV/JSON warns that it is unencrypted. Encrypted backup uses authenticated encryption, per-backup random salt/nonce, a password-based KDF with versioned parameters, and integrity validation. Once copied/shared, the app cannot control the copy. |
| Malicious catalog or backup file | Parse with strict size/field bounds; validate release manifest, schema/version, IDs, checksum/authentication, and SQLite integrity in a staging file before activation. Never execute backup contents or dynamic SQL. |
| App update/migration failure | Signed release artifacts; backup snapshot before migration; atomic migration; restore original on failure; no silent downgrade or destructive auto-repair. |

### Storage, encryption, key management

- Put the writable DB in each platform's private per-app data directory, not beside the executable or in a user-visible shared folder. Treat OS-managed backup as a separate transfer path and exclude the DB and keys from automatic cloud backups by default.
- Baseline recommendation: rely on OS device encryption and sandbox controls for MVP; do not claim the SQLite file itself is encrypted. Add a focused SQLCipher evaluation if the lost-device threat model requires DB-level encryption. SQLCipher adds native build compatibility, migration/export implications, key lifecycle, and third-party license review.
- If DB encryption is selected, generate a random per-install key, keep it in OS secure storage (Keychain/Keystore/DPAPI or platform secret service as available), never embed it or store it beside the DB. Make recovery independent of that device key through an explicit user-controlled encrypted backup/passphrase flow. Test key loss and restore before adoption.
- Keep catalog data unencrypted and read-only; it is not user data.
- No medical details in notification titles by default. OS previews and notification history may still retain what was displayed; explain preview settings.

### Backups and restore

- **CSV/JSON export:** human-readable and deliberately unencrypted unless the user chooses an encrypted container; destination picker; include a manifest with export time, zone, schema, catalog release, and metric definitions. Avoid hidden temporary files; clean app temp files after success/failure, while explaining that filesystem deletion is not guaranteed secure erase.
- **Encrypted backup:** snapshot a transactionally consistent DB; include schema and backup-format versions, catalog references/snapshots, schedule and event history, preference policy, and checksum/authentication metadata. Version encryption parameters; use a standard vetted AEAD and KDF implementation rather than custom cryptography.
- **Restore:** verify authentication, format and schema compatibility, SQLite integrity and foreign keys in a staging file; show backup date and counts; take a pre-restore local snapshot; migrate staged data; atomically replace live DB only after validation. Reject newer unsupported formats with an explicit recovery message.
- **No automatic sync:** each installation has its own history. Import/restore is user initiated and may create a copy on another device; warn that this is a transfer, not ongoing synchronization.

### Logs, crash handling, network, deletion

- Disable telemetry, analytics, crash uploads, and automatic diagnostics. Routine logs contain operation/error codes only, not medication names, doses, notes, observations, backup paths, or database content. Any user-initiated diagnostic bundle is previewed and scrubbed before export.
- Runtime network access is permitted only through the allowlist below, which is currently empty. Core functionality is tested with the network blocked; any future allowlisted feature is additionally tested for timeouts, invalid responses, revoked consent, and retries, and must never upload medication data implicitly.
- Permanent wipe cancels native requests, clears user DB and sidecars, removes key material and app-generated temporary files, then confirms completion. Explain it cannot erase OS/user backups, screenshots, shared exports, old filesystem snapshots, storage wear-leveling copies, or data copied elsewhere.

### Runtime network allowlist (PR-23) — currently empty

| # | Destination / endpoint class | Purpose | Status |
|---|---|---|---|
| — | _(no entries)_ | The installed app currently makes no outbound network connection of any kind. | **Empty (current state)** |

Rules:

- The allowlist covers **app runtime** behavior. These are outside it by design and are not app network features: the maintainer's build-time catalog download (only after GT-LIC); the user obtaining the installer or an update through a store or direct download; and user-initiated hand-off of an export/backup file to the OS share sheet or file picker (the OS, not the app, performs any transfer).
- CSP and Tauri capability/permission files grant no remote origin and no HTTP, updater, or remote-capable plugin. The exact CSP form for Tauri's internal IPC/asset origins (which are not network endpoints) is verified in B0-07/B1-01 rather than assumed here.
- Web access is allowed only for reviewed features. Adding an entry requires: a decision-register entry; product-owner and security review; the minimum-scope origin; updated CSP/capabilities; an updated egress test (B2-12); and updated privacy disclosure. Account, analytics, telemetry, crash-upload, sync, ad, and clinical-API endpoints are **never** eligible (PR-22). A health-platform integration that uses only on-device OS APIs is not a network entry but still needs its own PR-28 review.
- Evidence: an egress capture while exercising the full journey set (including Today → Taken/Skip → Undo, export, backup, restore) with a network blocker active shows zero DNS/HTTP attempts. NOT PASSED until captured per target (see [release-evidence.md](release-evidence.md)).

### Privacy, backup, deletion, and recovery summary

| Topic | Baseline behavior (Recommendation unless noted) | Evidence needed |
|---|---|---|
| Storage | `user-data.sqlite` in the OS per-app private data directory; read-only `catalog.sqlite` in the app bundle; no personal data in logs, temp files, or bundle. | Path + permission check per target; log redaction test. |
| Encryption at rest | OS device protection and sandbox for the MVP baseline; SQLCipher only if B0-05 shows the threat model needs it (D-15). Do not claim the DB file is encrypted unless it is. | B0-05 decision record; build test per target if adopted. |
| OS/cloud backup | Exclude DB and key material from automatic OS cloud backup where the platform allows; document what the platform actually does per target. | Per-target backup-behavior test (spike-dependent, not assumed). |
| User backup | Versioned, authenticated-encrypted file chosen/saved by the user; restore validated in staging, pre-restore snapshot, atomic swap. Local file operation, not sync. | Backup/restore test matrix (B2-09, B3-07) per target. |
| Export | CSV/JSON, unencrypted unless wrapped; warn before writing outside the app sandbox. | Export test + warning UX review. |
| Deletion | Archive by default; Undo voids with audit; privacy delete removes values; full wipe cancels reminders, removes DB, sidecars, keys, temp files; states limits (OS backups, exports, snapshots, flash wear-leveling). | Wipe test per target (B3-07). |
| Recovery | Pre-migration snapshot, integrity/foreign-key checks, no silent downgrade, explicit export path if DB is newer/corrupt. | Migration/failure-injection tests (B1-02, B3-07). |
| Notifications | Generic preview by default; detail only by explicit opt-in. | Lock-screen observation per physical device. |
| Compliance | No claim of legal/regulatory compliance (PR-18). | Specialist review (GT-LEGAL, NOT PASSED). |

### Compliance and review flags

Local-only storage and “not medical advice” wording do not establish regulatory, privacy, or app-store compliance. Before release, obtain appropriate review of intended-use claims and marketing, health-data privacy notices, consent/retention, accessibility, app-store category and permission disclosures, medication catalog licensing, and platform backup behavior. Avoid any medical efficacy or safety claim.

## 10. Test and acceptance plan

### Test layers

- **Rust unit tests:** schedule generation and version boundaries; `device_local`, `home_zone`, elapsed interval; DST gap/fold; date-line crossing; occurrence identity; conflicting outcomes; correction history; exact decimal amount encoding; report denominators and range boundaries.
- **SQLite integration tests:** foreign keys on every connection, uniqueness/check constraints, indexes, transactions, concurrent reads/writes, rollback on failure, migration from every supported schema, `integrity_check`, `foreign_key_check`, archive/delete/wipe behavior.
- **Backup/restore tests:** valid encrypted backup; wrong passphrase; truncated/corrupt ciphertext; malformed manifest; unsupported newer version; older version migration; disk-full/atomic replacement failure; restore preserves event and schedule snapshots.
- **Catalog tests:** pinned release manifest/checksum; deterministic ingestion; whitelisted TTY/source coverage; duplicate/retired/merged IDs; FTS matches and offline search; historical snapshots unchanged when replacing catalog.
- **Time tests:** fixed offsets, IANA zones, DST gap/fold, timezone changes, manual clock jumps, International Date Line, backdated local times, tzdb version upgrades, and current-device/home-zone/selected-zone display.
- **Notification/device tests:** Phase 0 matrix on physical iOS/Android; desktop native spikes before adding desktop notifications. Test app closed/terminated distinctions, device sleep/reboot, denied permissions, OS notification disabled, stale cancellation, duplicate prevention, schedule mutation, and “tap does not log.”
- **Accessibility:** keyboard-only desktop navigation (Windows, macOS), VoiceOver (iOS, macOS), TalkBack (Android), a Windows screen reader (for example Narrator), dynamic text/zoom, contrast, focus order, touch target size, chart tables, not-color-only states, reduce-motion. Target: WCAG 2.2 AA; conformance is claimed only with per-target audit evidence.
- **Network isolation and failure:** install with network available, then block all egress and exercise Today → Taken/Skip → Undo, schedules, logging, history, reports, export, backup, and restore (catalog search once GT-LIC passes). Verify no DNS/HTTP request or remote asset is needed, and that the empty allowlist (§9) holds; if an entry is ever added, test timeout, invalid response, revoked consent, and retry separately and verify the integration never blocks or silently duplicates a local dose; distribution/update flows are outside the app and inspected separately.
- **Cross-platform UI smoke:** on each of iOS, Android, Windows, macOS: the release package, clean install, upgrade preserving DB, uninstall/reinstall expectations, app-data path, native permission prompts, and low-storage behavior. A target is not "smoke-tested" without a build of that target.
- **Core-loop end-to-end:** scripted Today → identify due → Taken → Undo, and Today → Skip → Undo, plus change Taken↔Skip, on every target's UI shell; assertions on stored rows, not only on pixels.
- **Size/performance:** record bundle/install size by architecture, cold launch, catalog search latency (median/p95), timeline and chart query latency on the representative fixture, backup/restore time, and memory use. Establish measured baselines and regression budgets; do not promise unmeasured app size, speed, or delivery time.

### Representative deterministic fixture

Use synthetic records only:

- `Daily tablet A`: product snapshot `Example ingredient 5 mg / tablet`, logged amount `1 tablet`, daily 08:00 `America/New_York`, covering 2026-03-07 through 2026-03-10 under the proposed DST gap policy.
- `As-needed liquid B`: product snapshot `Example ingredient 10 mg/mL`, user-entered amounts `2 mL` and `1.5 mL` on distinct instants; no recurring schedule.
- For A, create one take occurrence on March 7, one explicitly skipped occurrence on March 8, no event for March 9, and two take events linked to the March 10 occurrence; include one correction and one observation.
- Expected A occurrence report for these four schedule dates: 4 occurrences total, 2 logged occurrences, 1 explicitly skipped, 1 not logged, 0 conflicts; the two March 10 event rows count once in occurrence totals. The March 8 scheduled instant resolves under policy to `2026-03-08T07:30:00Z`. B contributes two unscheduled events and no scheduled denominator.
- Render the same events grouped in New York, Los Angeles, original recorded local time, and current-device time. Assert the displayed day can change by zone while UTC/original snapshots remain unchanged.

### Given/When/Then acceptance criteria

- **PR-01 network failure:** Given a clean installed app with network egress blocked, when the user searches the catalog, edits a schedule, logs an event, opens reports, and exports, then each succeeds without DNS/HTTP or remote assets.
- **PR-02 snapshot:** Given a catalog medication selection, when a later app release retires or renames that catalog item, then the user's historical selected name, strength, form, units, and RXCUI/release snapshot remain visible and unchanged.
- **PR-02 medication edit:** Given a medication with existing events and a schedule, when the user edits its display name, strength, form, or units, then the app creates a new medication version and prior events/schedule versions continue to reference the earlier snapshot.
- **PR-03/07 schedule history:** Given schedule version 1 with future occurrences, when the user changes its time and saves, then version 1 and past occurrence snapshots remain intact, future occurrences reference version 2, and queued version-1 requests are cancellation-tracked.
- **PR-04 timestamps:** Given a backdated event entered as `2026-01-15T08:00:00-05:00`, when saved, then the app stores `2026-01-15T13:00:00Z`, the original local value/zone/offset, and a separate current `entered_at_utc`.
- **PR-05 not logged vs skip:** Given three schedule occurrences with one take event, one skip event, and no event, when history is opened, then the states are respectively logged, explicitly skipped, and not logged; none is inferred from reminders.
- **PR-05 notification separation:** Given a queued notification that the user opens, when the app handles the action, then a medication screen may open but no dose/skip row is created until an explicit ledger save.
- **PR-06 correction:** Given a saved event, when its time or amount is corrected, then the current value updates and a revision shows old/new values and change time. When the separate delete action is selected, deleted sensitive values are not retained in the event revision payload.
- **DST:** Given a New York 02:30 recurring slot on 2026-03-08, when generated, then exactly one occurrence exists at the first valid time and is visibly marked adjusted. Given the 2026-11-01 repeated 01:30, then exactly one occurrence uses the earlier offset.
- **PR-08 multiple entries:** Given two take events linked to one occurrence, when scheduled outcomes are aggregated, then the occurrence contributes once to logged count and both event rows remain visible. If an active skip is also linked, it becomes conflict/review instead of being silently counted.
- **PR-09 zone views:** Given an event at one UTC instant, when the user switches reporting zones, then the displayed/grouped local day changes as appropriate while `occurred_at_utc` and original local snapshot remain unchanged; the chart visibly names the selected zone.
- **PR-10 restore:** Given a valid encrypted backup and a modified destination DB, when restore is requested, then authentication, version, SQLite integrity, and foreign keys are checked in staging, a pre-restore snapshot is made, and replacement occurs only after success.
- **PR-11 deletion:** Given user data and queued reminders, when permanent wipe is confirmed, then app-generated records, sidecars, keys, and cancellable reminders are removed and the UI states limits on copies outside the app.
- **PR-13 permission:** Given notification permission denied or revoked, when the app opens Today and a schedule exists, then logging/history remain fully available, reminder status is clear, and no delivery is claimed.
- **PR-19 Today grouping:** Given a daily 08:00 schedule in zone Z and a current time of 09:00 local with no events, when Today opens, then the 08:00 occurrence is under Due now showing medication name, strength, and 08:00; the 20:00 occurrence of a second schedule is under Upcoming; and no card uses the words "missed" or "overdue".
- **PR-20 one-step Taken:** Given a Due now card, when the user taps Taken once, then exactly one `take` event exists with `occurred_at` ≈ now, linked to that occurrence, the card moves to Recorded, and no other screen was needed.
- **PR-20 Skip:** Given a Due now card, when the user taps Skip, then one `skip` event is linked, the card shows "Skipped" with text (not color alone), and the occurrence is counted as explicitly skipped, not not-logged.
- **PR-21 Undo:** Given a just-recorded Taken, when the user taps Undo, then the event status is `voided`, a `void` revision keeps the prior values, the occurrence returns to Due now, and reports count it as absent. Given a Taken recorded earlier, when the user undoes it from History, the same result holds.
- **PR-21 Taken↔Skip:** Given a recorded Taken, when the user chooses Change to Skip, then the take is voided and a skip is created in one transaction; at no point are both active.
- **PR-22/23 exclusions and network:** Given a release build with an egress blocker, when the full journey set runs, then zero connection attempts occur; and a repository scan finds no account/login/subscription/analytics code or dependency.
- **PR-25 catalog gate:** Given GT-LIC is not passed, when a distributed build is produced, then it contains no real catalog data and the catalog search UI is absent, while manual entry works.

## 11. Phased roadmap

Effort is relative (`S` small, `M` medium, `L` large, `XL` substantial uncertainty), not a calendar estimate. Phases 0–4 are the only delivery phases. Ticket-level detail (owner/model tier, file ownership, acceptance, tests, review gates, dependencies) is in [backlog.md](backlog.md); this section states objectives, exit gates, and deferrals. **Phases overlap by dependency, not by calendar:** a ticket starts when its own listed dependencies are done, not when the previous phase's exit gate passes, except where a gate is named as a dependency. Design review and usability-protocol work run in parallel with Phase 1 engineering and never block it.

Gate status vocabulary: **NOT PASSED — evidence-dependent** means no artifact exists yet; **PASSED** is allowed only with a link to the evidence artifact. All gates below are NOT PASSED at the time of this document.

### Phase 0 — feasibility and risk reduction

- **Objective:** resolve platform, catalog, timezone, privacy, network, and release risks for all four primary targets before polished UI or reminder promises; produce evidence, not code that ships.
- **Deliverables:** four-target build/install baseline; iOS and Android native notification prototype on physical devices; Windows/macOS notification-feasibility spike; native current-zone discovery spike; DST behavior tests; catalog provenance/redistribution decision; storage/backup/encryption decision; egress-check harness and CSP baseline; optional HealthKit / Android-Google health feasibility note (B0-09, separately authorized, non-blocking); Phase 0 evidence synthesis with a go/no-go per platform.
- **Requirements:** PR-02, PR-03, PR-07, PR-11, PR-13, PR-16, PR-17, PR-18, PR-22, PR-23, PR-24, PR-25, PR-28.
- **Dependencies:** none. Throwaway spike code lives only under `spikes/` and is never imported by product code.
- **Major tasks:** B0-00 … B0-09 in [backlog.md](backlog.md). Execute §8 spike; inspect current NLM release data and license with the source owner/specialist; test `jiff`/tzdb versioning and the local-zone bridge; evaluate per-app backup exclusion; verify Rust/Node/Tauri builds per target; choose the default DB encryption stance.
- **Acceptance:** each primary target has either a build/install evidence record or an explicit "blocked: <host/device/account missing>" record; all scheduled-notification scenarios that were run have collected evidence and a go/no-go decision per platform; the catalog decision is written; timezone gap/fold/zone-change behavior passes deterministic tests; the egress harness exists and shows zero connections for the baseline spike app.
- **Risks/mitigations:** notification or catalog ambiguity blocks feature promises—narrow the release or hold the catalog; native zone ID unavailable—defer `device_local` scheduling and constrain modes; encryption/build complexity—start with OS protections and a separate threat-driven SQLCipher decision; missing hardware (Windows host, Android/iOS devices, developer accounts)—record as a blocker and a human action, never substitute emulators for gate evidence.
- **Exit gate (G0):** product owner accepts, per platform, the reminder scope, catalog provenance status, and privacy/recovery stance based on evidence. **G0 gates reminder adapters (B2-06) and release claims, not Phase 1 engineering.**
- **Deferred:** polished design, broad platform parity, analytics, complex recurrence.

### Phase 1 — smallest useful vertical slice: the Today core loop

- **Objective:** deliver a local-first path on the shared codebase from adding a manual medication and a simple daily schedule, to Today showing what is due, to Taken/Skip, to Undo, plus a readable history — verified on every target that has a Phase 0 build baseline.
- **Deliverables:** Tauri shell with typed commands and empty-allowlist CSP; per-app SQLite DB/migrations; manual medication creation with snapshots; minimal schedule (daily/weekday wall-clock slots, `device_local` and `home_zone`) with occurrence materialization and DST policy; event ledger with explicit occurrence linking, void/undo, and correction history; Today screen; accessible history timeline; design foundation; core-loop automated tests; usability protocol.
- **Requirements:** PR-01, PR-02, PR-03 (create/end), PR-04, PR-05, PR-06, PR-07, PR-08 (partial), PR-11, PR-12, PR-15, PR-17, PR-19, PR-20, PR-21, PR-22, PR-23.
- **Dependencies:** B0-01 (build baseline for the targets being developed on), B0-04 (time engine findings), B0-05 default (OS-protected storage, reversible). Not dependent on the catalog, notification spike, or any mockup approval.
- **Major tasks:** B1-01 … B1-10 in [backlog.md](backlog.md).
- **Acceptance:** Given an installed app with the network blocked, a user can add a manual medication and daily schedule, open Today, see the due occurrence, tap Taken, tap Undo, tap Skip, and change Skip→Taken; restart preserves all values; History shows original/current time context; the PR-19–PR-21 Given/When/Then criteria pass as automated tests; the egress harness shows no connection.
- **Risks/mitigations:** ambiguous Today states—explicit text labels and tests; time bugs—independent time-semantics review; catalog legal gate delayed—manual add is complete; usability unknown—protocol written now, execution is a Phase 3 gate.
- **Exit gate (G1):** all Phase 1 acceptance tests pass on every target that has a build baseline; independent code, time, and UX-heuristic reviews recorded; known target gaps listed as blockers. Physical-device and stakeholder-usability evidence is **not** required for G1 and stays NOT PASSED.
- **Deferred:** charts, scheduled notifications, catalog, schedule edit/pause/resume, elapsed-interval mode, export/backup, accent colors.

### Phase 2 — complete MVP

- **Objective:** complete schedules, outcome reporting, export/backup, optional catalog, and permission-aware reminders on the shared codebase, with the core loop unchanged.
- **Deliverables:** immutable schedule versions with edit/pause/resume/end and elapsed-interval mode; zone-aware outcome reports and charts with table alternatives; CSV/JSON export; encrypted backup/restore; settings/privacy surfaces (permissions, deletion, display zone, about/disclaimer); catalog pipeline on synthetic fixtures (real bundle only after GT-LIC); reminder adapter for each mobile platform that passed G0; accent-color presets; network-isolation test suite.
- **Requirements:** PR-01–PR-15, PR-17, PR-19–PR-23, PR-25, PR-26.
- **Dependencies:** Phase 1 ledger and Today; G0 for reminder adapters; GT-LIC for real catalog data only.
- **Major tasks:** B2-01 … B2-12 in [backlog.md](backlog.md).
- **Acceptance:** fixture expectations in §10 match exactly; restore tests pass; permission denial degrades without blocking logging; no chart conflates missing and skipped; reminder adapter produces no dose event; the egress check passes; the core loop still passes its Phase 1 tests.
- **Risks/mitigations:** native scheduling varies—ship reminders only on platforms that passed; restore/cipher failures—test staged restore and key loss; report ambiguity—publish denominators and table exports; scope creep—accent colors are the first item cut.
- **Exit gate (G2):** requirement-level tests plus independent code/time/security/accessibility-heuristic reviews recorded; product review approves data portability, report definitions, and each platform's reminder statement.
- **Deferred:** automatic sync (excluded), user-configurable arbitrary recurrence, causal observation analysis, desktop scheduled reminders.

### Phase 3 — native integrations, hardening, and first release

- **Objective:** verify native integrations (reminder adapters, signing, notarization, store packaging) on real devices and hosts, and make each primary target releasable, understandable, and supportable. Phase 3 work is done per target; a target ships when **its** evidence matrix in [release-evidence.md](release-evidence.md) is complete, regardless of other targets.
- **Deliverables:** accessibility audit with assistive technology per target; stakeholder usability study; security/privacy review; performance/size baselines; backup/restore/upgrade/deletion recovery tests; signing, notarization, and store proofs-of-concept; release pipeline for targets that qualify; release checklist; legal/intended-use/store-disclosure review.
- **Requirements:** PR-01, PR-02, PR-10–PR-13, PR-15–PR-18, PR-24, PR-25, PR-27.
- **Dependencies:** Phase 2 acceptance; GT-LIC for catalog inclusion; human-owned accounts, devices, and reviewers.
- **Major tasks:** B3-01 … B3-10 in [backlog.md](backlog.md).
- **Acceptance:** on each releasing target: installed release package works with the network blocked; upgrade and restore retain expected records; accessibility and network-isolation suites pass with recorded evidence; physical-device, usability, signing, and store gates each have artifacts; catalog notices ship only if the catalog ships; measured size/performance recorded without unsupported guarantees.
- **Risks/mitigations:** store policy/review or source-data issue—review early and retain manual entry fallback; update fails—rollback snapshot and export path; logs reveal sensitive data—automated redaction tests; human gates slip—release that target later, do not waive the gate.
- **Exit gate (G3, per target):** release checklist complete for that target, all its matrix cells passed with artifacts, product owner sign-off.
- **Deferred:** marketing claims of clinical value, cross-device continuity.

### Phase 4 — cross-platform parity and improvements

- **Objective:** reach cross-platform parity: bring every primary target not yet released to its G3, and add improvements justified by usability or release evidence.
- **Deliverables:** completed per-target adapters, packages, and support matrices; Windows/macOS reminder integration only if B0-06 and a later spike justify it; optional chart/export improvements; localization beyond the initial locale; decision on an optional updater (would add an allowlist entry, so requires D-09 review).
- **Requirements:** PR-01, PR-03, PR-09, PR-13, PR-15–PR-17, PR-24, PR-26.
- **Dependencies:** stable core domain and migrations; per-target Phase 0/notification evidence; no synchronization assumed.
- **Major tasks:** B4-01 … B4-06 in [backlog.md](backlog.md).
- **Acceptance:** each target's release package passes network-isolation, data integrity, accessibility, restore, and target-specific notification criteria; unsupported capabilities are disclosed and not silently emulated.
- **Risks/mitigations:** platform drift and notification parity—adapters and explicit target gates; separate device histories surprise users—make no-sync behavior prominent and provide deliberate transfer via backup file.
- **Exit gate (G4):** each primary target has reached G3 or has a recorded, product-owner-accepted reason it has not.
- **Deferred:** cloud sync, backend, universal reminder guarantees, unvalidated integrations, Linux (non-primary).

### Workstream mapping

The nine "Phase 1–9" headings of the original `product-owner-prompt.md` are not delivery phases. They are workstreams (WS-1…WS-9) that run inside Phases 0–4 (decision D-04). Ticket detail is in [backlog.md](backlog.md).

| Workstream | Former PO "phase" theme | Phase 0 | Phase 1 | Phase 2 | Phase 3 | Phase 4 |
|---|---|---|---|---|---|---|
| WS-1 Product definition and UX | Product vision, user flows, wireframes | B0-08 | B1-10 | B2-08 | B3-02 | B4-04 |
| WS-2 Design system and UI | Geist-inspired design system, mockups | — | B1-07 | B2-05, B2-10 | B3-01 | — |
| WS-3 Architecture and platform shell | Stack, project setup, CI | B0-01, B0-07 | B1-01 | B2-12 | B3-08 | B4-01 |
| WS-4 Data and domain core | Local database, domain model, time rules | B0-04, B0-05 | B1-02, B1-03, B1-04, B1-05 | B2-03, B2-04, B2-11 | B3-07 | B4-03 |
| WS-5 Core experience | Today, Taken/Skip/Undo, History | — | B1-06, B1-08 | B2-05 | — | — |
| WS-6 Reminders and optional integrations | Notifications; optional health platforms | B0-02, B0-06, B0-09 | — | B2-06 | — | B4-02, B4-07 |
| WS-7 Data portability and catalog | Export, backup/restore, catalog | B0-03 | — | B2-01, B2-02, B2-07, B2-09 | — | — |
| WS-8 Quality, accessibility, security | Testing, accessibility, security, privacy | B0-00 | B1-09 | B2-12 | B3-01, B3-03, B3-04 | — |
| WS-9 Release and compliance | Signing, stores, legal, release | — | — | — | B3-05, B3-06, B3-09, B3-10 | B4-05, B4-06 |

Linux-first CI, authentication/registration/subscription flows, and a mockup-approval gate from the old brief are **not** carried into any workstream (PR-22; authoritative direction items 3 and 9).

**Critical path:** (a) Phase 1 core loop: B0-01 build baseline → B1-01 shell → B1-02 DB → B1-04 schedule/occurrence engine (needs B0-04 findings) → B1-05 ledger/undo → B1-06 Today → B1-09 core-loop tests; (b) release path: Phase 2 completion → physical-device, usability, signing, and store evidence per target (human-owned, longest lead time). The catalog (GT-LIC) and notification spikes (B0-02/B0-06) are **off** the Phase 1 critical path but on the release path for the features they gate.

## 12. Initial implementation backlog

The detailed backlog (owner/model tier, file ownership, acceptance criteria, tests, review gates, dependencies) is [backlog.md](backlog.md), which is the source of truth for ticket detail. This index exists so the roadmap and requirement IDs can be cross-checked. Effort: `S` small, `M` medium, `L` large, `XL` high uncertainty.

| Phase | Tickets |
|---|---|
| 0 | B0-00 doc-check tooling · B0-01 four-target build baseline · B0-02 iOS/Android notification spike · B0-03 catalog license decision · B0-04 time engine and zone probe · B0-05 privacy/backup/encryption decision · B0-06 Windows/macOS notification spike · B0-07 egress harness and CSP baseline · B0-08 Phase 0 evidence synthesis (G0) · B0-09 optional health-platform feasibility (non-blocking) |
| 1 | B1-01 shell/typed commands/CSP · B1-02 SQLite and migrations · B1-03 medication snapshots and manual entry · B1-04 schedule v1 and occurrence engine · B1-05 ledger, linking, undo, corrections · B1-06 **Today screen** · B1-07 design foundation · B1-08 History timeline · B1-09 core-loop end-to-end and accessibility harness · B1-10 usability protocol and UX heuristic review |
| 2 | B2-01 catalog pipeline (synthetic) · B2-02 catalog bundle and search UI (GT-LIC) · B2-03 schedule versioning, elapsed interval, reconciliation · B2-04 outcome linkage and report queries · B2-05 Reports UI and charts · B2-06 mobile reminder adapter (G0) · B2-07 CSV/JSON export · B2-08 Settings and privacy surfaces · B2-09 encrypted backup/restore · B2-10 accent presets · B2-11 notes and observations · B2-12 network-isolation suite |
| 3 | B3-01 accessibility audit per target · B3-02 stakeholder usability study (GT-UX) · B3-03 performance/size baselines · B3-04 security/privacy review · B3-05 signing/notarization proof (GT-SIGN) · B3-06 store readiness (GT-STORE) · B3-07 recovery/upgrade/deletion tests · B3-08 release pipeline · B3-09 legal/intended-use review (GT-LEGAL) · B3-10 release checklist and evidence compilation |
| 4 | B4-01 close remaining target gaps · B4-02 desktop reminders (conditional) · B4-03 additional schedule patterns (research-gated) · B4-04 localization · B4-05 optional updater decision · B4-06 Linux evaluation (only if requested; non-primary) · B4-07 optional health integration (conditional on B0-09 and separate authorization) |

Requirement coverage by ticket is listed in [backlog.md](backlog.md#requirement-traceability).

## 13. Decisions and open questions

The full decision register (with IDs D-01…, reversibility, owner, and linked tickets), capability register, excluded/deferred-feature register, and gate register are in [registers.md](registers.md). Summary:

### Recommended decisions

- Use Tauri v2, React/TypeScript/Vite, Rust domain/application logic, `rusqlite`, SQLite FTS5, and Chart.js with an accessible table alternative (Chart.js is the most reversible; B2-05 may swap it for inline SVG).
- Keep user DB and bundled catalog as separate files. Use immutable user snapshots and schedule versions.
- Store UTC microsecond instants plus original local/zone/offset context and a pinned IANA tzdb version. These data-format choices are the hardest to reverse; they are marked as such in the register.
- Local SQLite ledger as source of truth; Tauri 2 + Rust core + React/TypeScript/Vite; four primary targets (iOS, Android, Windows, macOS) from one shared codebase; no platform ranked first; release per platform on evidence. Reminder reliability is not a release claim on any platform until its own spike passes.
- Undo voids with audit (D-10); Due is derived (D-12); no updater in MVP (D-09); time engine is `jiff` 1.x with bundled tzdb (D-04a).
- No DuckDB, backend, accounts, authentication, subscriptions, arbitrary sync, telemetry, runtime medication API, or foreground-timer-only reminders. Web access only via a reviewed allowlist (currently empty).
- Start with OS-protected local storage and an explicitly encrypted user backup; decide on SQLCipher only after threat-model and platform-build assessment.
- Start catalog ingestion from a pinned current prescribable RxNorm release candidate, subject to GT-LIC. Keep manual entry as a complete fallback. Do not assume RxTerms is currently available or redistributable.

### Decisions dependent on a prototype

- Whether the Tauri scheduled notification plugin meets the intended iOS and Android behavior, including timezone trigger semantics, reboot, request capacity, app termination, exactness, and cancellation behavior (B0-02).
- Whether Windows and macOS can support any scheduled reminder through native integrations (B0-06).
- Whether device-current IANA zone ID can be read consistently on all four targets and whether the bundled tzdb/OS zone update lifecycle is acceptable (B0-04).
- Whether SQLCipher builds cleanly across target mobile and desktop artifacts and whether its licensing/key recovery cost is justified (B0-05).
- The exact CSP/capability configuration that yields an empty runtime allowlist (B0-07).
- Whether HealthKit and Android/Google health can represent the intended medication record at all, and whether either needs a network path (B0-09; optional).

### Decisions requiring product input

- Whether the default lock-screen notification should stay generic or allow medication-specific content after explicit opt-in.
- Which MVP dose units and observation inputs are essential; proposed default is free-form user-selected amount/unit with no conversions or clinical interpretation.
- Whether users need a non-encrypted CSV by default or should export only after an explicit sensitivity warning.
- Whether Today should apply any due window or grace period (D-12 default: none).
- The usability success threshold for GT-UX (B1-10 proposes; product owner approves).

### Decisions requiring specialist review

- Catalog terms for the exact release and derived/bundled SQLite output, including source-vocabulary restrictions, attribution, and update-currentness language (GT-LIC).
- Intended-use, marketing language, privacy notice/retention, app-store disclosures, backup exclusions, accessibility, signing/notarization requirements, and any regulatory review needed for each distribution market (GT-LEGAL, GT-SIGN, GT-STORE).

### High-impact questions

1. Who owns, and when can we supply, the evidence-dependent resources: physical iOS and Android devices, a Windows machine, a macOS machine, and developer-program/signing accounts for each store? Without them the corresponding gates stay NOT PASSED.
2. Who are the stakeholder participants for the usability gate (GT-UX), and what counts as success for "non-technical user unaided"?
3. What reminder timing tolerance is acceptable, and should reminders be excluded entirely on targets that cannot pass the feasibility gate?
4. Is a U.S. prescription-focused catalog acceptable, or must the first release cover OTC/supplement/device entries that may require another source or manual records?
5. Is optional health-platform integration (read, write, both, or neither) valuable enough to authorize B0-09?
6. Must the live database itself be encrypted against offline file theft, or is OS device protection plus user-encrypted backups an acceptable MVP boundary?

## Sources

Official/framework and data-source documentation checked on 2026-11-14:

- [Tauri v2 Notification plugin](https://v2.tauri.app/plugin/notification/) and [JavaScript API reference](https://v2.tauri.app/reference/javascript/notification/) — platform support; mobile-only schedule API; desktop immediate behavior.
- [Tauri v2 Updater plugin](https://v2.tauri.app/plugin/updater/) — optional signed updater artifacts and endpoint configuration.
- [Tauri v2 overview](https://v2.tauri.app/) — supported desktop and mobile targets and frontend independence.
- [Apple: Scheduling a notification locally](https://developer.apple.com/documentation/usernotifications/scheduling-a-notification-locally-from-your-app.md) — local request triggers, system delivery, cancellation.
- [Android: Alarms](https://developer.android.com/develop/background-work/services/alarms) and [notification runtime permission](https://developer.android.com/develop/ui/compose/notifications/notification-permission) — alarms outside app lifetime, exact-alarm conditions, Doze and notification permission.
- [Microsoft: App notification overview](https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/) and [schedule an app notification](https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/app-notifications-scheduled) — Windows native support and delivery window/power-off limitation.
- [freedesktop Desktop Notifications Specification](https://specifications.freedesktop.org/notification/latest/) and [GNOME Using Notifications](https://developer.gnome.org/documentation/tutorials/notifications.html) — desktop notification service and GNOME application association requirements.
- [NLM RxNorm Overview](https://www.nlm.nih.gov/research/umls/rxnorm/overview.html), [RxNorm Files/releases](https://www.nlm.nih.gov/research/umls/rxnorm/docs/rxnormfiles.html), [RxNorm Terms of Service](https://www.nlm.nih.gov/research/umls/rxnorm/docs/termsofservice.html), and [UMLS licensing/access](https://www.nlm.nih.gov/databases/umls.html) — scope, current release downloads, full-release licensing, source restrictions, attribution, and redistribution cautions.
- [NLM RxNorm FAQ](https://www.nlm.nih.gov/research/umls/rxnorm/faq.html) — RXCUI merge/retirement behavior and file references.
- [IANA Time Zone Database](https://www.iana.org/time-zones), [`jiff` documentation](https://docs.rs/jiff/latest/jiff/), and [`chrono-tz` documentation](https://docs.rs/chrono-tz/latest/chrono_tz/) — political time-rule updates and bundled IANA data/version.
- [SQLite foreign keys](https://www.sqlite.org/foreignkeys.html), [WAL](https://www.sqlite.org/wal.html), and [online backup API](https://www.sqlite.org/backup.html) — enforcement setup, sidecar/checkpoint considerations, and consistent database snapshots.

[^tauri-notifications]: The Tauri plugin's platform table lists iOS, Android, Windows, macOS, and Linux; its `Schedule` API reference explicitly says scheduling is mobile-only and desktop notifications are immediate.
[^tauri-updater]: Tauri documents an optional updater requiring signed update artifacts and configured endpoints. App-store policy and release processes remain independent of offline runtime behavior.
[^rxnorm-overview]: NLM describes RxNorm as normalized drug names/identifiers covering prescription and many OTC drugs in the U.S.; devices, supplements, foods, bulk powders, contrast media, and homeopathics are out of scope.
[^rxnorm-files]: NLM's current files page lists full RxNorm releases and Current Prescribable Content. As checked 2026-11-14, the current monthly release is dated 2026-11-02; the listing marks Current Prescribable Content as “no license required.” This is a download statement, not a legal determination that every derived redistribution is unrestricted.
[^rxnorm-terms]: NLM says full monthly/weekly RxNorm releases require a free UMLS license, the full dataset includes proprietary source-vocabulary data, product attribution is requested, and republished/redistributed data must be current or clearly identified as not current. Source-vocabulary restrictions may apply.
[^umls-license]: NLM issues UMLS licenses to individuals and directs users to review the license and source-provider restrictions before redistribution in an application.
[^rxnorm-faq]: NLM documents merges, retirement, and history files; retaining local display snapshots avoids retroactive personal-history changes.
[^iana-tzdb]: IANA updates tz data when political bodies change boundaries, UTC offsets, or daylight-saving rules.
[^time-engine]: Phase 0 spike (2026-11): `chrono-tz` 0.10.4 reports bundled tzdb 2025b; IANA's latest release at that time was 2026f. On the host spike, `jiff` 1.0.1 with `jiff-tzdb` 1.0.0 bundled 2026f. Re-verify versions when pinning.
[^sqlite-fk]: SQLite documents that foreign-key enforcement is disabled by default unless the application enables it on each connection.
[^sqlite-backup]: SQLite's online backup API creates a consistent snapshot without relying on copying a live database file.
[^sqlite-wal]: SQLite WAL adds `-wal`/`-shm` sidecars and checkpoints; these affect copying and recovery decisions.
[^android-alarms]: Android documents alarms outside application lifetime and exact-alarm restrictions; delivery can be affected by battery-saving policies.
[^android-permission]: Android 13+ requires runtime notification permission for non-exempt notifications.
[^windows-notifications]: Windows App SDK supports local app notifications, but Tauri plugin coverage does not itself establish scheduled-notification integration.
[^windows-scheduled]: Microsoft documents a five-minute scheduled-notification window and dropping notifications when the computer is off beyond that window.
[^freedesktop-notifications]: The freedesktop standard specifies passive desktop notifications, not a cross-desktop future-time scheduler.
[^gnome-notifications]: GNOME documents desktop-file/D-Bus activation requirements for notifications to be associated with an app when it is not running.
