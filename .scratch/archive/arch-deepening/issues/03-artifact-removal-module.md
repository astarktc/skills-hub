# 03: Artifact removal module, unsync commands into core, Managed-skill catalog into core

Status: resolved

Type: task
Source: `../spec.md` Q4, Q13, Q14; CONTEXT.md **Artifact removal**; ADR-0001 procedure for the new error variant. Prefer to start after 02 merges (shares the command wiring file) — not a hard blocker.

**What to build:** The existing skill-removal plan/execute/report shape grows into *the* Artifact removal module: plan by scope (one Managed skill, one skill×Tool pair, one Project, one Project×Tool pair, everything), execute once with one presence rule and shared-skills-dir dedupe, and report per-target outcomes. A row whose artifact could not be removed is kept with Sync status `error`; rows are deleted only on success — record this as ADR-0002. The two command-tier unsync commands become one plan-and-execute call each (no swallowed filesystem errors). The "only delete a path under a known Tool skills directory" safety refusal becomes a typed condition owned by the Tool registry, surfaced to the operator through the tagged error contract. The "a source without `SKILL.md` is invalid" admission rule joins skill discovery. The Managed-skill catalog (skills + targets + manifest-derived invocation mode) is assembled in core like the Tool catalog, with a deliberate failure policy instead of a silent empty list; the command only maps it.

**Blocked by:** 01

- [x] Unsync-one and unsync-all report every path they could not remove, with the row kept as `error`; the previous silent-count behaviour is gone and has tests
- [x] Removal tests cover: broken symlink, missing row, shared-dir group removed once and every member row settled, partial failure → report + typed cleanup error, each plan scope
- [x] `PATH_OUTSIDE_TOOL_DIRS` exists end to end: Rust variant, regenerated binding, `describeCommandError` branch, EN + ZH copy, and a core test that a path outside every Tool dir is refused
- [x] The missing-`SKILL.md` admission rule has a test in skill discovery; the command body no longer contains it
- [x] The Managed-skill catalog has core tests for: multiple skills, target-query failure policy, missing manifest, invocation mode; the command-tier helper test is retired
- [x] `docs/adr/0002-*.md` records the keep-row-with-error decision with the alternatives considered
- [x] Command bodies in the main command module are `spawn_blocking` + error mapping only; `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.

**Shipped** (`f381281` on `arch/03-artifact-removal`, not merged):

- `core/skill_removal.rs` → `core/artifact_removal.rs` (+ `core/tests/artifact_removal.rs`):
  `plan(store, home, &RemovalScope)` (read-only, dedupes by path so a shared skills dir is one
  target with every member row attached), `execute_unlocked(store, plan)` (one presence probe —
  `symlink_metadata`, so a broken symlink is present and removed; one settlement rule — rows
  deleted on success, transitioned to `error` on failure), `RemovalReport { targets, central_removed,
  record_deleted }` with `failures()` / `removed_rows()` / `failed_rows()`. Locked entry points:
  `remove_skill`, `unsync_skill_targets`, `unsync_all_skill_targets`, `unsync_skill_from_tool`.
- `global_sync.rs` lost `unsync_skill_targets`, `unsync_all_skill_targets`,
  `unsync_skill_from_tool_with_records(_unlocked)` and `remove_targets_for_tools` (and their tests).
- Wire: `unsync_skill` / `unsync_all_skills` / `unsync_skill_from_tool` now return
  `RemovalReportDto { targets, removed, failed }` (one entry per settled row, failures carry a
  `CommandError`). `useSkillLibrary` renders it: `unsyncAllComplete` / new `unsyncPartial` toast,
  `showActionErrors` per failed target (`errors.unsyncFailedTitle`), and a failed per-tool toggle
  fails the action instead of toasting success. Hook tests added for all three.
- `PATH_OUTSIDE_TOOL_DIRS` end to end: `SignalError::PathOutsideToolDirs` →
  `tool_adapters::ensure_path_within_tool_dirs(home, path)` (registry-owned) → `CommandError` →
  regenerated binding → `describeCommandError` branch → EN + ZH copy; core tests for allowed and
  refused paths, plus a command-tier wire-shape assertion.
- Admission rule: `skill_discovery::require_skill_md(dir) -> Result<PathBuf>` (typed
  `SkillInvalid { missing_skill_md }`), applied by `install_local_skill` (so `import_existing_skill`
  and the local-selection path both get it); the command body and the duplicated inline check in
  `install_local_skill_from_selection` are gone. Two discovery tests.
- `core/skill_catalog.rs::managed_skill_catalog(store)` + tests (multiple skills with their own
  targets, invocation mode from the manifest, missing manifest → default mode, dropped
  `skill_targets` table → the call fails with context). `get_managed_skills` is now a core call +
  `From<ManagedSkillEntry> for ManagedSkillDto`; the command-tier helper test is retired.
- `docs/adr/0002-keep-row-with-error-on-failed-artifact-removal.md`; CONTEXT.md **Artifact removal**
  links it.

**Deviations (with reasons):**

1. **Six scopes, not five.** The skill scope splits into `Skill` (global + project + central copy +
   record, for delete) and `SkillGlobal` (global rows only, for "uninstall from tool directories"),
   because unsync must not remove project artifacts. `Everything` is named `EveryGlobalTarget` for
   the same reason (it is the global sweep, as the old `unsync_all_skill_targets` was).
2. **No assignment-status filter when planning.** The old `plan_skill_removal` skipped rows whose
   status was not `synced|stale|error`. The module plans every row and lets the presence probe
   decide, because a row's status is a stale belief while disk is truth — and the unsync scopes must
   delete `pending`/`missing` rows too (the old bulk delete did). The old
   `plan_includes_only_deployed_assignment_statuses` test is replaced accordingly.
3. **A partly-failed skill deletion now keeps the skill** (record + central copy) and its failed
   rows, instead of deleting the record and cascading the rows away. This is the ADR-0002
   recommendation from the ticket; the EN/ZH `errors.deleteCleanupFailed` copy changed to match
   ("the skill was kept — you can retry").
4. **Failure detail stays a `String` in the core report** (as before) and maps to
   `CommandError::Other` at the seam. These are unclassified IO failures; `Other` is the sanctioned
   valve and one target's error is shared by every attached row.
5. **No new per-row store delete.** `skill_targets` has `UNIQUE(skill_id, tool)`, so
   `delete_skill_target(skill_id, tool)` already identifies a row; `delete_skill_targets` /
   `delete_all_skill_targets` are now callerless but kept (with `#[allow(dead_code)]`) as store
   capabilities rather than deleted.

**Follow-ups for 05 / 06:**

- **05**: `RemovalScope::Project` / `ProjectTool` are implemented and tested but uncalled (they carry
  `#[allow(dead_code)]` for that reason — drop it when wiring). Rewire
  `project_ops::remove_tool_with_cleanup`, `remove_project_with_cleanup` and
  `project_sync::unassign_and_cleanup` onto `plan` + `execute_unlocked` (they are already inside the
  guard, so use the unlocked seams). Note the module skips assignment rows whose project or tool
  cannot be resolved (it cannot locate an artifact for them); the legacy-orphan handling in
  `project_ops` — which falls back to `assignment.skill_name` — is covered by the module reading
  `skill_name` first, but the "project row gone" case still needs a caller decision. Those callers
  currently delete rows on failure or log-and-continue; ADR-0002 says keep-as-`error`, so expect
  behaviour changes in `core/tests/project_ops.rs` / `project_sync.rs`.
- **06**: `remove_skill_source` is now `spawn_blocking` + `ensure_path_within_tool_dirs` +
  `remove_path_any`. When 06 deletes the command, the registry rule survives for the import flow —
  call `ensure_path_within_tool_dirs` from wherever onboarding removes originals.
- The three unsync commands could later collapse into one scoped removal command; not done here
  because the frontend call sites and their toasts are distinct.
