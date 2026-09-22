# 03 — Frontend-A: fold `ProjectSyncReport`, bulk unassign button, disabled Tool-dir candidates, factory rename

Status: needs-info
Blocked by: 01
Spec: `.scratch/round16/spec.md` — D1, D2, D6, D9. Read the spec first. The wire map from ticket 01's report is
pasted under § Wire map below before this ticket is claimed.

## Goal

The project world shows every mutation through the fold: toggle-on toasts like toggle-off, bulk assign and resync
report per-assignment outcomes, bulk unassign exists. The Add flow shows a Tool-dir folder disabled with a
reason. `refreshProgress` is named as the factory it is.

## Work

1. **Fold** (`src/lib/reportOutcome.ts`): `projectSyncOutcome(report: ProjectSyncReport, ctx: { t, toolLabelById,
   action: "toggleOn" | "bulkAssign" | "resync" | "resyncAll" })` → `Outcome` — precedence conflict › failure ›
   skipped › success as the other folds; counters (`synced`, `alreadyAssigned`, `failed`) derived here, never on
   the wire; failure entries carry the skill *id* and tool label; toast copy per action (EN + ZH keys). Tests in
   `src/lib/reportOutcome.test.ts` beside the existing folds.
2. **Hook** (`src/components/projects/useProjectState.ts`): `toggleAssignment` returns the tagged result
   (assigned → `ProjectSyncReport`, unassigned → `RemovalReport`); `bulkAssign` returns `ProjectSyncReport`;
   `resyncProject` / `resyncAll` return `ProjectSyncReport`; new `bulkUnassign(skillId) → RemovalReport`. Each
   applies the returned `view` (`applyView`), failure path `refreshView` — no success-path refetch. Hook tests
   in `useProjectState.test.ts` mock `invokeTauri` by camelCase name.
3. **Page / matrix**: `ProjectsPage.tsx` folds toggle-on with `projectSyncOutcome` (toggle-off keeps
   `removalOutcome`), bulk assign, bulk unassign (`removalOutcome`, action `"bulkUnassign"`). `AssignmentMatrix`
   drops its own `summary.synced`/`failed` arithmetic (`:110–158`) — resync toasts come from the fold. Bulk
   unassign button beside bulk assign (`:438`): shown when the skill has ≥1 assignment in the project; bulk
   assign hidden when the row is saturated; **no confirmation**. Memo comparators updated.
4. **D6**: `LocalPickModal.tsx:39` `mapReason` maps `inside_tool_dir` → `t("localSkillInvalid.insideToolDir")`
   (EN: "This folder is a Tool's own skills copy — use Import instead", ZH equivalent). The candidate renders
   disabled via the existing `valid` path.
5. **D9**: rename `refreshProgress` → `newRefreshProgressChannel` in `src/hooks/useSkillLibrary.ts` (and its
   test if named).
6. **i18n**: every new key in **both** `en` and `zh` (`src/i18n/resources.ts`). No hardcoded UI text.

## Constraints

- Components call the backend only through `invokeTauri`; DTO types come from `src/components/projects/types.ts`.
- No new state library, no CSS modules; styles in `App.css`.
- `npm run lint && npm run test && npm run build` green (build = typescript-7). Work in your assigned worktree;
  commit on your branch; do not merge; `.scratch/` only via a dated `## Comments` entry here.

## Wire map (from ticket 01)

_(pasted by the orchestrator before claiming)_
