# Phase 3: IPC Commands - Context

**Gathered:** 2026-04-07
**Status:** Ready for planning

<domain>
## Phase Boundary

Tauri IPC command layer making all project management and sync operations callable from the frontend. Phases 1 and 2 already created 11 commands in `commands/projects.rs` — this phase adds the missing bulk-assign command, frontend TypeScript DTOs, and error prefix conventions. No UI rendering — that's Phase 4.

</domain>

<decisions>
## Implementation Decisions

### Bulk-Assign Command

- **D-01:** Single backend command `bulk_assign_skill` — one IPC call where the backend reads configured tools for the project, assigns the skill to each via `assign_and_sync`, and returns a detailed result.
- **D-02:** Response is `BulkAssignResultDto` with per-tool detail: `assigned: Vec<ProjectSkillAssignmentDto>` for successes and `failed: Vec<BulkAssignErrorDto>` (tool + error string) for failures. Frontend can show exactly which tools succeeded/failed.
- **D-03:** Follows Phase 2 D-06 pattern: continue on error — if one tool fails, keep assigning the rest and report all results.

### Frontend TypeScript DTOs

- **D-04:** Project DTOs go in a new `src/components/projects/types.ts` file, separate from existing `src/components/skills/types.ts`. This anticipates Phase 4's separate component tree and avoids mixing concerns.
- **D-05:** Types to define: `ProjectDto`, `ProjectToolDto`, `ProjectSkillAssignmentDto`, `ResyncSummaryDto`, `BulkAssignResultDto`, `BulkAssignErrorDto` — all mirroring their Rust counterparts.

### Error Response Contract

- **D-06:** Project commands use specific error prefixes for frontend detection, following the existing pattern (`MULTI_SKILLS|`, `TARGET_EXISTS|`, etc.).
- **D-07:** Three prefixes to implement:
  - `DUPLICATE_PROJECT|` — path already registered. Frontend can highlight duplicate and offer navigation.
  - `ASSIGNMENT_EXISTS|` — skill already assigned to that project+tool. Frontend shows "already assigned" instead of generic error.
  - `NOT_FOUND|` — project or skill ID doesn't exist (deleted between list load and action). Frontend triggers a refresh.

### Claude's Discretion

- Internal function signatures and parameter naming for the bulk-assign command
- Whether `BulkAssignErrorDto` is a new struct or reuses an existing pattern
- Test structure for the new bulk-assign command and error prefix validation
- Whether error prefixes are emitted from the command layer or the core layer

</decisions>

<canonical_refs>

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Existing IPC Layer (build on top of)

- `src-tauri/src/commands/projects.rs` — 11 existing project commands with sync-aware assign/unassign, resync_project, resync_all_projects
- `src-tauri/src/commands/mod.rs` — Existing command patterns, `format_anyhow_error()`, `expand_home_path()`, error prefix conventions (`MULTI_SKILLS|`, `TARGET_EXISTS|`, etc.)
- `src-tauri/src/lib.rs` — Command registration via `generate_handler![]`, all 11 project commands already registered (lines 108-118)

### Core Layer (called by commands)

- `src-tauri/src/core/project_sync.rs` — `assign_and_sync()`, `unassign_and_cleanup()`, `resync_project()`, `resync_all_projects()`, `list_assignments_with_staleness()`, `ResyncSummary`
- `src-tauri/src/core/project_ops.rs` — `ProjectDto`, `ProjectToolDto`, `ProjectSkillAssignmentDto` (Serialize DTOs), `register_project_path()`, `list_project_dtos()`
- `src-tauri/src/core/skill_store.rs` — `list_project_tools()`, `get_project_by_id()`, `get_skill_by_id()`, `ProjectToolRecord`, `ProjectSkillAssignmentRecord`

### Frontend DTO Contract

- `src/components/skills/types.ts` — Existing DTO pattern to follow (named exports, mirrors Rust DTOs)
- `src/components/projects/types.ts` — NEW file to create with project-specific DTOs

### Project Docs

- `.planning/PROJECT.md` — Constraints (minimize App.tsx changes, separate component tree)
- `.planning/REQUIREMENTS.md` — ASGN-01, ASGN-04, SYNC-02, SYNC-03 definitions

### Prior Phase Context

- `.planning/phases/01-data-foundation/01-CONTEXT.md` — Schema design, CRUD patterns, command module structure
- `.planning/phases/02-sync-logic/02-CONTEXT.md` — Sync atomicity, re-sync scope, staleness detection, concurrency

</canonical_refs>

<code_context>

## Existing Code Insights

### Reusable Assets

- `project_sync::assign_and_sync()` — Drop-in for per-tool assignment within bulk-assign loop
- `store.list_project_tools()` — Gets configured tools for a project (drives bulk-assign iteration)
- `format_anyhow_error()` — Error chain formatting, extend with new prefix detection
- `ResyncSummaryDto` — Pattern for bulk operation result DTOs (synced/failed counts + errors)

### Established Patterns

- Error prefix convention: command layer checks error string and prepends prefix before returning to frontend
- DTO naming: Rust `*Dto` structs with `#[derive(serde::Serialize)]`, mirrored as TypeScript `export type`
- Command wrapping: `spawn_blocking` + `SyncMutex` lock for sync operations
- Bulk operation error handling: continue on error, collect results (Phase 2 D-06)

### Integration Points

- `commands/projects.rs` — Add `bulk_assign_skill` command, add error prefix logic to existing `register_project` and `add_project_skill_assignment`
- `lib.rs:generate_handler![]` — Register new `bulk_assign_skill` command
- `src/components/projects/types.ts` — New file, Phase 4 will import from here

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

_Phase: 03-ipc-commands_
_Context gathered: 2026-04-07_
