---
phase: 02-sync-logic
plan: 02
subsystem: backend/sync
tags: [rust, sync, staleness, resync, concurrency, sqlite]
dependency_graph:
  requires: [02-01]
  provides:
    [
      resync_project,
      resync_all_projects,
      list_assignments_with_staleness,
      ResyncSummaryDto,
    ]
  affects: [commands/projects.rs, project_sync.rs, lib.rs]
tech_stack:
  added: []
  patterns:
    [
      continue-on-error iteration,
      hash-based staleness detection,
      mutex-protected bulk operations,
    ]
key_files:
  created: []
  modified:
    - src-tauri/src/core/project_sync.rs
    - src-tauri/src/core/tests/project_sync.rs
    - src-tauri/src/commands/projects.rs
    - src-tauri/src/lib.rs
decisions:
  - Staleness only checked for copy-mode synced targets; symlinks skipped per D-09
  - Missing source directory during staleness check silently skipped (no crash, status unchanged)
  - ResyncSummary tracks per-assignment errors to enable UI feedback on partial failures
  - sync_serialization test uses AtomicU32 counter to prove mutual exclusion without timing sensitivity
metrics:
  duration: 11 minutes
  completed: 2026-04-08T02:47:00Z
  tasks_completed: 3
  tasks_total: 3
  tests_added: 8
  tests_total: 112
  files_changed: 4
  lines_added: 603
  lines_removed: 2
---

# Phase 02 Plan 02: Re-sync, Staleness Detection, and Command Wiring Summary

Re-sync operations for project and all-projects, staleness detection for copy-mode targets via SHA-256 hash comparison, staleness-aware list command wiring, mutex-protected Tauri commands, and serialization verification.

## What Was Built

### Task 1: sync_single_assignment helper, resync_project, and resync_all_projects

**Commit:** 13c1e9a

Core functions added to `project_sync.rs`:

- `ResyncSummary` struct with project_id, synced count, failed count, and error messages
- `sync_single_assignment` (pub(crate)) -- re-syncs one assignment with overwrite=true, updates status/mode/content_hash in DB
- `resync_project` -- iterates all project assignments, calls sync_single_assignment for each, continues on error (D-06), records failures in ResyncSummary
- `resync_all_projects` -- iterates all registered projects, calls resync_project for each, returns Vec of summaries

Tests added: 3 (resync_updates_all, resync_continues_on_error, resync_all_multiple_projects)

### Task 2: Staleness detection with list command wiring, global sync independence

**Commit:** 8bdcd4d

Staleness detection:

- `list_assignments_with_staleness` -- checks copy-mode synced assignments for hash drift against source directory
- Only copy-mode targets checked (symlinks propagate changes instantly per D-09)
- Missing source directory gracefully skipped (no crash)
- Stale assignments updated in DB so status persists across list calls

Command wiring:

- `list_project_skill_assignments` Tauri command now calls `project_sync::list_assignments_with_staleness` instead of raw `store.list_project_skill_assignments` (SYNC-04 wiring)

Tests added: 4 (staleness_detected_for_copy, staleness_skipped_for_symlink, staleness_source_missing_no_crash, global_and_project_sync_independent)

### Task 3: Mutex-protected re-sync Tauri commands with serialization test

**Commit:** 1dbada4

Tauri commands:

- `ResyncSummaryDto` for IPC serialization
- `resync_project` command -- acquires SyncMutex, delegates to core function (INFR-02)
- `resync_all_projects` command -- acquires SyncMutex, delegates to core function (INFR-02)
- Both registered in `generate_handler!` in lib.rs

Tests added: 1 (sync_serialization -- proves mutual exclusion via AtomicU32 counter across 2 threads)

## Deviations from Plan

None -- plan executed exactly as written.

## Verification

1. All project_sync tests pass: `cargo test project_sync --lib -q` exits 0 (13 tests)
2. All tests pass (full suite): `cargo test --lib -q` exits 0 (112 tests)
3. Clippy clean: `cargo clippy --lib -- -D warnings` exits 0
4. Format clean: `cargo fmt --check` exits 0
5. ESLint clean: `npm run lint` exits 0
6. Frontend build: `npm run build` exits 0
7. list_project_skill_assignments uses project_sync::list_assignments_with_staleness (verified via grep)
8. resync_project and resync_all_projects acquire SyncMutex (verified via grep)
9. Both resync commands registered in generate_handler! (verified via grep)

## Self-Check: PASSED

All 4 modified files verified present. All 3 commit hashes verified in git log.
