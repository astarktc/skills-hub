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
- L1 round 2 re-check: round-1 findings closed; new P1 (nested bind mount) + P2 (EROFS). Round 3 (PR #9) re-check:
  EROFS and the original mount probes closed; two new P1s (mountinfo fails open on non-UTF-8 / no procfs; a filesystem
  mounted at the entry itself on macOS). Round 4 dispatched on `t3/removal-fence-round4`.
- `main` regression from the L2 review fix: `synchronous = FULL` pushed hosted-Windows SQLite contention past the 5 s
  busy timeout; `b8a530b` raises it to 30 s (two green hosted-Windows runs, PR #10).
- L1 merged (`401d3e5`) after round 4; the round-4 re-check (Sol, high) found nothing and judged no ordinary layout
  over-refused. All five lanes are on `main`; next, the final Astra review of `v1.3.2..main`.
- Final review (GPT-6 Astra, high): fix-then-ship, no P1; one P2 + two P3 fixed by the orchestrator
  (`.scratch/reviews/1.3.3-release-review.md` § Disposition). Released as 1.3.3.

## Closure — 2026-11-05

Shipped as **v1.3.3** (tag `v1.3.3` = `db339c7`, release run 42171004813, published 2026-11-06 06:05 UTC; 14 assets;
`updater.json` lists darwin-aarch64/x86_64, linux-x86_64, windows-aarch64/x86_64). Every lane item closed and removed
from BACKLOG: #63 #68 #72 #73 #74 (pointer) #75 #76 #90 #95 #96 #97 #98 #99 #100 #101 #102 #103 #104 #105 #106 #107.

Residue (extracted to BACKLOG): #112 matrix hit area, #113 Import's delete-before-deploy order, #114 sort not
remembered, #115 stale Enable count; #74's persistence half stays with #53. Dropped by name: #102's light-theme captures
(the README embeds one image per screen). Accepted limits recorded in `core/removal_fence.rs` and CONTEXT.md: the
check-then-remove race; a mount at or inside the removed directory (incl. a btrfs subvolume) is refused; Linux directory
removal fails closed without a readable `/proc/self/mountinfo`.
