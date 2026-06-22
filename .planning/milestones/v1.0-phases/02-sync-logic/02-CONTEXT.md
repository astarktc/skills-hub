# Phase 2: Sync Logic - Context

**Gathered:** 2026-04-07
**Status:** Ready for planning

<domain>
## Phase Boundary

Project-aware sync operations: assigning a skill creates a symlink/copy in the project's tool directory, unassigning removes it, re-sync repairs all assignments, and copy-mode targets are checked for staleness via content hashing. Concurrent sync operations are serialized via a global mutex. No UI — this phase delivers the core sync functions that Phase 3 (IPC Commands) will expose to the frontend.

</domain>

<decisions>
## Implementation Decisions

### Sync Atomicity

- **D-01:** Two-step assign: create DB record with status='pending', then immediately sync. If sync succeeds → update to 'synced' with content_hash and synced_at. If sync fails → record stays with status='error' + last_error. User can retry via re-sync.
- **D-02:** Assign command does both DB insert and sync inline — one call from the frontend returns the assignment with its final status (synced/error). No separate sync command needed for individual assignments.
- **D-03:** Unassign is strict: remove symlink/copy from filesystem first, then delete DB record. If filesystem removal fails, keep record with status='error' so user sees what went wrong.
- **D-04:** Enhance existing `remove_project_skill_assignment` command with sync cleanup rather than creating a new command. Phase 1's command becomes sync-aware.

### Re-sync Scope

- **D-05:** Full re-sync: 'Sync Project' and 'Sync All' re-sync ALL assignments regardless of current status. Uses `sync_dir_hybrid_with_overwrite(overwrite=true)` to guarantee target matches source.
- **D-06:** Continue on error: if one assignment fails during bulk sync, log the error on that assignment (status='error') and continue syncing the rest. Return a summary of successes and failures.

### Staleness Detection

- **D-07:** Staleness checked on assignment list load (when frontend loads assignments for a project). Automatic detection — no explicit user trigger needed.
- **D-08:** Store source content hash (`content_hash::hash_dir()`) in the `project_skill_assignments` row at sync time. On list load, recompute hash on source and compare. If different → status='stale'.
- **D-09:** Skip symlink-mode targets for staleness detection — symlinks propagate changes instantly by pointing to the source directory. Only copy-mode targets can be stale. This avoids unnecessary I/O for the common case.

### Concurrency

- **D-10:** Global mutex: one `Mutex<()>` registered in Tauri state, shared across all sync operations. Only one sync runs at a time across all projects. Simple to reason about, matches existing CancelToken pattern.
- **D-11:** Mutex implementation type is at Claude's discretion (std::sync::Mutex vs tokio::sync::Mutex — both valid since sync work runs in spawn_blocking).

### Claude's Discretion

- Mutex implementation choice (std::sync vs tokio::sync)
- Module organization: whether sync logic goes in a new `project_sync.rs` or extends `project_ops.rs`
- Internal function signatures and helper decomposition
- Test structure and coverage scope for sync operations
- Whether to add a `content_hash` column to the assignment table (new migration) or use the existing `last_error` field to store hash metadata

</decisions>

<canonical_refs>

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Sync Engine (reuse unchanged)

- `src-tauri/src/core/sync_engine.rs` — `sync_dir_hybrid()`, `sync_dir_hybrid_with_overwrite()`, `sync_dir_for_tool_with_overwrite()` (Cursor forces copy), `remove_path_any()`, `copy_dir_recursive()`
- `src-tauri/src/core/content_hash.rs` — `hash_dir()` for staleness detection

### Tool Adapters (path resolution)

- `src-tauri/src/core/tool_adapters/mod.rs` — `ToolAdapter.relative_skills_dir` is the key: target = `project_path / relative_skills_dir / skill_name`. Also `adapter_by_key()` for tool string → adapter lookup.

### Phase 1 Deliverables (build on top of)

- `src-tauri/src/core/project_ops.rs` — `register_project_path()`, `remove_project_with_cleanup()` (already has symlink removal), `to_project_dto()`, DTOs
- `src-tauri/src/commands/projects.rs` — 9 Tauri commands including `add_project_skill_assignment` (creates with status='pending'), `remove_project_skill_assignment` (DB-only, needs sync enhancement)
- `src-tauri/src/core/skill_store.rs` — Schema V4, `ProjectSkillAssignmentRecord`, all CRUD methods including `list_project_skill_assignments_for_project_tool()`, `aggregate_project_sync_status()`

### State Management

- `src-tauri/src/lib.rs` — Tauri state registration (`app.manage()`), `generate_handler![]` for command registration, `CancelToken` pattern for shared state

### Phase 1 Context (prior decisions)

- `.planning/phases/01-data-foundation/01-CONTEXT.md` — Schema design decisions (D-03, D-04), delete behavior (D-06, D-07), path handling (D-08, D-09)

### Project Docs

- `.planning/PROJECT.md` — Constraints (reuse sync_engine unchanged), brownfield context
- `.planning/REQUIREMENTS.md` — ASGN-02, ASGN-03, ASGN-05, SYNC-04, INFR-01, INFR-02 definitions

</canonical_refs>

<code_context>

## Existing Code Insights

### Reusable Assets

- `sync_engine::sync_dir_hybrid_with_overwrite()` — Drop-in for assign sync with overwrite=false (first assign) and re-sync with overwrite=true
- `sync_engine::sync_dir_for_tool_with_overwrite()` — Tool-aware variant that forces copy for Cursor. Use this instead of raw hybrid for tool-specific behavior.
- `sync_engine::remove_path_any()` — Handles symlinks, directories, and files. Already used by `remove_project_with_cleanup()`.
- `content_hash::hash_dir()` — Returns SHA-256 hex string of directory contents. Ready to use for staleness.
- `tool_adapters::adapter_by_key()` — Converts tool string (from DB) to ToolAdapter with relative_skills_dir.
- `project_ops::remove_project_with_cleanup()` — Pattern for iterating assignments and removing filesystem targets.

### Established Patterns

- Sync fallback chain: symlink → junction (Windows) → copy — handled transparently by `sync_dir_hybrid()`
- Cursor exception: `sync_dir_for_tool_with_overwrite()` forces copy mode for Cursor tool
- Assignment status lifecycle: pending → synced/error, with stale as a detection state
- Blocking work wrapped in `tauri::async_runtime::spawn_blocking`
- Shared state via `app.manage()` + `State<'_, T>` in commands

### Integration Points

- `commands/projects.rs:add_project_skill_assignment` — Currently creates DB record only. Phase 2 adds sync logic after DB insert.
- `commands/projects.rs:remove_project_skill_assignment` — Currently deletes DB record only. Phase 2 adds filesystem cleanup before delete.
- `lib.rs` — New global sync mutex needs registration via `app.manage()`
- `skill_store.rs` — May need `update_project_skill_assignment_status()` method and possibly a `content_hash` column

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches following existing codebase patterns.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

_Phase: 02-sync-logic_
_Context gathered: 2026-04-07_
