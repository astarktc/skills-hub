---
phase: 03-ipc-commands
reviewed: 2026-04-08T01:15:00Z
depth: standard
files_reviewed: 6
files_reviewed_list:
  - src/components/projects/types.ts
  - src-tauri/src/commands/projects.rs
  - src-tauri/src/commands/mod.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/commands/tests/commands.rs
  - src-tauri/src/core/tests/project_sync.rs
findings:
  critical: 0
  warning: 2
  info: 2
  total: 4
status: issues_found
---

# Phase 3: Code Review Report

**Reviewed:** 2026-04-08T01:15:00Z
**Depth:** standard
**Files Reviewed:** 6
**Status:** issues_found

## Summary

This review covers the Phase 3 IPC command layer for per-project skill distribution. The new `commands/projects.rs` module exposes 12 Tauri commands for project registration, tool configuration, skill assignment/unassignment, resync, and bulk operations. All commands are properly registered in `lib.rs` via `generate_handler!`, and the frontend DTO types in `src/components/projects/types.ts` are structurally aligned with the Rust DTOs. The `SyncMutex` global state is correctly wired for serializing filesystem operations. Tests are thorough, covering symlink/copy modes, error recovery, staleness detection, and concurrency serialization.

Two warnings were found: missing tool-key validation in `add_project_tool` (accepts arbitrary strings, deferring failure to sync time) and a fragile string-matching pattern in `register_project` error handling. Two info items note a debug `println!` in pre-existing code that was changed in this branch and a minor DTO mapping repetition that could be reduced.

## Warnings

### WR-01: add_project_tool accepts unvalidated tool keys

**File:** `src-tauri/src/commands/projects.rs:58-80`
**Issue:** The `add_project_tool` command validates that the project exists (line 67) but does not validate that the `tool` parameter corresponds to a known tool adapter. Any arbitrary string is accepted and stored in the `project_tools` table. While `assign_and_sync` later validates the tool key via `adapter_by_key()`, this creates a data integrity gap: a project can have tools in its configuration that will always fail at sync time. The user receives no feedback that they registered an invalid tool until they attempt to assign a skill.
**Fix:** Add validation against the tool adapter registry before inserting:

```rust
// After project existence check, before creating the record:
if crate::core::tool_adapters::adapter_by_key(&tool).is_none() {
    anyhow::bail!("unknown tool: {}", tool);
}
```

### WR-02: Fragile string prefix matching in register_project error handling

**File:** `src-tauri/src/commands/projects.rs:23-32`
**Issue:** The `register_project` command detects duplicate project errors by checking if the error message string contains the substring `"project already registered"` (line 24), then extracts the path by calling `strip_prefix("project already registered: ")` (line 26). This couples the command layer to the exact wording of the error message produced by `project_ops::register_project_path`. If the message text ever changes (e.g., different capitalization, added context via `.context()`), the DUPLICATE_PROJECT prefix will silently stop being emitted, and the frontend will display a raw error instead of its special duplicate-project UX flow.
**Fix:** Use a structured error type or a dedicated error variant instead of string matching. A pragmatic short-term alternative is to have `project_ops::register_project_path` return the error with the `DUPLICATE_PROJECT|` prefix directly:

```rust
// In project_ops::register_project_path:
if store.get_project_by_path(&path_str)?.is_some() {
    bail!("DUPLICATE_PROJECT|{}", path_str);
}
```

Then in the command, pass through without re-parsing:

```rust
// The format_anyhow_error passthrough list already includes DUPLICATE_PROJECT|
.map_err(format_anyhow_error)
```

## Info

### IN-01: Debug println! in delete_managed_skill (pre-existing, touched by this branch)

**File:** `src-tauri/src/commands/mod.rs:765`
**Issue:** The `delete_managed_skill` command contains a `println!("[delete_managed_skill] skillId={}", skillId)` debug statement. While this existed before Phase 3, the `mod.rs` file was modified in this branch (visibility changes to `format_anyhow_error`, `expand_home_path`, `now_ms`, and `remove_path_any` refactoring). The project uses `log` crate with `tauri-plugin-log` for logging (configured in `lib.rs`). This should use `log::debug!` instead of `println!` for consistency and to avoid polluting stdout in release builds.
**Fix:** Replace with structured logging:

```rust
log::debug!("[delete_managed_skill] skillId={}", skillId);
```

### IN-02: Repetitive DTO field-by-field mapping in projects.rs

**File:** `src-tauri/src/commands/projects.rs:152-163, 209-220, 337-348`
**Issue:** The `ProjectSkillAssignmentRecord` to `ProjectSkillAssignmentDto` conversion is repeated verbatim in three locations: `add_project_skill_assignment` (line 152), `list_project_skill_assignments` (line 209), and `bulk_assign_skill` (line 337). This is a minor code duplication issue. A `From` impl or helper function would reduce boilerplate and prevent field omission if new fields are added later.
**Fix:** Add a `From` impl on `ProjectSkillAssignmentDto`:

```rust
impl From<ProjectSkillAssignmentRecord> for ProjectSkillAssignmentDto {
    fn from(r: ProjectSkillAssignmentRecord) -> Self {
        Self {
            id: r.id,
            project_id: r.project_id,
            skill_id: r.skill_id,
            tool: r.tool,
            mode: r.mode,
            status: r.status,
            last_error: r.last_error,
            synced_at: r.synced_at,
            content_hash: r.content_hash,
            created_at: r.created_at,
        }
    }
}
```

---

_Reviewed: 2026-04-08T01:15:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
