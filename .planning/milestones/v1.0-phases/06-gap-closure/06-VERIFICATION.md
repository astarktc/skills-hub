---
phase: 06-gap-closure
verified: 2026-04-09T03:19:48Z
status: human_needed
score: 6/7 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Remove a configured tool column from a project that already has assigned skills"
    expected: "The tool column disappears, its assignment cells disappear after refresh, and the corresponding symlink/copy artifacts are gone from the project tool directory with no orphaned records left behind"
    why_human: "This phase includes an end-to-end user flow success criterion. The backend and frontend wiring are present, but I could not execute the Tauri/Rust flow here because the environment lacks cargo."
  - test: "Open the Projects assignment matrix for an assignment whose source or deployed target is missing"
    expected: "The matrix cell renders the missing state visually in red, persists after reload, and recovers to synced or stale after the source/target is restored and assignments are re-fetched"
    why_human: "The code wires the missing status through to the matrix and CSS, but visual confirmation of the rendered color/state in the desktop app still requires manual validation."
---

# Phase 6: Gap Closure — Tool Removal Cleanup & Missing Status Verification Report

**Phase Goal:** Tool column removal cascades to assignments/artifacts, and `missing` assignment status is produced when skill source is absent
**Verified:** 2026-04-09T03:19:48Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #   | Truth                                                                                                                                                     | Status      | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | Removing a tool column from a project deletes all assignments for that tool and cleans up filesystem artifacts (symlinks/copies) in the project directory | ✓ VERIFIED  | `src-tauri/src/core/project_ops.rs:102-159` implements `remove_tool_with_cleanup`, iterates tool assignments, calls `project_sync::unassign_and_cleanup`, and finally removes the tool row. Regression tests in `src-tauri/src/core/tests/project_ops.rs:252-338` verify assignment deletion, artifact cleanup, and tool-row removal.                                                                                                                                                            |
| 2   | When a skill's central repo directory no longer exists, assignments referencing it report `missing` status instead of silently failing                    | ✓ VERIFIED  | `src-tauri/src/core/project_sync.rs:233-277` computes `source_exists` and early-continues to persisted `missing` status when the source is absent. Test `missing_status_when_source_absent` in `src-tauri/src/core/tests/project_sync.rs:479-520` asserts both returned status and DB status become `missing`.                                                                                                                                                                                   |
| 3   | The "Remove tool column" E2E flow completes without orphaned data                                                                                         | ? UNCERTAIN | Frontend wiring exists: `src/components/projects/ToolConfigModal.tsx:60-62` submits selected tools, `src/components/projects/ProjectsPage.tsx:64-99` computes removals and calls `state.removeTools`, and `src/components/projects/useProjectState.ts:372-392` invokes `remove_project_tool` then re-fetches tools and assignments. Backend command is registered in `src-tauri/src/lib.rs:112-127`. I could not execute the Tauri/Rust flow because `cargo` is unavailable in this environment. |
| 4   | The assignment matrix displays `missing` status cells when appropriate                                                                                    | ✓ VERIFIED  | `src/components/projects/AssignmentMatrix.tsx:260-277` maps `assignment.status` into the cell class, and `src/App.css:2967-2969` defines `.matrix-cell.missing` with the danger background. Assignments are fetched from `list_project_skill_assignments` in `src/components/projects/useProjectState.ts:149-158` and refreshed after updates at `224-229`, `268-273`, and `316-339`.                                                                                                            |
| 5   | When a deployed target symlink/copy is absent, assignments show `missing` status                                                                          | ✓ VERIFIED  | `src-tauri/src/core/project_sync.rs:279-298` marks previously deployed assignments as `missing` when `target_exists` is false. Test `missing_status_when_target_absent` in `src-tauri/src/core/tests/project_sync.rs:818-867` covers symlink deletion and DB persistence.                                                                                                                                                                                                                        |
| 6   | Missing status is persisted to DB and survives UI refreshes                                                                                               | ✓ VERIFIED  | Backend persistence happens through `store.update_assignment_status(..., "missing", ...)` in `src-tauri/src/core/project_sync.rs:264-272` and `286-293`. Tests at `src-tauri/src/core/tests/project_sync.rs:511-519` and `858-866` re-read the store to confirm persistence. Frontend refresh path re-fetches assignments via `list_project_skill_assignments` in `src/components/projects/useProjectState.ts:149-158`, `224-229`, and `316-339`.                                                |
| 7   | If a missing source reappears, the next staleness check auto-recovers to synced/stale                                                                     | ✓ VERIFIED  | `src-tauri/src/core/project_sync.rs:300-338` recalculates status when both source and target exist, regardless of prior DB status, including `missing`. Test `missing_status_recovers_when_source_restored` in `src-tauri/src/core/tests/project_sync.rs:870-928` proves a previously missing assignment becomes `synced` or `stale`, not `missing`.                                                                                                                                             |

**Score:** 6/7 truths verified

### Required Artifacts

| Artifact                                   | Expected                                                      | Status     | Details                                                                                                         |
| ------------------------------------------ | ------------------------------------------------------------- | ---------- | --------------------------------------------------------------------------------------------------------------- |
| `src-tauri/src/core/project_ops.rs`        | `remove_tool_with_cleanup` function                           | ✓ VERIFIED | File exists, contains non-stub cleanup logic at `102-159`, and is consumed by the command layer.                |
| `src-tauri/src/commands/projects.rs`       | Updated `remove_project_tool` command with `SyncMutex`        | ✓ VERIFIED | `remove_project_tool` at `100-117` acquires `SyncMutex` before calling `project_ops::remove_tool_with_cleanup`. |
| `src-tauri/src/core/project_sync.rs`       | Missing status detection in `list_assignments_with_staleness` | ✓ VERIFIED | `224-340` implements source/target existence checks, persisted missing status, and recovery recalculation.      |
| `src-tauri/src/core/tests/project_ops.rs`  | Test coverage for tool removal cascade                        | ✓ VERIFIED | `252-474` contains the three planned cascade tests, including missing-skill handling.                           |
| `src-tauri/src/core/tests/project_sync.rs` | Test coverage for missing status detection                    | ✓ VERIFIED | `479-520` and `818-965` cover source missing, target missing, recovery, and both missing.                       |

### Key Link Verification

| From                                         | To                                             | Via                                    | Status  | Details                                                                                                                                                                                  |
| -------------------------------------------- | ---------------------------------------------- | -------------------------------------- | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src-tauri/src/commands/projects.rs`         | `src-tauri/src/core/project_ops.rs`            | `remove_tool_with_cleanup` call        | ✓ WIRED | `remove_project_tool` calls `project_ops::remove_tool_with_cleanup(&store, &projectId, &tool)` at `src-tauri/src/commands/projects.rs:110-113`.                                          |
| `src-tauri/src/core/project_ops.rs`          | `src-tauri/src/core/project_sync.rs`           | `unassign_and_cleanup` per assignment  | ✓ WIRED | `remove_tool_with_cleanup` delegates to `project_sync::unassign_and_cleanup` at `src-tauri/src/core/project_ops.rs:109-119`.                                                             |
| `src-tauri/src/core/project_sync.rs`         | `src-tauri/src/core/skill_store.rs`            | `update_assignment_status` for missing | ✓ WIRED | The gsd-tools pattern missed this because the call is multiline, but manual verification shows persisted `missing` writes at `src-tauri/src/core/project_sync.rs:264-272` and `286-293`. |
| `src/components/projects/ProjectsPage.tsx`   | `src/components/projects/useProjectState.ts`   | Tool config removal path               | ✓ WIRED | `handleToolConfigConfirm` computes `toRemove` and calls `state.removeTools(toRemove)` at `src/components/projects/ProjectsPage.tsx:67-74`.                                               |
| `src/components/projects/useProjectState.ts` | Tauri `remove_project_tool` command            | `invoke("remove_project_tool")`        | ✓ WIRED | `removeTools` invokes `remove_project_tool` and then re-fetches tools plus assignments at `src/components/projects/useProjectState.ts:372-392`.                                          |
| `src/components/projects/useProjectState.ts` | Tauri `list_project_skill_assignments` command | Assignment refresh after changes       | ✓ WIRED | Assignments are loaded on selection (`149-158`) and after toggle/bulk/resync/tool removal (`224-229`, `268-273`, `316-339`, `381-391`).                                                  |

### Data-Flow Trace (Level 4)

| Artifact                                       | Data Variable               | Source                                                                       | Produces Real Data                                                                                          | Status    |
| ---------------------------------------------- | --------------------------- | ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- | --------- |
| `src/components/projects/AssignmentMatrix.tsx` | `assignment.status`         | Tauri `list_project_skill_assignments` via `useProjectState`                 | Yes — backend `list_assignments_with_staleness` returns persisted assignment records with recomputed status | ✓ FLOWING |
| `src/components/projects/useProjectState.ts`   | `assignments` state         | `invoke("list_project_skill_assignments", { projectId })`                    | Yes — command maps backend DTOs in `src-tauri/src/commands/projects.rs:220-250`                             | ✓ FLOWING |
| `src-tauri/src/core/project_sync.rs`           | `assignment.status` updates | `SkillStore::update_assignment_status` plus filesystem existence/hash checks | Yes — source existence, target existence, and hash comparison drive status transitions                      | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior            | Command                                                                                                                                                                          | Result                                                                                                                                 | Status |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ------ |
| Full project checks | `npm run check`                                                                                                                                                                  | Frontend lint and build passed, but check stopped at `cargo fmt --all -- --check` because `cargo` is not installed in this environment | ? SKIP |
| Rust phase tests    | `cargo test --manifest-path /home/alexwsl/skills-hub/src-tauri/Cargo.toml -- project_ops::tests::remove_tool project_sync::tests::missing_status project_sync::tests::staleness` | Could not run: `/bin/bash: cargo: command not found`                                                                                   | ? SKIP |

### Requirements Coverage

| Requirement | Source Plan     | Description                                                                                      | Status        | Evidence                                                                                                                                                                                                                                 |
| ----------- | --------------- | ------------------------------------------------------------------------------------------------ | ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| TOOL-03     | `06-01-PLAN.md` | User can add or remove tool columns from a project at any time                                   | ? NEEDS HUMAN | Removal path is wired end-to-end through `ToolConfigModal` -> `ProjectsPage` -> `useProjectState.removeTools` -> Tauri `remove_project_tool` -> backend cleanup logic, but live user-flow confirmation still requires manual validation. |
| SYNC-01     | `06-01-PLAN.md` | Each assignment cell shows status: synced (green), stale (yellow), missing (red), pending (gray) | ✓ SATISFIED   | `AssignmentMatrix.tsx:263-277` applies `assignment.status` as a CSS class; `App.css:2836-2849` and `2967-2969` define synced/stale/pending/missing styles; backend produces missing status in `project_sync.rs:262-338`.                 |

### Anti-Patterns Found

No blocker or warning anti-patterns found in the phase files reviewed. Grep scans on `src-tauri/src/core/project_ops.rs`, `src-tauri/src/core/project_sync.rs`, and `src-tauri/src/commands/projects.rs` found no TODO/FIXME/placeholder markers or empty stub returns.

### Human Verification Required

### 1. Remove tool column flow

**Test:** In the Projects tab, configure a project with at least one tool column and assigned skills, then remove one tool column through the tool configuration modal.
**Expected:** The removed tool column no longer appears, assignments for that tool disappear after refresh, and the corresponding symlink/copy artifacts are removed from the project directory.
**Why human:** This is a roadmap success criterion covering an end-to-end desktop flow. The code paths are wired, but I could not run the Rust/Tauri backend in this environment because `cargo` is unavailable.

### 2. Missing status rendering and recovery

**Test:** Create or select a project assignment, then remove the central source directory or deployed target, open/reload the Projects matrix, and later restore the source/target.
**Expected:** The affected cell renders as missing, persists after reload, and after restoration plus re-fetch/resync it changes to synced or stale rather than staying missing.
**Why human:** Backend status production is implemented and CSS classes exist, but validating the actual red cell rendering and desktop interaction still requires a human run.

### Gaps Summary

No code gaps were found in the reviewed implementation. The backend cleanup and missing-status logic are present, substantive, tested in-source, and wired into the frontend refresh path. Remaining work is human confirmation of the desktop E2E flow and rendered UI states, especially because Rust commands could not be executed in this environment.

---

_Verified: 2026-04-09T03:19:48Z_
_Verifier: Claude (gsd-verifier)_
