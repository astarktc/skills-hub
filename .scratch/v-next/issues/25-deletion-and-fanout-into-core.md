# 25: Skill deletion and project fan-out into core

Status: resolved

Type: task
Blocked by: 19, 20

## What to build

Review #2 (Opus + Sol, verified): AGENTS.md says `commands/` is wiring only, yet the app's most destructive / most branching operations have their *implementation* inside a `#[tauri::command]`, so their only test surface is a running app — and `commands/tests/commands.rs` tests nothing above `expand_home_path`. At `943f85c`:

- `commands/mod.rs:986-1053` `delete_managed_skill`: 60 lines — global target removal, project artifact removal (`:1006-1031`, inspecting status literals `"synced"|"stale"` and hand-joining the project path — ticket 19 routes that through the resolver), central-dir removal + cascade, typed `DeleteCleanupFailed`. `core/project_ops.rs::remove_tool_with_cleanup` / `remove_project_with_cleanup` are the in-core precedent, which makes skill deletion the odd one out.
- `commands/projects.rs:346-413` `bulk_assign_skill`: project-scope fan-out — lookups, already-assigned dedupe, per-tool failure isolation into `BulkAssignErrorDto`. This is exactly the shape `global_sync::sync_skills_to_planned_tools` has *in core* with 15 tests.
- `commands/projects.rs:417-456` `update_project_gitignore`: project lookup, `is_dir` validation, tool→adapter→pattern derivation, write — untested composition over a well-tested `core/gitignore.rs`.

Deepen (mirror `global_sync`'s plan/execute split — the repo's best module):

- `core/skill_removal.rs` (or `installer::remove_managed_skill`): `(&SkillStore, skill_id) -> Result<RemovalReport>` owning global + project + central cleanup and the typed failure. Command becomes a 3-line wrapper.
- `project_sync::assign_skill_to_project_tools(...)` (name yours): probe → deterministic engine → per-target status-as-data, serving `bulk_assign`, `add_project_skill_assignment`, `resync_project`. Command maps to `BulkAssignErrorDto` only.
- `gitignore::update_for_project(&store, project_id, options)` wrapping the existing pattern module. (Ticket 28 later changes its *inputs*; keep the current derive-from-persisted-tools behaviour here.)
- Tests with temp dirs + a test store for each, asserting typed statuses — the deterministic halves become table-testable exactly like `core/tests/global_sync.rs`.

Do not retype the status strings here (ticket 27 owns that); do not change wire DTOs beyond what moving the bodies requires.

## Acceptance criteria

- [ ] `delete_managed_skill`, `bulk_assign_skill`, `update_project_gitignore` command bodies are DTO conversion + one core call; no filesystem or DB choreography in `commands/`.
- [ ] New core tests cover deletion (global + project + central, incl. partial failure → typed report) and project fan-out (dedupe, per-target failure isolation).
- [ ] Wire behaviour unchanged (same DTOs, same error codes).
- [ ] `npm run version:check && npm run check` green.

## Answer

Landed green in `94bd2e1` (Fable 5.1 child, medium thinking; clean rebase over ticket 24; 318 cargo + 67 vitest, full gate green on main).

- **`core/skill_removal.rs`** (plan/execute, mirrors `global_sync`): `plan_skill_removal(&store, skill_id) -> RemovalPlan { skill, targets: Vec<RemovalTarget { scope: Global | Project{project_id}, tool_key, path }> }` (DB reads only; project paths via `resolve_project_sync_target`); `execute_skill_removal(&store, skill_id, plan) -> RemovalReport { targets: Vec<RemovalTargetOutcome>, central_removed, record_deleted }` (per-target failures isolated as `Failed { error }`, then central copy, then `delete_skill`); `remove_skill` = plan + execute + `bail!(DeleteCleanupFailed { failures })` if any target failed. Preserved: artifacts removed *before* the DB delete cascades assignments; record still deleted on partial target failure; central `remove_dir_all` failure is hard (record left for retry); missing skill row still sweeps the *filesystem paths* of orphan `skill_targets` rows (the rows themselves are not deleted — same as base; review #3 wording fix). `delete_managed_skill` is a 3-line wrapper. Status filter `synced|stale|error` kept as literals for ticket 27.
- **`project_sync`**: `assign_skill_to_tools(&store, &project, &skill, &[tool], now) -> Vec<AssignTargetOutcome { tool_key, status: Assigned{record} | AlreadyAssigned | Failed{error} }>` (deterministic engine; sync failures stay inside the record as `status = "error"`); `assign_skill_to_project_tools(&store, project_id, skill_id, now)` (probe with typed `NotFound` + persisted tool list → serves `bulk_assign_skill`; command maps `Failed` → `BulkAssignErrorDto`); `assign_skill_to_project_tool(...)` (single target; `AlreadyAssigned` → typed `AssignmentExists`; serves `add_project_skill_assignment`). Record→DTO mapping deduplicated into `to_assignment_dto`.
- **`gitignore::update_for_project(&store, project_id, IgnoreUpdateOptions { add_to_gitignore, add_to_exclude })`** — lookup (typed `NotFound`), `is_dir` check, derive-from-persisted-tools, write. Inputs unchanged for ticket 28.
- **Legacy-orphan decision**: the `project_ops.rs` orphan branches **stay** — skill deletion can't reach orphan assignments (no `skill_id` to plan from), the project-side walkers can, `foreign_keys` is per-connection so external DB touches can still produce such rows; covered by ticket 19's tests. Documented in the module doc.
- Tests: `tests/skill_removal.rs` (10: global + project via divergent Pi mapping + central + record, missing-row sweep, already-missing targets, partial failure → `Failed` + typed `DeleteCleanupFailed`, cleanup-before-cascade), `tests/project_sync.rs` (+7: order, dedupe, unknown-tool isolation, sync failure as data, probe, `NotFound`, `AssignmentExists`), `tests/gitignore.rs` (+5).
- Wire nuance (flagged): `add_project_skill_assignment` with a nonexistent *skill* now yields typed `NOT_FOUND{kind:"skill"}` instead of `OTHER("skill not found: …")` — consistent with `bulk_assign_skill`; frontend handles `NOT_FOUND` generically.
- Left alone: the pre-existing `bulk_assign_*` tests still hand-simulate the old loop (could be pointed at the engine later); `resync_project` was already in core.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
