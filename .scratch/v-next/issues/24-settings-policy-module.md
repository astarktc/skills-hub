# 24: A typed settings policy module

Status: resolved

Type: task
Blocked by: 20

## What to build

Review #2 (Opus + Sol, verified): the only settings interface is `SkillStore::get_setting(&str) -> Option<String>` / `set_setting` (`skill_store.rs:271-291` at `943f85c`), so every caller re-implements key naming, parsing, defaulting and clamping: `commands/mod.rs:104` (`installed_tools_v1` via serde_json), `:733`, `:768-771`, `:829`; `cache_cleanup.rs:24-53`; `central_repo.rs:15`; `featured_skills.rs:52-58`; `installer.rs:1649`; and `lib.rs:44` re-parses `ui_zoom_level` for the pre-paint read. 14 of the 57 registered commands are two-line settings pass-throughs (get/set for `central_repo_path`, `git_cache_cleanup_days`, `git_cache_ttl_secs`, `github_token`, `auto_sync_enabled`, `global_tool_config`, `ui_zoom_level`). The clamp bounds are retyped as literals on the other side of the seam: `useSettingsState.ts:173` (`3650`) / `:193` (`3600`) vs `cache_cleanup.rs:14,17`. `useSettingsState` has 9 `useEffect`s and 13 `invokeTauri` calls, five of them one-shot mount loads of a single scalar.

Scope decided in grilling: **backend-persisted settings only** (central repo path, cache policy, GitHub token, zoom, auto-sync, global Tool selection). Frontend-only prefs (theme/localStorage) stay out.

Deepen:

- `core/settings.rs`: typed keys (enum/struct), per-setting parse/default/bound, `get_settings(&store) -> AppSettings` and `set_setting(&store, Setting)`; `SkillStore` stays the concrete SQLite adapter (no repository trait — one adapter is a hypothetical seam). Legacy/malformed stored values must parse to the default without crashing.
- Two commands replace fourteen (get-all / set-one, or similar); `AppSettings` is a `#[derive(TS)]` DTO carrying the bounds so the frontend clamps from data, not literals; `lib.rs` pre-paint zoom read uses the module.
- `useSettingsState`: five mount effects collapse to one load; clamps read the DTO bounds; hook tests updated (one mocked command instead of five). Preserve `~` expansion behaviour for the central repo path and the DB migration path.
- Table tests in core for parse/default/clamp/round-trip and malformed legacy values.

## Acceptance criteria

- [ ] No raw `get_setting`/`set_setting` calls outside `core/settings.rs`; no numeric bound literal in TypeScript for settings.
- [ ] Registered settings commands ≤ 2; bindings regenerated and committed; frontend clamps from DTO bounds.
- [ ] Core table tests for every setting (default, bound, malformed) pass; vitest green.
- [ ] `npm run version:check && npm run check` green.

## Answer

Landed green in `ffde9d2` + orchestrator follow-up `6df1b5c` (Fable 5.1 child, medium thinking; clean rebase over tickets 23/22; 296 cargo + 67 vitest, full gate green on main).

**Module** (`core/settings.rs`, the only caller of `SkillStore::get_setting/set_setting`):
- Private `keys` module spells every storage key once (`central_repo_path`, `git_cache_cleanup_days`, `git_cache_ttl_secs`, `github_token`, `auto_sync_enabled`, `global_selected_tools_v1`, `scan_selected_tools_only`, `ui_zoom_level`, internal `featured_skills_cache`, `installed_tools_v1`).
- Bounds as data: `IntRange` / `FloatRange` → `SettingsBounds` (ts-rs exported); `GIT_CACHE_CLEANUP_DAYS_RANGE 0..=3650`, `GIT_CACHE_TTL_SECS_RANGE 0..=3600`, `UI_ZOOM_LEVEL_RANGE 0.5..=3.0` + `DEFAULT_*`.
- `AppSettings` DTO (resolved central path, cache policy, token (`""` = none), auto-sync, `global_selected_tools: Option<Vec<String>>`, `scan_selected_tools_only`, zoom, `bounds`); `SettingUpdate` DTO (`#[serde(tag="key", content="value")]`, 7 variants incl. `GlobalToolConfig { selected_tools, scan_selected_only }`).
- `load_settings(&store, fallback_root)`, `apply_setting(&store, fallback_root, SettingUpdate) -> AppSettings` (clamps into bounds, trims token, NaN/inf zoom → default; returns the *effective* snapshot). Typed readers for core/lib: `resolve_central_repo_path` (moved here from `central_repo.rs`; blank override = unset), `git_cache_cleanup_days`, `git_cache_ttl_secs`, `github_token`, `ui_zoom_level` (out-of-range → default), featured-skills cache, and `record_installed_tools` (the ticket-23 exception, routed by the orchestrator with 2 tests). Malformed/legacy values never error.
- `central_repo::move_central_repo(store, new_base)` absorbed the relocation loop from `commands/` and now validates every skill before moving any (old code could partially move).

**Commands**: `get_settings` (also `ensure_central_repo`) and `update_setting(update)` replace 14 pass-throughs (handler count 57 → 46); `~` expansion for `CentralRepoPath` at the seam via `expand_home_path`. `lib.rs` pre-paint zoom + startup cache cleanup read the module. `cache_cleanup.rs` keeps a `pub use … as get_git_cache_ttl_secs` re-export for the installer's import (candidate cleanup later).

**Frontend**: `useSettingsState` — five mount loads → one `get_settings`, `adoptSettings`/`updateSetting`, `clampTo(value, bounds.x)`, exposes `bounds`; `SettingsPage` derives `min`/`max` from bounds (zero numeric literals); `useSyncOrchestration` reads auto-sync + tool config from the same snapshot. Shim exports `AppSettings`, `SettingUpdate`, `SettingsBounds` (`GlobalToolConfigDto` deleted). New `useSettingsState.test.ts` (5 tests).

**Tests**: `core/tests/settings.rs` 26 (default/bound/malformed/round-trip per setting, central-repo move, wire deserialization, installed-tools diff), `core/tests/central_repo.rs` +4.

**Behaviour notes**: out-of-range cache values are now clamped on write instead of rejected (frontend clamps first; returned snapshot is authoritative). Two hooks each call `get_settings` on mount (2 calls vs. 7 before; hooks don't share state by design).

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
