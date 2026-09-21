# 02 — Frontend: follow the regenerated bindings; derive counters in the fold; delete folds a report

Status: ready-for-agent
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
