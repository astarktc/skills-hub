# Quick Task 260422-h7p: Fix update_managed_skill_from_source to re-sync project-level copy-mode skill assignments - Context

**Gathered:** 2026-04-22
**Status:** Ready for planning

<domain>
## Task Boundary

Fix `update_managed_skill_from_source` in `src-tauri/src/core/installer.rs` to also re-sync project-level skill assignments that use copy mode (e.g., Cursor project targets), not just global tool targets. Currently the function only iterates `skill_targets` (tool sync) and ignores `project_skill_assignments`.

</domain>

<decisions>
## Implementation Decisions

### Sync scope

- Copy-mode project assignments only. Symlinks auto-update since they point to the central path that was just refreshed. No symlink verification/repair needed.

### Where to add logic

- Inline in `update_managed_skill_from_source`, appended after the existing tool-target re-sync loop (~line 984). Keeps all update propagation logic in one function.

### Content hash tracking

- Update `content_hash` column on `project_skill_assignments` after re-copying, so the record reflects the project copy is current with the new central version.

### Claude's Discretion

- Exact SQL/store method to use for updating project_skill_assignments content_hash
- Whether to include project targets in the `updated_targets` return field or add a separate field

</decisions>

<specifics>
## Specific Ideas

- The existing tool-target loop at lines 956-984 of `installer.rs` can serve as a template for the project assignment loop
- `list_project_skill_assignments_by_skill` (skill_store.rs:574) already exists to query assignments by skill_id
- `sync_dir_copy_with_overwrite` is the existing copy primitive used for tool targets — reuse for project targets
- `compute_content_hash` is already called earlier in the function — reuse the result

</specifics>

<canonical_refs>

## Canonical References

- `src-tauri/src/core/installer.rs:815-994` — `update_managed_skill_from_source` function
- `src-tauri/src/core/skill_store.rs:574-590` — `list_project_skill_assignments_by_skill`
- `src-tauri/src/core/skill_store.rs:83-100` — `project_skill_assignments` table schema
- `src-tauri/src/core/sync_engine.rs` — `sync_dir_copy_with_overwrite` primitive

</canonical_refs>
