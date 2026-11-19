# MedTracker documentation index

MedTracker is a local-first medication log (Tauri 2, Rust core, React/TypeScript/Vite, SQLite) for iOS and Android (first release is mobile only, owner decision D-29, 2026-11-19; Windows and macOS desktop are deferred, not removed, and the code keeps building on the macOS dev host). These documents are planning artifacts. Nothing here proves device, store, signing, usability, or legal readiness; every evidence-dependent gate is NOT PASSED until an artifact exists.

| Document | Purpose |
|---|---|
| [product-spec.md](product-spec.md) | Authoritative specification: direction, requirements (PR-01 to PR-28), domain and time model, data model, architecture, tests, Phases 0-4, workstream mapping. |
| [current-state.md](current-state.md) | What actually exists in the repository, known review findings, and gate status. |
| [backlog.md](backlog.md) | Tickets (B0-00 to B4-07, B0-10, B3-11; deferred desktop tickets DT-01..DT-07), governance rules, review separation, model tiers, traceability. |
| [registers.md](registers.md) | Capability, excluded, and deferred registers (incl. DF-14..DF-16 desktop); decisions (D-xx, D-29 mobile-only scope); gates (G0-G4, GT-*). |
| [ux-journeys.md](ux-journeys.md) | Personas, Mermaid flows, screen specifications, accessibility acceptance. |
| [release-evidence.md](release-evidence.md) | Per-platform evidence matrices (iOS/Android NOT PASSED; Windows/macOS DEFERRED by D-29). |
| [handoff-2026-11-18-paused.md](handoff-2026-11-18-paused.md) | Project-level handoff from the paused execution. |
| [planning-draft/](planning-draft/) | Superseded drafts kept for history only. Do not use as a source of truth. |

Related files at the repository root: [product-owner-prompt.md](../product-owner-prompt.md) (product-owner brief), [start-prompt.md](../start-prompt.md) (original planning brief), [MOBILE.md](../MOBILE.md) (host/mobile status of the Phase 1 slice).

When documents disagree, the product spec wins. Add new IDs only after checking the spec and registers for collisions.
