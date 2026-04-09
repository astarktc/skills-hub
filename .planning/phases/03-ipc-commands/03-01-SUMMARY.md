---
phase: 03-ipc-commands
plan: 01
subsystem: ipc-commands
tags: [tauri-commands, ipc, dto, error-handling, bulk-assign]
dependency_graph:
  requires: [02-01, 02-02]
  provides:
    [
      bulk_assign_skill command,
      error prefix infrastructure,
      TypeScript project DTOs,
    ]
  affects:
    [
      src-tauri/src/commands/projects.rs,
      src-tauri/src/commands/mod.rs,
      src-tauri/src/lib.rs,
      src/components/projects/types.ts,
    ]
tech_stack:
  added: []
  patterns:
    [error-prefix-passthrough, bulk-operation-continue-on-error, DTO-mirroring]
key_files:
  created:
    - src/components/projects/types.ts
  modified:
    - src-tauri/src/commands/projects.rs
    - src-tauri/src/commands/mod.rs
    - src-tauri/src/lib.rs
    - src-tauri/src/commands/tests/commands.rs
    - src-tauri/src/core/tests/project_sync.rs
decisions:
  - "bulk_assign_skill silently skips already-assigned tools (no error, just continue)"
  - "DUPLICATE_PROJECT| prefix constructed in register_project command layer, not core layer"
  - "ASSIGNMENT_EXISTS| pre-check done before sync mutex acquisition to fail fast"
  - "Added TOOL_NOT_WRITABLE| and SKILL_INVALID| to passthrough list (already used in code but missing from list)"
metrics:
  duration: 10m 37s
  completed: "2026-04-08T05:36:00Z"
  tasks: 3/3
  files_changed: 6
---

# Phase 03 Plan 01: IPC Commands and Error Prefix Infrastructure Summary

Wired bulk_assign_skill Tauri command with per-tool success/failure detail, added error prefix passthrough for 5 new prefixes (DUPLICATE_PROJECT|, ASSIGNMENT_EXISTS|, NOT_FOUND|, TOOL_NOT_WRITABLE|, SKILL_INVALID|), and created TypeScript DTO types for Phase 4 frontend consumption.

## Commits

| Task | Commit  | Message                                                                                               |
| ---- | ------- | ----------------------------------------------------------------------------------------------------- |
| 1    | 46c405b | feat(03-01): add bulk_assign_skill command, error prefix infrastructure, and lib.rs registration      |
| 2    | fae16a7 | test(03-01): add tests for bulk-assign behavior, error prefix passthrough, and command-layer contract |
| 3    | 0bf7a42 | feat(03-01): create TypeScript DTO types for frontend project consumption                             |

## Task Details

### Task 1: Add bulk_assign_skill command, error prefix infrastructure, and lib.rs registration

- Added `bulk_assign_skill` async Tauri command with `BulkAssignResultDto` and `BulkAssignErrorDto` DTOs
- Command iterates all configured project tools, skips already-assigned, continues on error
- Returns per-tool detail: assigned (with full DTO) and failed (with tool + error message)
- NOT_FOUND|project: and NOT_FOUND|skill: errors on missing entities
- Added DUPLICATE_PROJECT| prefix emission in `register_project` command
- Added ASSIGNMENT_EXISTS| pre-check in `add_project_skill_assignment` command
- Extended `format_anyhow_error` passthrough with 5 new prefixes (3 new + 2 existing-but-missing)
- Registered command in `generate_handler!` macro in lib.rs

### Task 2: Add tests for bulk-assign behavior, error prefix passthrough, and command-layer contract

- 3 bulk-assign behavior tests in project_sync.rs:
  - `bulk_assign_to_multiple_tools`: assigns skill to claude_code + cursor, verifies both targets exist
  - `bulk_assign_skips_already_assigned`: pre-assigns, then bulk-assign loop correctly skips
  - `bulk_assign_continues_on_error`: claude_code succeeds, cursor fails (deleted source), both have DB records
- 3 error prefix passthrough tests in commands.rs:
  - `format_anyhow_error_passthrough_duplicate_project`
  - `format_anyhow_error_passthrough_assignment_exists`
  - `format_anyhow_error_passthrough_not_found`
- 1 command-layer contract test:
  - `bulk_assign_skill_not_found_error_contract`: verifies both project and skill NOT_FOUND variants

### Task 3: Create TypeScript DTO types for frontend consumption

- Created `src/components/projects/types.ts` with 6 DTO types mirroring Rust counterparts
- ProjectDto, ProjectToolDto, ProjectSkillAssignmentDto, ResyncSummaryDto, BulkAssignResultDto, BulkAssignErrorDto
- Optional fields use `?: type | null` pattern matching existing skills/types.ts convention

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical Functionality] Added TOOL_NOT_WRITABLE| and SKILL_INVALID| to passthrough list**

- **Found during:** Task 1
- **Issue:** These prefixes were already emitted by existing code (lines 496, 515, 713 of mod.rs) but were missing from the format_anyhow_error passthrough check, meaning they would be mangled before reaching the frontend
- **Fix:** Added both to the passthrough if-block alongside the 3 new prefixes
- **Files modified:** src-tauri/src/commands/mod.rs
- **Commit:** 46c405b

## Verification

- `cargo build --manifest-path src-tauri/Cargo.toml` -- compiles without errors
- `cargo test --manifest-path src-tauri/Cargo.toml` -- 119 tests pass (7 new)
- `npx tsc --noEmit` -- TypeScript types compile without errors
- `npm run check` -- full suite passes (lint + build + rust:fmt:check + rust:clippy + rust:test)

## Self-Check: PASSED

All 6 key files verified present. All 3 commit hashes verified in git log.
