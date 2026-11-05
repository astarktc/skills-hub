# Archived efforts

Finished efforts, moved here intact by `git mv` per `docs/agents/issue-tracker.md` § Lifecycle. Walk evidence
(logs, diffs, captures) was pruned before the move; reports, reviews and tickets are complete. Residue numbers refer to
`.scratch/BACKLOG.md` at the time of archiving.

| Date | Effort | What it was | Residue |
| --- | --- | --- | --- |
| 2026-09-15 | v-next | Wayfinder epic: architecture deepening, typed IPC (tauri-specta), tool capability registry, invocation badge (v1.2.0–v1.2.2) | BACKLOG #11 #12 #17 #18 #30; ADR-0001 |
| 2026-09-15 | arch-deepening | Round-2 deep modules: mutation guard, Propagation, Artifact removal, presentation folds, acquisition | BACKLOG #13 #14 #27; ADR-0002 |
| 2026-09-15 | round3 | Review residue + notification UX (v1.2.3) | BACKLOG #21 |
| 2026-09-15 | round4 | Actionable Refresh failures, imported provenance, Re-point/Detach/Restore (v1.2.4) | BACKLOG #22 #23 #24 #25; ADR-0003 |
| 2026-09-15 | round5 | Git Re-point, discovery fix, review residue (v1.2.5) | BACKLOG #25 |
| 2026-09-15 | round6 | Atomic finalize, internal symlinks, slash branches, frontend shape (v1.2.6) | none |
| 2026-09-15 | round7 | Invocation badge + Skill Edit V1; cross-round backlog file (v1.2.7) | BACKLOG #03 #04 #05 #26 #29 |
| 2026-09-15 | round8 | Rust safety + regression slice (v1.2.8) | BACKLOG #19 |
| 2026-09-15 | round9 | Hygiene/defect patches + architecture panel that seeded round 10 (v1.2.9) | BACKLOG #02 #10; ADR-0004 |
| 2026-09-15 | round10 | Content identity, Update module, outcome fold (wave A, v1.2.10); git resolution, Manifest, mutation catalogs (wave B, v1.2.12); wave C deferred | BACKLOG #02 #06 #07 #08 #09 #15 #16 #20 #28 |
| 2026-09-15 | round11 | Intent-led global sync target set; visible selection; removable orphaned target rows (v1.2.11) | BACKLOG #07 #08 |
| 2026-09-16 | round12 | Tool-selection integrity: corrupt selection refuses (SETTING_CORRUPT), unknown keys pruned on read, add/import/bulk report not-detected skips (unreleased, `396374e`) | none |
| 2026-09-16 | round13 | Backlog bundle → v1.2.13: CHANGELOG 1.2.7–1.2.11 reconstructed, dead doc links, ADR-0005 (row is source of truth), workflow SHA pins + least privilege, filetime test, content_identity warn-once dropped, listing validity aligned with install, `TargetTransition::Recorded`, per-target project-removal outcomes | BACKLOG #32 #33 |
| 2026-09-20 | round14 | Small bundle → v1.2.14: `UpdateRequest` hardening (`SourceProposal`, one Acquired variant), `--release` propagation run in CI, Actions majors bumped + SHA-pinned, removal-toast tool labels via binder, Add skill button on My Skills | none |
| 2026-09-21 | round15 | Wave C → v1.2.15: fan-out reports cross the wire as core types (`CommandError` in core, per-row classification at settlement, five `*ReportDto` mirrors + mappers deleted, counters derived in the fold, delete + project toggle-off return `RemovalReport`, `DELETE_CLEANUP_FAILED` retired), ADR-0001 amended | BACKLOG #35 #36 |
| 2026-10-29 | round18 | Design recovery → v1.2.18: credentials in the Keychain, `~/.augment/skills` relocation, ZH removed, PRODUCT.md + ADR-0006 (styling architecture), frontend architecture / two blind design critiques / task-flow review, a grill that produced round 19 (D1–D33, 26 tickets) | BACKLOG #63; ADR-0006; `archive/round19/spec.md` |
| 2026-11-05 | round17 | Honest detection, scoped scan, overwrite ask → v1.2.17 (skills-only footprint is not installed; Review & Import honours the scan scope; first overwrite ask on `TARGET_EXISTS`); its 1.2.17 smoke superseded by round 19's 1.3.0 smoke | none |
| 2026-11-05 | round19 | The new world → v1.3.0 (19a) + v1.3.1 (19b): Tailwind v4 + primitives on Base UI, surface-state / action-door / lifecycle modules, disabled targets at two grains, one install batch + one overwrite ask, My Skills + inspector + bulk rail + notifications by run, Add / Projects / Settings dialog / Import review / Explore / ⌘K / card grid, dark two-tone theme, vocabulary (Harness · Deploy · Update), auto-deploy arrival-only, App.css deleted; ticket 34 parked | BACKLOG #88 #89 #91–#103 (and amendments); ADR-0006 amendment; `scripts/contrast.mjs`, `scripts/browser/cdp-keys.ts` |
