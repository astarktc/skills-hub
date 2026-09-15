# 05: Migrate the project-scope removal callers onto the Artifact removal module

Status: resolved

Type: task
Source: `../spec.md` Q4, Q13

**What to build:** Unassigning a skill from a Project Tool, removing a configured Tool from a Project, and removing a Project all become plan builders over the Artifact removal module: they choose a scope, execute once, and apply their final policy by reading the report. The three private removal implementations (with their three presence probes and three failure rules) are deleted. The orphan-row cleanup that project removal performs uses the same plan. The keep-row-with-`error` rule now holds across every scope.

**Blocked by:** 03

- [x] No project-scope code calls the low-level remove-path helper directly; every removal goes through plan → execute → report
- [x] Existing project-ops and project-sync cleanup tests pass unchanged or are rewritten to assert plan contents plus report outcomes
- [x] Removing a Tool whose assignment rows reference a skill that no longer exists still removes the artifacts and settles the rows, with a test
- [x] A failed artifact removal during unassign leaves the assignment row as `error` and the report says why; it is not deleted
- [x] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.

Shipped in `c5ec2ce` on `arch/05-project-removal-callers`.

**What shipped**

- `core/artifact_removal.rs`: new `RemovalScope::ProjectSkillTool { project_id, skill_id, tool_key }`
  (one assignment row — the unassign case had no scope). `Project` / `ProjectTool` lost their
  `#[allow(dead_code)]`. `plan`'s doc records that the project scopes pass an empty `home`.
- `project_ops::remove_project_artifact` **deleted**; `project_sync::unassign_and_cleanup`'s bespoke
  probe + `remove_path_any` + failure branch **deleted**. `grep remove_path_any project_ops.rs
  project_sync.rs` → no matches.
- All three callers are now plan → `execute_unlocked` → read report (unlocked seams; they are already
  inside the guard).
- `SyncStatus::has_deployed_artifact` removed — its only caller was the project-removal status gate,
  and the module asks the filesystem instead (one presence rule). Doc comment in `sync_status.rs`
  rewritten; the stale "`remove_tool_with_cleanup` has no trustworthy status" paragraph is gone.

**Final policies (all deliberate, per ADR-0002)**

| caller | before | now |
|---|---|---|
| `unassign_and_cleanup` | row → `error`, return raw fs `Err` | row → `error`, return typed `SignalError::DeleteCleanupFailed { failures }` naming the path |
| `remove_tool_with_cleanup` | logged and continued, then always `remove_project_tool` | returns `RemovalReport`; the project-tool row is removed **only** when every artifact went, so a stuck tool stays configured and retryable |
| `configure_project_tools` | removal failures invisible | continue semantics kept: every removal runs, the ignore update runs, then one `DeleteCleanupFailed` carrying every failure |
| `remove_project_with_cleanup` | deleted the project regardless | mirrors ticket 03's whole-skill rule: any failure keeps the project and its `error` rows and raises `DeleteCleanupFailed` |

**Deviations / notable behaviour changes**

1. `remove_tool_with_cleanup` signature is now `Result<RemovalReport>` (was `Result<()>`); it is
   `pub(crate)`, so no wire impact. Existing tests kept compiling unchanged.
2. The unassign artifact is now located from `assignment.skill_name` (the module's rule) rather than
   the live `skill.name`. Strictly more correct: a renamed skill's artifact on disk carries the name
   recorded at assign time.
3. Project removal no longer gates on `assignment.status.has_deployed_artifact()`. The plan covers
   every row and the presence rule handles absent paths as successful removals — which also fixes
   `missing`/`pending` rows whose artifact actually was on disk.
4. Assignment rows the plan skips (no locatable artifact: unknown tool key, no skill name) are deleted
   with the project via the `delete_project` cascade — documented on
   `remove_project_with_cleanup_unlocked`. `remove_tool_with_cleanup` cannot see an unknown tool key
   (rejected upstream by `configure_project_tools_unlocked`).
5. `errors.deleteCleanupFailed` copy (en + zh) generalised from "so the skill was kept" to
   "so what describes them was kept" — the variant now fires for project and tool removal too. Only
   `src/i18n/resources.ts` touched on the frontend; `src/bindings/index.ts` is unchanged.

**Tests added** (405 pass, up from 400; the `#[cfg(unix)]` ones make the target's *parent* dir `0o555`
and skip when run as root)

- `project_ops::remove_tool_with_cleanup_plans_orphan_rows_from_their_stored_skill_name` — plan
  contents + report outcomes for an orphaned (skill row gone) assignment.
- `project_ops::remove_tool_with_cleanup_keeps_the_tool_row_when_an_artifact_stays`
- `project_ops::configure_tools_applies_the_rest_then_raises_the_removal_failures`
- `project_ops::remove_project_with_cleanup_keeps_the_project_when_an_artifact_stays`
- `project_sync::unassign_failure_keeps_the_row_as_error_and_reports_the_path`

**Follow-ups for 07 (project mutations return a view)**

- `remove_tool_with_cleanup` already returns a `RemovalReport`; `configure_project_tools` currently
  throws it away except for the failure list. When 07 gives these commands a `ProjectViewDto`, the
  report is the natural carrier for per-target removal outcomes (paths kept, rows now `error`) — the
  frontend can then name them instead of relying on the `DeleteCleanupFailed` message list.
- Same for `remove_project_skill_assignment` / `remove_project`: both now raise
  `DeleteCleanupFailed` on failure. If 07 prefers report-data-over-error for these, the change is
  purely at the caller's final-policy line — the module already returns the full report.
