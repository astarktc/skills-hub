---
phase: 01-data-foundation
reviewed: 2026-04-07T12:00:00Z
depth: standard
files_reviewed: 9
files_reviewed_list:
  - src-tauri/src/core/skill_store.rs
  - src-tauri/src/core/tests/skill_store.rs
  - src-tauri/src/core/project_ops.rs
  - src-tauri/src/core/tests/project_ops.rs
  - src-tauri/src/commands/mod.rs
  - src-tauri/src/commands/projects.rs
  - src-tauri/src/core/mod.rs
  - src-tauri/src/core/sync_engine.rs
  - src-tauri/src/lib.rs
findings:
  critical: 1
  warning: 4
  info: 3
  total: 8
status: issues_found
---

# Phase 01: Code Review Report

**Reviewed:** 2026-04-07T12:00:00Z
**Depth:** standard
**Files Reviewed:** 9
**Status:** issues_found

## Summary

This review covers the Phase 01 data foundation work: new SQLite schema (V4 migration for projects, project_tools, project_skill_assignments tables), new `project_ops` business logic module, new `commands/projects.rs` Tauri command layer, and related changes to `skill_store.rs`, `lib.rs`, `mod.rs`, and `sync_engine.rs`.

The schema migration, CRUD operations, and command wiring are well-structured and follow existing codebase conventions. Test coverage is thorough for the new data layer. One critical bug exists in `sync_engine.rs` that can destroy source data during overwrite. Several warnings relate to missing input validation in command handlers and a duplicated utility function with divergent error handling.

## Critical Issues

### CR-01: `sync_dir_hybrid_with_overwrite` uses `remove_dir_all` on potential symlinks, risking source data destruction

**File:** `src-tauri/src/core/sync_engine.rs:76`
**Issue:** When `overwrite` is true, `std::fs::remove_dir_all(target)` is called without first checking whether `target` is a symlink. On Linux, `remove_dir_all` on a symlink that points to a directory will follow the symlink and delete the contents of the **source** directory, then fail. This destroys the canonical skill data in the central repo. The sibling function `sync_dir_copy_with_overwrite` (line 99) correctly uses `remove_path_any(target)` which checks `is_symlink()` first.
**Fix:**

```rust
// In sync_dir_hybrid_with_overwrite, line 75-77, replace:
        if overwrite {
            std::fs::remove_dir_all(target)
                .with_context(|| format!("remove existing target {:?}", target))?;
// With:
        if overwrite {
            remove_path_any(target)
                .with_context(|| format!("remove existing target {:?}", target))?;
```

## Warnings

### WR-01: Duplicate `remove_path_any` functions with divergent signatures and error handling

**File:** `src-tauri/src/commands/mod.rs:795-817`
**Issue:** There are two `remove_path_any` implementations: one in `commands/mod.rs` (takes `&str`, returns `Result<(), String>`) and one in `sync_engine.rs` (takes `&Path`, returns `Result<()>` with anyhow context). The `commands/mod.rs` version loses error context by converting to bare `String`. The `project_ops.rs` module correctly uses the `sync_engine` version, but the `commands/mod.rs` version is called from `unsync_skill_from_tool` (line 601) and `delete_managed_skill` (line 767). Having two implementations that handle the same concern differently is a maintenance hazard. If one is fixed for a platform edge case, the other may be missed.
**Fix:** Remove the `remove_path_any` function from `commands/mod.rs` and use `sync_engine::remove_path_any` in both call sites. Convert the `Path`/`anyhow::Error` types at the call site:

```rust
// In unsync_skill_from_tool (line 601):
sync_engine::remove_path_any(Path::new(&target.target_path))?;

// In delete_managed_skill (line 767):
if let Err(err) = sync_engine::remove_path_any(Path::new(&target.target_path)) {
    remove_failures.push(format!("{}: {}", target.target_path, err));
}
```

### WR-02: `add_project_skill_assignment` command does not validate that the skill or project exists before inserting

**File:** `src-tauri/src/commands/projects.rs:117-153`
**Issue:** The command directly constructs a `ProjectSkillAssignmentRecord` and calls `store.add_project_skill_assignment()` without verifying that the `projectId` or `skillId` exist in their respective tables. While the SQLite foreign key constraint will cause an error, the error message will be a raw SQLite constraint violation string (e.g., "FOREIGN KEY constraint failed") that is not user-friendly and not mapped to a prefixed error format like `TOOL_NOT_INSTALLED|`. The same concern applies to `add_project_tool` (line 55) not validating the project exists.
**Fix:** Add validation before insertion:

```rust
// Before creating the record:
let _project = store
    .get_project_by_id(&projectId)?
    .ok_or_else(|| anyhow::anyhow!("project not found: {}", projectId))?;
let _skill = store
    .get_skill_by_id(&skillId)?
    .ok_or_else(|| anyhow::anyhow!("skill not found: {}", skillId))?;
```

### WR-03: `remove_project_with_cleanup` silently skips symlink cleanup when skill has been deleted first

**File:** `src-tauri/src/core/project_ops.rs:102-116`
**Issue:** The cleanup loop uses `store.get_skill_by_id(&assignment.skill_id)` to resolve the skill name (line 104). If the skill has already been deleted (e.g., user deleted the skill before removing the project), the lookup returns `None` and the orphaned symlink in the project directory is never removed. The `skill.name` is needed to construct the target path on line 109 (`adapter.relative_skills_dir.join(&skill.name)`), but since the skill is gone, the symlink persists as filesystem garbage. Consider storing the skill name or target path directly on the assignment record, or attempting cleanup based on filesystem discovery.
**Fix:** As a near-term fix, store the skill name on the assignment record so cleanup does not depend on the skill existing. Alternatively, enumerate the project's skills directory and remove entries not matching any remaining assignments.

### WR-04: Schema V4 migration DDL duplicated between fresh-install and upgrade paths

**File:** `src-tauri/src/core/skill_store.rs:148-181` and `src-tauri/src/core/skill_store.rs:192-225`
**Issue:** The V4 migration DDL (CREATE TABLE projects, project_tools, project_skill_assignments with indexes) is copy-pasted identically in both the `user_version == 0` (fresh install) branch and the `user_version < 4` (upgrade) branch. This duplication means any future schema change to these tables must be applied in two places, and a mismatch would cause inconsistencies between fresh installs and upgrades.
**Fix:** Extract the V4 migration DDL into a constant (like `SCHEMA_V1`):

```rust
const MIGRATION_V4: &str = r#"
    BEGIN;
    CREATE TABLE IF NOT EXISTS projects ( ... );
    CREATE TABLE IF NOT EXISTS project_tools ( ... );
    CREATE TABLE IF NOT EXISTS project_skill_assignments ( ... );
    CREATE INDEX IF NOT EXISTS idx_psa_project ON project_skill_assignments(project_id);
    CREATE INDEX IF NOT EXISTS idx_psa_skill ON project_skill_assignments(skill_id);
    CREATE INDEX IF NOT EXISTS idx_pt_project ON project_tools(project_id);
    COMMIT;
"#;
```

Then reference it from both branches.

## Info

### IN-01: Debug `println!` left in production command handler

**File:** `src-tauri/src/commands/mod.rs:759`
**Issue:** `println!("[delete_managed_skill] skillId={}", skillId);` is a debug print statement left in the `delete_managed_skill` command handler. The codebase uses `log::info!` for operational logging (via `tauri-plugin-log`). Using `println!` bypasses the log framework and goes directly to stdout, which may not be visible in production builds.
**Fix:** Either remove it or replace with `log::info!("[delete_managed_skill] skillId={}", skillId);`

### IN-02: `dedup()` on `installed` vector without prior `sort()` may not remove all duplicates

**File:** `src-tauri/src/commands/mod.rs:142`
**Issue:** `installed.dedup()` only removes consecutive duplicates. If the same tool key appears non-consecutively in the vector (e.g., if `adapters_sharing_skills_dir` causes repeated pushes), duplicates will survive. In practice, each adapter is iterated once and pushes at most once, so duplicates should already be consecutive. However, calling `sort()` before `dedup()` (or using a `HashSet`) would be more robust.
**Fix:** Either `installed.sort(); installed.dedup();` or use `HashSet` to collect installed tool keys.

### IN-03: Chinese-language error messages in `format_anyhow_error` and `delete_managed_skill`

**File:** `src-tauri/src/commands/mod.rs:73-98` and `src-tauri/src/commands/mod.rs:783`
**Issue:** Several error messages are in Chinese (e.g., line 73: "TLS/..." messages, line 783: "..."). While the project does support Chinese i18n, error messages from the backend should ideally be language-neutral keys or English strings that the frontend translates via `formatErrorMessage`. The current approach means these backend error messages cannot be localized for users who switch to English. This is a pre-existing pattern, not introduced by this phase, so flagging as informational.
**Fix:** No action needed for this phase -- this is pre-existing technical debt. If addressed, convert backend error strings to structured error codes.

---

_Reviewed: 2026-04-07T12:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
