# Quick Task 260422-h7p: Fix update_managed_skill_from_source - Research

**Researched:** 2026-04-22
**Domain:** Rust backend -- installer.rs update propagation
**Confidence:** HIGH

## Summary

The `update_managed_skill_from_source` function (installer.rs:815-994) updates a skill's central copy and then re-syncs copy-mode tool targets (lines 956-984). It does NOT touch `project_skill_assignments`. For project assignments using copy mode (e.g., Cursor), the project copy becomes stale after a skill update with no way to fix it except a manual "Sync Project" from the UI.

The fix is straightforward: after the existing tool-target loop, add a parallel loop over project assignments returned by `list_project_skill_assignments_by_skill`. For each copy-mode assignment, resolve the target path via `resolve_project_sync_target`, call `sync_dir_copy_with_overwrite`, and update the assignment record via `update_assignment_status`.

**Primary recommendation:** Append a project-assignment re-sync loop after line 984, following the exact same pattern as the tool-target loop but using project_sync target resolution.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

- Copy-mode project assignments only. Symlinks auto-update since they point to the central path that was just refreshed.
- Inline in `update_managed_skill_from_source`, appended after the existing tool-target loop (~line 984).
- Update `content_hash` column on `project_skill_assignments` after re-copying.

### Claude's Discretion

- Exact SQL/store method to use for updating project_skill_assignments content_hash
- Whether to include project targets in the `updated_targets` return field or add a separate field
  </user_constraints>

## Existing Code Patterns

### Tool-target re-sync loop (installer.rs:956-984) [VERIFIED: codebase]

The existing loop:

1. Fetches all `SkillTargetRecord` rows for the skill via `store.list_skill_targets(skill_id)`
2. Skips targets whose tool adapter reports not installed
3. Force-copies if `mode == "copy"` OR `tool == "cursor"`
4. Calls `sync_dir_copy_with_overwrite(&central_path, &target_path, true)`
5. Upserts the target record with new `synced_at` and `mode: "copy"`
6. Pushes tool name to `updated_targets: Vec<String>`

### Key data structures [VERIFIED: codebase]

**`ProjectSkillAssignmentRecord`** (skill_store.rs:156-168):

- `id`, `project_id`, `skill_id`, `skill_name`, `tool`, `mode`, `status`, `last_error`, `synced_at`, `content_hash`, `created_at`
- `mode` is either `"symlink"`, `"copy"`, or `"junction"` (set at sync time by `sync_dir_for_tool_with_overwrite`)
- `content_hash` is `Option<String>` -- only populated for copy-mode assignments

**`ProjectRecord`** (skill_store.rs:141-146):

- `id`, `path` (filesystem path), `created_at`, `updated_at`

### Store methods available [VERIFIED: codebase]

| Method                                                                            | Location           | Use                                             |
| --------------------------------------------------------------------------------- | ------------------ | ----------------------------------------------- |
| `list_project_skill_assignments_by_skill(skill_id)`                               | skill_store.rs:574 | Get all project assignments for a given skill   |
| `get_project_by_id(project_id)`                                                   | skill_store.rs:673 | Get project record (needed for path resolution) |
| `update_assignment_status(id, status, last_error, synced_at, mode, content_hash)` | skill_store.rs:823 | Update assignment after re-sync                 |

### Target path resolution [VERIFIED: codebase]

`project_sync::resolve_project_sync_target(project_path, relative_skills_dir, skill_name)` returns `project_path.join(relative_skills_dir).join(skill_name)`. The `relative_skills_dir` comes from the tool adapter, same as for tool targets.

Note: The `ProjectSkillAssignmentRecord` does NOT have a stored `target_path` field (unlike `SkillTargetRecord` which does). Target paths must be recomputed from `project.path + adapter.relative_skills_dir + skill.name`.

### Content hash [VERIFIED: codebase]

The `content_hash` variable is already computed at line 931 (`compute_content_hash(&central_path)`) and is an `Option<String>`. It can be reused directly for updating project assignment records.

## Architecture Pattern for New Loop

### Required imports to add to installer.rs

```rust
use super::project_sync::resolve_project_sync_target;
```

`adapter_by_key` and `sync_dir_copy_with_overwrite` are already imported.

### Pseudocode for new loop (after line 984)

```rust
// Re-sync copy-mode project assignments
let project_assignments = store.list_project_skill_assignments_by_skill(skill_id)?;
for pa in project_assignments {
    if pa.mode != "copy" {
        continue; // symlinks auto-update
    }
    let project = match store.get_project_by_id(&pa.project_id)? {
        Some(p) => p,
        None => continue, // orphaned assignment
    };
    let project_path = PathBuf::from(&project.path);
    if !project_path.exists() {
        continue; // project directory gone
    }
    let adapter = match adapter_by_key(&pa.tool) {
        Some(a) => a,
        None => continue, // unknown tool
    };
    let target = resolve_project_sync_target(
        &project_path,
        adapter.relative_skills_dir,
        &record.name,
    );
    match sync_dir_copy_with_overwrite(&central_path, &target, true) {
        Ok(_outcome) => {
            store.update_assignment_status(
                &pa.id,
                "synced",
                None,
                Some(now),
                Some("copy"),
                content_hash.as_deref(),
            )?;
            updated_targets.push(format!("project:{}:{}", pa.project_id, pa.tool));
        }
        Err(e) => {
            log::warn!("failed to re-sync project assignment {}: {:#}", pa.id, e);
            let _ = store.update_assignment_status(
                &pa.id,
                "error",
                Some(&format!("{:#}", e)),
                None,
                None,
                None,
            );
        }
    }
}
```

## Design Decisions

### updated_targets field

**Recommendation:** Reuse the existing `updated_targets: Vec<String>` field. Push entries with a `"project:"` prefix to distinguish them from tool targets. This avoids changing `UpdateResult`, `UpdateResultDto`, or the frontend DTO type. The frontend currently only displays a count/toast from this field, so the prefix is invisible to users.

Alternatively, add a separate `updated_project_targets: Vec<String>` field -- but this requires changes to `UpdateResult`, `UpdateResultDto`, and the frontend `types.ts`. Unnecessary for a bug fix.

**Decision: Use existing field with `project:` prefix.** [ASSUMED -- Claude's discretion area]

### update_assignment_status method

The existing `update_assignment_status` (skill_store.rs:823) already accepts all needed parameters including `content_hash`. No new store method is needed. [VERIFIED: codebase]

### Error handling

Follow the tool-target pattern: log warnings for failures but continue processing remaining assignments. Use `let _ = store.update_assignment_status(...)` in the error path (matching resync_project pattern in project_sync.rs:183). Do not abort the entire update if one project re-sync fails.

## Common Pitfalls

### Pitfall 1: Missing project directory

**What goes wrong:** Project was deleted/moved but assignments still exist in DB.
**How to avoid:** Check `project_path.exists()` before attempting sync (shown in pseudocode above).

### Pitfall 2: Cursor force-copy logic

**What goes wrong:** A Cursor assignment may have `mode: "symlink"` in DB if it was synced before the Cursor force-copy rule was added, but it should still be re-copied.
**How to avoid:** Use `pa.mode == "copy" || pa.tool == "cursor"` (same guard as tool-target loop at line 967). This matches the existing force-copy behavior.

### Pitfall 3: N+1 queries for project records

**What goes wrong:** Each assignment calls `get_project_by_id` individually, causing repeated DB queries for assignments in the same project.
**How to avoid:** For a bug fix, this is acceptable -- the number of assignments per skill is small (typically < 20). If perf matters later, batch-fetch projects first. Not worth optimizing now.

## Assumptions Log

| #   | Claim                                                                         | Section          | Risk if Wrong                                         |
| --- | ----------------------------------------------------------------------------- | ---------------- | ----------------------------------------------------- |
| A1  | Reuse `updated_targets` with `project:` prefix rather than adding a new field | Design Decisions | Low -- frontend only uses count, easily changed later |

## Sources

### Primary (HIGH confidence)

- `src-tauri/src/core/installer.rs:815-994` -- update function, tool-target loop
- `src-tauri/src/core/skill_store.rs:574-605` -- `list_project_skill_assignments_by_skill`
- `src-tauri/src/core/skill_store.rs:823-851` -- `update_assignment_status`
- `src-tauri/src/core/project_sync.rs:13-19` -- `resolve_project_sync_target`
- `src-tauri/src/core/project_sync.rs:114-164` -- `sync_single_assignment` (reference pattern)
- `src-tauri/src/commands/mod.rs:624-653` -- `UpdateResultDto` and command wrapper

## Metadata

**Confidence breakdown:**

- Existing code patterns: HIGH -- all verified from codebase
- New loop design: HIGH -- follows exact same pattern as tool-target loop
- Return field strategy: MEDIUM -- discretion area, easy to change

**Research date:** 2026-04-22
**Valid until:** 2026-05-22 (stable codebase, no external deps)
