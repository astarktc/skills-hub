# Quick Task 260422-h7p: Summary

## What Changed

Added project-level copy-mode re-sync to `update_managed_skill_from_source` in `src-tauri/src/core/installer.rs`.

Previously, when a user clicked the refresh icon on a skill, only global tool targets were re-synced. Project-level skill assignments using copy mode (e.g., Cursor project targets) were ignored, leaving stale content in project directories.

## Implementation

After the existing tool-target re-sync loop (~line 998), a new loop iterates `project_skill_assignments` for the updated skill:

- Queries assignments via `list_project_skill_assignments_by_skill`
- Skips non-copy-mode assignments (symlinks auto-update)
- Resolves target paths via `resolve_project_sync_target`
- Copies updated content via `sync_dir_copy_with_overwrite`
- Updates `content_hash`, `synced_at`, `status` on the assignment record
- Appends `project:<project_id>:<tool>` entries to `updated_targets`
- Logs warnings and marks error status on failure (non-fatal)

## Files Modified

| File                                    | Change                                                                           |
| --------------------------------------- | -------------------------------------------------------------------------------- |
| `src-tauri/src/core/installer.rs`       | Added project assignment re-sync loop + import for `resolve_project_sync_target` |
| `src-tauri/src/core/tests/installer.rs` | Added `update_resyncs_project_copy_assignments` test                             |

## Commits

| Hash      | Description                                            |
| --------- | ------------------------------------------------------ |
| `74add86` | Add project assignment re-sync loop to update function |
| `457c052` | Add test for project copy-mode assignment re-sync      |
| `6104d2c` | Apply cargo fmt                                        |
| `c8ccc56` | Revert unrelated compute_content_hash scope creep      |

## Verification

- All 154 Rust tests pass (including new `update_resyncs_project_copy_assignments`)
- `npm run check` passes (lint, build, fmt, clippy, tests)
