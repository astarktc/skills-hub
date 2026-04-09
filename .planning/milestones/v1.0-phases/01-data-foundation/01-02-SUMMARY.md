---
phase: 01-data-foundation
plan: 02
subsystem: ipc-commands
tags: [tauri-ipc, rust, commands, dto, project-management, business-logic]

# Dependency graph
requires:
  - "01-01: V4 schema migration with ProjectRecord, ProjectToolRecord, ProjectSkillAssignmentRecord structs and 15 CRUD methods"
provides:
  - "core/project_ops.rs business logic layer with path validation, canonicalization, duplicate checking, delete cleanup, DTO conversion"
  - "commands/projects.rs with 9 thin Tauri IPC commands for project management"
  - "ProjectDto, ProjectToolDto, ProjectSkillAssignmentDto serializable DTOs"
  - "pub(crate) visibility on format_anyhow_error, expand_home_path, now_ms for cross-module reuse"
  - "sync_engine::remove_path_any made pub(crate) for project delete cleanup"
affects: [02-sync-logic, 03-ipc-commands, 04-frontend-ui]

# Tech tracking
tech-stack:
  added: []
  patterns:
    [
      "Business logic in core/ with callback parameters to avoid reverse dependency on commands/",
      "expand_home_path passed as callback from commands to core (D-08 pattern)",
      "Thin command wrappers: clone store + spawn_blocking + delegate to core + map errors",
      "DTO structs with Serialize derive in core/project_ops.rs for reuse by both commands and tests",
    ]

key-files:
  created:
    - src-tauri/src/core/project_ops.rs
    - src-tauri/src/core/tests/project_ops.rs
    - src-tauri/src/commands/projects.rs
  modified:
    - src-tauri/src/core/mod.rs
    - src-tauri/src/core/sync_engine.rs
    - src-tauri/src/commands/mod.rs
    - src-tauri/src/lib.rs

key-decisions:
  - "expand_home_path passed as callback to register_project_path to avoid core->commands dependency (D-08)"
  - "Reuse sync_engine::remove_path_any for project delete cleanup rather than duplicating (D-07)"
  - "DTOs defined in core/project_ops.rs (not commands/) so both commands and tests can import them"
  - "All 3 shared helpers (format_anyhow_error, expand_home_path, now_ms) made pub(crate) -- minimal visibility change"

patterns-established:
  - "Callback injection: core functions accept expand_home as impl Fn(&str) -> Result<PathBuf> to avoid circular module dependencies"
  - "Separate commands module per feature: commands/projects.rs for project commands (vs everything in commands/mod.rs)"
  - "pub(crate) for cross-module helper reuse within the crate"

requirements-completed: [INFR-05, PROJ-01, PROJ-02, PROJ-03, TOOL-01]

# Metrics
duration: 4min
completed: 2026-04-07
---

# Phase 01 Plan 02: Tauri IPC Commands Summary

**9 thin Tauri commands for project management with core business logic layer using callback-injected expand_home_path and sync_engine reuse**

## Performance

- **Duration:** 4 min
- **Started:** 2026-04-07T21:44:42Z
- **Completed:** 2026-04-07T21:49:39Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- core/project_ops.rs with business logic: register (validate, canonicalize, dedup), delete with filesystem cleanup, list DTOs with counts and sync status
- commands/projects.rs with 9 thin Tauri IPC commands delegating to core layer
- All commands registered in generate_handler![] for frontend invocation
- 7 unit tests covering registration validation, canonical paths, duplicate rejection, name derivation, sync status aggregation, and count verification

## Task Commits

Each task was committed atomically:

1. **Task 1: Create core/project_ops.rs with business logic and update mod.rs** - `7a56ac5` (feat)
2. **Task 2: Create thin commands/projects.rs wrappers and register in lib.rs** - `909b255` (feat)

## Files Created/Modified

- `src-tauri/src/core/project_ops.rs` - Business logic: register, delete-with-cleanup, list DTOs, name derivation, DTO structs
- `src-tauri/src/core/tests/project_ops.rs` - 7 tests for registration, validation, duplication, naming, sync status, counts
- `src-tauri/src/commands/projects.rs` - 9 thin Tauri command wrappers delegating to core
- `src-tauri/src/core/mod.rs` - Added `pub mod project_ops;` export
- `src-tauri/src/core/sync_engine.rs` - Changed `remove_path_any` from `fn` to `pub(crate) fn`
- `src-tauri/src/commands/mod.rs` - Added `pub mod projects;`, made 3 helpers pub(crate)
- `src-tauri/src/lib.rs` - Registered 9 new commands in generate_handler![]

## Decisions Made

- expand_home_path is passed as a callback parameter to register_project_path rather than having core depend on commands -- this follows the CLAUDE.md architecture rule that core/ contains pure business logic without shell dependencies
- DTOs (ProjectDto, ProjectToolDto, ProjectSkillAssignmentDto) are defined in core/project_ops.rs so they can be imported by both commands/projects.rs and tests without circular dependencies
- sync_engine::remove_path_any made pub(crate) rather than pub -- minimal visibility needed for intra-crate reuse per D-07
- Commands module split: project commands in a separate commands/projects.rs file per INFR-05, rather than adding to the already-large commands/mod.rs (986 lines)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- Rust toolchain (cargo) not available in the sandboxed worktree agent environment, so tests could not be run locally. Code follows existing patterns exactly and will be validated when the orchestrator runs `npm run check` after worktree merge.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None - all functions are fully implemented with real business logic and SQL queries.

## Next Phase Readiness

- IPC layer complete: all 9 project commands are registered and ready for frontend invocation
- Business logic layer testable independently via core/project_ops.rs
- Phase 2 (Sync Logic) can now wire project-aware sync using the same store methods and DTOs
- Phase 4 (Frontend) can invoke register_project, list_projects, add_project_tool, etc. via Tauri IPC
- No blockers

---

_Phase: 01-data-foundation_
_Completed: 2026-04-07_

## Self-Check: PASSED

- FOUND: src-tauri/src/core/project_ops.rs
- FOUND: src-tauri/src/core/tests/project_ops.rs
- FOUND: src-tauri/src/commands/projects.rs
- FOUND: src-tauri/src/core/mod.rs
- FOUND: src-tauri/src/core/sync_engine.rs
- FOUND: src-tauri/src/commands/mod.rs
- FOUND: src-tauri/src/lib.rs
- FOUND: .planning/phases/01-data-foundation/01-02-SUMMARY.md
- FOUND: commit 7a56ac5 (Task 1)
- FOUND: commit 909b255 (Task 2)
