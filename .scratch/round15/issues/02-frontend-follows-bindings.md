# 02 — Frontend: follow the regenerated bindings; derive counters in the fold; delete folds a report

Status: done — pending
Blocked by: 01
Spec: `.scratch/round15/spec.md` — decisions D3, D4, D6. Read the spec and ticket 01's result comment first.

## Goal

`npm run build` green again against the bindings ticket 01 regenerated, with **no** behaviour change the operator
can see except delete's cleanup failure (now report data, D6). The pure fold `src/lib/reportOutcome.ts` is the only
place counters are computed.

## Work

1. **Names follow the bindings.** Every import of a removed `*ReportDto` / `*TargetDto` / `*StatusDto` type
   (`src/lib/reportOutcome.ts`, `src/components/skills/types.ts`, `src/components/projects/types.ts`,
   `src/hooks/useSyncOrchestration.ts`, `src/components/projects/useProjectState.ts`, `src/lib/skillPresentation.ts`,
   `src/components/**` — `grep -rn "ReportDto\|TargetDto\|StatusDto\|OutcomeDto" src --include=*.ts --include=*.tsx`)
   switches to the core name now exported from `src/bindings/index.ts`. Components keep importing through the
   per-world shims (`types.ts`), never from `src/bindings` directly (AGENTS.md).
2. **Wire-shape changes to absorb (D3):**
   - `RemovalReport.targets[]` is per *target* with `rows[]` (one artifact, several member rows) — the fold emits
     one entry per row (`row.tool`), as the old flattened DTO did. `RowRef` is tagged `scope`: `global_target`
     (`id, skill_id, tool`) / `assignment` (`id, project_id, skill_id, tool`) — check the bindings for the exact tags.
   - `BatchTargetOutcome.tool_key` (was `tool`); `status.synced.outcome.mode_used` (was `status.mode_used`).
   - `PropagationOutcome` (was `PropagationTargetDto`) — same fields.
   - `ImportGroupOutcome` / `OriginalOutcome` — `path` is a string; same fields otherwise.
   - `SkillRefreshOutcome` (was `SkillRefreshResultDto`) — same fields.
3. **D4 — counters derived in the fold.** `refreshOutcome`, `removalOutcome`, `projectRemovalOutcome`,
   `syncOutcome`, `importOutcome` currently read `report.refreshed/failed/skipped/target_failures`,
   `report.removed/failed`, `report.imported/failed`, `report.synced`. Replace each with a small pure helper
   in `reportOutcome.ts` (e.g. `refreshCounts(report)`, `removalCounts(report)` counting rows) — the fold already
   walks every item; the counts feed the same toast copy and completion rules as today. Keep the precedence
   (conflict › failure › skipped › success) and every completion rule byte-identical: the existing tests in
   `src/lib/reportOutcome.test.ts` are the contract — update their fixtures to the new shapes, not their
   expectations (except where a fixture's hand-written counter disagreed with its items — then the items win, and
   say so in your report).
4. **D6 — delete folds a report.** `invokeTauri("deleteManagedSkill", id)` now returns `RemovalReport`.
   `deleteOutcome(report, ctx)` folds it like `removalOutcome`: no failed target → success toast
   `status.skillRemoved`; failed targets → one error entry per kept row (title: reuse `errors.deleteCleanupFailed`'s
   intent — add `errors.deleteKeptTargetTitle` `{ tool }` or similar), a warning toast saying the skill was kept
   and can be retried (new key, e.g. `status.skillDeleteKept` with `{ failed }`), `closeModal: false`,
   `reload: true` (delete keeps its refetch). Remove the `DELETE_CLEANUP_FAILED` branch from
   `src/commandError.ts` (the `satisfies Record<CommandError["code"], true>` guard will force it) and the now-unused
   `errors.deleteCleanupFailed` key if nothing else reads it. **EN and ZH** for every new key in
   `src/i18n/resources.ts`.
5. **Hook tests.** `src/hooks/useSkillLibrary.test.ts` (fixture at `:350` hand-writes refresh counters),
   `useAddSkillFlow.test.ts`, `useSyncOrchestration.test.ts`, `components/projects/useProjectState.test.ts`,
   `lib/skillPresentation.test.ts` — update fixtures to the new shapes. Expectations stay.

## Gate for this ticket

`npm run lint && npm run test && npm run build` green (build uses typescript-7 — do not use a bare `npx tsc`);
`grep -rn "ReportDto\|TargetDto\|StatusDto\|OutcomeDto\|DELETE_CLEANUP_FAILED" src` returns nothing outside
`src/bindings/index.ts` (which must not contain them either after ticket 01).

## Constraints

- State your approach and continue — no need to wait for confirmation.
- Never commit. Never touch `CHANGELOG.md`, `src-tauri/`, or `src/bindings/index.ts`. `.scratch/` is tracked: you
  may append dated `## Comments` to this ticket; never archive or `git mv` under it.
- Prefer targeted edits over whole-file rewrites; re-read a file after writing it.
- No component-rendering tests (project rule); hook- and pure-function-level only.
- Report: files touched, new i18n keys, vitest counts before/after, any fixture whose counters disagreed with its
  items, and anything the spec or ticket got wrong.

## Comments

### 2026-09-21 — lane B implementation and verification

Claimed and implemented on `main` against `d3ea34a`; no commits, stash, backend/bindings/CHANGELOG changes,
archives, or live-data runs. Status remains claimed for the parent to close with its commit SHA.

- `npm run lint && npm run test && npm run build` passed. Vitest: **358 → 361 passed**, 15 files both times
  (retired one obsolete command-error test, added four fold/hook regressions). Build used typescript-7;
  existing dynamic-import and large-chunk warnings remain. `git diff --check` passed.
- Folds consume bare sync arrays, `tool_key`, grouped removal rows, and `propagation.targets`. Counts live only
  in `reportOutcome.ts` helpers: refresh counts include both acquisition skip kinds and reassert failures;
  removal counts rows, not artifacts; import counts group settlement, not target/original failures.
  Existing precedence/completion expectations are unchanged except D6's delete report behavior.
- Delete names every kept row's tool, warns, reloads the settled catalog, retains confirmation, and succeeds on
  retry. Tests cover both RowRef tags, shared artifact fan-out/counts, typed errors and label fallback, no-target
  success, both locales, and a real hook invocation → fold → reload → retry sequence. Whole-command rejection
  remains separately tested with `OTHER`; the retired code and translation key are gone.
- New keys in **both EN and ZH**: `errors.deleteKeptTargetTitle` (`{{tool}}`) and `status.skillDeleteKept`
  (`{{failed}}`, skill kept + retry). Removed `errors.deleteCleanupFailed`.
- Files touched: this ticket; `src/commandError.ts`, `src/commandError.test.ts`;
  `src/components/projects/{AssignmentMatrix.tsx,types.ts,useProjectState.ts,useProjectState.test.ts}`;
  `src/components/skills/types.ts`; `src/hooks/{useAddSkillFlow.test.ts,useSkillLibrary.test.ts,
  useSyncOrchestration.ts,useSyncOrchestration.test.ts}`; `src/i18n/resources.ts`;
  `src/lib/{reportOutcome.ts,reportOutcome.test.ts,skillPresentation.ts}`.

**Fixture discrepancy:** `reportOutcome.test.ts`'s project-removal test constructed `relabelled` by spreading a
report with `removed: 1`, then retaining only its failed target. Its actual items mean removed **0**, failed **1**.
The old test only asserted the labelled error title, so no expectation changed. No other counter/item mismatch
was found; the single-mutation hook table's counters were consistent and are now removed entirely.

**Spec/ticket corrections:**

1. The literal grep gate is overbroad, as lane A noted. Its only surviving names are the legitimate catalog/view
   types `ToolStatusDto`, `SkillTargetDto`, and `GitignoreStatusDto`; excluding these yields no matches.
   `ProjectSkillAssignmentDto` also legitimately remains (it does not match that exact grep expression).
2. `syncOutcome` did not read `report.synced/skipped/failed`; its decisions already follow entries. No unused
   sync-counter helper was added. The existing per-tool skip counter stays in the fold.
3. `useSkillLibrary.ts` already passed delete's return value to `deleteOutcome` and executed its completion;
   D6 needs no production hook change. `skillPresentation.test.ts` uses only catalog/view fixtures and needs
   no changes either. Invocation Edit's additional `PropagationReport` nesting follows lane A's bindings.
