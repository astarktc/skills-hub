# 03 — Frontend-A: fold `ProjectSyncReport`, bulk unassign button, disabled Tool-dir candidates, factory rename

Status: claimed
Blocked by: 01 (done — 2a79874, merged to main)
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

From ticket 01's report (merged to main as `2a79874`). Note the status enum is `ProjectSyncOutcomeStatus`
(not `ProjectSyncStatus` — that name was already taken by `ProjectDto.sync_status`).

```ts
export type ProjectSyncReport = { items: ProjectSyncOutcome[] };
export type ProjectSyncOutcome = {
  assignment_id: string | null;   // null only when no row could be created (e.g. unknown tool)
  skill_id: string; skill_name: string; tool: string;   // tool = registry key
  status: ProjectSyncOutcomeStatus;
};
export type ProjectSyncOutcomeStatus =
  | { status: "synced" } | { status: "already_assigned" } | { status: "failed"; error: CommandError };
```

| Command | Was | Now |
|---|---|---|
| `toggleProjectSkillAssignment(projectId, skillId, tool)` | `{ view, assigned: boolean, report: RemovalReport \| null }` | `{ kind: "assigned"; view; report: ProjectSyncReport } \| { kind: "unassigned"; view; report: RemovalReport }` |
| `bulkAssignSkill(projectId, skillId)` | `{ view, failed: BulkAssignErrorDto[] }` | `{ view; report: ProjectSyncReport }` (`BulkAssignErrorDto` gone) |
| `resyncProject(projectId)` | `{ view, summary: ResyncSummary }` | `{ view; report: ProjectSyncReport }` |
| `resyncAllProjects()` | `{ summaries: ResyncSummary[], projects }` | `{ report: ProjectSyncReport; projects: ProjectDto[] }` — one report spanning every project |
| **new** `bulkUnassignSkill(projectId, skillId)` | — | `BulkUnassignResultDto = { view; report: RemovalReport }` |

- `ResyncSummary` is gone. `CommandError` union unchanged.
- `LocalSkillCandidate` unchanged in shape; new `reason` code `"inside_tool_dir"` (always with `valid: false`; it
  takes precedence over `invalid_frontmatter`).
- A sync failure inside a freshly created row comes back as `{ status: "failed", error }` **with** `assignment_id`
  present — the cell is red in the matrix and the operator must hear about it via the fold.
- Types re-exported from `src/components/projects/types.ts`: `ProjectSyncReport`, `ProjectSyncOutcome`,
  `ProjectSyncOutcomeStatus`, `BulkUnassignResultDto`.
- Lane 01 made interim build-forced edits you replace: `useProjectState.ts:~327` (toggle returns only the
  unassigned report), `:~385–399` (resync returns `result.report`); `AssignmentMatrix.tsx:~111–116, ~138–141`
  (counts synced/failed inline — marked interim); `ProjectsPage.tsx:~162–167` (`handleBulkAssign` builds a failed
  list from `report.items`); `useProjectState.test.ts` stubs follow the new shapes.

## Comments

### 2026-09-22 — lane 03 (frontend-A) implementation

Branch `round16/frontend-a`: `5a80cfe` (D6 + D9), `384fd0a` (D1 + D2 frontend). No Rust, bindings, App.tsx,
SkillCard, Re-point code or `gitRepoint.*`/`changeSource.*`/`unlocatable.*` keys touched. Status left `claimed`
for the orchestrator to close with the merge SHA.

- **Fold** `projectSyncOutcome` (`src/lib/reportOutcome.ts`): counters `synced/alreadyAssigned/failed` derived in
  `projectSyncCounts`; one error entry per failed item, title `errors.syncFailedTitle { name: skill_name, tool:
  label }`, message `describeCommandError`. Precedence collapses to failure › success (the report has no conflict
  or skip outcome; `already_assigned` is neither). `completion.reload` is always false (the view came back);
  `closeModal = !failed`. Toasts: toggleOn success `status.syncEnabled` (symmetric with toggle-off's
  `status.syncDisabled`), failure = entries only; bulkAssign `projects.bulkAssignSuccess` (plural) /
  `projects.bulkAssignNothing` (all already assigned) / `projects.bulkAssignPartial` (existing key, now used);
  resync `projects.resyncSuccess`/`resyncPartial` (existing); resyncAll new `projects.resyncAllSuccess`/`Partial`.
  `removalOutcome` gains `"bulkUnassign"`: `projects.bulkUnassignSuccess` (plural) / `projects.bulkUnassignPartial`,
  kept rows as `errors.unsyncFailedTitle` entries, never reloads; zero targets keeps the `unsyncNothingPlanned` warning.
- **Hook**: `ToggleResult` tagged union (`assigned` → `ProjectSyncReport`, `unassigned` → `RemovalReport`);
  `bulkAssign` → `ProjectSyncReport`; `bulkUnassign(skillId)` → `RemovalReport` (pending cells = only the cells that
  hold an assignment); `resyncProject` now also takes the failure-path `refreshView`; `resyncAll` keeps its single
  `getProjectView` read for the selected project, now through `refreshView` (a failed read no longer masks the
  report) and converges on a thrown batch too.
- **Page/matrix**: `ProjectsPage` folds toggle-on/off, bulk assign/unassign and both resyncs; `AssignmentMatrix`
  lost its interim counters and its `notify`/`notifyError` props (its memo comparator never compared them);
  resync props are `() => Promise<void>`. Row: `All Tools` shown when >1 tool and not saturated; `Unassign All`
  shown when >1 tool and ≥1 assignment; both disabled while any of the row's cells is pending; `title` tooltips.
  No confirmation. Comparators updated (`onBulkUnassign`; `showBulkAssign` prop removed — derived in the row).
- **D6** `LocalPickModal.mapReason`: `inside_tool_dir` → `localSkillInvalid.insideToolDir`. **D9** rename done
  (no test referenced the old name).
- **i18n** (EN + ZH): `localSkillInvalid.insideToolDir`; `projects.{allToolsTitle, unassignAll, unassignAllTitle,
  resyncAllSuccess, resyncAllPartial, bulkAssignSuccess_one/_other, bulkAssignNothing, bulkUnassignSuccess_one/_other,
  bulkUnassignPartial}`. Removed `projects.bulkAssignFailed` (only the interim handler read it).
- **Gate**: `npm run lint` clean; `npm run test` 15 files / 374 tests (+12: 6 fold, 6 hook); `npm run build`
  (typescript-7) ok with the pre-existing dynamic-import / chunk-size warnings.

**Deviations** (for review):
1. `Unassign All` also requires >1 configured tool (D2 says "shown when ≥1 assignment"). With one tool the row's
   checkbox already is "all", and `All Tools` has always been hidden there — the gate keeps the pair symmetric.
2. Failure entries carry skill *name* + tool label, not a skill id: no project-sync failure has a click action
   (the only reason an entry carries an id), and the report already carries `skill_name`.
3. `resyncAll` failure entries cannot name the project — `ProjectSyncOutcome` has no `project_id`. If the operator
   needs it, the backend item would have to gain one (not in this lane).
