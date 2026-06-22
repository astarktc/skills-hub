---
phase: 01-data-foundation
fixed_at: 2026-04-07T12:30:00Z
review_path: .planning/phases/01-data-foundation/01-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 01: Code Review Fix Report

**Fixed at:** 2026-04-07T12:30:00Z
**Source review:** .planning/phases/01-data-foundation/01-REVIEW.md
**Iteration:** 1

**Summary:**

- Findings in scope: 5
- Fixed: 5
- Skipped: 0

## Fixed Issues

### CR-01: `sync_dir_hybrid_with_overwrite` uses `remove_dir_all` on potential symlinks, risking source data destruction

**Files modified:** `src-tauri/src/core/sync_engine.rs`
**Commit:** 9609537
**Applied fix:** Replaced `std::fs::remove_dir_all(target)` with `remove_path_any(target)` in `sync_dir_hybrid_with_overwrite`. The `remove_path_any` function correctly checks `is_symlink()` first and uses `remove_file` for symlinks instead of `remove_dir_all`, preventing traversal into and deletion of the symlink source directory contents.

### WR-01: Duplicate `remove_path_any` functions with divergent signatures and error handling

**Files modified:** `src-tauri/src/commands/mod.rs`
**Commit:** edce2ca
**Applied fix:** Removed the local `remove_path_any(path: &str) -> Result<(), String>` from `commands/mod.rs` (was lines 795-817). Added `remove_path_any` to the existing `sync_engine` import. Updated both call sites (`unsync_skill_from_tool` and `delete_managed_skill`) to use `sync_engine::remove_path_any` with `std::path::Path::new()` conversion. This consolidates the implementation to a single version with proper anyhow error context.

### WR-02: `add_project_skill_assignment` command does not validate that the skill or project exists before inserting

**Files modified:** `src-tauri/src/commands/projects.rs`
**Commit:** 5191026
**Applied fix:** Added validation calls to `store.get_project_by_id()` and `store.get_skill_by_id()` with clear error messages before record insertion in `add_project_skill_assignment`. Also added `store.get_project_by_id()` validation in `add_project_tool`. Both now return descriptive "project not found" or "skill not found" errors instead of raw SQLite foreign key constraint violations.

### WR-03: `remove_project_with_cleanup` silently skips symlink cleanup when skill has been deleted first

**Files modified:** `src-tauri/src/core/project_ops.rs`
**Commit:** b9f092d
**Applied fix:** Replaced the `if let Ok(Some(skill))` pattern with an explicit `match` that handles all three cases: `Ok(Some(skill))` performs cleanup as before, `Ok(None)` logs a warning identifying the orphaned symlink (skill_id, project path, tool), and `Err(e)` logs a warning about the lookup failure. This changes the behavior from silently skipping to explicitly logging the orphan for diagnostic traceability. A full fix (storing skill name on the assignment record) requires a schema migration and is deferred to a future phase. Status: fixed: requires human verification (logic change in cleanup behavior).

### WR-04: Schema V4 migration DDL duplicated between fresh-install and upgrade paths

**Files modified:** `src-tauri/src/core/skill_store.rs`, `src-tauri/src/commands/tests/commands.rs`
**Commit:** 6bdbf2b
**Applied fix:** Extracted the V4 migration DDL into a `MIGRATION_V4` constant (following the existing `SCHEMA_V1` pattern). Both the fresh-install branch (`user_version == 0`) and the upgrade branch (`user_version < 4`) now reference `MIGRATION_V4`. Also updated test file `commands/tests/commands.rs` to pass `&Path` instead of `&str` to `remove_path_any`, aligning with the WR-01 signature change. All 96 Rust tests pass.

---

_Fixed: 2026-04-07T12:30:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
