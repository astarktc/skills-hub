# Phase 6: Gap Closure — Tool Removal Cleanup & Missing Status - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-08
**Phase:** 06-gap-closure
**Areas discussed:** Tool removal strategy, Missing status triggers, SyncMutex for tool removal

---

## Tool Removal Strategy

| Option                            | Description                                                                                                                                                 | Selected |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| Iterate unassign_and_cleanup      | Query assignments for the tool, call unassign_and_cleanup() for each, then delete the tool row. Consistent with remove_project and skill deletion patterns. | ✓        |
| Batch DB delete + filesystem pass | Delete all assignment rows in one SQL query, then do a separate filesystem pass. Faster but introduces a second cleanup pattern.                            |          |
| Claude decides                    | Let Claude pick based on code patterns.                                                                                                                     |          |

**User's choice:** Iterate unassign_and_cleanup (Recommended)
**Notes:** Consistent with existing patterns. Guaranteed correct even if slower for many assignments.

---

## Missing Status Triggers

| Option                  | Description                                                                                                               | Selected |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------- | -------- |
| Source absent only      | In list_assignments_with_staleness, when source.exists() is false, set status to 'missing'. Covers the audit gap cleanly. |          |
| Source OR target absent | Check both: source skill dir missing OR deployed target symlink/copy missing. Catches more edge cases.                    | ✓        |
| Claude decides          | Let Claude pick based on codebase.                                                                                        |          |

**User's choice:** Source OR target absent
**Notes:** Both central repo source and deployed target are checked. Catches user-deleted symlinks as well as missing skills.

### Follow-up: Persist to DB?

| Option                     | Description                                                                         | Selected |
| -------------------------- | ----------------------------------------------------------------------------------- | -------- |
| Yes, update DB to missing  | When detected, update DB status same as stale detection. Persists across refreshes. | ✓        |
| Computed only, no DB write | Return 'missing' in DTO but don't persist. Resets if source reappears.              |          |

**User's choice:** Yes, update DB to missing (Recommended)
**Notes:** Follows the same pattern as stale detection persistence.

---

## SyncMutex for Tool Removal

| Option             | Description                                                                              | Selected |
| ------------------ | ---------------------------------------------------------------------------------------- | -------- |
| Yes, add SyncMutex | Acquire lock before cleanup. Consistent with Phase 5 remove_project fix. Prevents races. | ✓        |
| No, keep it simple | Skip mutex. Tool removal is user-initiated and unlikely to race.                         |          |

**User's choice:** Yes, add SyncMutex (Recommended)
**Notes:** Consistency with other sync-touching operations.

---

## Claude's Discretion

- Query approach for listing assignments filtered by tool
- Target-absent check implementation details
- Test structure and coverage scope
- Helper function vs inline cleanup loop

## Deferred Ideas

None — discussion stayed within phase scope.
