# B0-09 iOS HealthKit medications feasibility: findings

Date: 2026-11-18. Authorized by the product owner. Environment: Xcode 27.2, iOS 27.2 **simulator** (iPhone 17), fake data only.
**Nothing here is physical-device evidence. No HLT-IOS scenario is PASS.** Raw evidence: `evidence/` (11 screenshots, 6 console logs, `sim-results.json`).

Labels: DOCUMENTED (Apple docs/SDK headers), SIMULATOR-VERIFIED (observed in `evidence/`), UNVERIFIED.

## Capability matrix

| Capability | Result | Label | Evidence |
|---|---|---|---|
| HealthKit available | `isHealthDataAvailable = true` | SIMULATOR-VERIFIED | run4 |
| Read medication list | Works after the user grants per-object access; returned the fake medication (name, archived flag, hasSchedule) | SIMULATOR-VERIFIED | run4, screenshots 09, 11 |
| Authorization UX | System "Medications Access" sheet lists each medication with a toggle and our usage text | SIMULATOR-VERIFIED | screenshots 02, 05, 08 |
| Read dose events | Works after the medication grant; returned one event (logStatus taken, scheduleType schedule) | SIMULATOR-VERIFIED | run4, screenshot 11 |
| Dose-event authorization | Follows the medication grant. Requesting read authorization for the dose-event type directly throws an uncatchable `NSInvalidArgumentException` | SIMULATOR-VERIFIED | run1, run-auth-read-dose |
| Write dose events (third-party) | **Not possible.** Requesting share authorization throws; `HKMedicationDoseEvent.init` is unavailable in the SDK headers | SIMULATOR-VERIFIED + DOCUMENTED (headers) | run-write-probe |
| Denied vs ungranted vs empty | **Indistinguishable**: all return 0 results with no error | SIMULATOR-VERIFIED | run2, run3 |
| Repeat request after "Don't Allow" | Returns OK with no sheet; the user must change access in Health | SIMULATOR-VERIFIED | screenshot 06 |
| Grant lifecycle | Uninstall and reinstall resets the grant | SIMULATOR-VERIFIED | |
| Revoke while running, locked device, deleted medication | Not run | UNVERIFIED | |
| Background delivery for medication types | Not documented | UNVERIFIED | |
| Physical iPhone behavior | Not run | UNVERIFIED | |

Doc-page caveat: several Apple doc pages returned 404 or an empty body when fetched. Claims rest on Apple's JSON docs endpoints, SDK headers, the HealthKit privacy page and App Review guideline 5.1.3 (fetched 2026-11-18). Sources are listed in `spikes/sources.md` terms; third-party write support and background delivery for medication types are not documented anywhere I could fetch.

## Recommendation: DEFER (stay out of the MVP)

Reading the medication list is feasible and useful for onboarding. It does not block anything else, but it needs physical-device proof and a security/privacy review before it ships.

Re-entry criteria (all required):
1. A product requirement from the owner for Health import.
2. Security/privacy review (RV-SEC) of the design below.
3. All four HLT-IOS scenarios PASS on a physical iPhone running iOS 26 or later with a dev-signed build (including revoke-while-running and locked-device read).
4. Confirmation on a device of the write restriction (HLT-IOS-03).

## Design rule for any future integration

- Import **copies medication names only**, each one confirmed by the user. It never creates or changes ledger doses and never counts as proof a dose was taken.
- Treat denied, ungranted and empty identically as "unknown". Never show "no medications in Health".
- The app must work fully with Health access denied, unavailable or later revoked. The local ledger stays the source of truth.
- A privacy policy is required; Health data may not go to ads or data brokers (App Review 5.1.3).
- Dose events from Health, if ever displayed, are labelled as unverified external information.

## What was run

Throwaway SwiftUI app (`project.yml`, `HKSpike.entitlements`, `Sources/App.swift`), ad-hoc signed, simulator only, no network. One obviously fake medication ("ZZ Fakemed Test") and one fake Taken log were created by hand in the simulator's Health app. The spike app was not installed anywhere else.

## Not covered

Everything marked UNVERIFIED above, plus Android Health Connect (separate spike, no simulator evidence exists).
