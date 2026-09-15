# Reviewer: Claude Fable 5.1 (anthropic/claude-fable-5-1, thinking high) — range 943f85c...cdbbd15

Mechanical checks run: cargo test --all (316 pass, export test skipped), vitest 89, eslint clean, versions 1.1.9 in sync, README rows = ToolId arms (45), project-dir table diffed vs old match (0 mismatches), zero "cursor" strings outside as_key, zero get_setting/set_setting outside core/settings.rs, zero raw invoke outside bindings, zero AppHandle in installer/central_repo.

## Standards — 0 hard, 9 judgement
1. Duplicated `now_ms()` — 4 copies (`central_repo.rs:53`, `installer.rs:292`, `cache_cleanup.rs:95`, `commands/mod.rs:728`).
2. Circular import: `install_finalize.rs:22` `use super::installer::now_ms` while installer imports finalize back.
3. Middle Man + stale comment: `cache_cleanup.rs:11-14` re-export; `installer.rs:7` still routes through the alias.
4. Data Clumps: gitignore booleans cross the wire in two shapes (`update_project_gitignore` positional vs `configure_project_tools(gitignore: Option<IgnoreUpdateOptions>)`); FE converts `{addToGitignore,addToExclude}`→snake twice (`ProjectsPage.tsx:28,92`, `useProjectState.ts:389-396`).
5. `useSettingsState.ts:63-70` `adoptSettings(snapshot)` overwrites all six fields on every single-setting write; zoom hotkey (`:134`) writes without adopting → stale-snapshot clobber risk. Race class pre-existed, blast radius grew 1→6 fields.
6. AGENTS.md:123 "Core never reads the environment" overclaims — `install_finalize.rs:249-256` reads `std::env::var("SKILLS_HUB_COMPUTE_HASH")` (+ pre-existing env reads in `git_fetcher.rs:34,274,282`, `sync_engine.rs:221`).
7. AGENTS.md:95 stale example `invoke('get_managed_skills')` → should be `invokeTauri("getManagedSkills")`.
8. Duplicated cleanup walker shape ×3 in `project_ops.rs:139-150, 245-257, 262-274`.
9. Side effect in getter: `commands/mod.rs:194-206` `get_settings` calls `ensure_central_repo`; now called on mount by two hooks.

## Spec
- T21 (c): `useAddSkillFlow.ts:384-390` `handleImport` awaits `loadManagedSkills(); fetchPlan()` inside `runAction` body → plan-refetch failure after successful import suppresses success toast and leaves modal open. Old (`943f85c` `useAddSkillFlow.ts:421-427`) swallowed and closed. Edge regression.
- T22 (c): `install_finalize.rs:111-124` `move_into` copy fallback can leave partial `dest` in central repo on mid-copy failure (Drop only removes staging). Answer claim "no partial dirs" not unconditional. Cross-device only. Polish.
- T25 (c): Answer "sweeps orphan skill_targets" — `skill_removal.rs:182-201` sweeps FS paths but doesn't delete rows; same as before. Wording.
- T26 (b): `skill_discovery.rs:118` root is a candidate when `is_claude_skill_dir(root)` w/o SKILL.md; old listing required SKILL.md (`943f85c installer.rs:1115`) → lists valid then `SKILL_INVALID` at install (`installer.rs:729-734`). Not disclosed. Cosmetic.
- T27 (b): `project_ops.rs:243` `remove_project_with_cleanup` now cleans Error rows (has_deployed_artifact = Synced|Stale|Error); previously synced|stale only. Disclosed in Answer; scope creep only.
- T27 (c): none — `next_status` reproduces old branches exactly.
- T28 (c): `useProjectState.ts:355-360` `setPendingIgnore(null)` before `configureProjectTools` awaited → retry after tools-side failure runs with `gitignore: null`. Old `pendingGitignoreRef` nulled only after tools persisted (`943f85c ProjectsPage.tsx:69-73`). Edge regression.
- T30: none; all 44 commands verified, positional order checked at every multi-arg site.
- No findings: 19, 20, 23, 29.

## Integration OK'd
t25×t28 gitignore ordering (test pins it); t20×t22×t26 staging/discovery; t25×t27 removal planning before cascade; GithubApiError wire codes; t23 shared_with delta unobservable.

## Regressions vs polish
- Regression (edge, low): T28 ignore intent dropped early; T21 import modal/toast on plan-reload failure.
- Polish: everything else.
