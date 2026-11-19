# MedTracker Registers

Capability register, excluded and deferred features, decision register, and gate register. Requirement IDs `PR-xx` are defined in [product-spec.md](product-spec.md); tickets `Bx-xx` in [backlog.md](backlog.md). Nothing here is implemented or verified unless a linked artifact says so; none currently exists.

## 1. Capability register

Legend — **Planned**: intended, not built. **Gated**: built only after the named gate has an artifact. **Spike**: feasibility unknown until the named ticket runs. **Not promised**: no commitment on that target. **Excluded**: out of scope. Every cell is "not verified" at baseline. **Windows and macOS columns are DEFERRED by owner decision D-29 (2026-11-19): first release is mobile only (iOS + Android). Cells in those columns describe deferred, non-release intent and carry no release commitment.**

| Capability | Req | iOS | Android | Windows (DEFERRED, D-29) | macOS (DEFERRED, D-29) | Phase / tickets |
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
| Bundled U.S. catalog search-as-you-type (preferred medication-entry speed-up, D-30, U.S.-only D-31); manual entry always available | PR-02, PR-25 | Gated (GT-LIC) | Gated (GT-LIC) | Gated (GT-LIC) | Gated (GT-LIC) | 2 · B2-01, B2-02 (next planned work) |
| Scheduled local reminders | PR-13 | Spike (B0-02) → Gated (G0) | Spike (B0-02) → Gated (G0) | Not promised (spike B0-06) | Not promised (spike B0-06) | 2 · B2-06; 4 · B4-02 |
| Notification permission status, generic preview | PR-13 | Planned | Planned | Planned (immediate only) | Planned (immediate only) | 2 · B2-06, B2-08 |
| CSV/JSON export | PR-10 | Planned | Planned | Planned | Planned | 2 · B2-07 |
| Encrypted backup / staged restore | PR-10, PR-12 | Planned | Planned | Planned | Planned | 2 · B2-09 |
| Full data wipe with stated limits | PR-11 | Planned | Planned | Planned | Planned | 2 · B2-08 |
| Database-level encryption at rest | PR-11 | Spike (B0-05) | Spike (B0-05) | Spike (B0-05) | Spike (B0-05) | 0 · B0-05; DF-07 |
| Accent color presets | PR-26 | Planned (Could) | Planned (Could) | Planned (Could) | Planned (Could) | 2 · B2-10 |
| Responsive layout: phone, tablet, desktop window | PR-15 | Planned | Planned | Planned | Planned | 1 · B1-07 |
| Screen-reader and keyboard accessibility | PR-15 | VoiceOver · Planned | TalkBack · Planned | Narrator/other · Planned; keyboard | VoiceOver · Planned; keyboard | 1 · B1-09; 3 · B3-01 |
| Core loop works with no connectivity; runtime network allowlist currently empty | PR-01, PR-23 | Planned | Planned | Planned | Planned | 0 · B0-07; 2 · B2-12 |
| Health-platform integration (HealthKit / Android-Google health): NOT PURSUED (D-30) | PR-28 | Not pursued (D-30) | Not pursued (D-30) | Not applicable | Not applicable | 0 · B0-09 (closed); 4 · B4-07 (closed); DF-08 (closed) |
| Accounts, login, subscription, arbitrary sync, ads, analytics, telemetry | PR-22 | Excluded | Excluded | Excluded | Excluded | — |
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
| EX-07 | Runtime medication API; remote fonts, scripts, or styles | PR-17, PR-23. (Other web access is allowed only through the reviewed allowlist, currently empty.) |
| EX-08 | Linux as a primary target, Linux-first development order, Linux-only CI gating | Latest request (D-26). |
| EX-09 | Prescription creation/refill, image/barcode/pill identification, comprehensive drug information | Spec non-goals. |
| EX-10 | Administrator role | No accounts exist to administer. |
| EX-11 | Health-platform integration (HealthKit, Android Health Connect / Google health): import, export, or permission flows | Closed by owner decision D-30 (2026-11-19): standalone app, not pursued. Re-entry only under the D-30 criterion. |

## 3. Deferred features

| ID | Feature | Why deferred | Revisit trigger |
|---|---|---|---|
| DF-01 | Caregiver / family persona features (shared or multi-person records) | Needs a sharing model without accounts; out of MVP. | Usability evidence (B3-02) shows demand; separate design. |
| DF-02 | Healthcare-provider access | Clinical workflow; out of scope. | Explicit product decision. |
| DF-03 | Arbitrary recurrence (RRULE) editor | Complexity (spec non-goal). | B4-03 research shows needed patterns. |
| DF-04 | One-tap "Taken" from a notification | Would conflate interaction with a ledger event. | Only with a design that preserves PR-05, reviewed by RV-SEC/RV-UX. |
| DF-05 | Observation charts / co-occurrence overlays | Risk of implied causation. | Descriptive overlay design with non-causality text approved. |
| DF-06 | Windows/macOS scheduled reminders | Official plugin schedules mobile only; needs native integration. Now also subsumed by the desktop deferral (D-29, DF-14..DF-16). | Desktop re-entry criterion (D-29) plus B0-06 / REM-WIN / REM-MAC follow-up spike pass (DT-06). |
| DF-07 | SQLCipher/database-level encryption | Build, key-management, and licensing cost. | B0-05 shows the threat model requires it. |
| DF-08 | Optional health-platform integration (HealthKit, Android Health Connect / Google health) | **CLOSED by owner decision D-30 (2026-11-19): not pursued.** Original reasons: consent, provenance, duplicate handling; platform acceptance never proves a dose; any endpoint would need the PR-23 allowlist process. | Closed. Re-entry only under the D-30 criterion (explicit owner request AND a need a bundled catalog cannot meet AND all HLT-* scenarios PASS on physical devices). |
| DF-09 | OTC/vitamin/supplement/device catalog beyond RxNorm | MVP catalog (RxTerms/RxNorm) excludes dietary supplements; vitamin D alone is largely absent (only combination products). NIH DSLD is the only supplements candidate; its terms are **UNVERIFIED** (fetch blocked). | Post-MVP: DSLD terms review under GT-LIC-style process; until then manual entry (D-31). |
| DF-10 | Localization beyond the initial locale | Not needed for core loop. | B4-04. |
| DF-11 | In-app updater | Would add an allowlist entry; stores/installers cover updates. | B4-05 decision. |
| DF-12 | Tablet-specific layouts beyond responsive scaling | Responsive layout suffices for MVP. | Usability findings. |
| DF-13 | Linux support | Non-primary. | Explicit request (B4-06). |
| DF-14 | Windows desktop release (build, installer, signing, store/direct distribution, evidence matrix) | Owner decision D-29 (2026-11-19): first release is mobile only; cost and focus; no Windows host exists; Windows scheduled reminders already NO-GO under the Tauri notification plugin. Owner: product owner. | Mobile G3 passed AND explicit owner request AND native desktop reminder spike REM-WIN passes (or owner accepts no scheduled reminders on Windows) AND a Windows host and signing path are available. Tickets: DT-01, DT-04, DT-05. |
| DF-15 | macOS desktop release (build, signing, notarization, distribution, evidence matrix). The macOS dev host remains in use as the iOS build/signing host and for fast tests; this deferral covers only a macOS app release. | Owner decision D-29: cost and focus; no desktop signing/notarization in the first release; macOS scheduled reminders already NO-GO under the Tauri notification plugin. Owner: product owner. | Mobile G3 passed AND explicit owner request AND native desktop reminder spike REM-MAC passes (or owner accepts no scheduled reminders on macOS) AND Mac desktop signing/notarization is available. Tickets: DT-02, DT-04, DT-05. |
| DF-16 | Desktop usability and parity work: desktop window layouts, keyboard-first navigation, desktop screen-reader audits (Windows screen reader, macOS VoiceOver), desktop reminders (supersedes DF-06 scope for release), desktop packaging smoke tests | Owner decision D-29; no desktop release. Code keeps building on the macOS dev host for iteration and tests only. | Same re-entry criterion as DF-14/DF-15. Tickets: DT-03, DT-06, DT-07. |

## 4. Decision register

Types: **Decided** (made by the user or by reconciliation), **Recommended** (agent recommendation, reversible), **Prototype-dependent**, **Product input**, **Specialist review**. Reversibility: **Easy** (contained rewrite), **Moderate**, **Hard** (stored-data or release-visible; needs migration).

| ID | Decision | Type | Reversibility | Evidence / trigger | Tickets |
|---|---|---|---|---|---|
| D-01 | Tauri 2, React + TypeScript + Vite, Rust core | Recommended | Moderate (UI framework easy, Tauri/Rust hard) | Phase 0 builds on all four targets | B0-01, B1-01 |
| D-02 | SQLite via `rusqlite`; separate `user-data.sqlite` and read-only `catalog.sqlite`; no DuckDB | Recommended | Hard once users have data | Measured need only | B1-02 |
| D-03 | Time model: UTC microseconds + IANA zone + offset + original local text + tzdb version | Recommended | Hard | B0-04 vectors | B0-04, B1-04 |
| D-04 | Phases 0–4 are the only delivery phases; PO prompt phases become workstreams WS-1…WS-9 (mapping table: [product-spec.md](product-spec.md#workstream-mapping)) | Decided (user) | Easy | — | all |
| D-04a | Time engine: `jiff` 1.x with bundled tzdb, replacing the earlier `chrono-tz` suggestion. Reason: Phase 0 spike found `chrono-tz` 0.10.4 bundles tzdb 2025b, stale against IANA 2026f. Library decision from the spike, not a device claim; currency re-checked at pin time and tzdb version recorded per occurrence | Recommended (spike finding) | Moderate (engine swappable behind `time/`; stored tzdb versions are Hard) | B0-04 vectors | B0-04, B1-04 |
| D-05 | Four primary targets; no platform first; release order from per-platform evidence. **Amended for the first release by D-29: mobile only (iOS + Android); Windows/macOS deferred.** | Decided (user) | Easy | Evidence matrices | B0-01, B0-08, B3-10 |
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
| D-16 | macOS host with iOS simulator is the day-to-day development convenience and (per D-29) the iOS build/signing host; confers no desktop release priority | Recommended | Easy | — | B0-01 |
| D-17 | Reminder scope per target (full / best-effort / in-app only); first release covers iOS and Android only (D-29) | Prototype-dependent | Easy | B0-02, B0-06, G0 | B0-08, B2-06 |
| D-18 | Model policy: T1 = Haiku 5.5, T2 = Sonnet 5.5, T3 = Sonnet 5.5 with high reasoning (required for DST/time-zone, backup, security, architecture); author≠reviewer separation | Decided | Easy | — | all |
| D-19 | Accent presets are Could and the first cut under schedule pressure | Recommended | Easy | — | B2-10 |
| D-20 | Catalog source candidate: pinned RxNorm Current Prescribable Content release. **Updated 2026-11-19 by [spikes/catalog/FINDINGS.md](../spikes/catalog/FINDINGS.md) (research only):** recommended MVP source is **RxTerms (monthly)** for search and display, with NLM RxNorm Current Prescribable Content as the provenance baseline and RXCUI cross-check. Excluded: full RxNorm (UMLS license), RxClass/SNOMED, NDC package rows, DailyMed label text, NLM logo. RxTerms redistribution terms are **UNVERIFIED** on the primary page. Not approved until GT-LIC passes. | Specialist review | Easy until bundled | GT-LIC | B0-03, B2-01 |
| D-21 | Generic lock-screen notification text by default | Product input (spec recommends generic) | Easy | Product owner | B2-06 |
| D-22 | Whether `device_local` mode is promised per target | Prototype-dependent | Moderate | B0-04 probe | B0-04, B1-04 |
| D-23 | Plain-export default/warning policy | Product input | Easy | Product owner | B2-07 |
| D-24 | Essential dose units and observation inputs (default: free-form amount/unit) | Product input | Moderate | Product owner | B1-05, B2-11 |
| D-25 | U.S. prescription-focused catalog scope acceptable. **Reconciled by D-31 (2026-11-19):** U.S. scope and prescription-focused MVP catalog accepted; OTC/vitamins/supplements are not in the MVP catalog and rely on manual entry. Scope question stays open only for supplements (see DF-09). | Product input (reconciled by D-31) | Easy | Product owner | B0-03 |
| D-26 | Linux is non-primary and gates nothing | Decided (user) | Easy | — | B4-06 |
| D-27 | Web access only for reviewed features via an explicit runtime allowlist, currently empty; additions follow the PR-23 process; connectivity failure never blocks local logging | Decided (user) | Easy | B0-07, B2-12 | B0-07, B1-01, B2-12 |
| D-28 | HealthKit and Android/Google health are optional, separately authorized feasibility work; they never block the MVP and an external platform's acceptance never proves a dose was taken | Decided (user); **superseded by D-30** (2026-11-19: not pursued) | Easy | B0-09 | B0-09 (closed), B4-07 (closed) |
| D-29 | **First release is MOBILE ONLY (iOS + Android).** Windows and macOS desktop are DEFERRED, not removed: the code keeps building on the macOS dev host for fast iteration and tests, but there is no desktop release, no desktop signing/notarization, no Windows host, and no desktop usability/parity work in the release. This amends D-05 (four primary targets) and the original definition of done (four signed builds) for the first release. Rationale: cost and focus; Windows/macOS scheduled reminders were already NO-GO under the Tauri notification plugin. Re-entry: mobile G3 passed AND explicit owner request AND native desktop reminder spike REM-WIN/REM-MAC passes AND a Windows host plus Mac desktop signing are available. The macOS host remains the iOS build/signing host. Dated 2026-11-19. | Decided (owner decision) | Easy (nothing removed; deferred tickets DT-xx and DF-14..DF-16 are retained) | Re-entry criterion above | B0-01, B0-06, B3-05, B3-10, B4-01, DT-01..DT-07 |
| D-31 | **First release is U.S.-ONLY.** U.S. English only; U.S. medication catalog only; U.S. date, time and measurement conventions by default (device settings still respected); no localization work; no non-U.S. regulatory or store-region work. Reconciled with D-25: the U.S. scope is decided, and the MVP catalog is prescription-focused. **OTC, vitamins and supplements are NOT covered by the MVP catalog source** (RxNorm does not cover dietary supplements), so manual entry must be easy for them; a supplements source (NIH DSLD) is a post-MVP item whose terms are **UNVERIFIED**. Re-entry for another region: explicit owner request AND a catalog source AND legal review for that region. Dated 2026-11-19. | Decided (owner decision) | Easy (no localization built; DF-10 stays deferred) | Re-entry criterion above | D-25, D-20, D-30, DF-09, DF-10, B0-03, B2-02, GT-LIC |
| D-30 | **MedTracker is a STANDALONE app. Apple Health / HealthKit and Android Health Connect / Google health integrations are NOT PURSUED (closed, not merely deferred).** Basis: the B0-09 simulator spike ([spikes/healthkit-ios/FINDINGS.md](../spikes/healthkit-ios/FINDINGS.md), 2026-11-18; simulator only, no physical-device evidence): (1) import would only copy medication names; (2) denied, ungranted and empty are indistinguishable; (3) third-party apps cannot write dose events; (4) it would need physical-device proof, a privacy policy and a security review; (5) a bundled medication list gives the same convenience offline. Re-entry criterion (all required): an explicit owner request AND a user need that a bundled catalog cannot meet AND all HLT-* scenarios passing on physical devices. External platform acceptance never proves a dose was taken. Preferred direction: a bundled offline medication catalog to speed up medication entry (search-as-you-type; still gated by GT-LIC, D-25 US scope; manual entry always available; ticket B2-02). Supersedes D-28. Dated 2026-11-19. | Decided (owner decision) | Easy (nothing shipped; no health code in the product) | Re-entry criterion above | B0-09 (closed), B4-07 (closed, not pursued), DF-08 (closed), GT-HEALTH (not applicable), B2-02 |

### Open questions needing the product owner

1. Who provides, and when, the physical iOS and Android devices, the Android SDK/NDK toolchain, and the Apple Developer and Google Play accounts? (Without them GT-DEV-IOS/AND and GT-SIGN/GT-STORE stay NOT PASSED.) Windows host and Mac desktop signing are deferred (D-29).
2. Who are the stakeholder participants for GT-UX and what counts as "unaided success" for a non-technical user?
3. What reminder timing tolerance is acceptable, and should reminders be omitted on targets that fail their spike?
4. Closed by D-31 (2026-11-19): the first release is U.S.-only and the MVP catalog is prescription-focused (D-25 reconciled). Supplements remain open under DF-09.
5. Must the live database be encrypted against offline file theft, or are OS protection plus encrypted user backups acceptable (D-15)?
6. Closed by D-30: health-platform integration is not pursued (standalone app).

## 5. Gate register

All gates are **NOT PASSED — evidence-dependent** at baseline, except GT-DEV-WIN and GT-DEV-MAC, which are **DEFERRED by owner decision D-29** (they are not passed and not waived; they re-open on the desktop re-entry criterion). For the first release, G3 and G4 are evaluated on iOS and Android only. A gate flips to Passed only when a reviewer (not the author) links an artifact in `docs/evidence/`.

| Gate | Meaning | Evidence required | Owner | Tickets |
|---|---|---|---|---|
| G0 | Phase 0 exit: per-platform reminder scope, catalog status, privacy stance accepted | B0-08 synthesis + product-owner acceptance | Product owner | B0-08 |
| G1 | Phase 1 exit: core loop passes automated tests on every in-scope target with a build (macOS dev host, iOS, Android; desktop is deferred, D-29) | B1-01…B1-09 test logs + RV records | Product owner | B1-09 |
| G2 | Phase 2 exit: MVP requirement tests and reviews | B2 test logs + RV records | Product owner | B2-12 |
| G3 | Per-target release readiness for iOS and Android (Windows/macOS deferred, D-29) | Complete iOS and Android matrices in [release-evidence.md](release-evidence.md) | Product owner | B3-10 |
| G4 | Every in-scope (mobile) target at G3 or accepted exception | Matrix + decision | Product owner | B4-01 |
| GT-DEV-IOS | Physical iPhone/iPad run of notification, install, accessibility scenarios | Evidence tables from a real device (simulator never counts) | H | B0-02, B3-01 |
| GT-DEV-AND | Physical Android run including Doze, exact-alarm, reboot | Evidence tables from a real device (emulator never counts) | H | B0-02, B3-01 |
| GT-DEV-WIN | **DEFERRED (D-29, DF-14).** Real Windows host install and scenarios | Evidence from a real host; VM recorded as VM. Not required for the first release | H | DT-01, DT-06 |
| GT-DEV-MAC | **DEFERRED (D-29, DF-15).** Real Mac desktop-app install and scenarios | Evidence from a real host. Not required for the first release | H | DT-02, DT-06 |
| GT-LIC | Catalog redistribution approved (spec §6 checklist; 10-item checklist in [spikes/catalog/FINDINGS.md](../spikes/catalog/FINDINGS.md) §5). **NOT PASSED.** The 2026-11-19 catalog spike is research only and does not pass it. | Written release-specific approval by a named human (owner/legal review); reviewer-linked artifact in `docs/evidence/` | H specialist (owner/legal review) | B0-03 |
| GT-UX | Non-technical users complete the core loop unaided | Session records vs. pre-approved threshold | H | B1-10, B3-02 |
| GT-A11Y | WCAG 2.2 AA verified per mobile target (VoiceOver, TalkBack) with assistive technology | Audit reports per target | H + T2 | B3-01 |
| GT-NET | Zero outbound connections per target; empty allowlist holds | Egress captures per target | T2 + reviewer | B0-07, B2-12 |
| GT-REC | Upgrade, failed-migration recovery, restore, wipe per target | Test logs per target | T2 + reviewer | B3-07 |
| GT-SIGN | Signed installable artifact per in-scope (mobile) target; costs/renewal/ownership documented | Signed artifact install record | H | B3-05 |
| GT-STORE | Distribution channel and disclosures accepted per target | Submission/beta records | H | B3-06 |
| GT-LEGAL | Intended-use, privacy, disclosure, attribution review | Written specialist review | H specialist | B3-09 |
| GT-HEALTH | **NOT APPLICABLE by owner decision D-30 (2026-11-19).** Health-platform integration is not pursued, so this gate is neither required nor passed nor waived for any release. | None; any future health gate would come from a new decision under the D-30 re-entry criterion | — | B0-09 (closed), B4-07 (closed) |
