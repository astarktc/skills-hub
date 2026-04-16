---
phase: 260416-nwg
fixed_at: 2026-04-16T18:45:00Z
review_path: .planning/quick/260416-nwg-cherry-pick-clean-upstream-commits-and-m/260416-nwg-REVIEW.md
iteration: 1
findings_in_scope: 2
fixed: 2
skipped: 0
status: all_fixed
---

# Phase 260416-nwg: Code Review Fix Report

**Fixed at:** 2026-04-16T18:45:00Z
**Source review:** .planning/quick/260416-nwg-cherry-pick-clean-upstream-commits-and-m/260416-nwg-REVIEW.md
**Iteration:** 1

**Summary:**

- Findings in scope: 2
- Fixed: 2
- Skipped: 0

## Fixed Issues

### CR-01: Marketplace manifest can escape the repo root and scan/copy arbitrary local directories

**Files modified:** `src-tauri/src/core/installer.rs`
**Commit:** 88f2aa9
**Applied fix:** Added canonicalization of both the repo root directory and each resolved plugin source path in `parse_marketplace_json()`. The resolved path is now checked with `starts_with(&repo_root)` to reject any path that escapes the cloned repo via `../` traversal or symlink tricks. Paths that fail canonicalization (e.g., nonexistent targets) are also filtered out since `canonicalize().ok()?` returns `None`.

### WR-01: Managed-skill deletion still cleans project artifacts using global tool paths

**Files modified:** `src-tauri/src/commands/mod.rs`
**Commit:** cd9d962
**Applied fix:** Changed the project artifact cleanup path in `delete_managed_skill()` from `adapter.relative_skills_dir` (global tool path) to `crate::core::tool_adapters::project_relative_skills_dir(&adapter)` (project-scoped path). This ensures deletion targets the same directory that `project_sync.rs` uses when creating the symlink, so tools like Cursor, OpenCode, and Codex whose project path differs from global path will have their artifacts properly cleaned up.

## Skipped Issues

None.

---

_Fixed: 2026-04-16T18:45:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
