---
phase: 06-gap-closure
reviewed: 2026-04-08T22:15:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - src-tauri/src/core/project_ops.rs
  - src-tauri/src/commands/projects.rs
  - src-tauri/src/core/project_sync.rs
  - src-tauri/src/core/tests/project_ops.rs
  - src-tauri/src/core/tests/project_sync.rs
findings:
  critical: 0
  warning: 2
  info: 2
  total: 4
status: issues_found
---

# Phase 06: Code Review Report

**Reviewed:** 2026-04-08T22:15:00Z
**Depth:** standard
**Files Reviewed:** 5
**Status:** issues_found

## Summary

Reviewed the project operations, project sync, and Tauri command layers for the per-project skill distribution feature, plus their associated test files. The code is well-structured overall: the command layer properly delegates to core modules, error handling follows project conventions (anyhow + format_anyhow_error), the sync mutex is correctly acquired for all mutating sync operations, and all commands are registered in `generate_handler!`. Test coverage is thorough with 20 tests covering assignment, unassignment, resync, staleness detection, recovery, bulk operations, and concurrency.

Two warnings were found in the orphan-cleanup path of `remove_tool_with_cleanup`, where filesystem cleanup uses the wrong identifier for path resolution and one error branch leaves dangling DB records. Two informational items note a repeated DB query that could be hoisted and a large function that deviates from the project's layering convention.

## Warnings

### WR-01: Orphan cleanup uses skill_id (UUID) instead of skill.name for filesystem path

**File:** `src-tauri/src/core/project_ops.rs:130-134`
**Issue:** When a skill record is missing from the DB (`Ok(None)` branch), the orphan cleanup code constructs the filesystem target path using `assignment.skill_id` as the directory name. However, the actual symlink/copy was originally created using `skill.name` (a human-readable name like `"my-skill"`). Since `skill_id` is a UUID (e.g., `"a1b2c3d4-..."`), the resolved path will never match the real filesystem artifact, and the orphan symlink will silently remain.

**Fix:** Store `skill_name` in the `ProjectSkillAssignmentRecord` (add a column to the `project_skill_assignments` table) so the correct filesystem name is available even when the skill record is deleted. Alternatively, attempt a directory listing of the tool's skills dir to find and remove any entry that does not correspond to a currently-assigned skill:

```rust
// In project_ops.rs, Ok(None) branch of remove_tool_with_cleanup:
// Option A: If skill_name is added to the assignment record:
let target = project_sync::resolve_project_sync_target(
    Path::new(&project.path),
    adapter.relative_skills_dir,
    &assignment.skill_name, // Use stored name, not skill_id
);
```

### WR-02: Err branch in remove_tool_with_cleanup leaks assignment DB records

**File:** `src-tauri/src/core/project_ops.rs:148-155`
**Issue:** When `store.get_skill_by_id(&assignment.skill_id)` returns `Err(e)` (a database error during lookup), the code only logs a warning. Neither the filesystem artifact nor the DB assignment record is cleaned up. Since `store.remove_project_tool` at line 158 still executes and removes the tool row, the assignment record becomes orphaned -- it references a tool that no longer exists for this project.

**Fix:** Add a fallback cleanup for the DB record in the `Err` branch, matching the pattern used in the `Ok(None)` branch:

```rust
Err(e) => {
    log::warn!(
        "remove_tool_with_cleanup: error looking up skill {}: {:#}",
        assignment.skill_id,
        e
    );
    // Best-effort: clean up the assignment record to avoid orphaned rows
    if let Err(e2) =
        store.remove_project_skill_assignment(&project.id, &assignment.skill_id, tool)
    {
        log::warn!("failed to remove assignment record after lookup error: {:#}", e2);
    }
}
```

## Info

### IN-01: Redundant per-assignment project lookup in list_assignments_with_staleness

**File:** `src-tauri/src/core/project_sync.rs:241-244`
**Issue:** Inside the `for mut assignment in assignments` loop, each iteration calls `store.get_project_by_id(&assignment.project_id)` to resolve the project path for target existence checks. Since all assignments share the same `project_id` (the function parameter), this fetches the same row repeatedly. For a project with many assignments, this is unnecessary database round-trips.

**Fix:** Fetch the project record once before the loop:

```rust
pub fn list_assignments_with_staleness(
    store: &SkillStore,
    project_id: &str,
) -> Result<Vec<ProjectSkillAssignmentRecord>> {
    let project = store.get_project_by_id(project_id)?;
    let assignments = store.list_project_skill_assignments(project_id)?;
    // ...
    for mut assignment in assignments {
        // Use `project` directly instead of re-fetching
        let target_exists = if let Some(ref skill) = skill_opt {
            if let Some(adapter) = tool_adapters::adapter_by_key(&assignment.tool) {
                if let Some(ref p) = project {
                    let target = resolve_project_sync_target(
                        Path::new(&p.path),
                        adapter.relative_skills_dir,
                        &skill.name,
                    );
                    target.exists() || target.symlink_metadata().is_ok()
                } else {
                    false
                }
            } else { false }
        } else { false };
        // ...
    }
}
```

### IN-02: update_project_gitignore contains ~115 lines of business logic in the command layer

**File:** `src-tauri/src/commands/projects.rs:391-555`
**Issue:** The `update_project_gitignore` command handler contains substantial business logic: the `remove_block` closure, pattern collection, and dual-file (`.gitignore` + `.git/info/exclude`) read-modify-write logic. This deviates from the project's architecture convention where `commands/` should only handle Tauri wrappers, DTO conversion, and error formatting, while business logic belongs in `core/`.

**Fix:** Extract the gitignore manipulation logic into a function in `core/project_ops.rs` (e.g., `update_gitignore_for_project`) and have the command handler delegate to it. This would also make the logic unit-testable without Tauri state machinery.

---

_Reviewed: 2026-04-08T22:15:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
