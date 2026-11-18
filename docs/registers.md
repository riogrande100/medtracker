# MedTracker Registers

Capability register, excluded and deferred features, decision register, and gate register. Requirement IDs `PR-xx` are defined in [product-spec.md](product-spec.md); tickets `Bx-xx` in [backlog.md](backlog.md). Nothing here is implemented or verified unless a linked artifact says so; none currently exists.

## 1. Capability register

Legend — **Planned**: intended, not built. **Gated**: built only after the named gate has an artifact. **Spike**: feasibility unknown until the named ticket runs. **Not promised**: no commitment on that target. **Excluded**: out of scope. Every cell is "not verified" at baseline.

| Capability | Req | iOS | Android | Windows | macOS | Phase / tickets |
|---|---|---|---|---|---|---|
| Open to Today; Due now / Upcoming / Recorded / Earlier-not-logged | PR-19 | Planned | Planned | Planned | Planned | 1 · B1-06 |
| One-step Taken / Skip / Log as needed | PR-20 | Planned | Planned | Planned | Planned | 1 · B1-05, B1-06 |
| Undo (void with audit), restore, Taken↔Skip change | PR-21 | Planned | Planned | Planned | Planned | 1 · B1-05, B1-06, B1-08 |
| Manual medication with immutable snapshots | PR-02 | Planned | Planned | Planned | Planned | 1 · B1-03 |
| Daily/weekday wall-clock schedule (`device_local`, `home_zone`) | PR-03 | Planned | Planned | Planned | Planned | 1 · B1-04 |
| `device_local` promise (needs native IANA zone) | PR-03 | Spike | Spike | Spike | Spike | 0 · B0-04 |
| Edit/pause/resume schedule versions; `elapsed_interval` | PR-03, PR-07 | Planned | Planned | Planned | Planned | 2 · B2-03 |
| History with original / device / selected-zone views | PR-09 | Planned | Planned | Planned | Planned | 1 · B1-08 |
| Reports and charts with table alternatives | PR-08, PR-09 | Planned | Planned | Planned | Planned | 2 · B2-04, B2-05 |
| Notes and observations | PR-04 | Planned | Planned | Planned | Planned | 2 · B2-11 |
| Bundled catalog search | PR-02, PR-25 | Gated (GT-LIC) | Gated (GT-LIC) | Gated (GT-LIC) | Gated (GT-LIC) | 2 · B2-01, B2-02 |
| Scheduled local reminders | PR-13 | Spike (B0-02) → Gated (G0) | Spike (B0-02) → Gated (G0) | Not promised (spike B0-06) | Not promised (spike B0-06) | 2 · B2-06; 4 · B4-02 |
| Notification permission status, generic preview | PR-13 | Planned | Planned | Planned (immediate only) | Planned (immediate only) | 2 · B2-06, B2-08 |
| CSV/JSON export | PR-10 | Planned | Planned | Planned | Planned | 2 · B2-07 |
| Encrypted backup / staged restore | PR-10, PR-12 | Planned | Planned | Planned | Planned | 2 · B2-09 |
| Full data wipe with stated limits | PR-11 | Planned | Planned | Planned | Planned | 2 · B2-08 |
| Database-level encryption at rest | PR-11 | Spike (B0-05) | Spike (B0-05) | Spike (B0-05) | Spike (B0-05) | 0 · B0-05; DF-07 |
| Accent color presets | PR-26 | Planned (Could) | Planned (Could) | Planned (Could) | Planned (Could) | 2 · B2-10 |
| Responsive layout: phone, tablet, desktop window | PR-15 | Planned | Planned | Planned | Planned | 1 · B1-07 |
| Screen-reader and keyboard accessibility | PR-15 | VoiceOver · Planned | TalkBack · Planned | Narrator/other · Planned; keyboard | VoiceOver · Planned; keyboard | 1 · B1-09; 3 · B3-01 |
| Offline operation; empty network allowlist | PR-01, PR-23 | Planned | Planned | Planned | Planned | 0 · B0-07; 2 · B2-12 |
| Accounts, login, subscription, sync, ads, analytics, telemetry | PR-22 | Excluded | Excluded | Excluded | Excluded | — |
| Clinical claims (dosing advice, interactions, diagnosis) | PR-22 | Excluded | Excluded | Excluded | Excluded | — |

## 2. Excluded features (not deferred; no revisit planned)

| ID | Feature | Reason |
|---|---|---|
| EX-01 | Accounts, registration, login, authentication | Latest request; no backend. |
| EX-02 | Subscriptions, payments, paywalls | Latest request. |
| EX-03 | Cloud or arbitrary synchronization; automatic cross-device restore | Latest request; local backup/restore file only. |
| EX-04 | Advertising, analytics, telemetry, crash upload | Latest request; privacy posture. |
| EX-05 | Clinical claims: dose recommendation/calculation, interaction checks, diagnosis, treatment change, adherence coaching, efficacy | Product principle; regulatory exposure. |
| EX-06 | Blocking external integrations (any release gate depending on a reachable third-party service) | Latest request. |
| EX-07 | Runtime medication API; remote fonts, scripts, or styles | PR-17, PR-23. |
| EX-08 | Linux as a primary target, Linux-first development order, Linux-only CI gating | Latest request (D-26). |
| EX-09 | Prescription creation/refill, image/barcode/pill identification, comprehensive drug information | Spec non-goals. |
| EX-10 | Administrator role | No accounts exist to administer. |

## 3. Deferred features

| ID | Feature | Why deferred | Revisit trigger |
|---|---|---|---|
| DF-01 | Caregiver / family persona features (shared or multi-person records) | Needs a sharing model without accounts; out of MVP. | Usability evidence (B3-02) shows demand; separate design. |
| DF-02 | Healthcare-provider access | Clinical workflow; out of scope. | Explicit product decision. |
| DF-03 | Arbitrary recurrence (RRULE) editor | Complexity (spec non-goal). | B4-03 research shows needed patterns. |
| DF-04 | One-tap "Taken" from a notification | Would conflate interaction with a ledger event. | Only with a design that preserves PR-05, reviewed by RV-SEC/RV-UX. |
| DF-05 | Observation charts / co-occurrence overlays | Risk of implied causation. | Descriptive overlay design with non-causality text approved. |
| DF-06 | Windows/macOS scheduled reminders | Official plugin schedules mobile only; needs native integration. | B0-06 + follow-up spike pass (B4-02). |
| DF-07 | SQLCipher/database-level encryption | Build, key-management, and licensing cost. | B0-05 shows the threat model requires it. |
| DF-08 | Optional health-platform integration (for example Android Health Connect), from the sibling docs branch | Optional and non-blocking only; needs consent, provenance, duplicate handling; may or may not use the network. If it needs any endpoint it must pass the PR-23 allowlist process. Proposed ID `PR-28` if approved. | Product decision after Phase 3. |
| DF-09 | OTC/supplement/device catalog beyond RxNorm | Source and licensing unknown. | Product input on catalog scope (D-25). |
| DF-10 | Localization beyond the initial locale | Not needed for core loop. | B4-04. |
| DF-11 | In-app updater | Would add an allowlist entry; stores/installers cover updates. | B4-05 decision. |
| DF-12 | Tablet-specific layouts beyond responsive scaling | Responsive layout suffices for MVP. | Usability findings. |
| DF-13 | Linux support | Non-primary. | Explicit request (B4-06). |

## 4. Decision register

Types: **Decided** (made by the user or by reconciliation), **Recommended** (agent recommendation, reversible), **Prototype-dependent**, **Product input**, **Specialist review**. Reversibility: **Easy** (contained rewrite), **Moderate**, **Hard** (stored-data or release-visible; needs migration).

| ID | Decision | Type | Reversibility | Evidence / trigger | Tickets |
|---|---|---|---|---|---|
| D-01 | Tauri v2, React + TypeScript + Vite, Rust core | Recommended | Moderate (UI framework easy, Tauri/Rust hard) | Phase 0 builds on all four targets | B0-01, B1-01 |
| D-02 | SQLite via `rusqlite`; separate `user-data.sqlite` and read-only `catalog.sqlite`; no DuckDB | Recommended | Hard once users have data | Measured need only | B1-02 |
| D-03 | Time model: UTC microseconds + IANA zone + offset + original local text + tzdb version | Recommended | Hard | B0-04 vectors | B0-04, B1-04 |
| D-04 | Phases 0–4 are the only delivery phases; PO prompt phases become workstreams WS-1…WS-9 | Decided (user) | Easy | — | all |
| D-05 | Four primary targets; no platform first; release order from per-platform evidence | Decided (user) | Easy | Evidence matrices | B0-01, B0-08, B3-10 |
| D-06 | Excluded: accounts, auth, subscriptions, sync, ads, analytics, clinical claims | Decided (user) | Easy to keep, costly to reverse | — | B1-01, B2-08, B2-12 |
| D-07 | No mockup approval gate on Phase 1; stakeholder usability evidence is a Phase 3 gate | Decided (user) | Easy | GT-UX | B1-10, B3-02 |
| D-08 | CI: target-neutral logic tests on cheapest runner each change; per-target builds on demand/pre-release; not a Linux gate | Recommended | Easy | Cost data after B3-08 | B1-01, B3-08 |
| D-09 | No in-app updater in MVP | Recommended | Easy | Keeps allowlist empty | B4-05 |
| D-10 | Undo = `voided` status + `void` revision retaining values; delete stays the privacy action | Decided | Moderate (schema) | B1-05 tests | B1-05 |
| D-11 | Minimal wall-clock schedule + occurrences land in Phase 1; versioning/elapsed in Phase 2 | Decided | Easy | — | B1-04, B2-03 |
| D-12 | "Due" is derived; no due window or grace period | Product input (default: none) | Easy | Usability findings | B1-06 |
| D-13 | Chart.js initially; inline SVG if touch/accessibility/bundle size poor | Recommended | Easy | B2-05 prototype | B2-05 |
| D-14 | Geist-inspired design language; any Geist asset bundled locally after license check | Recommended | Easy | License verified at adoption | B1-07 |
| D-15 | Baseline: OS device protection + sandbox, no SQLCipher; encrypted user backup | Prototype-dependent | Hard after release | B0-05 | B0-05, B2-09 |
| D-16 | macOS host with iOS simulator is the day-to-day development convenience; confers no release priority | Recommended | Easy | — | B0-01 |
| D-17 | Reminder scope per target (full / best-effort / in-app only) | Prototype-dependent | Easy | B0-02, B0-06, G0 | B0-08, B2-06 |
| D-18 | Claude model tiers T1/T2/T3 and author≠reviewer separation | Decided | Easy | — | all |
| D-19 | Accent presets are Could and the first cut under schedule pressure | Recommended | Easy | — | B2-10 |
| D-20 | Catalog source candidate: pinned RxNorm Current Prescribable Content release | Specialist review | Easy until bundled | GT-LIC | B0-03 |
| D-21 | Generic lock-screen notification text by default | Product input (spec recommends generic) | Easy | Product owner | B2-06 |
| D-22 | Whether `device_local` mode is promised per target | Prototype-dependent | Moderate | B0-04 probe | B0-04, B1-04 |
| D-23 | Plain-export default/warning policy | Product input | Easy | Product owner | B2-07 |
| D-24 | Essential dose units and observation inputs (default: free-form amount/unit) | Product input | Moderate | Product owner | B1-05, B2-11 |
| D-25 | U.S. prescription-focused catalog scope acceptable | Product input | Easy | Product owner | B0-03 |
| D-26 | Linux is non-primary and gates nothing | Decided (user) | Easy | — | B4-06 |
| D-27 | Runtime network allowlist empty by default; additions follow the PR-23 process | Decided (user) | Easy | B0-07, B2-12 | B0-07, B1-01, B2-12 |

### Open questions needing the product owner (at most five)

1. Who provides, and when, the physical iOS and Android devices, a Windows host, a macOS host, and the developer-program/signing accounts for each store? (Without them GT-DEV-* and GT-SIGN/GT-STORE stay NOT PASSED.)
2. Who are the stakeholder participants for GT-UX and what counts as "unaided success" for a non-technical user?
3. What reminder timing tolerance is acceptable, and should reminders be omitted on targets that fail their spike?
4. Is a U.S. prescription-focused catalog acceptable for the first release (D-25)?
5. Must the live database be encrypted against offline file theft, or are OS protection plus encrypted user backups acceptable (D-15)?

## 5. Gate register

All gates are **NOT PASSED — evidence-dependent** at baseline. A gate flips to Passed only when a reviewer (not the author) links an artifact in `docs/evidence/`.

| Gate | Meaning | Evidence required | Owner | Tickets |
|---|---|---|---|---|
| G0 | Phase 0 exit: per-platform reminder scope, catalog status, privacy stance accepted | B0-08 synthesis + product-owner acceptance | Product owner | B0-08 |
| G1 | Phase 1 exit: core loop passes automated tests on every target with a build | B1-01…B1-09 test logs + RV records | Product owner | B1-09 |
| G2 | Phase 2 exit: MVP requirement tests and reviews | B2 test logs + RV records | Product owner | B2-12 |
| G3 | Per-target release readiness | Complete matrix in [release-evidence.md](release-evidence.md) | Product owner | B3-10 |
| G4 | Every primary target at G3 or accepted exception | Matrix + decision | Product owner | B4-01 |
| GT-DEV-IOS | Physical iPhone/iPad run of notification, install, accessibility scenarios | Evidence tables from a real device (simulator never counts) | H | B0-02, B3-01 |
| GT-DEV-AND | Physical Android run including Doze, exact-alarm, reboot | Evidence tables from a real device (emulator never counts) | H | B0-02, B3-01 |
| GT-DEV-WIN | Real Windows host install and scenarios | Evidence from a real host; VM recorded as VM | H | B0-01, B0-06 |
| GT-DEV-MAC | Real Mac install and scenarios | Evidence from a real host | H | B0-01, B0-06 |
| GT-LIC | Catalog redistribution approved (spec §6 checklist) | Written release-specific approval by a named human | H specialist | B0-03 |
| GT-UX | Non-technical users complete the core loop unaided | Session records vs. pre-approved threshold | H | B1-10, B3-02 |
| GT-A11Y | WCAG 2.2 AA verified per target with assistive technology | Audit reports per target | H + T2 | B3-01 |
| GT-NET | Zero outbound connections per target; empty allowlist holds | Egress captures per target | T2 + reviewer | B0-07, B2-12 |
| GT-REC | Upgrade, failed-migration recovery, restore, wipe per target | Test logs per target | T2 + reviewer | B3-07 |
| GT-SIGN | Signed installable artifact per target; costs/renewal/ownership documented | Signed artifact install record | H | B3-05 |
| GT-STORE | Distribution channel and disclosures accepted per target | Submission/beta records | H | B3-06 |
| GT-LEGAL | Intended-use, privacy, disclosure, attribution review | Written specialist review | H specialist | B3-09 |
