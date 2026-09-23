# Handoff — 2026-09-22 · round 16 ("close the backlog") shipped as 1.2.16; one operator ticket open

The queue is `.scratch/BACKLOG.md` — this note does not repeat it.

## Where things stand

- **1.2.15 verified** at session start (all five targets, `.sig`s, `updater.json`).
- **Round 16 is code-complete and released**: main @ `d8218d0` ("release: 1.2.16"), tag `v1.2.16`, `release.yml`
  run 35799656007 **green** — release published 2026-09-23T00:08Z with all five targets, macOS/Linux `.sig`s and
  `updater.json` (same asset shape as 1.2.15).
- What shipped (spec: `.scratch/round16/spec.md`): `ProjectSyncReport` for toggle-on / bulk assign / resync (D1,
  `project_id` on every item); `RemovalScope::ProjectSkill` + `bulk_unassign_skill` + "Unassign All" (D2);
  `repoint_skill_source` over `RepointTarget::{Git, Local}` for every provenance + one "Change source…" modal on
  every managed card and the detail view (D3–D5); Tool-dir folders flagged at listing (D6); BACKLOG #12 #15 #28
  closed; ADR-0003 amended; CONTEXT.md/AGENTS.md updated.
- **Astra review** (`.scratch/round16/review/astra-review.md`): fix-then-ship; M1 (a refused settlement write was
  reported as a settled row) fixed in the fuller form (`AssignAttempt`, engine returns `Result`, two trigger-injected
  regression tests); S1 + nits fixed. Ticket 09 records the disposition.
- **Every ticket is terminal except 08** (`ready-for-human`): the Windows junction smoke on the installed 1.2.16
  build — operator-only, post-release. Round 16 stays **live** until it closes.
- Gate at close: `npm run version:check && npm run check` green; cargo 661, vitest 393, eslint 0 warnings.
- BACKLOG: `Now` / `Later` / `Parked` empty; **Future efforts** #26 #37–#41; next free **#42**.

## Exact next step

1. Operator installs the v1.2.16 build and smokes the spec's Closure checklist (toggle-on toast; bulk assign / unassign toasts; resync via the fold; Change source
   git → local and back; imported → GitHub URL makes it refreshable; Add flow shows a `~/.claude/skills/*` folder
   disabled with a reason).
2. Operator runs ticket 08's checklist on the Windows host (Developer Mode **off**, non-elevated) and pastes the
   `dir /AL` output under its `## Comments`; set `Status: done — <evidence>` (or `wontfix — <reason>` if it cannot be
   run).
3. Then close the effort: write `## Closure — <date>` in `.scratch/round16/spec.md` (release `v1.2.16` / `d8218d0`,
   residue: none expected), extract-then-archive per `docs/agents/issue-tracker.md` § Lifecycle (nothing under
   `worktrees/` remains — they were removed after merge), add the `.scratch/archive/README.md` row, delete this note.
4. Next work is a **Future efforts** pick (#37 UI audit is the natural first — it starts with a grill, not code).

## Gotchas learned this session

- **Parallel worktrees under `.scratch/` break `npm run lint`**: each carries a `tsconfig.json`, and typescript-eslint's
  projectService refuses "multiple candidate TSConfigRootDirs". Fixed by `globalIgnores([... '.scratch/**'])` in
  `eslint.config.js` — but also remove worktrees as soon as their lane merges.
- `npm run check` **does not fail on react-hooks warnings** (`exhaustive-deps`). Run `npx eslint src` and read the
  tail after touching a `useCallback` deps array; a missing `notify` slipped past the gate this round.
- Opus 5.5 as implementer: four lanes, zero rebase conflicts beyond the regenerated bindings, every deviation disclosed
  and sound. Astra as reviewer found one real contract hole by control-flow inspection that no ordinary-path test
  covered — a settlement *write* failing is a different fault than a sync failing. Worth keeping this pairing.
- The reviewer's "smallest correct fix" again would have bent the rule (leaving the assign engine's catch-all);
  the fuller fix removed the producer. Same lesson as round 15 — take the fuller fix when it deletes the wrong claim.
- Two lanes both needing a compat i18n key (`gitRepoint.action`) is a smell that the frontend lanes shared a seam
  (`reportOutcome.ts`) the brief should have assigned to one of them explicitly.
