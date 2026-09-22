# 01 — Backend-A: `ProjectSyncReport`, bulk unassign, Tool-dir candidates flagged at listing

Status: ready-for-agent
Spec: `.scratch/round16/spec.md` — decisions D1, D2, D6. Read the spec first; this ticket is the project-world
backend half. Ticket 03 (frontend-A) follows the bindings you regenerate.

## Goal

Every project-world mutation answers with a core report the frontend fold understands. Bulk assign gains its
inverse. The Add flow's local listing flags a Tool-dir folder before Install has to refuse it.

## Work

1. **D1 — `ProjectSyncReport`** in `src-tauri/src/core/project_sync.rs`:
   ```rust
   #[derive(Clone, Debug, serde::Serialize, specta::Type)]
   pub struct ProjectSyncReport { pub items: Vec<ProjectSyncOutcome> }
   #[derive(Clone, Debug, serde::Serialize, specta::Type)]
   pub struct ProjectSyncOutcome {
       pub assignment_id: Option<String>,   // None only when no row could be created
       pub skill_id: String,
       pub skill_name: String,
       pub tool: String,                    // registry key
       pub status: ProjectSyncStatus,
   }
   #[derive(Clone, Debug, serde::Serialize, specta::Type)]
   #[serde(tag = "status", rename_all = "snake_case")]
   pub enum ProjectSyncStatus { Synced, AlreadyAssigned, Failed { error: CommandError } }
   ```
   Produced by four paths, each classifying **where the row settles** (after the `{:#}` chain is written to
   the row's `last_error` where that happens today) — core never calls `CommandError::from_anyhow` on an error
   it then `?`-propagates:
   - `assign_skill_to_tools` (`:224–342` region): today returns `Vec<AssignTargetOutcome>` with
     `AssignTargetStatus::Failed { error: anyhow::Error }`. Either make `ProjectSyncOutcome` *the* fan-out item
     (preferred — one type) or settle each `AssignTargetOutcome` into one before it leaves core. A sync failure
     that `sync_single_assignment` already stores in the record (`status = error`, `last_error`) must surface as
     `Failed { error }` **with the row's `assignment_id` present** — that row exists; the cell is red; the
     operator must hear about it. `AlreadyAssigned` stays a status, not a failure.
   - `toggle_skill_assignment` (`:626–650`): `ToggleOutcome::Assigned { report: ProjectSyncReport }` (batch of
     one). `assign_skill_to_project_tool_unlocked`'s `AssignmentExists` bail for `AlreadyAssigned` cannot happen
     from toggle (the same critical section just read the row absent) — keep it as an internal invariant, not a
     wire condition.
   - `resync_project_unlocked` (`:395–425`) and `resync_all_projects` (`:430`): return `ProjectSyncReport`
     (resync-all: one report spanning every project — `ProjectSyncOutcome` needs no `project_id`; the command
     already returns the project list alongside). **Delete `ResyncSummary`** and the `format!("{}: {:#}", …)`
     prose.
2. **Commands** (`src-tauri/src/commands/projects.rs`):
   - `ToggleAssignmentResultDto { view, assigned: bool, report }` → replace with an internally-tagged answer
     the frontend can switch on without a boolean, e.g.
     `#[serde(tag = "kind", rename_all = "snake_case")] enum ToggleAssignmentResultDto { Assigned { view, report: ProjectSyncReport }, Unassigned { view, report: RemovalReport } }`.
   - `BulkAssignResultDto { view, failed }` → `{ view, report: ProjectSyncReport }`. Delete `BulkAssignErrorDto`
     and the seam-side `CommandError::from_anyhow` loop (`:330–337`).
   - `ResyncProjectResultDto` / `ResyncAllResultDto`: `summary`/`summaries` → `report: ProjectSyncReport`
     (resync-all keeps `projects`).
3. **D2 — bulk unassign**:
   - `artifact_removal::RemovalScope::ProjectSkill { project_id: String, skill_id: String }` — plan every
     assignment row of that skill in that project; execute and settle exactly like `ProjectSkillTool` (row kept
     with sync status `error` on failed removal, deleted on success — ADR-0002). Extend the scope's `Display`
     and any exhaustive matches; add a planning test beside the `ProjectSkillTool` ones in
     `core/tests/artifact_removal.rs`.
   - `project_sync::unassign_skill_from_project(store, project_id, skill_id) -> Result<RemovalReport>` —
     **entry point** under `mutation_guard::serialized`, calling `artifact_removal::execute_unlocked` (the
     guard is non-reentrant; never call another entry point). Unknown project/skill → the existing `NotFound`
     signal.
   - Command `bulk_unassign_skill(projectId, skillId) -> Result<BulkUnassignResultDto { view, report:
     RemovalReport }, CommandError>` with `#[tauri::command] #[specta::specta]`, registered in
     `collect_commands![…]` in `src-tauri/src/lib.rs`.
4. **D6 — listing flags Tool-dir folders** (`src-tauri/src/core/installer.rs:296` `list_local_skills`):
   take `home: &Path` (core never resolves roots — the command `list_local_skills_cmd` at
   `commands/mod.rs:300` resolves it at the seam the way `installer_paths` does); for each candidate whose
   absolute folder `tool_adapters::tool_holding_path(home, …)` locates, emit `valid: false, reason:
   Some("inside_tool_dir")`. Existing `reason` values are codes the frontend maps (`LocalPickModal.tsx:39`) —
   add this code, no prose. Unit test in `core/tests/` with a temp home holding a fake `~/.claude/skills/x`.
5. **Bindings**: `cd src-tauri && cargo test --all` regenerates `src/bindings/index.ts` — commit it. Review the
   diff: `CommandError` union unchanged; `ProjectSyncReport`, `ProjectSyncOutcome`, `ProjectSyncStatus`,
   `RemovalScope` (+`project_skill`), `BulkUnassignResultDto`, the toggle enum appear; `ResyncSummary`,
   `BulkAssignErrorDto` disappear. Re-export the new DTO types from `src/components/projects/types.ts` (the
   per-world shim) — components never import `src/bindings` directly. **Do not touch frontend logic** beyond
   what `npm run build` needs to stay green — if the build breaks on the removed types, make the smallest
   type-only adjustment and note it in your report; ticket 03 owns the real frontend work.

## Constraints

- AGENTS.md invariants: new command → `#[specta::specta]` + `collect_commands!`; `Option<T>` is `T | null`;
  `commands/` is wiring only; every sync-target mutation entry point wraps its own body in
  `mutation_guard::serialized`; removal goes through `artifact_removal` only.
- No `#[serde(rename)]` to fake old wire words. No `*ReportDto` mirrors. No counters on the wire.
- Backend prose: English, structured `detail` fields for diagnostics, never localized copy.
- Tests: `cargo test --all` (CI runs `--all`), clippy clean (`npm run rust:clippy`), `cargo fmt`.
- Work in your assigned worktree only. Commit on your branch with conventional messages; do not merge, do not
  touch `.scratch/` beyond appending a dated `## Comments` entry to this ticket, never `git mv`/archive.

## Report back

Old→new wire map (every renamed/removed/added TS type and command, with the new field shapes) — ticket 03's
brief is built from it. Any deviation from D1/D2/D6 and why. Gate output (`cargo test --all` count, clippy).
