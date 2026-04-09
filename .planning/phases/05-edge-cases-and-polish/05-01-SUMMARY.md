---
phase: 05-edge-cases-and-polish
plan: 01
subsystem: backend
tags: [tauri-commands, settings, sync, cleanup, projects]
dependency_graph:
  requires: []
  provides:
    [
      get_auto_sync_enabled,
      set_auto_sync_enabled,
      unsync_all_skills,
      unsync_skill,
      update_project_path,
      path_exists_field,
    ]
  affects:
    [
      commands/mod.rs,
      commands/projects.rs,
      core/project_ops.rs,
      core/skill_store.rs,
      lib.rs,
    ]
tech_stack:
  added: []
  patterns:
    [settings-toggle-command, bulk-cleanup-command, store-helper-methods]
key_files:
  created: []
  modified:
    - src-tauri/src/commands/mod.rs
    - src-tauri/src/commands/projects.rs
    - src-tauri/src/core/project_ops.rs
    - src-tauri/src/core/skill_store.rs
    - src-tauri/src/core/tests/skill_store.rs
    - src-tauri/src/lib.rs
decisions:
  - "auto_sync_enabled defaults to true when no setting exists (per D-03)"
  - "missing status treated as error in aggregate_project_sync_status (SYNC-01)"
  - "remove_project now acquires SyncMutex to prevent race conditions (INFR-02)"
  - "delete_managed_skill cleans up project directory artifacts before cascade delete (INFR-03)"
metrics:
  duration_seconds: 777
  completed: "2026-04-09T00:15:50Z"
  tasks_completed: 2
  tasks_total: 2
  tests_added: 5
  tests_passing: 124
  files_modified: 6
---

# Phase 5 Plan 1: Backend Edge-Case Commands Summary

5 new Tauri commands, 4 new store methods, INFR-02/INFR-03/SYNC-01/PROJ-04 fixes, and 5 new tests for settings toggle, bulk unsync, project path validation, and missing status handling.

## Completed Tasks

| Task | Name                                                                           | Commit  | Key Changes                                                                                                                                                                                                            |
| ---- | ------------------------------------------------------------------------------ | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1    | Auto-sync toggle, bulk/per-skill unsync, INFR-03 skill deletion cleanup        | f75dcc8 | 4 new commands (get/set auto_sync_enabled, unsync_all_skills, unsync_skill), 3 new store methods, modified delete_managed_skill for project artifact cleanup, fixed aggregate_project_sync_status for "missing" status |
| 2    | Missing project detection (PROJ-04), update_project_path, SyncMutex fix, tests | b3fd406 | path_exists field on ProjectDto, update_project_path command + store method, SyncMutex on remove_project, 5 new tests                                                                                                  |

## Implementation Details

### New Commands (registered in generate_handler!)

1. **get_auto_sync_enabled** - Returns bool, defaults to `true` when no setting exists
2. **set_auto_sync_enabled** - Accepts bool, persists as "true"/"false" string
3. **unsync_all_skills** - Removes all global skill_targets filesystem artifacts + DB rows
4. **unsync_skill** - Removes all skill_targets for one skill + filesystem cleanup
5. **update_project_path** - Validates new path via canonicalize + is_dir, checks duplicates, updates DB

### Modified Commands

1. **delete_managed_skill** - Now fetches skill record before cleanup, cleans up project directory artifacts (INFR-03) for synced/stale/error assignments before cascade delete
2. **remove_project** - Now acquires SyncMutex before calling remove_project_with_cleanup (INFR-02)

### New Store Methods

1. `delete_all_skill_targets()` - DELETE FROM skill_targets (bulk)
2. `delete_skill_targets(skill_id)` - DELETE FROM skill_targets WHERE skill_id
3. `list_project_skill_assignments_by_skill(skill_id)` - Query assignments by skill for INFR-03 cleanup
4. `update_project_path(project_id, new_path, now_ms)` - UPDATE projects SET path, updated_at

### DTO Changes

- `ProjectDto` gained `path_exists: bool` field, computed via `Path::new(&record.path).is_dir()` in `to_project_dto`

### Bug Fixes

- **SYNC-01**: `aggregate_project_sync_status` now matches `"missing"` alongside `"error"` in its match arm
- **INFR-02**: `remove_project` acquires `SyncMutex` to prevent concurrent remove + sync filesystem corruption
- **INFR-03**: `delete_managed_skill` cleans up project directory artifacts before SQL CASCADE delete

### Tests Added (5)

1. `delete_all_skill_targets_clears_table` - Verifies bulk delete
2. `delete_skill_targets_removes_only_specified_skill` - Verifies scoped delete
3. `aggregate_project_sync_status_treats_missing_as_error` - Verifies SYNC-01 fix
4. `update_project_path_changes_path` - Verifies path + updated_at update
5. `list_project_skill_assignments_by_skill_returns_correct_rows` - Verifies skill-scoped query

## Deviations from Plan

None - plan executed exactly as written.

## Known Pre-existing Issues

7 tests in `tests/gitignore.rs` are failing (pre-existing, confirmed by running on clean HEAD). These are out of scope for this plan and relate to the gitignore block removal logic, not to any changes made here.

## Threat Surface Scan

No new threat surface introduced beyond what was documented in the plan's threat model. All new commands operate on DB-stored paths (not arbitrary user paths) and use existing sync_engine primitives for filesystem operations.

## Self-Check: PASSED

All 6 modified files exist. Both commit hashes (f75dcc8, b3fd406) verified. All 22 acceptance criteria confirmed via grep. 124 library tests pass (5 new). Pre-existing gitignore test failures (7) confirmed on clean HEAD.
