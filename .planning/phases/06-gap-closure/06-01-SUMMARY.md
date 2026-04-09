---
phase: 06-gap-closure
plan: 01
subsystem: backend/project-sync
tags: [tool-removal, missing-status, cascade-cleanup, staleness-detection]
dependency_graph:
  requires: []
  provides: [remove_tool_with_cleanup, missing-status-detection]
  affects: [project_ops, project_sync, commands/projects]
tech_stack:
  added: []
  patterns: [early-continue-pattern, best-effort-cleanup-loop, tdd]
key_files:
  created: []
  modified:
    - src-tauri/src/core/project_ops.rs
    - src-tauri/src/commands/projects.rs
    - src-tauri/src/core/project_sync.rs
    - src-tauri/src/core/tests/project_ops.rs
    - src-tauri/src/core/tests/project_sync.rs
decisions:
  - "Used unassign_and_cleanup for all assignments unconditionally (no status filtering per D-01/D-03)"
  - "Used early-continue pattern in list_assignments_with_staleness for clean missing detection"
  - "Both-exist block not gated on prior DB status to enable D-07 auto-recovery"
metrics:
  duration: 11m
  completed: 2026-04-09
  tasks_completed: 2
  tasks_total: 2
  files_modified: 5
---

# Phase 06 Plan 01: Gap Closure -- Tool Removal Cascade and Missing Status Detection Summary

Backend-only fixes closing the last two partially-met v1.0 milestone requirements: tool column removal now cascades to assignments and filesystem artifacts (TOOL-03), and list_assignments_with_staleness now produces "missing" status when skill source or deployed target is absent (SYNC-01).

## What Was Done

### Task 1: Tool Removal Cascade Cleanup

Added `remove_tool_with_cleanup` function in `project_ops.rs` that iterates ALL assignments for a specified tool unconditionally (no status filtering per D-01/D-03), calls `unassign_and_cleanup` per assignment to remove filesystem artifacts, handles orphaned skills (missing DB record) with direct filesystem cleanup via adapter path resolution, then deletes the tool DB row.

Updated `remove_project_tool` command in `commands/projects.rs` to acquire SyncMutex (per D-02) and delegate to the new function instead of a bare DB delete.

Three TDD tests:

- `remove_tool_with_cleanup_deletes_assignments_and_artifacts` -- full cascade with 2 skills
- `remove_tool_with_cleanup_leaves_other_tools_intact` -- cross-tool isolation
- `remove_tool_with_cleanup_handles_missing_skill_gracefully` -- orphaned skill handling

### Task 2: Missing Status Detection

Replaced the simple staleness-only check in `list_assignments_with_staleness` with an early-continue pattern:

1. Source absent -> mark "missing", persist, continue
2. Target absent (for previously-deployed assignments) -> mark "missing", persist, continue
3. Both exist -> recalculate staleness (copy: hash comparison, symlink: synced)

Critical design: the both-exist block runs for ANY current DB status including "missing". This directly satisfies D-07: an assignment stuck in "missing" auto-recovers to "synced" or "stale" when source and target reappear, because the recalculation is not gated on prior DB status.

Four TDD tests:

- `missing_status_when_source_absent` (renamed from staleness_source_missing_no_crash)
- `missing_status_when_target_absent`
- `missing_status_recovers_when_source_restored` (D-07 litmus test)
- `missing_status_source_and_target_both_absent`

## Commits

| Commit  | Type | Description                                                                       |
| ------- | ---- | --------------------------------------------------------------------------------- |
| 63be3dc | test | Add failing tests for remove_tool_with_cleanup (TDD RED)                          |
| 80e9bd6 | feat | Implement remove_tool_with_cleanup cascade (TDD GREEN)                            |
| 19cbbcf | test | Add failing tests for missing status detection (TDD RED)                          |
| b2d7520 | feat | Implement missing status detection in list_assignments_with_staleness (TDD GREEN) |

## Deviations from Plan

None -- plan executed exactly as written.

## Verification Results

- `cargo test --lib` -- 130 passed, 0 failed (all lib tests including 7 new ones)
- `cargo test -- project_ops::tests::remove_tool` -- 3 passed
- `cargo test -- project_sync::tests::missing_status` -- 4 passed
- `cargo test -- project_sync::tests::staleness` -- 2 passed (no regressions)
- `cargo fmt --check` -- pass
- `cargo clippy -- -D warnings` -- pass
- `npm run build` -- pass
- Pre-existing: 7 gitignore integration tests fail on base commit (unrelated)

## Known Stubs

None.

## Self-Check: PASSED

All 5 modified files exist on disk. All 4 commit hashes verified in git log.
