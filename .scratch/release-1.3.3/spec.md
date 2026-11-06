# 1.3.3 — hygiene patch (opened 2026-11-05)

Operator 2026-11-05: clear the small and bug-shaped BACKLOG items before roadmap work (round 20), led by #107. The item
text in `.scratch/BACKLOG.md` is each ticket's brief; **re-verify every item against the code before changing it**
(`docs/agents/issue-tracker.md` § Re-verify before scheduling). An item already fixed, or not reproducible, is reported
back with evidence, not forced.

## Lanes (parallel worktrees; one implementer each, one reviewer each)

| Lane | Items | Area |
|---|---|---|
| L1 removal safety | #107, then #76, #105 | Rust: `sync_engine`, `artifact_removal`, `onboarding_import`, `git_cache`; #76 adds a typed error (binding + `describeCommandError` + i18n) |
| L2 Rust hygiene | #104, #100, #63, #99 (1), #103 | Rust: `skill_store`, tests, `tool_adapters` path join |
| L3 reporting | #95, #68, #101, #74 (pointer only), #106, #99 (2)(3)(4) | Frontend reporter / folds / notifications / updates; #95 adds a report field (Rust) |
| L4 My Skills reading | #72, #73 | Frontend: library header, list order/grouping, inspector targets, run headline order, bulk copy |
| L5 projects, layers, import, fixture | #96, #75, #97, #98, #90 | Frontend projects world, popover/dialog layers, import review, fixture backend |
| Final (orchestrator) | #102 | README screenshots from the fixture, after L1–L5 merge |

## Rules for every lane

- Follow AGENTS.md (invariants, Definition of done). Gate before reporting done: `npm run version:check && npm run check`.
- Never run `npm run tauri:dev` (it mutates the operator's live library); `npm run dev:fixture` is the only way to drive
  the UI.
- Do not edit `CHANGELOG.md`, `.scratch/BACKLOG.md` or the version: the orchestrator writes them at merge. Report each
  user-facing change in one plain sentence instead.
- Commit in your worktree branch as you go (small commits, one item per commit where possible).
- `src/i18n/resources.ts` and `src/bindings/index.ts` are shared with other lanes: add keys near their family; regenerate
  bindings with `cargo test` (never hand-edit).

## Status

- L1–L5: dispatched 2026-11-05.
- L3 merged (`f0c4a96`): Sol review fix-then-merge, one P2 (HTML scan not quote-aware) fixed by the orchestrator.
- L4 merged (`8f69777`): Sol review fix-then-merge, three P2s — two Enable-copy accuracy bugs fixed by the orchestrator;
  "Problems first is not persisted" declined: sort was never persisted (pre-existing `useState`), outside #72.
- L5 merged (`9b90c68`): Sol review fix-then-merge, two P2s fixed by the orchestrator (a refused cell Undo was spent
  before admission; one failed project-view read hid another's rewrite). For later: the 1.3.0 whole-cell hit area makes
  matrix misclicks likelier (Undo mitigates); deleting a divergent Import original before the deploy could become
  move-aside → deploy → delete-or-restore.
- L2 merged: Sol (high) review fix-then-merge, two P2s fixed by the orchestrator — WAL now runs `synchronous = FULL`
  (the store is authoritative), and the legacy-migration backup copies the file set whole before removing it.
- L1: Sol (high) review FAIL — three P1s (case-insensitive leaf alias deletes a recorded central copy; cache ancestor
  alias; no configured-library boundary, cache cleanup unprotected) + #76 gap; round-2 implementer dispatched on
  `t3/removal-fence-round2`. PR #7 (draft, round 1) passed Windows CI.
