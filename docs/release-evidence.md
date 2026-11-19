# MedTracker Release Evidence Matrices

**Status at baseline (2026-11-18, amended 2026-11-18): every iOS and Android cell below is NOT PASSED — evidence-dependent. Windows and macOS cells are DEFERRED by owner decision D-29 (first release is mobile only); DEFERRED means neither passed nor waived, and no Windows/macOS cell may be set to Passed unless the desktop re-entry criterion in D-29 is met and the cell is re-opened.** A Phase 1 slice exists on `main` and its handoff records host-side test results (8/8 UI tests, 10/10 Rust tests), but no device, installed-package, signing, store, usability, or legal artifact exists for any target ([current-state.md](current-state.md)). Host-side unit tests are not evidence for any cell below.

Cell values: **Not passed** (no artifact), **Blocked: <reason>** (author-set), **Passed — <artifact link>** (reviewer-set only; see [backlog.md](backlog.md#review-separation)). Simulator, emulator, and VM runs are recorded as such and **never** satisfy a physical-device or real-host cell. Release order between iOS and Android is decided only from these matrices (PR-16, PR-24); neither is ahead by assumption; Linux is not part of them. The Windows and macOS columns and sections are retained for the deferred desktop work (DF-14, DF-15, DF-16; DT-01..DT-07).

A target is releasable (G3) only when **every** applicable row for that target is Passed. For the first release only iOS and Android are in scope.

## 1. Shared evidence matrix

| # | Evidence item | Req / gate | Ticket | iOS | Android | Windows (DEFERRED, D-29) | macOS (DEFERRED, D-29) |
|---|---|---|---|---|---|---|---|
| 1 | Clean-checkout build and install of the release configuration | PR-24 | B0-01, B3-08 | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| 2 | Real device / real host launch and app-data path recorded | GT-DEV-* | B0-01 | Not passed (physical device) | Not passed (physical device) | DEFERRED (D-29) | DEFERRED (D-29) |
| 3 | Core loop on target: Today → Taken → Undo; Skip → Undo; Taken↔Skip; restart persistence | PR-19–PR-21 | B1-09 | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| 4 | Network-isolation: egress capture shows zero connections while the allowlist is empty; core loop works with the network blocked | PR-01, PR-23, GT-NET | B0-07, B2-12 | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| 5 | Accessibility audit with the target's assistive technology | PR-15, GT-A11Y | B3-01 | Not passed (VoiceOver) | Not passed (TalkBack) | DEFERRED (D-29) | DEFERRED (D-29) |
| 6 | Stakeholder usability on this form factor | PR-27, GT-UX | B3-02 | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| 7 | Scheduled-reminder evidence and verdict (full / best-effort / in-app only) | PR-13 | B0-02, B0-06, B2-06 | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| 8 | Time-zone/DST behavior (engine per D-04a, tzdb version recorded) and native zone probe | PR-03, PR-04, D-04a, D-22 | B0-04 | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| 9 | Privacy: data location, OS-backup behavior, log redaction, notification preview | PR-11, PR-18 | B0-05, B3-04 | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| 10 | Backup, restore, upgrade, failed-migration recovery, full wipe | PR-10–PR-12, GT-REC | B2-09, B3-07 | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| 11 | Performance and size baselines recorded (measurements, no promises) | PR-16 | B3-03 | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| 12 | Signed installable artifact installed on a clean device/host | GT-SIGN | B3-05 | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| 13 | Distribution channel accepted; disclosures match behavior | GT-STORE | B3-06 | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| 14 | Legal / intended-use / privacy-notice review | PR-18, GT-LEGAL | B3-09 | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| 15 | Catalog included **only if** GT-LIC passed; otherwise confirmed absent | PR-25, GT-LIC | B0-03, B2-02 | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| 16 | Security review findings dispositioned | PR-22, PR-23 | B3-04 | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| 17 | Release checklist signed off | G3 | B3-10 | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| 18 | Health-platform integration: **not pursued (D-30)**; no HealthKit or Health Connect code, permission, or import path is planned. Absence to be confirmed in the B3-04 security review | PR-28 (withdrawn), GT-HEALTH (not applicable) | B0-09 (closed), B4-07 (closed) | Not applicable (D-30) | Not applicable (D-30) | Not applicable | Not applicable |
| 19 | System file picker / share sheet for export and backup works on the device; temp files cleaned; cancel leaves data untouched | PR-10, PR-12, GT-REC | B3-11 | Not passed (physical device) | Not passed (physical device) | DEFERRED (D-29) | DEFERRED (D-29) |

## 2. Platform-specific evidence

### iOS

| Item | What must be shown | Status |
|---|---|---|
| Toolchain and provisioning | Xcode build of the release configuration; provisioning/team requirements documented (Apple developer-program enrollment is a human-owned step) | Not passed |
| Notifications (UserNotifications) | Spec §8 scenarios on a physical device: locked, foreground, background, terminated, reboot, permission denied/re-enabled, Focus suppression noted, zone change, DST, clock jump; no dose written on tap | Not passed |
| Backup behavior | What iCloud/device backup does to the app's database and key material, verified by test; exclusion applied where the platform allows | Not passed (unverified) |
| Distribution | Beta/test channel and store submission path, privacy disclosures matching actual behavior (currently no data collected, no runtime network); costs and renewal recorded | Not passed |
| Accessibility | VoiceOver, Dynamic Type, Reduce Motion, contrast, touch targets | Not passed |

### Android

| Item | What must be shown | Status |
|---|---|---|
| Toolchain | Android SDK/NDK/JDK present and documented (not present on the planning host); release build installs on a physical device | Blocked: no SDK/Java on planning host; setup tracked by B0-10 |
| Notifications and alarms | Runtime notification permission, exact-alarm access granted/missing, Doze, battery saver, reboot, force-stop, zone change on a physical device; plugin option ≠ exact alarm unless demonstrated | Not passed |
| Backup behavior | Auto Backup/OS backup behavior for the database and keys verified by test; exclusion configured and tested | Not passed (unverified) |
| Distribution | Play App Signing/keystore custody, build integrity, store listing and data-safety disclosures; costs recorded | Not passed |
| Accessibility | TalkBack, font scaling, contrast, touch targets | Not passed |

### Windows (DEFERRED by D-29, DF-14)

| Item | What must be shown | Status |
|---|---|---|
| Host and toolchain | Real Windows host (none available to the planning session); build and install of the installer/package form chosen | DEFERRED (D-29) |
| Notifications | Immediate notification behavior for an installed app; native scheduling feasibility (B0-06) incl. app closed, sleep, restart; **no scheduled-reminder promise** unless this passes | DEFERRED (D-29) |
| App data and backup | Per-user app-data path, file-history/OS backup behavior, uninstall/reinstall data expectations | DEFERRED (D-29) |
| Signing and distribution | Code-signing mechanism, certificate source/cost/renewal, SmartScreen/store implications — candidates to verify, not decisions | DEFERRED (D-29) |
| Accessibility | Keyboard-only operation, Windows screen reader, high-contrast and text scaling | DEFERRED (D-29) |

### macOS desktop app (DEFERRED by D-29, DF-15; the Mac remains the iOS build/signing host)

| Item | What must be shown | Status |
|---|---|---|
| Host and toolchain | Real Mac build and install of the release configuration, Apple silicon and Intel as supported by the chosen artifact | DEFERRED (D-29) |
| Notifications | Immediate behavior; native scheduling feasibility (B0-06) incl. app closed, sleep/wake, restart; no scheduled-reminder promise unless it passes | DEFERRED (D-29) |
| App data and backup | Container/app-support path, Time Machine/iCloud behavior, uninstall data expectations | DEFERRED (D-29) |
| Signing and distribution | Developer signing, notarization, and store-versus-direct distribution path — candidates to verify; costs and renewal recorded | DEFERRED (D-29) |
| Accessibility | Keyboard-only, VoiceOver, text size, contrast, reduce motion | DEFERRED (D-29) |

## 3. Catalog licensing gate (GT-LIC)

| Check | Evidence | Status |
|---|---|---|
| Exact source release, URL, date, checksum recorded | B0-03 artifact | Not passed |
| Source terms read against exact fields/term types; whitelist approved | B0-03 artifact | Not passed |
| Redistribution of the derived bundled SQLite in each store approved by a named human | Written approval | Not passed |
| Attribution and currentness text drafted and placed in About | Screenshot/text | Not passed |
| Update policy: catalog changes only via app releases | Release notes | Not passed |
| Distributed builds contain no real catalog data until all above pass | Build inspection (PR-25 test) | Not passed |

## 4. Network allowlist verification (currently empty)

| Check | Evidence | Status |
|---|---|---|
| CSP and Tauri capabilities contain no remote origin or network plugin | Config review + automated allowlist-diff test (B2-12) | Not passed |
| No HTTP client, updater, analytics, or sync dependency | Dependency scan | Not passed |
| Egress capture across the full journey set per target | Capture logs per target | Not passed |
| Distribution/updates occur through the store or installer, outside the app | Release process doc | Not passed |
| If an allowlist entry is ever added: register entry, minimum-scope origin, timeout/invalid-response/revoked-consent/retry tests, and proof that local logging still works with that endpoint failing | Register + test logs | Not applicable until an entry exists |

## 5. Privacy, backup, deletion, and recovery verification

| Check | iOS | Android | Windows (DEFERRED, D-29) | macOS (DEFERRED, D-29) |
|---|---|---|---|---|
| Database and keys live only in the private app-data area | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| OS/cloud backup exclusion behavior tested (not assumed) | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| Logs contain no medication, dose, note, path, or DB content (redaction test) | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| Generic notification preview by default; detail only by opt-in | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| Encrypted backup: wrong passphrase, truncation, malformed manifest, newer format, disk-full | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| Restore staged, safety snapshot, atomic swap | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| Upgrade from prior DB preserves records; failed migration keeps original | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| Full wipe removes DB, sidecars, keys, temp files; limits text shown | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |
| Statement of what the app cannot erase (OS backups, exports, snapshots) shown to user | Not passed | Not passed | DEFERRED (D-29) | DEFERRED (D-29) |

## 6. Updating this document

- Authors may add artifact links and set **Blocked** with a reason.
- Only a reviewer who did not produce the artifact may set **Passed**, linking the file under `docs/evidence/<ticket>/`.
- Removing a row requires a decision-register entry in [registers.md](registers.md).
