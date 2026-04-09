---
phase: 02-sync-logic
plan: 01
subsystem: backend/sync
tags: [rust, sync, sqlite, migration, concurrency]
dependency_graph:
  requires: [01-01, 01-02]
  provides:
    [
      project_sync module,
      V5 migration,
      SyncMutex,
      assign_and_sync,
      unassign_and_cleanup,
    ]
  affects: [commands/projects.rs, skill_store.rs, lib.rs]
tech_stack:
  added: []
  patterns:
    [
      COALESCE partial update,
      Arc<Mutex> concurrency serialization,
      sync_engine delegation,
    ]
key_files:
  created:
    - src-tauri/src/core/project_sync.rs
    - src-tauri/src/core/tests/project_sync.rs
  modified:
    - src-tauri/src/core/skill_store.rs
    - src-tauri/src/core/tests/skill_store.rs
    - src-tauri/src/core/mod.rs
    - src-tauri/src/core/project_ops.rs
    - src-tauri/src/core/tests/project_ops.rs
    - src-tauri/src/commands/projects.rs
    - src-tauri/src/lib.rs
decisions:
  - V5 migration uses ALTER TABLE for incremental upgrades; fresh DDL includes content_hash inline
  - SyncMutex uses poisoned-mutex recovery via unwrap_or_else(e.into_inner())
  - content_hash stored only for copy-mode targets (symlinks don't need it per D-09)
  - Symlink to non-existent source succeeds on Linux; error test uses cursor/copy mode instead
metrics:
  duration: 14 minutes
  completed: 2026-04-08T02:24:00Z
  tasks_completed: 2
  tasks_total: 2
  tests_added: 8
  tests_total: 104
  files_changed: 9
  lines_added: 634
  lines_removed: 24
---

# Phase 02 Plan 01: Core Sync Logic Summary

Project sync module with assign/unassign operations, V5 schema migration for content_hash, SyncMutex for concurrency, and enhanced Tauri commands wired to filesystem sync.

## What Was Built

### Task 1: V5 migration, SyncMutex, update_assignment_status, get_project_skill_assignment

**Commit:** a2434fd (RED), 2ed1b76 (GREEN)

Schema changes:

- SCHEMA_VERSION bumped from 4 to 5
- V5 migration adds `content_hash TEXT NULL` column to `project_skill_assignments`
- Fresh-install DDL includes `content_hash` inline in CREATE TABLE
- `add_project_skill_assignment` INSERT now includes `content_hash` column

New SkillStore methods:

- `update_assignment_status` - COALESCE-based partial update for status, last_error, synced_at, mode, content_hash
- `get_project_skill_assignment` - lookup by (project_id, skill_id, tool) composite key

SyncMutex:

- `pub struct SyncMutex(pub Arc<Mutex<()>>)` defined in lib.rs
- Registered via `app.manage()` following CancelToken pattern

DTO updates:

- `ProjectSkillAssignmentRecord` struct includes `content_hash: Option<String>`
- `ProjectSkillAssignmentDto` in project_ops.rs includes `content_hash`
- All row-mapping closures updated (list, list_for_project_tool, get)

Tests added: 3 (v5_migration_adds_content_hash, update_assignment_status_coalesce, get_project_skill_assignment_returns_none)

### Task 2: project_sync.rs core module with assign_and_sync, unassign_and_cleanup

**Commit:** 86a632a

Core module `project_sync.rs`:

- `assign_and_sync` - Creates DB record, calls sync_engine, updates status to synced/error, stores content_hash for copy-mode
- `unassign_and_cleanup` - Removes filesystem artifact via remove_path_any, deletes DB record; on failure preserves record with error status
- `resolve_project_sync_target` - Computes project_path/relative_skills_dir/skill_name
- `sync_mode_to_str` - Converts SyncMode enum to string for DB storage

Commands enhanced:

- `add_project_skill_assignment` now takes `SyncMutex` state, acquires lock, delegates to `project_sync::assign_and_sync`
- `remove_project_skill_assignment` now takes `SyncMutex` state, acquires lock, delegates to `project_sync::unassign_and_cleanup`

Tests added: 5 (assign_creates_symlink, assign_stores_hash_for_copy, assign_records_error_on_sync_failure, unassign_removes_symlink, unassign_target_not_found_cleans_db)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fresh-install DDL had duplicate content_hash column**

- **Found during:** Task 1 GREEN phase
- **Issue:** The V4 migration DDL was updated to include `content_hash` in CREATE TABLE, but the fresh-install path also ran the V5 ALTER TABLE, causing "duplicate column name" error
- **Fix:** Removed the redundant ALTER TABLE from the fresh-install path since the DDL already includes it
- **Files modified:** src-tauri/src/core/skill_store.rs
- **Commit:** 2ed1b76

**2. [Rule 1 - Bug] Error test used symlink mode which succeeds on non-existent source**

- **Found during:** Task 2 test verification
- **Issue:** Linux symlinks can point to non-existent paths without error, so the test using claude_code (symlink mode) with a non-existent source succeeded instead of failing
- **Fix:** Changed test to use "cursor" tool which forces copy mode; copy fails when source doesn't exist
- **Files modified:** src-tauri/src/core/tests/project_sync.rs
- **Commit:** 86a632a

## Verification

1. All existing tests pass: `cargo test --lib -q` exits 0 (104 tests)
2. New project_sync tests pass: `cargo test project_sync --lib -q` exits 0 (5 tests)
3. New skill_store V5 tests pass: `cargo test skill_store::tests:: --lib -q` exits 0 (33 tests)
4. Clippy clean: `cargo clippy --lib -- -D warnings` exits 0
5. Format clean: `cargo fmt --check` exits 0
6. Full check: `npm run check` exits 0 (with cargo in PATH)

## Self-Check: PASSED

All 3 created files verified present. All 3 commit hashes verified in git log.
