# Reviewer: Claude Opus 5 (cortex/claude-opus-5, thinking high) — range 943f85c...cdbbd15

Checks: tsc-7 -b --force exit 0; vitest 89/89. Positively verified acceptance criteria for 19,20,21,23,24,25,27,30 (details in raw output).

## Standards — 4 hard, 12 judgement
Hard:
1. `project_ops.rs:199` `bail!("unknown tool: {}", tool)` in new `configure_project_tools` — prose error over wire as OTHER; `commandError.ts:110` now returns `e.message` raw. Siblings: `gitignore.rs:226` "project directory does not exist", `project_ops.rs:112,237,333` "project not found" (vs typed NotFound at :192-197). ADR 0001 / English-primary.
2. `resources.ts:269` `errors.skillNotFoundInRepo` only in `en`, missing in `zh`. Pre-existing; t29 rewrote the emitter (`useAddSkillFlow.ts:474`).
3. AGENTS.md:95 stale `invoke('get_managed_skills')` vs :60-63.
4. AGENTS.md:122 "Core never reads the environment" false: 6 `std::env::var` in core (`git_fetcher.rs:34,274,282,296`, `sync_engine.rs:221`, `install_finalize.rs:253` relocated by t22).
Judgement:
5. `catalog.rs:78 vs :82` project entry has `skills_dir` = global dir while `shared_with` groups by project-relative dir — two-mappings trap re-created; unread by FE.
6. `sync_status.rs:74/:181` `has_deployed_artifact` (Synced|Stale|Error) vs `has_deployed_target` (Synced|Stale|Missing) — near-identical predicates, separate impl blocks.
7. `install_finalize.rs:22` imports `installer::now_ms` back (cycle); 4 `now_ms` copies incl. new `central_repo.rs:53`.
8. `cache_cleanup.rs:14` middle-man re-export (stale comment).
9. `useSettingsState.ts:55,57,58` defaults retyped (1.0/30/60) vs `settings.rs:39-43`.
10. Data clump: `update_project_gitignore` positional bools vs `configure_project_tools` DTO; camel→snake conversions at `ProjectsPage.tsx:26-29, 91-94`.
11. `commands/mod.rs:186-200` `get_settings` side effect `ensure_central_repo`, called on mount from two hooks.
12. `tool_adapters/mod.rs` accessors `.cloned()` out of static registry.
13. `useCandidatePick.ts:181` `install` not memoised.
14. `lib.rs:37,40` `install_local`/`install_git` registered but zero FE call sites.
15. Six direct `toast.*` calls bypass reporter (`useSkillLibrary.ts:174`, `useExploreState.ts:83,100,128`, `useSyncOrchestration.ts:247,283`).
16. `useProjectState.test.ts:249,258` fixture uses code `"INTERNAL"` which is not a CommandError code.

## Spec
- T22 (c): `installer.rs:88-96` passes folder-derived name as `NameIntent::UserProvided` → local flow never applies SKILL.md preference; behaviour preserved vs base; interface honesty.
- T26 (b)/(c): folder-URL git listing widened — base ran only `collect_skill_dirs(&dir)` (`943f85c installer.rs:1012-1039`); new `git_candidates_in` (`installer.rs:609-636`) runs full `discover_skills` (marketplace + depth-5 walk). Not in Answer drift list.
- T28 (c): `useProjectState.ts:355-360` intent consumed before call; modal stays open (`ProjectsPage.tsx:45-51`); retry with `gitignore === null`. Base also cleared early (`943f85c ProjectsPage.tsx:73-83`) but closed modal + toast.warning → no misleading retry. Test `:232` doesn't cover retention.
- T29 (c): `skill_matching.rs:80-83` bidirectional containment — empty candidate name matches every target (`target.contains("")`). Table test lacks blank-candidate case. Polish.
- No findings: 19, 20, 21, 23, 24, 25, 27, 30.

## Integration
- T28 regression (minor): fix = move `setPendingIgnore(null)` after await.
- Polish: three cleanup gates differ (orphan branch: none; removal: has_deployed_artifact; reconcile: has_deployed_target) — comment why.
- **Verified correct**: t23 virtual group × t28 — FE sends `agents_skills`, `adapter_by_key` resolves group literal (`mod.rs:184-190`), patterns derive `/.agents/skills/`, 9 constituents excluded from `listed`.
- Polish: `skill_store.rs:1092-1112` full row scan vs GROUP BY (O(rows) per project on list_projects).

## Regressions vs polish
- Regression: T28 only.
- No Answer claims something the code does not do.
