---
phase: 06-gap-closure
fixed_at: 2026-04-08T22:45:00Z
review_path: .planning/phases/06-gap-closure/06-REVIEW.md
iteration: 1
findings_in_scope: 2
fixed: 2
skipped: 0
status: all_fixed
---

# Phase 06: Code Review Fix Report

**Fixed at:** 2026-04-08T22:45:00Z
**Source review:** .planning/phases/06-gap-closure/06-REVIEW.md
**Iteration:** 1

**Summary:**

- Findings in scope: 2
- Fixed: 2
- Skipped: 0

## Fixed Issues

### WR-01: Orphan cleanup uses skill_id (UUID) instead of skill.name for filesystem path

**Files modified:** `src-tauri/src/core/skill_store.rs`, `src-tauri/src/core/project_ops.rs`, `src-tauri/src/core/project_sync.rs`, `src-tauri/src/commands/projects.rs`, `src-tauri/src/core/tests/project_ops.rs`, `src-tauri/src/core/tests/skill_store.rs`, `src/components/projects/types.ts`
**Commit:** e2d24fe
**Applied fix:** Added `skill_name` column to the `project_skill_assignments` table via schema migration V6 (with backfill from the `skills` table for existing rows). Added `skill_name` field to `ProjectSkillAssignmentRecord` struct and `ProjectSkillAssignmentDto`. Populated it in `assign_and_sync` from `skill.name`. Updated both `remove_tool_with_cleanup` and `remove_project_with_cleanup` to use `assignment.skill_name` for filesystem path resolution in orphan cleanup branches, ensuring the correct human-readable name is used instead of the UUID. Updated all SQL queries (INSERT/SELECT), all test construction sites, and the frontend TypeScript DTO type.

### WR-02: Err branch in remove_tool_with_cleanup leaks assignment DB records

**Files modified:** `src-tauri/src/core/project_ops.rs`
**Commit:** 505dd9b
**Applied fix:** Added best-effort `store.remove_project_skill_assignment()` call in the `Err(e)` branch of the `get_skill_by_id` match in `remove_tool_with_cleanup`, matching the cleanup pattern already used in the `Ok(None)` branch. This prevents orphaned assignment rows when `remove_project_tool` executes afterward and removes the tool row.

---

_Fixed: 2026-04-08T22:45:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
