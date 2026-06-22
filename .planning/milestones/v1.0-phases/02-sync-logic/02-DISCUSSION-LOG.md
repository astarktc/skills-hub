# Phase 2: Sync Logic - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-07
**Phase:** 02-sync-logic
**Areas discussed:** Sync atomicity, Re-sync scope, Staleness detection, Concurrency scope

---

## Sync Atomicity

| Option                  | Description                                                                                                                                 | Selected |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| Two-step (Recommended)  | Create DB record with status='pending', then sync. If sync succeeds -> 'synced'. If fails -> record stays with status='error' + last_error. | ✓        |
| Atomic (all-or-nothing) | Sync first, only insert DB record if sync succeeds. No pending/error states.                                                                |          |
| You decide              | Let Claude pick.                                                                                                                            |          |

**User's choice:** Two-step
**Notes:** Matches Phase 1 schema design with pending/synced/stale/error status lifecycle.

---

| Option               | Description                                                                                           | Selected |
| -------------------- | ----------------------------------------------------------------------------------------------------- | -------- |
| Inline (Recommended) | Assign command creates record AND syncs in one call. Frontend gets back assignment with final status. | ✓        |
| Separate commands    | Assign creates record (pending), frontend calls separate sync command.                                |          |
| You decide           | Let Claude pick.                                                                                      |          |

**User's choice:** Inline
**Notes:** One invoke, one response — simpler frontend integration.

---

| Option                      | Description                                                                                          | Selected |
| --------------------------- | ---------------------------------------------------------------------------------------------------- | -------- |
| Remove then delete (Rec.)   | Remove symlink/copy first, then delete DB record. If removal fails, keep record with status='error'. | ✓        |
| Delete, best-effort cleanup | Delete DB record immediately. Best-effort filesystem cleanup.                                        |          |
| You decide                  | Let Claude pick.                                                                                     |          |

**User's choice:** Remove then delete
**Notes:** Strict unassign — filesystem cleanup blocks DB delete.

---

| Option                  | Description                                                             | Selected |
| ----------------------- | ----------------------------------------------------------------------- | -------- |
| Enhance existing (Rec.) | Existing remove_project_skill_assignment is enhanced with sync cleanup. | ✓        |
| New command             | Phase 1's command stays DB-only, new command handles both.              |          |
| You decide              | Let Claude pick.                                                        |          |

**User's choice:** Enhance existing
**Notes:** Keep command surface area minimal.

---

## Re-sync Scope

| Option                     | Description                                                           | Selected |
| -------------------------- | --------------------------------------------------------------------- | -------- |
| Dirty only (Recommended)   | Re-sync only pending/stale/error assignments. Skip already-synced.    |          |
| Full re-sync               | Re-sync every assignment. Overwrite existing. Guarantees consistency. | ✓        |
| Dirty default + force flag | Default to dirty-only with optional force flag.                       |          |
| You decide                 | Let Claude pick.                                                      |          |

**User's choice:** Full re-sync
**Notes:** User chose full re-sync over recommended dirty-only — values consistency guarantee.

---

| Option                  | Description                                                                         | Selected |
| ----------------------- | ----------------------------------------------------------------------------------- | -------- |
| Overwrite (Recommended) | Remove existing and re-create. Uses sync_dir_hybrid_with_overwrite(overwrite=true). | ✓        |
| Skip if valid           | Only overwrite if target is missing, broken, or pointing elsewhere.                 |          |
| You decide              | Let Claude pick.                                                                    |          |

**User's choice:** Overwrite
**Notes:** Full overwrite for maximum consistency.

---

| Option                   | Description                                                     | Selected |
| ------------------------ | --------------------------------------------------------------- | -------- |
| Continue on error (Rec.) | Log error on assignment, continue syncing rest. Return summary. | ✓        |
| Fail fast                | Stop entire sync on first failure.                              |          |
| You decide               | Let Claude pick.                                                |          |

**User's choice:** Continue on error
**Notes:** Per-assignment error tracking with summary report.

---

## Staleness Detection

| Option               | Description                                                        | Selected |
| -------------------- | ------------------------------------------------------------------ | -------- |
| On list load (Rec.)  | Compute hash when frontend loads assignments. Automatic detection. | ✓        |
| On sync trigger only | Only check on Sync Project / Sync All. No automatic detection.     |          |
| On app startup       | Check all copy-mode assignments on startup. Could slow startup.    |          |
| You decide           | Let Claude pick.                                                   |          |

**User's choice:** On list load
**Notes:** Automatic detection when user views assignments.

---

| Option                      | Description                                                         | Selected |
| --------------------------- | ------------------------------------------------------------------- | -------- |
| In assignment record (Rec.) | Store content_hash in project_skill_assignments row at sync time.   | ✓        |
| Separate hash table         | Separate lookup table keyed by skill_id. Shared across assignments. |          |
| You decide                  | Let Claude pick.                                                    |          |

**User's choice:** In assignment record
**Notes:** Keeps it in existing schema, per-assignment granularity.

---

| Option               | Description                                                       | Selected |
| -------------------- | ----------------------------------------------------------------- | -------- |
| Skip symlinks (Rec.) | Only check copy-mode targets. Symlinks propagate instantly.       | ✓        |
| Check all modes      | Check symlinks (verify link target) and copies (hash comparison). |          |
| You decide           | Let Claude pick.                                                  |          |

**User's choice:** Skip symlinks
**Notes:** Avoids unnecessary I/O for the common symlink case.

---

## Concurrency Scope

| Option              | Description                                                           | Selected |
| ------------------- | --------------------------------------------------------------------- | -------- |
| Global mutex (Rec.) | One Mutex<()> in Tauri state, one sync at a time across all projects. | ✓        |
| Per-project mutex   | One Mutex per project_id. Parallel across projects.                   |          |
| You decide          | Let Claude pick.                                                      |          |

**User's choice:** Global mutex
**Notes:** Simple, matches CancelToken pattern. Sync operations are fast enough.

---

| Option                  | Description                                                        | Selected |
| ----------------------- | ------------------------------------------------------------------ | -------- |
| std::sync::Mutex (Rec.) | Standard library mutex. Fine since sync work is in spawn_blocking. |          |
| tokio::sync::Mutex      | Async-aware mutex. More correct for async context.                 |          |
| You decide              | Let Claude pick.                                                   | ✓        |

**User's choice:** You decide (Claude's discretion)
**Notes:** Implementation detail deferred to Claude.

---

## Claude's Discretion

- Mutex implementation type (std::sync vs tokio::sync)
- Module organization (new project_sync.rs vs extending project_ops.rs)
- Internal function signatures and helper decomposition
- Test structure and coverage
- Whether content_hash needs a new column or schema migration

## Deferred Ideas

None — discussion stayed within phase scope.
