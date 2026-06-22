---
phase: 05-edge-cases-and-polish
plan: 03
subsystem: frontend
tags:
  [
    projects-tab,
    missing-project,
    error-translation,
    sync-disable,
    update-path,
    gitignore,
  ]
dependency_graph:
  requires: [update_project_path, path_exists_field]
  provides:
    [
      missing-project-warning-ui,
      sync-disable-for-missing,
      error-prefix-translation,
      bulk-assign-failure-surfacing,
      update-path-flow,
      gitignore-graceful-degradation,
    ]
  affects:
    [
      ProjectList.tsx,
      AssignmentMatrix.tsx,
      ProjectsPage.tsx,
      EditProjectModal.tsx,
      useProjectState.ts,
      types.ts,
      resources.ts,
      App.css,
      Header.tsx,
    ]
tech_stack:
  added: []
  patterns:
    [error-prefix-parsing, path-exists-conditional-ui, folder-picker-repoint]
key_files:
  created: []
  modified:
    - src/components/projects/types.ts
    - src/components/projects/useProjectState.ts
    - src/components/projects/ProjectList.tsx
    - src/components/projects/AssignmentMatrix.tsx
    - src/components/projects/ProjectsPage.tsx
    - src/components/projects/EditProjectModal.tsx
    - src/i18n/resources.ts
    - src/App.css
    - src/components/skills/Header.tsx
decisions:
  - "formatProjectError exported from useProjectState.ts, applied in ProjectsPage.tsx catch blocks (not inline in hook)"
  - "pathMissing passed as disabled prop to MatrixRow for cell-level disabling"
  - "EditProjectModal gitignore catch explicitly sets both states to false for graceful degradation"
metrics:
  duration_seconds: 644
  completed: "2026-04-09T00:44:52Z"
  tasks_completed: 2
  tasks_total: 2
  tests_added: 0
  tests_passing: 124
  files_modified: 9
---

# Phase 5 Plan 3: Projects Tab Frontend Polish Summary

Missing-project detection UI with warning badges and sync-disable, error prefix translation for DUPLICATE_PROJECT/ASSIGNMENT_EXISTS/NOT_FOUND, bulk assign failure surfacing, update-path folder picker flow, and gitignore edge case handling.

## Completed Tasks

| Task | Name                                                                                                                                    | Commit  | Key Changes                                                                                                                                                                                                                                                   |
| ---- | --------------------------------------------------------------------------------------------------------------------------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1    | ProjectDto type update, error prefix parsing, bulk assign failure surfacing, and update-path action                                     | 749fb0f | path_exists on ProjectDto, formatProjectError export, bulkAssign returns result, updateProjectPath action, 11 i18n keys                                                                                                                                       |
| 2    | ProjectList warning badge, AssignmentMatrix sync-disable, ProjectsPage error translation and update-path flow, gitignore edge case, CSS | ad588f2 | ProjectList warning badge + update-path button, AssignmentMatrix disabled controls + banner, formatProjectError in all catch blocks, bulk assign failure toast, handleUpdatePath with folder picker, EditProjectModal graceful gitignore fallback, CSS styles |

## Implementation Details

### ProjectDto Type (types.ts)

- Added `path_exists: boolean` field to TypeScript `ProjectDto`, mirroring backend DTO from Plan 05-01

### Error Prefix Parsing (useProjectState.ts)

- New exported `formatProjectError(raw, t)` function handles three prefix patterns:
  - `DUPLICATE_PROJECT|<path>` -> translated duplicate error message
  - `ASSIGNMENT_EXISTS|` -> translated assignment exists message
  - `NOT_FOUND|<detail>` -> translated not-found message
- Applied in all ProjectsPage.tsx catch blocks (handleAddProject, handleRemoveProject, handleToggleAssignment, handleBulkAssign, handleUpdatePath)

### Bulk Assign Failure Surfacing (useProjectState.ts + ProjectsPage.tsx)

- `bulkAssign` now captures and returns `BulkAssignResultDto` instead of discarding it
- Return type updated to `Promise<BulkAssignResultDto | undefined>`
- `handleBulkAssign` in ProjectsPage checks `result.failed.length > 0` and shows `toast.warning` with tool-specific failure details

### Update Path Flow (useProjectState.ts + ProjectsPage.tsx)

- New `updateProjectPath(projectId, newPath)` action calls `invoke("update_project_path")` and refreshes project list
- `handleUpdatePath` in ProjectsPage opens native folder picker via `@tauri-apps/plugin-dialog`, validates selection, calls state action, shows success toast

### ProjectList Warning Badge (ProjectList.tsx)

- AlertTriangle icon with `.project-warning-badge` CSS class when `!p.path_exists`
- `(not found)` text in italicized red after the path via `.project-path-missing`
- FolderOpen "Update Path" button appears for missing projects in actions row
- `.missing` CSS class reduces opacity for missing project items
- `onUpdatePath` prop added to ProjectListProps

### AssignmentMatrix Sync Disable (AssignmentMatrix.tsx)

- `pathMissing` derived boolean from `!project.path_exists`
- Sync Project and Sync All buttons: `disabled={pathMissing}` with explanatory title tooltip
- Warning banner: `.matrix-path-missing-banner` with AlertTriangle icon and `syncDisabledMissing` message
- MatrixRow `disabled` prop: disables all cell checkboxes (`disabled={isPending || disabled}`) and bulk assign button
- Error-state click handler guarded with `!disabled`
- Memo comparison function updated to include `disabled` prop

### Gitignore Edge Case (EditProjectModal.tsx)

- `.catch(() => {})` replaced with `.catch(() => { setAddToGitignore(false); setAddToExclude(false); })`
- When project has no `.git` directory or gitignore is unreadable, checkboxes default to unchecked

### CSS Additions (App.css)

- `.project-warning-badge` - Inline-flex warning icon with `--status-warning` color
- `.project-path-missing` - Italic red text for missing path indicator
- `.update-path-btn` - Accent-colored folder picker button with hover state
- `.project-item.missing` - Reduced opacity for missing project rows
- `.matrix-path-missing-banner` - Full-width red banner with danger background and border

### i18n Keys Added (resources.ts)

- `projects.assignmentExistsError` - Duplicate assignment message
- `projects.notFoundError` - Not found message
- `projects.pathMissing` - "(not found)" suffix
- `projects.pathMissingWarning` - Full warning tooltip text
- `projects.updatePath` - Update Path button label
- `projects.updatePathSuccess` - Success toast message
- `projects.bulkAssignPartial` - Partial bulk assign summary
- `projects.bulkAssignFailed` - Bulk assign failure details
- `projects.syncDisabledMissing` - Explanation for disabled sync

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Header.tsx activeView type missing 'projects'**

- **Found during:** Task 1 build verification
- **Issue:** Header component's `activeView` prop type was `'myskills' | 'explore' | 'detail' | 'settings'` but App.tsx passes `'projects'` since Phase 4. Build failed with TS2322.
- **Fix:** Added `| 'projects'` to Header's activeView type
- **Files modified:** src/components/skills/Header.tsx
- **Commit:** 749fb0f

**2. [Rule 1 - Bug] handleAddProject missing dependency 't' in useCallback**

- **Found during:** Task 2 lint verification
- **Issue:** After adding `formatProjectError(... , t)` to the catch block, the `[state]` dependency array was missing `t`, causing react-hooks/exhaustive-deps warning and React Compiler preservation error
- **Fix:** Updated dependency array to `[state, t]`
- **Files modified:** src/components/projects/ProjectsPage.tsx
- **Commit:** ad588f2

## Known Stubs

None. All data flows are wired to backend commands from Plan 05-01.

## Threat Surface Scan

No new threat surface beyond what was documented in the plan's threat model. Update path uses native OS folder picker (trusted input), error messages strip internal prefixes before display, and gitignore fallback is read-only with no security impact.

## Self-Check: PASSED

All 9 modified files exist. Both commit hashes (749fb0f, ad588f2) verified. All 30 acceptance criteria confirmed via grep. Lint and build pass clean. No Rust changes in this plan (frontend-only).
