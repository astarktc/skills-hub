# Phase 5: Edge Cases and Polish - Context

**Gathered:** 2026-04-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Graceful error handling for stale paths, orphaned assignments, and .gitignore concerns — plus a new auto-sync toggle for global tool directories with per-skill and bulk unsync actions, and fixes for 4 partially-met requirements from earlier phases (PROJ-01, ASGN-04, SYNC-01, INFR-02).

</domain>

<decisions>
## Implementation Decisions

### Auto-Sync Toggle (New Feature)

- **D-01:** Single global toggle — ON = auto-sync new skills to all installed tool directories on install (current behavior). OFF = install to central repo only, no tool directory deployment. Persisted as a setting in SQLite (`auto_sync_enabled` key) via dedicated get/set commands following the `get_github_token`/`set_github_token` pattern.
- **D-02:** Toggle lives in the My Skills toolbar area, above the skill list. Next to it: an "Uninstall from tool directories" button.
- **D-03:** Default state: ON — matches current behavior. Existing users see no change.
- **D-04:** Turning toggle OFF only affects future installs. Skills already synced to tool directories remain deployed. User uses the bulk "Uninstall from tool directories" button to clean up existing deployments.
- **D-05:** Frontend implementation: the 5 install paths in `App.tsx` (lines ~1065, ~1152, ~1218, ~1287, ~1462) check the persisted setting before calling `sync_skill_to_tool`. If OFF, skip sync after install. The `handleSyncAllManagedToTools` function (line ~1665) also checks the setting.

### Uninstall from Tool Directories (Bulk)

- **D-06:** "Uninstall from tool directories" button removes ALL skills from ALL tool directories in one operation. Deletes all `SkillTargetRecord` rows and removes symlinks/copies from tool directories. Skills remain in central repo and My Skills list.
- **D-07:** No confirmation dialog for the bulk operation. The action is clearly labeled and easily reversible (turn auto-sync back on and re-sync).
- **D-08:** After bulk unsync, all tool pills on SkillCards turn grey (inactive) since no targets exist. Automatic status refresh triggered by the operation.

### Per-Skill Unsync Icon

- **D-09:** Each SkillCard gets an `Unlink` icon (lucide-react) next to the existing trash icon. Tooltip: "Uninstall from tool directories".
- **D-10:** Always visible on every skill card, even if no tool targets exist (greyed out/disabled when nothing to unsync).
- **D-11:** Immediate action, no confirmation. Removes the skill from all tool directories (deletes its `SkillTargetRecord` rows + filesystem cleanup). Tool pills turn grey after action completes.
- **D-12:** Any button that syncs or unsyncs skills from global tool directories automatically triggers a refresh of the skill list / status cards.

### Missing Project Detection (PROJ-04)

- **D-13:** Detection on project list load — check each project's path exists when the Projects tab mounts and after any mutation. No background polling.
- **D-14:** Warning badge: yellow/orange warning triangle on the project row in the left panel. Path subtitle shows "(not found)". Project is still selectable but sync operations are disabled with explanation.
- **D-15:** Available actions for missing project: "Remove Project" (existing) and "Update Path" (new — re-point to new location via folder picker). Update Path opens the folder picker and reassigns the project to the selected directory.
- **D-16:** Backend adds a `path_exists` boolean to `ProjectDto`. Frontend checks this field to render warning state and disable sync buttons.

### Orphaned Assignments (INFR-03 — Redefined)

- **D-17:** Keep `ON DELETE CASCADE` on `skill_id` FK. When a skill is deleted from central library, all its `project_skill_assignments` rows are cascade-deleted automatically. No "missing" indicator needed — assignments simply disappear from the matrix on refresh.
- **D-18:** INFR-03 is redefined as: "Skill deletion cleans up all deployed artifacts (tool directories + project directories) before cascade delete." Before deleting the skill record, iterate all `SkillTargetRecord` rows (global) and `ProjectSkillAssignmentRecord` rows (project) to remove symlinks/copies from the filesystem, then let CASCADE clean up the DB.
- **D-19:** The "missing" CSS class for SYNC-01 is still needed for the `project_skill_assignments.status` field when a sync operation fails or filesystem target is missing. This is error-state handling, not orphan handling.

### Audit Fixes (Partially-Met Requirements)

- **D-20:** PROJ-01 fix: Parse `DUPLICATE_PROJECT|` error prefix in the projects UI. Add `formatProjectError()` helper in `useProjectState.ts` or `ProjectsPage.tsx` that maps prefixes to i18n keys, following the `formatErrorMessage()` pattern in `App.tsx`.
- **D-21:** ASGN-04 fix: Surface `BulkAssignResultDto.failed[]` in `useProjectState.ts`. When `failed.length > 0`, show a warning toast listing which tools failed and why.
- **D-22:** INFR-02 fix: Add `SyncMutex` parameter to `remove_project` command in `commands/projects.rs` and acquire the lock before calling `remove_project_with_cleanup`. Matches the existing pattern in `add_project_skill_assignment` and `remove_project_skill_assignment`.
- **D-23:** SYNC-01 fix: Add `.matrix-cell.missing` CSS class (red tint) for error/missing status. Update `aggregate_project_sync_status` in `skill_store.rs` to handle "missing" status in its match arm.

### .gitignore Prompt (PROJ-05)

- **D-24:** Already designed in Phase 4 context D-13: two checkboxes in registration modal for `.gitignore` (shared) and `.git/info/exclude` (private). Phase 4 implemented this in `EditProjectModal.tsx`. Phase 5 verifies it works correctly and handles edge cases (project with no `.git` directory, read-only `.gitignore`).

### Claude's Discretion

- Exact toolbar layout and spacing for the auto-sync toggle + unsync button
- Backend command naming for the new get/set auto-sync setting and bulk unsync commands
- Whether `update_project_path` is a new command or extends existing `register_project`
- CSS styling details for warning badges and disabled states
- Error message wording for missing project sync attempts
- Test coverage scope for new features

</decisions>

<canonical_refs>

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Auto-Sync Behavior (must understand current flow)

- `src/App.tsx` lines 62, 906-913 — `syncTargets` state, ephemeral per-tool toggle
- `src/App.tsx` lines 1065-1106, 1152-1192, 1218-1260, ~1287, ~1462 — 5 install paths that call `sync_skill_to_tool` after install
- `src/App.tsx` lines 1665-1736 — `handleSyncAllManagedToTools` bulk sync function
- `src/App.tsx` lines 1738-1750 — newly installed tools auto-sync trigger

### Settings Storage (follow existing pattern)

- `src-tauri/src/core/skill_store.rs` lines 218-238 — `get_setting`/`set_setting` helpers
- `src-tauri/src/commands/mod.rs` lines 676-701 — `get_github_token`/`set_github_token` pattern to follow

### Skill Card UI (add unsync icon here)

- `src/components/skills/SkillCard.tsx` lines 62-78, 134-168 — tool pill rendering, synced/unsynced distinction
- `src/components/skills/SkillCard.tsx` — trash icon button pattern to follow for Unlink icon

### Project UI (missing project detection)

- `src/components/projects/ProjectList.tsx` — project row rendering, add warning badge here
- `src/components/projects/useProjectState.ts` lines 54-56 — `normalizeError` (needs prefix parsing)
- `src/components/projects/ProjectsPage.tsx` line 51-52 — error display (raw prefix leaks)
- `src/components/projects/AssignmentMatrix.tsx` lines 239-253 — status class handling (needs missing class)

### Sync Serialization (INFR-02 fix)

- `src-tauri/src/commands/projects.rs` lines 26-36 — `remove_project` (missing SyncMutex)
- `src-tauri/src/commands/projects.rs` lines 117-164 — `add_project_skill_assignment` (SyncMutex pattern to follow)
- `src-tauri/src/lib.rs` line 14, 40 — SyncMutex definition and registration

### Bulk Assign (ASGN-04 fix)

- `src/components/projects/useProjectState.ts` lines 237-246 — bulk assign handler (ignores failed[])
- `src-tauri/src/commands/projects.rs` — `bulk_assign_skill` command returning `BulkAssignResultDto`

### Skill Deletion Cleanup

- `src-tauri/src/core/installer.rs` — skill deletion flow
- `src-tauri/src/commands/mod.rs` — `delete_skill` command
- `src-tauri/src/core/sync_engine.rs` — `remove_path_any()` for filesystem cleanup
- `src-tauri/src/core/project_ops.rs` — `remove_project_with_cleanup()` pattern for iterating and removing

### Project Docs

- `.planning/PROJECT.md` — Constraints, key decisions
- `.planning/REQUIREMENTS.md` — PROJ-04, PROJ-05, INFR-03 definitions
- `.planning/v1.0-MILESTONE-AUDIT.md` — Audit findings driving the 4 fix items

### Prior Phase Context

- `.planning/phases/01-data-foundation/01-CONTEXT.md` — Schema design, CASCADE behavior (D-06)
- `.planning/phases/02-sync-logic/02-CONTEXT.md` — Sync atomicity (D-01), SyncMutex (D-10)
- `.planning/phases/03-ipc-commands/03-CONTEXT.md` — Error prefix convention (D-06/D-07), bulk assign (D-01/D-02)
- `.planning/phases/04-frontend-component-tree/04-CONTEXT.md` — Toolbar layout (D-04), status colors (D-08), gitignore (D-13)

</canonical_refs>

<code_context>

## Existing Code Insights

### Reusable Assets

- `SkillStore::get_setting()`/`set_setting()` — Drop-in for persisting auto-sync toggle state
- `sync_engine::remove_path_any()` — Handles symlink/directory/file removal for unsync operations
- `SkillStore::list_skill_targets()` — Get all global tool targets for a skill (for per-skill unsync)
- `format_anyhow_error()` / error prefix pattern — Extend for project error translation
- `@tauri-apps/plugin-dialog::open()` — Folder picker for "Update Path" on missing projects
- `AddProjectModal.tsx` — Folder picker + path input pattern to reuse for update path flow

### Established Patterns

- Settings commands: dedicated `get_X`/`set_X` pairs wrapping `store.get_setting()`/`set_setting()`
- Tool pill rendering: `.tool-pill.active` (green) / `.tool-pill.inactive` (grey) classes
- Sync status lifecycle: pending → synced/error/stale
- Modal pattern: `if (!open) return null`, confirmation text, action callbacks
- Error prefix detection: `raw.startsWith("PREFIX|")` → map to i18n key

### Integration Points

- `App.tsx` install paths (5 locations) — Add auto-sync setting check before sync calls
- `App.tsx` toolbar area — Add toggle switch + unsync button to FilterBar or new toolbar component
- `SkillCard.tsx` action buttons — Add Unlink icon alongside existing trash icon
- `commands/projects.rs:remove_project` — Add SyncMutex parameter
- `commands/mod.rs` — Add `get_auto_sync_enabled`/`set_auto_sync_enabled` commands, add bulk unsync command
- `useProjectState.ts` — Add error prefix parsing, surface bulk assign failures
- `ProjectList.tsx` — Add warning badge for missing projects
- `ProjectDto` (project_ops.rs) — Add `path_exists` field
- `skill_store.rs:aggregate_project_sync_status` — Handle "missing" in match arm

</code_context>

<specifics>
## Specific Ideas

- Auto-sync toggle: user wants to turn it OFF so new skills go to central repo only, then use per-project assignment to control where skills appear. This is the core use case for the fork.
- Per-skill Unlink icon: always visible, immediate action, matches the "quick toggle" philosophy of the existing trash icon.
- Missing projects: "Update Path" via folder picker lets users handle moved directories without losing assignment history.
- Skill deletion: clean up deployed artifacts (both global and project) before CASCADE delete ensures no orphaned files on disk.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

_Phase: 05-edge-cases-and-polish_
_Context gathered: 2026-04-08_
