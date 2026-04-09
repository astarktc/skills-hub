# Phase 6: Gap Closure — Tool Removal Cleanup & Missing Status - Context

**Gathered:** 2026-04-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Fix two partially-met requirements identified by the v1.0 milestone audit: (1) `remove_project_tool` must cascade to assignments and filesystem artifacts, not just delete the DB tool row; (2) the `missing` assignment status must be produced by backend code when skill source or deployed target is absent.

</domain>

<decisions>
## Implementation Decisions

### Tool Removal Cleanup (TOOL-03)

- **D-01:** `remove_project_tool` iterates all assignments for the tool via `unassign_and_cleanup()` per assignment, then deletes the tool row. This follows the established pattern used by `remove_project_with_cleanup()` (Phase 5) and skill deletion cleanup.
- **D-02:** The command function in `commands/projects.rs` must acquire `SyncMutex` before cleanup, since it now performs filesystem operations. Follows the same pattern as the Phase 5 fix for `remove_project`.
- **D-03:** The function needs access to the project record and skill records to call `unassign_and_cleanup()`. Query assignments for the tool, look up each skill, then call `unassign_and_cleanup(store, project, skill, tool_key)` in a loop.

### Missing Status Production (SYNC-01)

- **D-04:** `list_assignments_with_staleness()` in `project_sync.rs` is the detection point. Currently at line 238 it checks `source.exists()` and skips when false. Instead, when source is absent, set status to `missing`.
- **D-05:** Two triggers for `missing` status: (a) central repo skill directory does not exist (`source.exists()` is false), (b) deployed target symlink/copy does not exist at the project's tool directory. Either condition produces `missing`.
- **D-06:** Detection persists to DB — when `missing` is detected, call `store.update_assignment_status()` with `"missing"`, same as how `stale` detection persists. Status survives across UI refreshes.
- **D-07:** If the source reappears (skill reinstalled or restored), the next staleness check will find `source.exists()` true and recalculate status normally (synced or stale), overwriting the `missing` status.

### Claude's Discretion

- Exact query approach for listing assignments filtered by tool (SQL query vs filter in Rust)
- Whether target-absent check uses the same path resolution as `assign_and_sync` or a simpler existence check
- Test structure and coverage scope for the two fixes
- Whether to add a dedicated helper function or inline the cleanup loop in `remove_project_tool`

</decisions>

<canonical_refs>

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Tool Removal (the bug)

- `src-tauri/src/core/skill_store.rs:703-711` — Current `remove_project_tool()` — only deletes DB tool row
- `src-tauri/src/commands/projects.rs:100-112` — Command wrapper — no SyncMutex, no cleanup

### Cleanup Pattern (follow this)

- `src-tauri/src/core/project_sync.rs:262` — `unassign_and_cleanup()` — canonical per-assignment cleanup
- `src-tauri/src/core/project_sync.rs:29` — `assign_and_sync()` — shows path resolution pattern
- `src-tauri/src/core/project_ops.rs` — `remove_project_with_cleanup()` — iterates assignments + cleanup pattern from Phase 5

### SyncMutex Pattern (follow this)

- `src-tauri/src/commands/projects.rs:117-164` — `add_project_skill_assignment` — SyncMutex acquisition pattern
- `src-tauri/src/commands/projects.rs:26-36` — `remove_project` — Phase 5 SyncMutex fix pattern
- `src-tauri/src/lib.rs:14,40` — SyncMutex definition and registration

### Missing Status Detection (the bug)

- `src-tauri/src/core/project_sync.rs:224-260` — `list_assignments_with_staleness()` — where detection goes
- `src-tauri/src/core/project_sync.rs:238` — `source.exists()` check that currently skips when false
- `src-tauri/src/core/skill_store.rs:946` — `aggregate_project_sync_status` — already handles "missing" in match arm

### Frontend (already supports missing)

- `src/components/projects/AssignmentMatrix.tsx:158` — Missing banner rendering
- `src/components/projects/ProjectList.tsx:64` — Missing project class
- `src/App.css` — `.matrix-cell.missing` CSS class (Phase 5 D-23)

### Audit Source

- `.planning/v1.0-MILESTONE-AUDIT.md` — Gap definitions driving this phase

### Prior Phase Context

- `.planning/phases/02-sync-logic/02-CONTEXT.md` — Sync atomicity (D-01/D-02), SyncMutex (D-10)
- `.planning/phases/05-edge-cases-and-polish/05-CONTEXT.md` — INFR-03 cascade design (D-17/D-18), missing CSS (D-23)

</canonical_refs>

<code_context>

## Existing Code Insights

### Reusable Assets

- `unassign_and_cleanup()` — Per-assignment cleanup (DB + filesystem), the building block for tool removal
- `remove_project_with_cleanup()` — Iterative cleanup pattern to follow
- `list_project_skill_assignments()` — Query all assignments for a project (filter by tool in Rust or SQL)
- `update_assignment_status()` — Persist status changes to DB
- `tool_adapters::adapter_by_key()` — Resolve tool adapter for path computation
- `resolve_project_sync_target()` — Compute target path for existence check

### Established Patterns

- Cleanup operations: iterate records, call per-record cleanup, then delete parent
- SyncMutex: acquired in command layer, passed through to blocking closure
- Status lifecycle: pending -> synced/stale/error/missing, detected in list call, persisted to DB
- Test fixtures: `tempfile` crate for temp directories, `SkillStore::new_test()` for in-memory DB

### Integration Points

- `commands/projects.rs:remove_project_tool` — Add SyncMutex param, add cleanup loop before DB delete
- `project_sync.rs:list_assignments_with_staleness` — Add source-absent and target-absent checks producing "missing"
- `skill_store.rs:remove_project_tool` — May need to return assignments before deleting, or add a query method

</code_context>

<specifics>
## Specific Ideas

- Tool removal cleanup should mirror the `remove_project_with_cleanup` pattern: iterate, clean, then delete parent row.
- Missing detection covers two cases: (a) skill deleted/corrupted in central repo, (b) user manually removed a symlink from their project. Both show red "missing" in the matrix.
- If source reappears, next refresh auto-recovers from missing -> synced/stale.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

_Phase: 06-gap-closure_
_Context gathered: 2026-04-08_
