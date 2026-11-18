# Planning handoff (work paused for cost; resume here)

**Branch:** `riogrande100-reconcile-medtracker-planning` (from `main` @ `8f8d43a`). **All changes are uncommitted.** Docs only; no application code touched.

## Done
- `docs/product-spec.md` edited in place: Amendment A (authoritative reconciliation), four primary targets, PR-19–PR-27 added, undo/void semantics, Due-is-derived invariants, empty network allowlist (§9), catalog licensing gate GT-LIC (§6), privacy/backup/deletion summary, per-target notification stances, Phase 0–4 rewritten with exit gates G0–G4, §12 now an index to the backlog, §13 updated.
- Created: `docs/current-state.md`, `docs/backlog.md` (B0-00…B4-06 with owner/tier, file ownership, acceptance, tests, review gates, dependencies, Mermaid graph, traceability), `docs/registers.md` (capability, excluded, deferred, decisions D-01…D-27, gates), `docs/ux-journeys.md` (personas, Mermaid flows, screen specs), `docs/release-evidence.md` (iOS/Android/Windows/macOS matrices; all NOT PASSED).

## Not done / open inconsistencies
1. `start-prompt.md` and `product-owner-prompt.md` are **not yet updated** (still iOS-first/Linux-first, auth/subscription/sync, mockup gate, Phases 1–9). Required: align to Amendment A, with stable IDs and Phases 0–4.
2. The "PO phases 1–9 → workstreams WS-1…WS-9 mapped to Phases 0–4" table is referenced (spec Amendment A, D-04) but **not written anywhere**.
3. **No validation run**: markdown lint and link/anchor checks not executed (tools not installed; try `npx markdownlint-cli2` and a link checker). Verify anchors such as `backlog.md#governance-rules`, `#review-separation`, `#requirement-traceability`, and spec `#amendment-a--authoritative-reconciliation`.
4. Spec sections not re-read after edits for residual stale text (e.g. §4/§5 wording, any remaining "first platform" phrasing, `deleted` flag vs `status` in §5/§10 tests).
5. Requirement ID collision: sibling branch `riogrande100-user-friendly-medtracker-platforms` uses `PR-19` for health integrations and permits runtime network; do not merge as-is (see current-state.md). Phase1-slice branch code is unaudited and diverges from D-10/PR-07.
6. Evidence-dependent gates remain NOT PASSED (devices, Windows host, Android SDK absent, stakeholder usability, licensing, signing, stores).

## Exact next action
Edit `product-owner-prompt.md` and `start-prompt.md` to match Amendment A, add the WS mapping table to `product-spec.md`, then run markdown/link checks and fix findings; then report to parent and commit if approved.
