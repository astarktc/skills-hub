---
phase: quick
plan: 260409-rnu
title: Wire handleImport to respect autoSyncEnabled
one_liner: "handleImport now branches on autoSyncEnabled: sync-back when ON, cleanup originals via path-validated remove_skill_source when OFF"
completed: "2026-04-10T01:03:40Z"
duration: 180s
tasks_completed: 2
tasks_total: 2
key_files:
  created: []
  modified:
    - src-tauri/src/commands/mod.rs
    - src-tauri/src/lib.rs
    - src/App.tsx
decisions:
  - "Path safety validation uses default_tool_adapters() to check all 40+ known tool skill directories before allowing deletion"
  - "Cleanup errors are collected but non-fatal since the skill is already safely imported to central repo"
  - "All group.variants are cleaned up in the else branch, not just the chosen variant"
---

# Quick Task 260409-rnu: Wire handleImport to respect autoSyncEnabled

handleImport now branches on autoSyncEnabled: sync-back when ON, cleanup originals via path-validated remove_skill_source when OFF.

## What Changed

### Task 1: Add remove_skill_source Tauri command (3ac8803)

- Added `remove_skill_source` async Tauri command to `src-tauri/src/commands/mod.rs`
- Command validates the target path is under a known tool skills directory (checked against all 40+ tool adapters) before calling `remove_path_any`
- Returns `UNSAFE_PATH|` prefixed error if path validation fails
- Added `default_tool_adapters` to the existing import from `crate::core::tool_adapters`
- Registered command in `generate_handler!` in `src-tauri/src/lib.rs`

### Task 2: Branch handleImport on autoSyncEnabled (873d9e0)

- Modified `handleImport` in `src/App.tsx` to wrap the sync loop in `if (autoSyncEnabled)`
- Added `else` branch that iterates ALL `group.variants` and calls `remove_skill_source` for each
- `import_existing_skill` call remains outside the branch -- always executes regardless of setting
- Moved `selectedInstalledIds`, `targets`, and sync variables inside the `if` block to avoid unused-variable lint errors
- Applied `cargo fmt` to fix formatting in the new Rust code

## Commits

| Task | Commit  | Message                                                                        |
| ---- | ------- | ------------------------------------------------------------------------------ |
| 1    | 3ac8803 | feat(quick-260409-rnu): add remove_skill_source Tauri command with path safety |
| 2    | 873d9e0 | feat(quick-260409-rnu): branch handleImport on autoSyncEnabled                 |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Rust formatting mismatch**

- **Found during:** Task 2 verification
- **Issue:** `cargo fmt --check` flagged the `dirs::home_dir()` chain formatting
- **Fix:** Ran `cargo fmt` to auto-format
- **Files modified:** src-tauri/src/commands/mod.rs
- **Commit:** Included in 873d9e0

## Verification

- `npm run check` passes clean (lint + build + rust:fmt:check + rust:clippy + rust:test)
- `remove_skill_source` validates paths against all known tool skill directories before deletion
- `handleImport` branches: autoSyncEnabled ON runs sync loop, OFF removes variant paths
- No other install flows modified

## Self-Check: PASSED

All files exist, all commits verified, all must_have artifacts confirmed.
