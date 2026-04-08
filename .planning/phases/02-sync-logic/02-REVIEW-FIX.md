---
phase: 02-sync-logic
fixed_at: 2026-04-07T22:58:00Z
review_path: .planning/phases/02-sync-logic/02-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 02: Code Review Fix Report

**Fixed at:** 2026-04-07T22:58:00Z
**Source review:** .planning/phases/02-sync-logic/02-REVIEW.md
**Iteration:** 1

**Summary:**

- Findings in scope: 3
- Fixed: 3
- Skipped: 0

## Fixed Issues

### WR-01: hash_dir failure after successful sync leaves DB inconsistency

**Files modified:** `src-tauri/src/core/project_sync.rs`
**Commit:** 1802b69
**Applied fix:** Replaced the `?` propagation on `content_hash::hash_dir(source)` with a `match` that catches the error, logs it via `log::warn!`, and falls back to `None` for the hash. This ensures the assignment record is updated to "synced" status even when hash computation fails, since the filesystem sync itself succeeded. Applied to both `assign_and_sync` (line 64) and `sync_single_assignment` (line 141). Also removed the now-unused `anyhow::Context` import.

### WR-02: resync_all_projects aborts remaining projects on single project failure

**Files modified:** `src-tauri/src/core/project_sync.rs`
**Commit:** 7161ec0
**Applied fix:** Replaced the `?` operator on `resync_project(store, &project.id, now)` with a `match` block. On error, the failure is logged via `log::warn!` and a `ResyncSummary` with the project-level error is pushed to the results vector. The loop continues to process remaining projects instead of aborting early.

### WR-03: COALESCE on content_hash silently retains stale hash across mode changes

**Files modified:** `src-tauri/src/core/skill_store.rs`
**Commit:** 4dc42f5
**Applied fix:** Removed the `COALESCE(?5, content_hash)` wrapper from the `update_assignment_status` SQL, changing it to plain `content_hash = ?5`. Passing `None` now explicitly sets `content_hash` to NULL rather than preserving the previous value. This prevents stale copy-mode hashes from persisting when an assignment switches to symlink mode. The `COALESCE` wrappers on `synced_at` and `mode` remain, as those have correct preserve-on-None semantics.

## Skipped Issues

None -- all in-scope findings were fixed.

---

_Fixed: 2026-04-07T22:58:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
