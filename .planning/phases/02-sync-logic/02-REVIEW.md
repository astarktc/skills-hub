---
phase: 02-sync-logic
reviewed: 2026-04-07T22:15:00Z
depth: standard
files_reviewed: 9
files_reviewed_list:
  - src-tauri/src/commands/projects.rs
  - src-tauri/src/core/mod.rs
  - src-tauri/src/core/project_ops.rs
  - src-tauri/src/core/project_sync.rs
  - src-tauri/src/core/skill_store.rs
  - src-tauri/src/core/tests/project_ops.rs
  - src-tauri/src/core/tests/project_sync.rs
  - src-tauri/src/core/tests/skill_store.rs
  - src-tauri/src/lib.rs
findings:
  critical: 0
  warning: 3
  info: 2
  total: 5
status: issues_found
---

# Phase 02: Code Review Report

**Reviewed:** 2026-04-07T22:15:00Z
**Depth:** standard
**Files Reviewed:** 9
**Status:** issues_found

## Summary

Reviewed the Phase 2 sync logic implementation: project registration, tool/skill assignment, filesystem sync, staleness detection, resync commands, and all supporting database CRUD. The code is well-structured, follows project conventions (command layer delegates to core, DTOs at the boundary, `spawn_blocking` for sync work), and has strong test coverage (12 integration tests covering happy paths, error paths, staleness, cascades, and mutex serialization).

Three warnings were found: a hash computation failure that can leave the DB and filesystem inconsistent, an early-exit in `resync_all_projects` that skips remaining projects on a single project failure, and a `content_hash` COALESCE that can silently retain stale hashes. Two informational items note defensive-but-silent error swallowing in the staleness check and the hardcoded initial `mode` value in `assign_and_sync`.

## Warnings

### WR-01: hash_dir failure after successful sync leaves DB inconsistent

**File:** `src-tauri/src/core/project_sync.rs:65-66`
**Issue:** In `assign_and_sync`, when the filesystem sync succeeds in copy mode but `content_hash::hash_dir(source)` fails, the `?` operator propagates the error and the function returns `Err`. At this point, the DB record inserted on line 51 is still in "pending" status, but the filesystem sync has already completed successfully. The DB and filesystem are now inconsistent: the target directory exists with correct content, but the assignment record says "pending" with no synced_at timestamp. A subsequent resync would overwrite with `overwrite=true`, which is benign, but the user sees an error for a sync that actually succeeded.

**Fix:** Catch the hash_dir error and fall back to storing the record without a content_hash, or update the status to "synced" before attempting the hash and update the hash separately:

```rust
match sync_engine::sync_dir_for_tool_with_overwrite(tool_key, source, &target, false) {
    Ok(outcome) => {
        let mode_str = sync_mode_to_str(&outcome.mode_used);
        let hash = if matches!(outcome.mode_used, SyncMode::Copy) {
            match content_hash::hash_dir(source) {
                Ok(h) => Some(h),
                Err(e) => {
                    log::warn!("failed to compute content hash after sync: {:#}", e);
                    None
                }
            }
        } else {
            None
        };
        store.update_assignment_status(
            &record.id,
            "synced",
            None,
            Some(now),
            Some(mode_str),
            hash.as_deref(),
        )?;
        // ...
    }
    // ...
}
```

The same pattern exists in `sync_single_assignment` at lines 137-143 and should be fixed identically.

### WR-02: resync_all_projects aborts remaining projects on single project failure

**File:** `src-tauri/src/core/project_sync.rs:197`
**Issue:** The `?` operator on `resync_project(store, &project.id, now)?` causes `resync_all_projects` to abort the entire loop if any single project's resync encounters a DB error (e.g., from `list_project_skill_assignments` or `get_project_by_id`). This is inconsistent with `resync_project` itself, which gracefully handles individual assignment failures and continues to the next. If the user has 5 projects and the 2nd has a DB issue, projects 3-5 are silently skipped.

**Fix:** Handle per-project errors the same way `resync_project` handles per-assignment errors -- record the failure and continue:

```rust
pub fn resync_all_projects(store: &SkillStore, now: i64) -> Result<Vec<ResyncSummary>> {
    let projects = store.list_projects()?;
    let mut summaries = Vec::with_capacity(projects.len());

    for project in &projects {
        match resync_project(store, &project.id, now) {
            Ok(summary) => summaries.push(summary),
            Err(e) => {
                log::warn!("resync_all: failed to resync project {}: {:#}", project.id, e);
                summaries.push(ResyncSummary {
                    project_id: project.id.clone(),
                    synced: 0,
                    failed: 0,
                    errors: vec![format!("project-level error: {:#}", e)],
                });
            }
        }
    }

    Ok(summaries)
}
```

### WR-03: COALESCE on content_hash silently retains stale hash across mode changes

**File:** `src-tauri/src/core/skill_store.rs:745`
**Issue:** The SQL `content_hash = COALESCE(?5, content_hash)` means passing `None` for `content_hash` preserves whatever value was there before. If an assignment was previously synced in copy mode (with a hash), then resync switches it to symlink mode, the old `content_hash` persists in the DB even though it is now semantically meaningless. This is currently safe because `list_assignments_with_staleness` only checks staleness when `mode == "copy"` (line 214), so the stale hash does not cause false positives. However, this creates a subtle coupling: any future code that reads `content_hash` without also checking `mode` will get incorrect data.

**Fix:** Explicitly clear `content_hash` when syncing in non-copy mode. In `assign_and_sync` and `sync_single_assignment`, pass an explicit empty-string or explicit clearing value when `hash` is `None`. Alternatively, change the SQL to not use COALESCE for `content_hash` so it gets properly cleared:

```sql
UPDATE project_skill_assignments
SET status = ?1, last_error = ?2,
    synced_at = COALESCE(?3, synced_at),
    mode = COALESCE(?4, mode),
    content_hash = ?5
WHERE id = ?6
```

This way, passing `None` explicitly sets `content_hash` to NULL rather than preserving the old value.

## Info

### IN-01: Staleness check silently swallows multiple error types

**File:** `src-tauri/src/core/project_sync.rs:216-229`
**Issue:** The staleness detection in `list_assignments_with_staleness` silently swallows errors from `store.get_skill_by_id` (line 216, via `if let Ok`), `content_hash::hash_dir` (line 219, via `if let Ok`), and `store.update_assignment_status` (line 222, via `let _`). This is documented as intentional defensive behavior and tested by `staleness_source_missing_no_crash`. However, the `update_assignment_status` failure on line 222 creates a divergence: the in-memory `assignment.status` is set to "stale" (line 221) and returned to the caller, but the DB may still say "synced" if the write failed. This means the next time the list is fetched, the assignment would re-compute staleness (cheap but unnecessary).

**Fix:** Consider logging the `update_assignment_status` failure:

```rust
if let Err(e) = store.update_assignment_status(
    &assignment.id, "stale", None, None, None, None,
) {
    log::warn!("failed to persist stale status for assignment {}: {:#}", assignment.id, e);
}
```

### IN-02: Hardcoded initial mode in assign_and_sync

**File:** `src-tauri/src/core/project_sync.rs:44`
**Issue:** The initial `mode` is hardcoded to `"symlink"` on line 44, but the actual mode used is determined by `sync_engine::sync_dir_for_tool_with_overwrite` which may fall back to copy (e.g., for Cursor). The mode gets updated to the correct value in the `update_assignment_status` call on line 71-78. However, if the function exits early between lines 51 and 71 (which currently cannot happen but could in future refactors), the DB would record an incorrect mode. The initial "pending" record with `mode: "symlink"` is a white lie that gets corrected later.

**Fix:** Use `"pending"` or `"auto"` as the initial mode value to reflect that the actual mode is not yet known:

```rust
let record = ProjectSkillAssignmentRecord {
    // ...
    mode: "auto".to_string(),
    status: "pending".to_string(),
    // ...
};
```

---

_Reviewed: 2026-04-07T22:15:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
