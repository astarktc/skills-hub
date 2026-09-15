# Reviewer: GPT-5.6 Sol (cortex-responses/openai-gpt-5.6-sol, thinking high) — range 943f85c...cdbbd15

## Standards
### Hard violations
1. Cursor project-scope symlink via `agents_skills` virtual group (`tool_adapters/mod.rs:180-198`, `catalog.rs:55-78`, `sync_engine.rs:143-152`, `project_sync.rs:40-61`) — group adapter has `supports_symlink: true`; Cursor-only install presents the group → symlink path. AGENTS.md do-not + CONTEXT.md.
2. Unknown stored lifecycle value can become healthy on list (`skill_store.rs:172-203`, `sync_status.rs:149-169`, `project_sync.rs:394-419`) — `read_lifecycle` → Error, `next_status` recomputes copy-mode Error → Synced/Stale on hash match, `reconcile_assignment` persists + clears `last_error`. CONTEXT.md:55-56 "never as healthy".
3. `specta-typescript = "0.0.12"` not an exact `=` pin (`Cargo.toml:40-42`) vs AGENTS.md:71-73.
4. `migrate_legacy_db_if_needed` calls `dirs::data_dir()` in core (`skill_store.rs:1164-1176`) vs AGENTS.md "core never reads the environment".
### Judgement
5. Middle Man: `cache_cleanup.rs:11-14` re-export `pub use super::settings::git_cache_ttl_secs as get_git_cache_ttl_secs`.

## Spec
- T23 (c): Cursor forced-copy ineffective at project scope (= #1).
- T24 (a): `SkillStore::set_onboarding_completed` still calls `self.set_setting("onboarding_completed", …)` (`skill_store.rs:356-362`) — spec "No raw get_setting/set_setting outside core/settings.rs"; dead-code residue.
- T27 (c): unknown-value policy "never healthy, never rewrites on read" not upheld via `listProjectSkillAssignments` reconcile (= #2).
- T28 (c): retry after gitignore-write failure drops the ignore update — `project_ops.rs:203-220` persists tool changes before writing gitignore; hook clears `pendingIgnore` before invoke (`useProjectState.ts:353-367`); modal stays open (`ProjectsPage.tsx:43-51`) but next confirm passes `null`. Answer acknowledges failure-UX change at ticket line 33.
- T30 (a): exact-pin criterion partial (= #3).
- No findings: 19, 20, 21, 22, 25, 26, 29.

## Regressions vs polish (reviewer's own classification)
- Regression: T28 retry loses ignore update.
- Bugs not proven new vs 943f85c: Cursor group capability; unknown lifecycle → healthy.
- Polish: env seam residue, pin syntax, onboarding raw call, middle man.
