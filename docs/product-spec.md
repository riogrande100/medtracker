# Medtracker Product Specification and Engineering Roadmap

**Status:** Proposed; no application code is included

**Evidence checked:** 2026-11-14
**Scope assumption:** U.S.-focused personal medication companion; one local data set per device; iOS, Android, Windows, and macOS are primary product targets, with mobile usability and easy dose logging driving the design.

## Evidence labels

- **Verified** means the statement is supported by a cited official source.
- **Recommendation** means a product or engineering decision proposed here, not a framework mandate.
- **Spike** means public documentation does not settle a material implementation or reliability question.
- **Assumption** means a provisional product choice that should be revisited if user research contradicts it.

## 1. Executive recommendation

Build a Tauri v2 app with a TypeScript/React frontend, a small Rust domain/application layer, and a private SQLite database. Keep the local ledger responsive and available to the main taking-medication flow, but allow deliberate, user-controlled network access for approved services, catalog updates, app updates, and health-platform integration. Keep the medication catalog in a separately versioned, read-only bundled SQLite file unless a reviewed online catalog service becomes materially better. Use SQLite for the ledger, schedule calculations, filters, and charts; do not add DuckDB unless representative workloads demonstrate a need that SQLite cannot meet.

The primary product outcome is that a non-technical person can identify what is due and record a dose or skip quickly, confidently, and with minimal reading or navigation. Optimize the Today and take/skip flows before expanding reporting.

Treat the schedule engine as the source of truth for expected occurrences. Treat native notifications as best-effort delivery requests only. A reminder, notification tap, or notification dismissal must never write a taken-dose event. Validate iOS and Android notifications first because mobile is the primary taking-medication experience. Windows and macOS remain first-class product targets, but desktop reminders and health integrations must use target-specific adapters rather than being assumed equivalent to mobile.[^tauri-notifications]

Adopt immutable schedule versions and persisted occurrence snapshots. Store UTC instants, IANA zone identifiers, original local wall time, original offset, entry time, schedule-version reference, and time-zone-database version. Keep corrections understandable through a compact revision history rather than full event sourcing. Show history in original, current-device, or selected reporting-zone time without rewriting stored facts.

For the catalog, investigate the current RxNorm Current Prescribable Content monthly release first. NLM lists it as downloadable without a UMLS license, while the full release requires one; however, NLM's broader redistribution terms and the source-vocabulary restrictions still require a release-specific review before bundling. RxTerms is not a verified dependency for the MVP: its current downloadable format and redistribution terms were not established from the current NLM release pages. Do not ship a catalog until provenance and redistribution are approved.[^rxnorm-files][^rxnorm-terms][^umls-license]

The highest risks are:

1. Whether the chosen catalog subset can lawfully be redistributed in a packaged app, including in its derived searchable form.
2. Whether reminders are sufficiently dependable under app termination, device sleep/reboot, permissions, travel, and time-zone changes.
3. Whether a bundled IANA time-zone database and the device's reported current zone can be kept consistent across targets.
4. Whether users expect encrypted app databases, OS-backup exclusion, or simple per-device storage; each changes recovery and key-management work.
5. Whether Android Health Connect and supported Google health APIs can read or write the intended medication record without weakening user consent, adding a server unnecessarily, or misrepresenting data as clinical records.

Medtracker is not specified as completely offline. The app may use the network for explicit, documented features such as health-platform authorization, Google/Android health integration, catalog retrieval, updates, and optional synchronization later. Manual logging should remain resilient during a temporary connection failure and integrations must never block or silently duplicate a dose record. No advertising, behavioral analytics, or undisclosed data transfer is part of this design.

## 2. Product specification

### Problem statement

People taking medication need a trustworthy, low-effort way to see what is due and record what they say they took, skipped, or observed. The primary use case is helping a user such as the product owner's wife complete daily medication tracking without navigating dense forms, interpreting ambiguous labels, or remembering whether an item was already recorded. Medtracker should keep the primary action obvious while preserving history across schedule, catalog, clock, or time-zone changes.

### Target users and primary use cases

- A person maintaining their own medication log, including while traveling.
- A person who wants to take a medication and record it in as few understandable steps as possible.
- A caregiver supporting setup or reviewing history, without sharing an account in the MVP.
- A person who wants to record explicit skips, notes, or observations and later inspect or export the raw history.
- A person who wants reminders but understands that notifications can be denied, delayed, suppressed, or missed.

**Not in the initial scope:** clinician workflows, pharmacy or prescription management, medical advice, or guaranteed two-way health-platform synchronization.

### Product principles

1. A recorded event is a user's statement, not an independently verified clinical fact.
2. A missing log is “not logged,” not “missed” or “skipped.”
3. A reminder is not a dose record.
4. Corrections retain a readable audit trail unless the user chooses the separate privacy-oriented delete action.
5. Catalog updates never overwrite a user's medication snapshot.
6. Charts describe recorded data and expose their time zone, date range, denominator, and exclusions.
7. Local data, export, backup, restore, and deletion are first-class product surfaces.
8. No feature should infer safe dosage, treatment effectiveness, or clinical causation.
9. Taking and recording a due medication should be one clear, accessible action; setup detail must not obstruct daily use.
10. Network-enabled integrations are explicit, permission-based, failure-tolerant, and never replace the user's authoritative ledger action.

### MVP scope

- One local profile per installation, with no Medtracker account required.
- A Today view that groups medications by due-now/upcoming/complete state and provides a large, plainly labeled **Taken**, **Skip**, and **Undo** flow.
- A confirmation step only where needed to prevent accidental records; avoid requiring navigation through medication details for routine logging.
- Add a medication from an approved bundled U.S. catalog or enter one manually.
- Store a historical medication snapshot: selected display name, strength, form, units, optional catalog identifier/release, and optional user display name.
- Create, pause, resume, edit, and discontinue versioned schedules. MVP recurrence choices: selected weekdays at one or more wall-clock times, plus fixed elapsed intervals anchored to a user-confirmed instant.
- Log a dose now or backdated; explicitly skip a scheduled occurrence; add a note or user-defined observation; correct or delete an event.
- Review a chronological ledger and descriptive scheduled-occurrence summary.
- Export a user-selected CSV/JSON report; create and restore a versioned, encrypted backup after the recovery spike.
- Show notification permission state and a generic, privacy-preserving reminder where the target platform passes the feasibility gate.
- Responsive touch-first mobile UI and adapted Windows/macOS desktop layouts; localization-ready date, number, and unit display.
- Optional Android health integration only after a Phase 0 feasibility and consent review. A connection failure must leave the local ledger usable.

### Explicit non-goals

- Dose recommendations, dose calculations, interaction checking, diagnoses, treatment changes, contraindication screening, adherence coaching, risk alerts, or clinical-improvement claims.
- Prescription creation/refill, medication identification by image, barcode scanning, pill recognition, or comprehensive drug information.
- Medtracker advertising, behavioral analytics, unexplained third-party telemetry, or automatic medical interpretation.
- A Medtracker account or automatic cross-device synchronization unless separately approved after the core product succeeds.
- A generic scheduling engine, arbitrary RRULE editor, or user-programmable workflows.
- Guaranteeing notification delivery, including during force-stop, reboot, OS focus modes, permission denial, or battery restrictions.
- Clinical records, clinician access, or claims of regulatory or privacy compliance.

### Post-MVP capabilities

- Windows/macOS target-specific notification improvements and desktop UX.
- Android/Google health import or export where supported, with per-record provenance and user confirmation.
- Additional chart filters and user-defined observation visualizations.
- Optional encrypted user-controlled import/export workflows and more detailed audit browsing.
- Additional schedule patterns only where user research demonstrates need.
- Platform-specific notification enhancements only after separate validation.

### Functional and nonfunctional requirements

| ID | Requirement |
|---|---|
| PR-01 | Core local logging, history, and correction shall remain usable during a temporary connection failure. Every network destination shall have a documented purpose and consent boundary; no personal data shall be sent to an unapproved service or used for advertising or behavioral analytics. |
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
| PR-16 | iOS, Android, Windows, and macOS shall be treated as primary product targets. Each platform shall pass a target-specific build, accessibility, reminder, data-protection, and distribution gate; unsupported capabilities shall be disclosed rather than emulated silently. |
| PR-17 | UI fonts, scripts, styles, and chart code shall be locally bundled. Network use shall be restricted to reviewed endpoints and visible user features, with no arbitrary runtime code loading. |
| PR-18 | The app shall not claim legal, privacy, security, or regulatory compliance solely because records are local or encrypted. |
| PR-19 | Health-platform integrations shall be optional and separately authorized. Imported data shall retain source/provenance and duplicate status; exporting shall be explicit and never treat platform acceptance as proof that a dose was taken. |

## 3. User journeys and information architecture

### Minimum navigation

Use a responsive shell with four primary destinations:

1. **Today** — active medications, next expected occurrences, quick log/skip actions, and a clear permission/reminder status.
2. **History** — event timeline and filters by medication, event type, range, and reporting zone.
3. **Reports** — descriptive charts, metric definitions, date range, and zone selector.
4. **Medications** — catalog/manual add, medication details, schedules, pause/resume/discontinue.

Settings is a secondary destination reachable from the navigation/header: time display defaults, notification permissions and privacy, health-integration permissions and connection status, export/backup/restore, data deletion, catalog/version details, and about/disclaimer. On narrow screens, use a bottom navigation bar; on desktop, use a compact side rail. Keep add flows as modal sheets on mobile and dialogs or panels on desktop, with full-screen alternatives for accessibility. Keep the routine Today dose action inline: identify the medication, confirm the amount/time already shown when needed, then save with one large button and an immediate undo option.

### Journeys

- **First launch:** explain local storage and the absence of a required Medtracker account; disclose that network services, health authorization, system notifications, and OS backups are separate; request no permission until a user enables the corresponding feature; offer catalog/manual add.
- **Add medication:** search the bundled catalog by normalized/display name, brand, ingredient, strength, or form; show source/release and the distinction between product strength and amount entered per event. If not found, choose manual entry. Store a snapshot and optional custom display name.
- **Create a schedule:** choose “follows this device's current time zone,” “fixed in a selected home time zone,” or “fixed elapsed interval anchored at [instant].” Preview upcoming instants with zone abbreviations/offsets and explain DST policies. The preview is descriptive; it never tells the user to change medication use.
- **Log now:** select the medication, enter an optional amount and unit plus note/observation, confirm the event time in the current zone, and save. No reminder is required.
- **Backdate:** choose date/time and an explicit zone or fixed offset; show the resolved UTC instant and the entered local context before save. `entered_at` remains the current system-reported instant.
- **Skip/correct/delete:** an explicit Skip action creates a skip event tied to an occurrence when selected. Correct creates a revision. Delete is distinct, warns that deleted values will not be available in the event audit, and offers a separate permanent data wipe in Settings.
- **Travel:** show active zone mode, detected current IANA zone when available, and upcoming instants in both schedule and current-device zones. Ask the user to choose a schedule version/mode change; never advise how to alter medication timing.
- **Review charts:** show the range, reporting zone, scheduled-occurrence denominator, explicit skips, logged occurrences, missing logs, and conflicting entries. Provide table/export access and metric definitions.
- **Export/backup/restore:** let the user choose destination and format; warn before a plain export leaves the app sandbox. For encrypted backup, collect a passphrase or use a user-controlled key; verify a restore in a staging file before replacing live data.
- **Manage permission:** show app-level state and a link to OS settings when denied. Distinguish “reminder request queued” from “notification permission granted” and “dose logged.” Permission denial never prevents manual logging.
- **Connect a health platform:** explain exactly which Android/Google data may be read or written and for what purpose; request authorization only when the user chooses Connect; show last status, source record, duplicate review, revoke/disconnect, and retry separately from the local dose ledger. A failed, revoked, or unavailable connection never hides the manual logging flow.

## 4. Domain model and time semantics

### Core entities and invariants

- **UserMedication:** a user's active or archived medication identity with a stable local ID and pointer to its current immutable `MedicationVersion`. Catalog reference is convenience, not the source of historical display.
- **MedicationVersion:** immutable selected name, strength, form, unit, optional catalog identifier/release, and optional user display name effective from an instant. Editing any of those fields creates a new version; prior events and schedule versions retain their original reference.
- **Schedule:** user intent for one medication; may be absent for as-needed medication.
- **ScheduleVersion:** immutable schedule definition with effective bounds, recurrence semantics, zone policy, recurrence parameters, and DST policy. Editing, pausing, resuming, ending, or changing time-zone behavior creates a new version.
- **ScheduledOccurrence:** one materialized expected reminder/logging opportunity under a particular schedule version. It captures intended local date/time, resolved UTC instant, zone, offset, fold, and tzdb version. It is not proof of a dose.
- **DoseEvent:** one user-entered take or skip statement. It stores `occurred_at` and `entered_at` separately and may link to zero or more scheduled occurrences. One occurrence can link to multiple events.
- **DoseEventRevision:** compact before/after record of edits and links; not a full event-sourcing log.
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

### Timestamp semantics

Capture these values separately:

- `occurred_at_utc`: UTC instant the user says the event happened. For a new “now” entry, initialize from the device wall clock, but make clear it is user/system-reported, not independently verified.
- `entered_at_utc`: UTC instant the record was created on this installation, as reported by the system clock.
- `original_local_datetime`: local date/time as entered or resolved at entry, preserved as text.
- `original_zone_id`: IANA zone when available (for example `America/Los_Angeles`); store the entered fixed offset separately.
- `original_offset_seconds`: signed offset used to resolve the event at entry.
- `schedule_version_id` and occurrence link(s): only when the user explicitly relates the event to an occurrence.
- `tzdb_version`: version used to resolve an occurrence. Pin a bundled IANA database implementation (for example `chrono-tz`, whose generated data exposes an IANA database version) and record that version.[^iana-tzdb][^chrono-tz]

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

**New York spring-forward.** On `2026-03-08`, the New York clock jumps from `01:59:59 -05:00` to `03:00:00 -04:00`. A wall-clock slot of 02:30 does not exist. Under the proposed gap policy, its one occurrence resolves to the first valid time, `2026-03-08 03:00 America/New_York (-04:00) = 2026-03-08T07:00:00Z`, marked adjusted. The next date's 02:30 is `2026-03-09T06:30:00Z`.

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
| `dose_event` | `id PK`, `medication_id FK`, `medication_version_id FK`, `kind CHECK(take, skip)`, `occurred_at_utc`, `entered_at_utc`, original local datetime, zone ID, offset seconds, optional amount mantissa/scale/unit, optional note, current revision number, deleted flag. `take` may have no amount; `skip` has no amount. |
| `dose_event_occurrence` | `(dose_event_id, occurrence_id) PK`, linked-at instant, link source `user/system-import`; indexes both FKs. Application rules disallow implicit links and identify conflicting take/skip links. |
| `dose_event_revision` | `id PK`, event ID, revision number, change instant, actor `user`, reason, changed-field list, before/after values for corrections. Enforce unique `(dose_event_id, revision_number)`. A delete tombstone records deletion without retaining deleted values. |
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
- Keep fonts, scripts, chart packages, styles, and core layout assets bundled; do not load arbitrary UI code from the network. Approved API responses must be typed, bounded, validated, and separable from the core rendering path.

### Rust modules and boundaries

Keep one Tauri application. Suggested modules:

```text
domain/          medications, schedules, events, occurrence/outcome rules
time/            IANA zone resolution, DST policy, reporting conversion
application/     validated use cases and transaction boundaries
storage/         SQLite repositories and migrations
catalog/         read-only search and bundled release metadata
notifications/   platform adapter, queue reconciliation, permission state
integrations/    health-platform ports, authorization, provenance, duplicate review
export/          CSV/JSON serialization and destination handoff
backup/          snapshot, encryption envelope, validate/restore
commands/         typed Tauri command DTOs and error mapping
```

Tauri commands accept typed input DTOs, validate in Rust, invoke application use cases, and return typed DTOs/errors. Do not expose raw SQL or generic “execute query” commands to the frontend. Return explicit failures (validation, permission denied, storage full, migration failure, backup invalid, notification unsupported) rather than success-shaped defaults.

Keep the main Today screen driven by local state. Network calls belong behind explicit integration commands with visible pending, denied, disconnected, and retry states. No integration adapter may write a taken event directly; it may propose an imported record for review or export an already saved local record.

### Database access

Use `rusqlite` initially with a small repository layer and explicit migrations. A single serialized writer is appropriate for one person's local log; run potentially blocking DB work away from the UI thread, enable foreign keys on connection setup, and keep transactions short. Use SQLite FTS5 for bundled catalog search. SQLite's WAL mode may improve concurrent reads, but it creates sidecar/checkpoint considerations; choose journal mode after mobile/desktop testing, not by default assumption.[^sqlite-wal]

### Time engine

Use a pinned Rust IANA timezone implementation, such as `chrono-tz`, for deterministic recurrence calculations and an explicit ambiguity/gap resolver. `chrono-tz` is generated from IANA data and exposes its database version, but it does not by itself identify the device's current zone; a native Tauri bridge/prototype must establish that mapping on each target.[^chrono-tz] Test each target's local-zone identifier, app-resume events, and transitions before deciding whether `device_local` mode can be promised.

### Notifications

Define `ReminderScheduler` domain port with operations to request/cancel/reconcile an occurrence, query permission status, and capture supported interactions. A native adapter receives immutable occurrence IDs and UTC instants; it cannot write dose events. The schedule engine and ledger remain usable when the adapter is unavailable or permission is denied.

Use the official Tauri notification plugin as the first prototype because it exposes immediate and scheduled mobile notifications. Its current docs state that `Schedule` is mobile-only and desktop notifications are immediate, so desktop reminders need a different integration.[^tauri-notifications] For the first release, cap the queued horizon to a short rolling window chosen by measured per-platform limits; the database schedule remains authoritative. Never enqueue unbounded reminders.

### Health integrations

Treat Android Health Connect and any applicable Google health API as optional adapters, not as the primary database. A Phase 0 spike must verify the currently supported data types, medication-related write/read limitations, permissions, Android version/support requirements, application OAuth/developer-console requirements, account handling, and whether Tauri needs a native bridge. Do not map health data into medication events by name alone. Each imported candidate needs source, timestamp, source record ID, and a user-reviewed duplicate decision before it can become a local ledger event. Export remains explicit; an integration acceptance response only means the receiving platform accepted the request.

### Catalog pipeline

1. Pin a named monthly NLM release and keep its download/release note, source terms, checksum, release identifier, and ingestion manifest in version control or release metadata.
2. Download only during the maintainer's build process, not at runtime. Verify published checksum where provided.
3. Parse the RRF files deterministically; retain only whitelisted sources/term types and fields needed for display/search.
4. Generate a read-only SQLite DB with a source/release manifest and FTS index. Preserve exact normalized name, brand/ingredient/strength/form labels and source RXCUI, but do not treat them as prescribing data.
5. Run deterministic-output, row-count, duplicate-ID, label, and license/attribution checks; inspect a release-to-release diff for retired/merged identifiers.
6. Bundle the generated catalog in app releases. Catalog changes create no writes to `user_medication`; users may see “catalog entry no longer current” while historical snapshots remain.

**Verified source shape and coverage:** NLM provides downloadable monthly ZIP releases; the RRF files are UTF-8, pipe-delimited text. RxNorm concepts use RXCUIs, individual source/name atoms use RXAUIs, and normalized drug names can express ingredient, strength, and dose form. RxNorm covers U.S. prescription and many OTC drugs, including generic/branded clinical drugs and packs, but is not a complete list of supplements, devices, foods, or every medication-like product. Keep the release ID and source/term type with each catalog row; use RXCUI as a reference, not a timeless substitute for the user's text snapshot.[^rxnorm-overview][^rxnorm-files]

RxNorm says RXCUIs may be merged, split, archived, or retired, while RxNorm-normalized concepts generally persist and history files describe changes. Preserve source release and user snapshot rather than silently remapping a personal medication.[^rxnorm-faq]

### Packaging and release

Build signed iOS, Android, Windows, and macOS artifacts from pinned Rust/Node dependencies, include a software bill of materials and catalog provenance, and test the actual signed install package (not only a dev build). Keep signing keys outside the repository. App-store delivery, desktop installer download, app-update checks, and health-service network calls are explicit reviewed flows rather than assumptions that all platforms behave alike. Tauri's updater is optional; it is not a requirement for using the core application.[^tauri-updater]

### Dependency-selection criteria

Prefer maintained, permissively licensed crates/plugins with iOS, Android, Windows, and macOS support, explicit permission behavior, typed APIs, auditable source, and platform-specific tests. Pin versions and lockfiles. Review every transitive network behavior, data destination, OAuth scope, and notice. No DuckDB for MVP: SQLite covers the ledger and aggregations without a second engine or larger analytical surface.

## 7. Reporting and chart definitions

Each report shows metric definition, date range, reporting zone, schedule mode(s), excluded/conflicting rows, and whether the value is a count or percentage. No generic adherence score.

1. **Scheduled occurrence outcomes (stacked daily bars):** denominator is all active generated occurrences in the selected schedule versions whose scheduled instants fall in the selected range. Categories are logged, explicitly skipped, not logged, and conflicting. Multiple linked take events count once in the occurrence denominator; they remain separate in the event ledger. A schedule paused/ended before an occurrence contributes no occurrence. An unlogged state is not a missed dose.
2. **Dose-event timeline:** each active take/skip event once at `occurred_at_utc`, with amount/unit only when entered. Supports notes and correction indicators. No dose is inferred from notification activity.
3. **Elapsed time between recorded events:** difference between consecutive active take-event UTC instants for the same medication, optionally excluding events whose occurred time is corrected/uncertain only when user explicitly applies that filter. State that it is descriptive elapsed time, not an appropriate interval recommendation. For multiple entries tied to one scheduled occurrence, keep them distinct and do not combine without an explicit user-defined compatible-unit action.
4. **As-needed medication activity:** event counts/amounts by day or week, without scheduled denominator or adherence percent.
5. **Observation series:** display only user-chosen labels and values; no correlation coefficient or causal language in MVP. If a later descriptive overlay is added, label it as co-occurrence only and provide a non-causality explanation.

**Missing data:** show not logged as its own category. Never fill missing events with zero dose or “missed.” **Partial/multiple amounts:** show each event entry as entered; do not sum incompatible units or infer active ingredient quantity. **Schedule edits:** use the schedule version that generated each occurrence; show version boundaries in the report. **Skip and take conflict:** exclude from logged/skipped totals and expose in a review count until resolved.

**Date boundaries:** For event charts, group each UTC event by the selected reporting zone's local calendar day. “Original recorded local time” groups using the preserved original local date, not a recalculation. “Current-device local time” converts UTC instants using the current device zone and can change when viewed elsewhere. “Selected time zone” converts UTC instants using a chosen IANA zone and current bundled rules. Label each view. For occurrence charts, provide two explicit grouping options: by each occurrence's stored schedule-local date, or by its scheduled UTC instant projected into the chosen reporting zone. The default is schedule-local date, with the selected zone and schedule mode visible.

Export the table underlying each chart with occurrence IDs, event IDs, schedule-version IDs, original timestamps, reporting zone, metric definition, and filter parameters, subject to privacy choices.

## 8. Notification capability matrix and feasibility spike

### Capability matrix

The table distinguishes the official Tauri plugin's documented behavior from the underlying platform's native APIs. It does not promise identical delivery. Permissions, OS focus settings, battery policy, device sleep, and user settings can suppress or delay display.

| Platform | Official Tauri v2 notification plugin | Native scheduling facts / constraints | MVP stance |
|---|---|---|---|
| iOS | Supports immediate and scheduled notifications; scheduling is mobile-only. Plugin provides permission checks/request and actions. | Apple's UserNotifications framework accepts local calendar/time-interval triggers; the OS handles display when the app is backgrounded/not running, and pending requests can be canceled. User authorization/settings still apply. The plugin docs do not establish reboot, queue-cap, time-zone-travel, or precise-delivery guarantees. | First reminder spike. Release only with permission/privacy UX and measured reconciliation behavior. No claim of guaranteed delivery. |
| Android | Supports immediate and scheduled mobile notifications. Plugin includes schedule helpers; exact alarm behavior and reboot restoration are not established by its overview. | Android alarms can operate outside app lifetime and while asleep. Exact alarms on Android 12+ require declared special access; Android 13+ notifications need runtime `POST_NOTIFICATIONS` permission. Battery-saving policies can delay inexact alarms. AlarmManager state should not be assumed to survive reboot; verify rescheduling and force-stop behavior on devices.[^android-alarms][^android-permission] | Phase 0 feasibility, then Phase 4 platform release. Require explicit permission handling; if exact-alarm access is unavailable, state the weaker behavior or omit reminders. |
| Windows | Plugin supports notifications on Windows only for installed apps, but its schedule API is mobile-only, so plugin notifications are immediate. | Windows App SDK supports scheduled app notifications independently of Tauri. Microsoft's docs specify a five-minute delivery window and say a notification is dropped if the computer is off beyond five minutes; a background-task alternative is suggested for stronger delivery but needs a separate integration.[^windows-notifications][^windows-scheduled] | No reminder promise in MVP. Prototype WinRT/Windows App SDK integration before adding scheduled reminders. |
| macOS | Plugin supports immediate notification; plugin scheduling is mobile-only. | Apple UserNotifications supports local scheduled requests. A native Swift/Objective-C bridge, permission UX, bundle identity/signing behavior, app-not-running behavior, sleep/wake, and restart semantics need a macOS spike. | Desktop later; no timer-based fallback. |
| Linux | Plugin supports immediate notification; plugin scheduling is mobile-only. | Freedesktop notifications describe passive desktop notifications, not a portable scheduler. | Outside the primary supported platform scope. Re-evaluate only if Linux becomes a funded target. |

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

- Tauri v2 minimal app using the official notification plugin on iOS and Android; record plugin versions and signed-build configuration.
- One native one-shot request, one daily wall-clock request, one cancellation/reschedule after schedule-version change, and an occurrence-ID payload that opens the app without logging a dose.
- Test iOS and Android on physical devices; include locked screen, app foreground/background/terminated, reboot, permission denied/re-enabled, battery saver/Doze, time-zone change while app closed, DST gap/repeated time, clock jump, and schedule edit with old request pending.
- Capture OS/version/device, build/signing mode, permission state, scheduled UTC and local components, API return/error, pending-request list where exposed, manual notification-center observation and timestamp, interaction callback, reboot/resume reconciliation, and duplicates/stale requests.
- On Android separately test exact-alarm permission missing/granted, notification permission missing/granted, Doze, and reboot. Do not assume a plugin option maps to an exact alarm.

**Pass for a platform:** in a controlled physical-device run with permissions allowed and no Focus/Do-Not-Disturb suppression, all planned scenarios produce no duplicate/stale schedule after reconciliation; at least 19 of 20 timed deliveries in each ordinary locked/background scenario appear within five minutes of target; the app correctly reports unsupported/denied state; and boot/time-zone-change behavior is documented. This is a prototype threshold, not a user-facing delivery guarantee.

**Fail or uncertain:** if the platform cannot reschedule after reboot/zone change, has unexplained stale requests, or cannot meet the threshold, remove scheduled reminders from that release or narrow them to a clearly labeled best-effort feature with user consent. Do not replace native scheduling with a foreground JavaScript timer. Repeat a smaller desktop spike before any desktop reminder commitment.

## 9. Security, privacy, and recovery design

### Proportionate threat model

| Threat | Control and limit |
|---|---|
| Lost device | Private app directory; rely on OS device encryption and lock-screen security; exclude database from automatic cloud backup unless a future explicit sync/backup feature changes that policy; generic notification previews. This does not protect an unlocked or compromised device. |
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
- Maintain an allowlist of network destinations, scopes, data fields, and initiating features. Health, catalog, and update requests fail visibly and never upload medication data implicitly. Core local logging and history are tested with network access blocked, while online integration paths are tested separately for timeouts, invalid responses, revoked access, and retries.
- Permanent wipe cancels native requests, clears user DB and sidecars, removes key material and app-generated temporary files, then confirms completion. Explain it cannot erase OS/user backups, screenshots, shared exports, old filesystem snapshots, storage wear-leveling copies, or data copied elsewhere.

### Compliance and review flags

Local-only storage and “not medical advice” wording do not establish regulatory, privacy, or app-store compliance. Before release, obtain appropriate review of intended-use claims and marketing, health-data privacy notices, consent/retention, accessibility, app-store category and permission disclosures, medication catalog licensing, and platform backup behavior. Avoid any medical efficacy or safety claim.

## 10. Test and acceptance plan

### Test layers

- **Rust unit tests:** schedule generation and version boundaries; `device_local`, `home_zone`, elapsed interval; DST gap/fold; date-line crossing; occurrence identity; conflicting outcomes; correction history; exact decimal amount encoding; report denominators and range boundaries.
- **SQLite integration tests:** foreign keys on every connection, uniqueness/check constraints, indexes, transactions, concurrent reads/writes, rollback on failure, migration from every supported schema, `integrity_check`, `foreign_key_check`, archive/delete/wipe behavior.
- **Backup/restore tests:** valid encrypted backup; wrong passphrase; truncated/corrupt ciphertext; malformed manifest; unsupported newer version; older version migration; disk-full/atomic replacement failure; restore preserves event and schedule snapshots.
- **Catalog tests:** pinned release manifest/checksum; deterministic ingestion; whitelisted TTY/source coverage; duplicate/retired/merged IDs; FTS matches; historical snapshots unchanged when replacing catalog.
- **Time tests:** fixed offsets, IANA zones, DST gap/fold, timezone changes, manual clock jumps, International Date Line, backdated local times, tzdb version upgrades, and current-device/home-zone/selected-zone display.
- **Notification/device tests:** Phase 0 matrix on physical iOS/Android; desktop native spikes before adding desktop notifications. Test app closed/terminated distinctions, device sleep/reboot, denied permissions, OS notification disabled, stale cancellation, duplicate prevention, schedule mutation, and “tap does not log.”
- **Accessibility:** keyboard-only desktop navigation, VoiceOver/TalkBack, dynamic text/zoom, contrast, focus order, touch target size, chart tables, not-color-only states, reduce-motion.
- **Network and failure tests:** disable connectivity and verify local Today, logging, history, correction, schedule calculation, and restore remain usable. Record every approved endpoint and purpose; test timeout, invalid response, revoked consent, partial import, duplicate import, denied export, and retry behavior. Verify health integration never blocks or silently creates a local dose.
- **Cross-platform UI smoke:** signed iOS, Android, Windows, and macOS packages; clean install; upgrade preserving DB; uninstall/reinstall expectations; app-data path; native permission prompts; responsive layouts; and low-storage behavior.
- **Size/performance:** record bundle/install size by architecture, cold launch, catalog search latency (median/p95), timeline and chart query latency on the representative fixture, backup/restore time, and memory use. Establish measured baselines and regression budgets; do not promise unmeasured app size, speed, or delivery time.

### Representative deterministic fixture

Use synthetic records only:

- `Daily tablet A`: product snapshot `Example ingredient 5 mg / tablet`, logged amount `1 tablet`, daily 08:00 `America/New_York`, covering 2026-03-07 through 2026-03-10 under the proposed DST gap policy.
- `As-needed liquid B`: product snapshot `Example ingredient 10 mg/mL`, user-entered amounts `2 mL` and `1.5 mL` on distinct instants; no recurring schedule.
- For A, create one take occurrence on March 7, one explicitly skipped occurrence on March 8, no event for March 9, and two take events linked to the March 10 occurrence; include one correction and one observation.
- Expected A occurrence report for these four schedule dates: 4 occurrences total, 2 logged occurrences, 1 explicitly skipped, 1 not logged, 0 conflicts; the two March 10 event rows count once in occurrence totals. The March 8 scheduled instant resolves under policy to `2026-03-08T07:00:00Z`. B contributes two unscheduled events and no scheduled denominator.
- Render the same events grouped in New York, Los Angeles, original recorded local time, and current-device time. Assert the displayed day can change by zone while UTC/original snapshots remain unchanged.

### Given/When/Then acceptance criteria

- **PR-01 network failure:** Given connectivity and a health integration are unavailable, when Today is opened and a due medication is logged, then the local event saves once, remains visible, and exposes a separate integration/retry state without preventing use of the app.
- **PR-19 health integration:** Given the user authorizes a supported health adapter and a record already exists locally, when the adapter receives the same record, then the app reports or proposes a duplicate rather than silently creating a second dose; revoked authorization fails visibly and does not erase local history.
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

## 11. Phased roadmap

Effort is relative (`S` small, `M` medium, `L` large, `XL` substantial uncertainty), not a calendar estimate.

### Phase 0 — feasibility and risk reduction

- **Objective:** resolve platform, catalog, timezone, privacy, and release risks before meaningful UI investment.
- **Deliverables:** user-tested Today/take/skip wireflow; iOS and Android native notification prototype; native current-zone discovery spike; DST behavior tests; catalog provenance decision; Android Health Connect/Google health capability and consent spike; storage/backup and four-platform distribution review; Tauri build baseline for iOS, Android, Windows, and macOS.
- **Requirements:** PR-02, PR-03, PR-07, PR-11, PR-13, PR-16–PR-19.
- **Dependencies:** none.
- **Major tasks:** validate the routine taking-medication flow with the intended user; execute §8 spike; inspect current NLM release data and license; test `chrono-tz` versioning and local-zone bridges; evaluate Android Health Connect and applicable Google health endpoints/scopes; evaluate per-app backup exclusion; verify Rust/Node/Tauri builds for all four targets; choose default DB encryption stance.
- **Acceptance:** all four target platforms have a documented build/distribution path and mobile test builds install; all scheduled-notification scenarios have a go/no-go decision; health-integration support and data mapping are documented or explicitly deferred; catalog source/fields and redistribution are documented; timezone gap/fold/zone-change behavior passes deterministic tests.
- **Risks/mitigations:** notification or catalog ambiguity blocks feature promises—narrow release or hold release; native zone ID unavailable—defer `device_local` scheduling and clearly constrain modes; encryption/build complexity—start with OS protections and a separate threat-driven SQLCipher decision.
- **Exit gate:** product owner accepts platform scope, catalog provenance, reminder scope, and privacy/recovery stance based on evidence.
- **Deferred:** polished design, broad platform parity, analytics, complex recurrence.

### Phase 1 — smallest useful vertical slice

- **Objective:** deliver the easiest possible mobile path from seeing a due medication to a corrected historical event.
- **Deliverables:** app shell, Today view, large Taken/Skip/Undo flow, per-app SQLite DB/migrations, manual medication creation, catalog search if licensing gate passes, event ledger, time-context capture, correction history, accessible timeline.
- **Requirements:** PR-01, PR-02, PR-04, PR-05, PR-06, PR-11, PR-12, PR-15, PR-17.
- **Dependencies:** Phase 0 platform and storage decisions; catalog provenance for catalog UI.
- **Major tasks:** set up typed Tauri commands; build repository/migration layer; create medication snapshots; logging/backdating UI; test timezone capture; implement archive and delete distinctions.
- **Acceptance:** the intended user can add a manual medication and, from Today, record one due dose in no more than the agreed core steps without help; log/backdate/correct still work with connectivity disabled; restart preserves values and a catalog update simulation does not alter snapshots.
- **Risks/mitigations:** app-state leakage or ambiguous entry UX—explicit labels and tests; catalog legal gate delayed—keep manual add useful but do not market catalog availability before approval.
- **Exit gate:** usability and Phase 1 acceptance tests pass on iOS and Android test builds, and the user ledger survives app restart/upgrade.
- **Deferred:** charts, scheduled notifications, production desktop release, arbitrary custom observation schema.

### Phase 2 — MVP completion

- **Objective:** complete schedules, occurrence outcomes, descriptive reports, catalog, export/backup, and permission-aware reminders for the first mobile release candidate.
- **Deliverables:** immutable schedule versions; occurrence materializer; zone-aware reports; catalog bundle/search; CSV/JSON export; encrypted backup/restore; reminder adapter only if spike passed; settings/privacy surfaces.
- **Requirements:** PR-01–PR-13, PR-15, PR-17.
- **Dependencies:** Phase 1 ledger; Phase 0 notification/catalog gates.
- **Major tasks:** implement recurrence modes and DST policies; schedule reconciliation; outcome classification; chart queries; migration/backup restore; catalog pipeline; permission and preview UX.
- **Acceptance:** fixture expectations in §10 match exactly; restore tests pass; permission denial and network failure do not block logging; no chart conflates missing and skipped; reminder and health adapters never create an unreviewed dose event; network failure tests pass.
- **Risks/mitigations:** native scheduling varies—ship reminders only on passed platform; restore/cipher failures—test staged restore and key loss; report ambiguity—publish denominators and table exports.
- **Exit gate:** requirement-level tests and product review approve data portability, definitions, and platform-specific reminder statement.
- **Deferred:** automatic sync, all desktop targets, user-configurable arbitrary recurrence, causal observation analysis.

### Phase 3 — hardening and first mobile release

- **Objective:** ship a safe, understandable, supportable first release on the mobile platform selected through Phase 0 and usability testing, while retaining the remaining three targets as committed follow-up releases.
- **Deliverables:** release packaging/signing, migration/backup recovery path, accessibility and usability pass, privacy/health permission disclosures, catalog attribution/license artifacts, network/data-flow test report, diagnostics policy, release-size/performance baselines.
- **Requirements:** PR-01, PR-02, PR-10–PR-13, PR-15–PR-19.
- **Dependencies:** Phase 2 acceptance and all legal/platform release gates.
- **Major tasks:** test store/package review; verify automatic backup exclusions; exercise full deletion; crash/error UX without upload; test upgrade from prior database; publish source/attribution/limitations.
- **Acceptance:** signed install/update paths work; upgrade and restore retain expected records; accessibility, usability, and network-failure suites pass; catalog release manifest and notices ship; all measured performance/size results are recorded without unsupported guarantee.
- **Risks/mitigations:** store policy/review or source-data issue—review early and retain manual entry fallback; update fails—store rollback snapshot and user export path; support logs reveal sensitive data—automated redaction tests.
- **Exit gate:** release checklist, source licensing, privacy disclosures, and target-specific capability statement are approved.
- **Deferred:** marketing claims of clinical value, cross-device continuity, secondary platforms.

### Phase 4 — cross-platform and integration expansion

- **Objective:** finish and maintain iOS, Android, Windows, and macOS releases, then add integrations justified by platform tests and user value.
- **Deliverables:** platform-specific adapters, signed packages, support matrices, desktop UX parity for core flows, and optional Android/Google health export/import if the feasibility gate passed.
- **Requirements:** PR-01, PR-03, PR-09, PR-13, PR-15–PR-19.
- **Dependencies:** stable core domain and migrations; separate notification spike per target; health integration gate where applicable.
- **Major tasks:** Android exact/inexact alarm and boot recovery work; desktop notification feasibility for macOS/Windows; Windows/macOS core-flow adaptation; target UI/accessibility validation; app-data and OS-backup tests; health integration duplicate/revocation tests; compatibility matrix.
- **Acceptance:** each target's signed release package passes network-failure resilience, data integrity, accessibility, restore, and target-specific notification criteria; supported health flows pass consent, provenance, and duplicate tests; unsupported capabilities are disclosed and not silently emulated.
- **Risks/mitigations:** platform drift and notification parity—adapters and explicit target gates; separate device histories surprise users—make no-sync behavior prominent and provide deliberate transfer.
- **Exit gate:** each platform independently meets its test plan and distribution/privacy review.
- **Deferred:** cloud sync, backend, universal reminder guarantees, unvalidated integrations.

**Critical path:** catalog redistribution decision and notification feasibility → timezone/domain policy → schedule/occurrence data model → report denominator and migration tests → backup/restore and privacy review → signed first release. Core manual logging can proceed in parallel with catalog review, but catalog distribution cannot.

## 12. Initial implementation backlog

Effort: `S` small, `M` medium, `L` large, `XL` high uncertainty. Dependencies name actual prerequisites.

| ID / title | Purpose and scope | Dependencies | Acceptance criteria | Effort / requirements |
|---|---|---|---|---|
| B0-01 — Build target and distribution spike | Create minimal Tauri v2 builds for candidate iOS, Android, and one desktop target; document signing, app-data paths, and developer prerequisites. No product UI. | None | A clean checkout produces installable test builds; list any unsupported dependency/platform step and app-store/manual update implications. | M; PR-16, PR-17 |
| B0-02 — Native reminder feasibility prototype | Implement one-shot/daily schedule, cancel, permission, interaction payload, reboot/zone/DST tests on iOS and Android. | B0-01 | Evidence table records device/OS, request, observed result, permission, timing, duplicate/stale behavior; formal go/no-go per platform. | XL; PR-13, PR-16 |
| B0-06 — Primary user flow usability spike | Prototype the Today, due-medication, Taken/Skip/Undo flow for mobile and test it with the intended primary user. | None | The user can identify the due medication and complete a routine record without assistance; labels, target sizes, contrast, and ambiguous states are corrected. | M; PR-05, PR-15 |
| B0-07 — Android/Google health feasibility | Evaluate Health Connect and applicable Google health APIs, supported medication-related data, scopes, Tauri native integration, duplicate handling, and export acceptance semantics. | B0-01 | Written go/no-go and data-mapping decision identifies supported OS versions, required permissions/credentials, source provenance, and user-review behavior, or defers integration. | L; PR-01, PR-19 |
| B0-03 — Catalog license and field-selection decision | Compare current NLM RxNorm full and Current Prescribable Content release access, data fields, coverage, provenance, source restrictions, and app redistribution terms. Confirm RxTerms availability rather than assume it. | None | Written release-specific decision approves exact source/fields/attribution or blocks bundled catalog; manual entry remains available. | L; PR-02, PR-18 |
| B0-04 — Time-zone engine and native zone probe | Prototype Rust IANA zone resolution, gap/fold policy, tzdb version, and device-current-zone identification on selected targets. | B0-01 | Deterministic tests cover examples in §4; device zone returns valid IANA ID or a surfaced unsupported result; version is recorded. | L; PR-03, PR-04, PR-07 |
| B0-05 — Privacy, backup, and encryption decision | Confirm app-data location, cloud-backup exclusions, OS protections, SQLCipher need, user backup format, and deletion limitations. | B0-01 | Decision matrix identifies platform settings, key-storage tradeoffs, restore mechanism, and statements the product can/cannot make. | M; PR-10–PR-12, PR-18 |
| B1-01 — Tauri shell and typed command boundary | Set up React/TypeScript/Vite, responsive navigation shell, Today-first layout, Rust command DTOs/errors, local asset policy, and network capability allowlist. | B0-01, B0-06 | iOS/Android test builds show the reviewed core flow; a typed command round-trip works; no raw SQL or arbitrary remote code is exposed. | M; PR-01, PR-15, PR-17 |
| B1-02 — SQLite schema, repository, and migrations | Add personal DB location, schema, `user_version` migrations, constraints, foreign-key setup, backup-before-migration, and integrity checks. | B1-01, B0-05 | Integration tests prove invalid references fail, transactions rollback, migration failure retains original, and reopen preserves records. | L; PR-11, PR-12 |
| B1-03 — Medication snapshots and manual entry | Add user medication create/edit/archive, exact historical label fields, optional custom name, and product-strength versus event-amount help text. | B1-02 | Manual entry works during network failure; catalog selection fields can be nullable; archive preserves linked history; no dosage calculation exists. | M; PR-02 |
| B1-04 — Event ledger and correction history | Implement take/skip events, separate UTC occurred/entered timestamps, local/zone/offset capture, optional amount/note, revision and delete behavior. | B1-02, B1-03, B0-04 | Unit/integration tests cover backdating, edit revision, delete redaction, and notification-independent manual entry. | L; PR-04–PR-06 |
| B1-05 — Accessible history timeline | Build event timeline with original/current/selected zone views, filters, correction markers, and no-log distinction. | B1-04 | Screen reader and keyboard tests pass; switching display zone never mutates stored UTC/original context. | M; PR-05, PR-09, PR-15 |
| B2-01 — Catalog ingestion and bundled search | Implement pinned-release importer, provenance manifest, approved field/TTY filter, generated read-only DB, FTS index, and release diff tests. | B0-03, B1-03 | Build is deterministic; approved license and notices are packaged; the existing catalog remains available during a network failure; update leaves personal snapshots untouched. | L; PR-01, PR-02, PR-17, PR-18 |
| B2-02 — Versioned schedules and occurrence engine | Implement three schedule modes, version lifecycle, occurrence materialization, gap/fold/line-date policies, and tzdb snapshots. | B1-02, B0-04 | Deterministic fixture matches timestamps; edits/pause/resume/end never rewrite past occurrences or backfill silently. | XL; PR-03, PR-07, PR-08 |
| B2-03 — Outcome linkage and scheduled reports | Implement user event-to-occurrence linking, conflict state, daily counts, zone/day-boundary choice, and event tables. | B1-04, B2-02 | Given §10 fixture, returns 4 total / 2 logged / 1 skipped / 1 not logged / 0 conflict; double-linked entries count once. | L; PR-05, PR-08, PR-09 |
| B2-04 — Native reminder adapter and reconciliation | Add an adapter only for platforms that passed B0-02; manage queue horizon, cancellation, stale versions, permission state, and privacy-safe payload. | B0-02, B2-02 | OS acceptance/delivery/action states remain separate; permission denial works; tests prove no action creates a dose event. | L; PR-07, PR-13 |
| B2-05 — Export and encrypted backup/restore | Add CSV/JSON and versioned encrypted snapshot format; staged validation, pre-restore snapshot, atomic replacement. | B1-02, B1-04, B0-05 | Corrupt/wrong-key/newer-format cases fail explicitly; valid restore passes integrity and preserves snapshots. | XL; PR-10–PR-12 |
| B2-06 — Descriptive charts and metric definitions | Add occurrence outcome bars, event timeline, elapsed event intervals, PRN activity, table alternative, and visible context/definitions. | B2-03, B1-05 | Charts match fixture, expose denominator/zone/range, and do not infer clinical outcomes or hide missing/conflicting data. | M; PR-08, PR-09, PR-15 |
| B2-07 — Optional Android health adapter | Implement a reviewed health adapter with explicit authorization, provenance, duplicate review, import/export status, and revocation handling. Only proceed if B0-07 passes. | B0-07, B1-04 | Missing or revoked permission is explicit; duplicate candidates require review; adapter failure leaves the local ledger unchanged; exported acceptance is not labeled as a verified dose. | XL; PR-01, PR-19 |
| B3-01 — Release hardening and privacy verification | Verify signed packages, reviewed network egress, backup exclusions, deletion, accessibility, usability, upgrade, catalog notices, and measured baselines. | B2-01–B2-07 | Release checklist passes on each target; no undisclosed network request; signed artifact upgrades without data loss; product limitations documented. | L; PR-01, PR-10–PR-19 |

## 13. Decisions and open questions

### Recommended decisions

- Use Tauri v2, React/TypeScript/Vite, Rust domain/application logic, `rusqlite`, SQLite FTS5, and Chart.js with an accessible table alternative.
- Keep user DB and bundled catalog as separate files. Use immutable user snapshots and schedule versions.
- Store UTC microsecond instants plus original local/zone/offset context and a pinned IANA tzdb version.
- Treat iOS and Android as the first usability/reliability test targets while keeping Windows and macOS as primary deliverables. Release each independently after its gate passes.
- Optimize Today and routine Taken/Skip/Undo actions around the intended primary user's needs before adding advanced reporting.
- No DuckDB, advertising, behavioral analytics, Medtracker account requirement, or foreground-timer-only reminders. A backend is not required unless a later, explicit integration cannot operate without one.
- Start with OS-protected local storage and an explicitly encrypted user backup; decide on SQLCipher only after threat-model and platform-build assessment.
- Start catalog ingestion from a pinned current prescribable RxNorm release candidate, subject to explicit coverage and redistribution approval. Keep manual entry as a complete fallback. Do not assume RxTerms is currently available or redistributable.
- Evaluate Android Health Connect and applicable Google health integration in Phase 0; ship it only if the platform supports useful, consented data exchange without duplicate or medical-meaning ambiguity.

### Decisions dependent on a prototype

- Whether the Tauri scheduled notification plugin meets the intended iOS and Android behavior, including the actual timezone trigger semantics, reboot, request capacity, app termination, exactness, and cancellation behavior.
- Whether device-current IANA zone ID can be read consistently and whether the bundled tzdb/OS zone update lifecycle is acceptable.
- Whether SQLCipher builds cleanly across target mobile and desktop artifacts and whether its licensing/key recovery cost is justified.
- Which desktop notification integrations, if any, can satisfy the product's reminder expectations.
- Which Android and Google health data APIs support the intended medication record and whether Tauri requires a maintained native bridge.

### Decisions requiring product input

- Which of iOS or Android is available first for user testing with the intended primary user.
- Whether the default lock-screen notification should stay generic or allow medication-specific content after explicit opt-in.
- Which MVP dose units and observation inputs are essential; proposed default is free-form user-selected amount/unit with no conversions or clinical interpretation.
- Whether users need a non-encrypted CSV by default or should export only after an explicit sensitivity warning.

### Decisions requiring specialist review

- Catalog terms for the exact release and derived/bundled SQLite output, including source-vocabulary restrictions, attribution, and update-currentness language.
- Intended-use, marketing language, privacy notice/retention, app-store disclosures, backup exclusions, accessibility, and any regulatory review needed for each distribution market.

### High-impact questions

1. Which of iOS or Android should host the first usability prototype with the primary user, while all four platforms remain release targets?
2. What reminder timing tolerance is acceptable, and should reminders be excluded entirely on targets that cannot pass the feasibility gate?
3. Is a U.S. prescription-focused catalog acceptable, or must the first release cover OTC/supplement/device entries that may require another source or manual records?
4. Is read-only Health Connect/Google health import, write-only export, or both needed, and which record types are valuable?
5. Which amount units and user-defined observations are essential in the first useful release?

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
- [IANA Time Zone Database](https://www.iana.org/time-zones) and [`chrono-tz` documentation](https://docs.rs/chrono-tz/latest/chrono_tz/) — political time-rule updates and bundled IANA data/version.
- [SQLite foreign keys](https://www.sqlite.org/foreignkeys.html), [WAL](https://www.sqlite.org/wal.html), and [online backup API](https://www.sqlite.org/backup.html) — enforcement setup, sidecar/checkpoint considerations, and consistent database snapshots.

[^tauri-notifications]: The Tauri plugin's platform table lists iOS, Android, Windows, macOS, and Linux; its `Schedule` API reference explicitly says scheduling is mobile-only and desktop notifications are immediate.
[^tauri-updater]: Tauri documents an optional updater requiring signed update artifacts and configured endpoints. App-store policy and release processes remain separate from the core local ledger.
[^rxnorm-overview]: NLM describes RxNorm as normalized drug names/identifiers covering prescription and many OTC drugs in the U.S.; devices, supplements, foods, bulk powders, contrast media, and homeopathics are out of scope.
[^rxnorm-files]: NLM's current files page lists full RxNorm releases and Current Prescribable Content. As checked 2026-11-14, the current monthly release is dated 2026-11-02; the listing marks Current Prescribable Content as “no license required.” This is a download statement, not a legal determination that every derived redistribution is unrestricted.
[^rxnorm-terms]: NLM says full monthly/weekly RxNorm releases require a free UMLS license, the full dataset includes proprietary source-vocabulary data, product attribution is requested, and republished/redistributed data must be current or clearly identified as not current. Source-vocabulary restrictions may apply.
[^umls-license]: NLM issues UMLS licenses to individuals and directs users to review the license and source-provider restrictions before redistribution in an application.
[^rxnorm-faq]: NLM documents merges, retirement, and history files; retaining local display snapshots avoids retroactive personal-history changes.
[^iana-tzdb]: IANA updates tz data when political bodies change boundaries, UTC offsets, or daylight-saving rules.
[^chrono-tz]: `chrono-tz` documentation describes generated IANA-based timezone data and exposes `IANA_TZDB_VERSION`.
[^sqlite-fk]: SQLite documents that foreign-key enforcement is disabled by default unless the application enables it on each connection.
[^sqlite-backup]: SQLite's online backup API creates a consistent snapshot without relying on copying a live database file.
[^sqlite-wal]: SQLite WAL adds `-wal`/`-shm` sidecars and checkpoints; these affect copying and recovery decisions.
[^android-alarms]: Android documents alarms outside application lifetime and exact-alarm restrictions; delivery can be affected by battery-saving policies.
[^android-permission]: Android 13+ requires runtime notification permission for non-exempt notifications.
[^windows-notifications]: Windows App SDK supports local app notifications, but Tauri plugin coverage does not itself establish scheduled-notification integration.
[^windows-scheduled]: Microsoft documents a five-minute scheduled-notification window and dropping notifications when the computer is off beyond that window.
[^freedesktop-notifications]: The freedesktop standard specifies passive desktop notifications, not a cross-desktop future-time scheduler.
[^gnome-notifications]: GNOME documents desktop-file/D-Bus activation requirements for notifications to be associated with an app when it is not running.
