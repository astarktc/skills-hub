# 01 — Backend-A: `ProjectSyncReport`, bulk unassign, Tool-dir candidates flagged at listing

Status: done — 2a79874
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

## Comments

### 2026-09-22 — backend-A implementation (branch `round16/backend-a`, commits `c84a65a`, `cbb2474`)

Done. Gate: `cargo fmt && cargo clippy --all-targets -- -D warnings` clean; `cargo test --all` **655 passed**
(+8 new tests over the 647 baseline); `npm run build` green; `npm run test` 362 passed; `npm run lint` clean.

**Wire map (old → new)**
- NEW `ProjectSyncReport = { items: ProjectSyncOutcome[] }`;
  `ProjectSyncOutcome = { assignment_id: string | null, skill_id, skill_name, tool, status: ProjectSyncOutcomeStatus }`;
  `ProjectSyncOutcomeStatus = { status: "synced" } | { status: "already_assigned" } | { status: "failed"; error: CommandError }`.
- `ToggleAssignmentResultDto { view, assigned: boolean, report: RemovalReport | null }` →
  `{ kind: "assigned"; view; report: ProjectSyncReport } | { kind: "unassigned"; view; report: RemovalReport }`.
- `BulkAssignResultDto { view, failed: BulkAssignErrorDto[] }` → `{ view, report: ProjectSyncReport }`;
  `BulkAssignErrorDto` REMOVED.
- `ResyncProjectResultDto { view, summary: ResyncSummary }` → `{ view, report: ProjectSyncReport }`.
- `ResyncAllResultDto { summaries: ResyncSummary[], projects }` → `{ report: ProjectSyncReport, projects }` (one report
  spanning every project). `ResyncSummary` REMOVED.
- NEW command `commands.bulkUnassignSkill(projectId, skillId) → BulkUnassignResultDto { view: ProjectViewDto, report: RemovalReport }`.
- `LocalSkillCandidate` shape unchanged; new `reason` code `"inside_tool_dir"` (with `valid: false`).
- `CommandError` union unchanged.

**Deviations**
1. Status enum is `ProjectSyncOutcomeStatus`, not `ProjectSyncStatus`: that name is taken by the existing project
   roll-up enum (`sync_status::ProjectSyncStatus`, already on the wire as `ProjectDto.sync_status`); specta refuses
   duplicate names. Pairs with `ProjectSyncOutcome` like `RemovalTargetOutcome`/`RemovalTargetStatus`.
2. `RemovalScope` does not appear in the bindings (it is `#[serde(skip)]`/`#[specta(skip)]` on `RemovalReport` since
   round 15), so `project_skill` has no TS face; the ticket's bindings checklist expected it.
3. `resync_all_projects`: a project-level failure (store failure listing a project's assignments) now fails the whole
   command instead of producing a prose `"project-level error: …"` summary — items carry no `project_id`, and "only a
   store failure fails the whole operation" is the established rule. Per-assignment failures stay report data.
4. Toggle-on with an unknown tool is now report data (`failed` + `UNKNOWN_TOOL`, `assignment_id: null`), not a command
   error — the batch-of-one reading of D1. `AssignmentExists` stays an internal invariant bail.
5. `assign_and_sync` is `#[cfg(test)]` (a fixture over the private `assign_and_settle`, which returns the settled
   failure as `CommandError`); production reaches first syncs only through the fan-out.
6. A listing candidate both inside a Tool dir and manifest-invalid reports `inside_tool_dir` (editing `SKILL.md`
   would not make it installable).

**Frontend lines touched (type-level follow-ups only; ticket 03 replaces them with the fold)**: `types.ts` re-exports
(−`ResyncSummary`, −`BulkAssignErrorDto`, +`ProjectSyncReport`, +`ProjectSyncOutcome`, +`ProjectSyncOutcomeStatus`,
+`BulkUnassignResultDto`); `useProjectState.ts` resync return types → `ProjectSyncReport`, toggle returns
`result.kind === "unassigned" ? result.report : null`; `AssignmentMatrix.tsx` props types + interim inline counts from
`report.items`; `ProjectsPage.tsx` `handleBulkAssign` derives the failed list from `report.items`;
`useProjectState.test.ts` stubs follow the new shapes.

- 2026-09-22 (orchestrator) — rebased onto main (only `src/bindings/index.ts` conflicted; regenerated from the union, never hand-merged) and merged fast-forward. Deletion review: removed symbols are `ToggleAssignmentResultDto` (struct → tagged enum), `BulkAssignErrorDto`, `ResyncSummary`, `AssignTargetStatus`/`AssignTargetOutcome` (replaced by `ProjectSyncOutcome`), and the two resync signatures. Deviations 1–6 accepted; #3 (a DB read failure in resync-all fails the command) is the round-15 rule, not a regression. Gate on main: cargo 662, vitest 362, lint, build, bindings clean.
