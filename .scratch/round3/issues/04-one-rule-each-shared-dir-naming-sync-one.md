# 04: One rule each — shared-skills-dir group, project artifact name, sync-one-assignment

Status: done — ccaa6a3

**What to build:** Three domain rules that exist in two or three copies become one each. (a) "Which Tools share a skills dir" is answered only by the Tool registry's `adapters_sharing_skills_dir`; Propagation and the global sync batch call it instead of comparing directories inline. (b) "Which name locates a Project assignment's artifact on disk" is one core helper returning the **stored** assignment name (what was materialised), never the live Managed-skill name; Artifact removal, Propagation and project sync all call it, and a test that renames the Managed skill after assignment proves removal and propagation still find the artifact. (c) "Sync one assignment through the capability-aware entry point and record `SyncCompleted`" is one function taking the hash rule as a parameter; project sync's single-assignment path and Propagation's per-assignment path both call it — if the two hash rules prove equivalent, collapse them and record why in Comments.

Source: `../spec.md` Q10, Q11 and the verified-facts section; `../source-13-review-followups.md` #1, #5, #6. Skill: **tdd**.

**Blocked by:** 03

- [x] Only the registry compares `relative_skills_dir` values
- [x] Rename-divergence test present and green
- [x] The project-assignment path reaches `sync_dir_for_tool_with_overwrite` through one function
- [x] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Status reconciliation: ready-for-agent → done — ccaa6a3. Evidence: Cited ccaa6a3 (integrated as 7d80a14), followed by 9d49351/2322172, unifies shared-dir and assignment rules; src-tauri/src/core/project_sync.rs:53,108.

### 2026-09-04 — implementation (branch `r3/04-one-rule-each`)

**Shipped**

- `ccaa6a3` refactor(core): (a) Propagation's global-row grouping and the global sync batch's
  override match now ask `tool_adapters::adapters_sharing_skills_dir` and match rows / override keys
  by tool key (adapters still resolved per row through `adapter_by_key`, so test shadows keep
  working). `grep 'relative_skills_dir =='` in core now hits only `tool_adapters/mod.rs:645`
  (the registry; `catalog.rs:83` compares the *project*-scope dir for virtual groups, also inside
  the registry module). No second registry fn — mapping to keys at the call site was enough.
- `9d49351` refactor(project-sync): (b) `project_sync::assignment_artifact_name(store, &assignment)
  -> Result<Option<String>>` — stored `assignment.skill_name`, doc comment explains stored-not-live
  (Q10); `resolve_assignment_artifact(store, project_path, adapter, &assignment)` is the path
  sibling. Callers: `artifact_removal::plan` (project rows), `propagation::propagate_one_assignment`,
  `project_sync::{assign_and_sync, sync_single_assignment, observe_assignment}` — the reconcile pass
  was a fourth live-name site (`observe_assignment`, review #5's `project_sync.rs:~408`) and is
  covered too. `resolve_project_sync_target` stays as the join primitive; in production only the
  helper calls it now. (c) `project_sync::sync_assignment_target(store, project_path, source,
  &assignment, overwrite, now, central_hash: impl FnOnce() -> Option<String>) ->
  Result<SyncOutcome>` — resolves the adapter and the stored-name target, calls
  `sync_dir_for_tool_with_overwrite`, records `SyncCompleted` (the only place for a project
  assignment). `assign_and_sync`, `sync_single_assignment` and `propagate_one_assignment` call it.
  Tests (TDD, red first): `propagation::a_project_artifact_is_found_by_its_stored_name_after_the_skill_is_renamed`
  (was creating a second artifact under the live name and leaving the stored one at `v1`),
  `project_sync::resync_rematerialises_the_artifact_under_its_stored_name_after_a_rename`,
  `project_sync::reconcile_observes_the_artifact_under_its_stored_name_after_a_rename` (was
  reporting `Missing`), and `artifact_removal::a_project_artifact_is_removed_under_its_stored_name_after_the_skill_is_renamed`
  (green from the start — removal already planned from the stored name; it locks the shared rule).
  The rename is `store.upsert_skill` with a new name (finalize never renames, no entry point).

**Q11 — are the two hash rules equivalent?** The *rule* is the same and now exists once: "only a
copy records a content hash; a link follows the central copy and cannot drift"
(`outcome.mode_used.can_drift()` inside `sync_assignment_target`). The *supply* is not: Propagation
receives the freshly finalized central hash from `finalize_and_propagate_unlocked` — computed once
for N targets, and `None` in release builds unless `SKILLS_HUB_COMPUTE_HASH` is set
(`install_finalize::should_compute_content_hash`) — while project sync hashes the source dir on
demand (`hash_source`, formerly `hash_after_sync`, feature-flag-blind, logs and records `None` on a
hashing failure). Collapsing to "always hash the source" would re-hash per target during Refresh and
change what release builds record; collapsing to "always the stored skill hash" would change what a
legacy `content_hash = NULL` row records on re-sync. So: one rule inside the function, the supplier as
the closure parameter, both documented on `sync_assignment_target`. `SyncFailed` settlement stayed with
each caller (error / count / report data) — unchanged policies, and the ticket scoped `SyncCompleted`.

**Deviations / notes**

- The helper takes `&SkillStore` (not the pure `&assignment -> &str` the orchestrator sketched)
  because Artifact removal carried a pre-V6 "empty stored name → consult the live skill row" fallback;
  keeping it *and* having one rule meant the fallback moves into the helper. Behaviour-preserving for
  every caller (the live name was what the other sites used anyway when the column is empty).
- `assign_and_sync` keeps a pre-check `adapter_by_key(tool_key).is_none() → UnknownTool` so no row
  is written for an unknown tool (as before); `sync_assignment_target` resolves the adapter again
  (cheap registry lookup) so it is self-contained.
- The propagation module doc's decision 2 names `project_sync::sync_assignment_target` as the road a
  project assignment takes to the capability-aware entry point.
- No frontend change; `cargo test` did not regenerate `src/bindings/index.ts`.
- Flake observed once during the full gate while sibling worktrees compiled:
  `refresh::tests::acquisitions_overlap_instead_of_running_one_at_a_time` (762 ms vs 0.6 × 1.2 s);
  re-run green; untouched.

**Gate**: `npm run version:check && npm run check` green — vitest 157 (12 files), cargo 435.

