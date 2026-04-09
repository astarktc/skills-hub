---
phase: 03-ipc-commands
fixed_at: 2026-04-08T01:30:00Z
review_path: .planning/phases/03-ipc-commands/03-REVIEW.md
iteration: 1
findings_in_scope: 2
fixed: 2
skipped: 0
status: all_fixed
---

# Phase 3: Code Review Fix Report

**Fixed at:** 2026-04-08T01:30:00Z
**Source review:** .planning/phases/03-ipc-commands/03-REVIEW.md
**Iteration:** 1

**Summary:**

- Findings in scope: 2
- Fixed: 2
- Skipped: 0

## Fixed Issues

### WR-01: add_project_tool accepts unvalidated tool keys

**Files modified:** `src-tauri/src/commands/projects.rs`
**Commit:** b79012a
**Applied fix:** Added validation against the tool adapter registry (`crate::core::tool_adapters::adapter_by_key`) before inserting the tool record. If the tool key does not correspond to a known adapter, the command now bails with `"unknown tool: {tool}"` immediately, preventing invalid tool keys from being stored in the `project_tools` table.

### WR-02: Fragile string prefix matching in register_project error handling

**Files modified:** `src-tauri/src/core/project_ops.rs`, `src-tauri/src/commands/projects.rs`, `src-tauri/src/core/tests/project_ops.rs`
**Commit:** 16daa91
**Applied fix:** Moved the `DUPLICATE_PROJECT|` error prefix from the command layer into the core layer (`project_ops::register_project_path`), so the error is emitted at the source rather than reconstructed via fragile string matching. The command layer now uses a simple `.map_err(format_anyhow_error)` passthrough, which already has `DUPLICATE_PROJECT|` in its prefix passthrough list. Updated the `register_rejects_duplicate` test to assert on the new `DUPLICATE_PROJECT|` prefix instead of the old `"already registered"` substring.

## Skipped Issues

None -- all in-scope findings were fixed.

---

_Fixed: 2026-04-08T01:30:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
