# 03: One lookup per assignment in Propagation and project sync; the Refresh overlap test asserts ordering

Status: done — 5d8c14b

**What to build:** Propagating or syncing one project assignment resolves its Tool adapter and its skill once and hands them down, instead of re-fetching in each helper; a row's own Tool is always a member of its shared-dir group, even when a test overrides the registry; the seven-positional-parameter sync helper takes one small context type; the project-sync resolver that has one in-module caller stops being `pub`; the artifact-naming helper's doc comment states that its live-name fallback fires only for an empty (un-backfilled pre-V6) stored name. The Refresh acquisition-overlap test proves concurrency by observed ordering (a barrier or observed in-flight count ≥ 2), never by elapsed time, so the gate stops flaking under parallel cargo load.

Source: `../spec.md` Q10; `../source-12-followups.md` #4, #10, #11, #12, #16, #19.

**Blocked by:** None (can start immediately)

- [x] Test: with a registry override that maps a row's Tool to an adapter sharing no dir, Propagation still writes that row's own target (row's Tool in its group)
- [x] The overlap test passes ten consecutive runs under `cargo test --all` with the rest of the suite (`for i in $(seq 10)` loop output in Comments) and contains no wall-clock comparison
- [x] Adapter and skill are fetched once per assignment on both the Propagation and the project-sync paths (no second `adapter_by_key` / `get_skill_by_id` for the same assignment)
- [x] Behaviour unchanged: every existing Propagation / project-sync test passes without assertion edits
- [x] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Status reconciliation: ready-for-agent → done — 5d8c14b. Evidence: Cited 5d8c14b integrated as 6ed07b6; 30f9520/279c31f replaces elapsed timing and d78ce9a/760eaff deduplicates assignment lookups. src-tauri/src/core/tests/refresh.rs:1214 and src-tauri/src/core/project_sync.rs:108 retain these rules.

### 2026-09-05 — implementation (branch r4/03-propagation-project-sync-dedupe)

**Shipped** (three commits on `f057190`; touches only `core/propagation.rs`, `core/project_sync.rs`,
`core/artifact_removal.rs` (one call site), `core/tests/propagation.rs`, `core/tests/refresh.rs`):

- `5d8c14b` fix(propagation): a row's own Tool is always in its shared-dir group — #11. Red first:
  `a_row_whose_tool_shares_no_dir_with_the_registry_is_still_its_own_group` shadows Cursor with
  `.lone-tool/skills`; before the fix the row produced **no outcome at all** (`no outcome for Global
  { tool: "cursor" } in []`). The group filter is now `r.tool == row.tool || sharing.contains(..)`.
- `30f9520` test(refresh): prove acquisition overlap by observed in-flight count — #4. An `OverlapDoor`
  (Mutex counter + Condvar + `AtomicUsize` max) every acquisition passes through: until overlap has been
  witnessed, an acquisition waits at the door for a second one. Assertion: `max_in_flight >= 2`. No
  `Instant`, no elapsed comparison; the only duration is a 10 s rendezvous bound so a sequential
  regression fails (verified: with `ACQUIRE_POOL_SIZE = 1` the test fails in 10.05 s with
  "acquisitions ran one at a time: at most 1 in flight"; once timed out the door opens for the rest, so
  a regression costs one bound, not eight). `LATENCY_MS` removed; `slow_acquire` doc reworded (it still
  serves the ordering and cancellation tests).
- `d78ce9a` refactor(project_sync): resolve adapter and skill once per assignment — #10 #12 #16 #19.
  `AssignmentSyncContext<'a> { store, project_path, adapter: &'static ToolAdapter, skill: &SkillRecord,
  overwrite, now }` replaces the 7 positional params; `sync_assignment_target(&ctx, assignment,
  central_hash)`. `assignment_artifact_name(assignment, live_name: impl FnOnce() -> Result<Option<String>>)`
  and `resolve_assignment_artifact(project_path, adapter, assignment, live_name)` take a live-name
  *supplier* (the same shape as the existing `central_hash` supplier), so the "only for an empty stored
  name" rule stays in one place and callers holding the skill supply `skill.name` without a store read;
  `artifact_removal::push_assignments` supplies the store lookup. `require_adapter` is the one typed
  `UnknownTool` lookup in the module; `assign_and_sync` and `sync_single_assignment` resolve once and
  hand down; `observe_assignment` no longer re-queries. `resolve_project_sync_target` → `pub(crate)`.
  Doc comment of `assignment_artifact_name` states the fallback fires **only** for an empty stored
  name (un-backfilled pre-V6 row), is best-effort, and never overrides a non-empty stored name.

**Gate**: `npm run version:check` → Version OK (1.2.3). `npm run check` exit 0: vitest 12 files / 172
tests, build OK, clippy clean, `cargo test` 446 passed (445 at base + 1 new). `cargo test --all` ×10:

```
run 1: ok … run 10: ok   (no FAIL)
```
Plus 5 runs of the overlap test under 8 `yes > /dev/null` CPU burners: all ok.

**Deviations**
- `artifact_removal.rs` gained a 6-line change at one call site (it is the only out-of-module caller of
  `resolve_assignment_artifact`) — needed so the name rule has one signature. No behaviour change: it
  still reads the store only for an un-backfilled row.
- `sync_assignment_target` now `expect`s the name resolution (a live skill name always locates the
  artifact) instead of raising the previously-typed-but-now-unreachable `NotFound`; the skill is in hand
  by construction. Same pattern as `assign_skill_to_project_tool_unlocked`'s `expect`.
- The global-rows path in `propagate_global_rows` still calls `adapter_by_key` twice per row (driver
  lookup + group build); the finding (#10) and the acceptance are about the per-*assignment* paths, so
  it was left alone.

**Notes for the orchestrator**
- Branch is based on `f057190`; `main` has since taken ticket 01 (`0fc6210`, `f30a8d9`, `f04e35c`).
  `git diff f057190..HEAD` is exactly the five files above; no overlap with ticket 01's files, so the
  rebase should be clean. No DTO / binding changes.
- No `.scratch` or version changes; worktree clean except the three commits.
