# 20: Environment seam — home-root parameters and `AppHandle` eviction from core

Status: resolved

Type: task
Blocked by: None (can start immediately)

## What to build

Core creates its environment instead of accepting it (review #2, verified). Two shapes of the same problem at `943f85c`:

1. **`dirs::home_dir()` inline** in `core/tool_adapters/mod.rs:537,542` (`resolve_default_path`, `resolve_detect_path`, and `is_tool_installed` built on them). Consequence: `global_sync::plan_batch_tool_targets` and `sync_skills_to_tools` are the only untested functions in an otherwise exhaustively tested module; both tool-status assemblers in `commands/mod.rs` are untestable; `core/onboarding.rs:66,70` re-implements the joins by hand because the real functions can't take a fake home — even though `onboarding.rs:40-56` (`build_onboarding_plan` → `build_onboarding_plan_in_home(home)`) is the exact precedent for the fix.
2. **`tauri::AppHandle<R>` threaded through 10 installer interfaces** (`core/installer.rs` — 15 `AppHandle` mentions in core) to reach two path lookups: `app_cache_dir()` in `clone_to_cache*` (`installer.rs:1461/1543`) and the rarely-taken `app_data_dir()` fallback in `central_repo.rs:23-27`. Consequence: `core/tests/installer.rs` constructs `tauri::test::mock_app()` 16 times; every signature carries `<R: tauri::Runtime>` noise. `cache_cleanup.rs:56-65` / `temp_cleanup.rs` already show the right shape (thin `AppHandle` adapter → path-parameterised inner function).
3. **`expand_home_path` lives in the wiring tier** (`commands/mod.rs:286-300`) and is injected into core as a closure (`project_ops.rs:82,:251`), so core depends on commands; the hand-written test fake has diverged (`core/tests/project_ops.rs:16` says `"path is empty"`, production says `"storage path is empty"`).

Deepen: one environment seam.

- `tool_adapters`: `skills_dir_in(home, &adapter)`, `detect_dir_in(home, &adapter)`, `is_installed_in(home, &adapter)` (names your call); keep thin `dirs::home_dir()` wrappers for production. Thread `home` through `plan_batch_tool_targets`; delete the hand-rolled joins in `onboarding.rs`.
- Installer: resolve `central_dir` / `cache_dir` once at the command seam (or a small `InstallerPaths` value) and make every installer function take plain paths. `AppHandle` disappears from `core/installer.rs` and `core/central_repo.rs` signatures (a single thin production adapter per command is fine).
- Move `expand_home_path` into core; delete the injected closure parameters and the divergent test fake.
- Add the tests this unlocks: `plan_batch_tool_targets` against a temp home; installedness/path resolution table test in `core/tests/tool_adapters.rs`. Existing tests keep passing minus the 16 `mock_app()` constructions.

## Acceptance criteria

- [ ] Zero `dirs::home_dir()` calls in core outside the thin production wrappers; zero `AppHandle` in `core/installer.rs` and `core/central_repo.rs`; zero `mock_app()` in `core/tests/installer.rs`.
- [ ] `expand_home_path` lives in core; no closure injection from commands; the divergent fake is deleted.
- [ ] `plan_batch_tool_targets` and tool-adapter path/installedness resolution have temp-home tests.
- [ ] No behaviour change for the running app (same paths resolved in production).
- [ ] `npm run version:check && npm run check` green.

## Answer

Landed green in `8550081` (Fable 5.1 child, medium thinking; rebased over ticket 19 with two trivial conflicts resolved by the orchestrator — installer import line, adjacent test additions; 248 cargo tests + full gate green on main).

- New `core/environment.rs`: `home_dir()` is the only `dirs::home_dir()` call left in core; `expand_home_path_in(home, input)` (pure) + thin `expand_home_path`. Moved out of `commands/mod.rs`; the divergent test fake and the two `commands/tests` cases are gone (replaced by a table test in `core/tests/environment.rs`).
- `tool_adapters`: `skills_dir_in` / `detect_dir_in` / `is_installed_in(home, &adapter)` **replace** `resolve_default_path` / `resolve_detect_path` / `is_tool_installed` (old wrappers deleted rather than kept — zero callers remained, and keeping them would invite the pattern back).
- `central_repo::resolve_central_repo_path(store, fallback_root)` — `AppHandle`-free; the home→`app_data_dir` fallback lives in the one production adapter `commands::resolve_central_repo_path_for_app` (also used by `lib.rs` startup cleanup). Resolution order unchanged.
- Installer: `InstallerPaths { home, central_dir, cache_dir }` built once per command by `commands::installer_paths`; all 7 public entry points take `&InstallerPaths`; `clone_to_cache*` / `fetch_skill_files` take `cache_dir`. `home` is needed for installedness checks and `~/.agents/.skill-lock.json`; the unused `try_enrich_from_skill_lock` wrapper deleted. Zero `AppHandle` in `installer.rs` / `central_repo.rs`; zero `mock_app()` in installer tests (they now use an isolated temp git cache instead of the real app cache dir).
- `global_sync::{plan_batch_tool_targets, sync_skills_to_tools, unsync_skill_from_tool_with_records}(home, …)`; `onboarding::build_onboarding_plan(home, central_dir, store)` uses the real adapter functions; `project_ops::{register_project_path, update_project_path}(store, home, …)` — closure injection gone.
- Tests added: temp-home `plan_batch_tool_targets` / `sync_skills_to_tools` / unsync (incl. shared-dir group, unknown tool, not-installed skip); adapter path/installedness table; `expand_home_path_in` table; central-repo default root; `~/proj` expansion on register.
- AGENTS.md: new "Core never reads the environment" invariant (thin-adapter exceptions: `cache_cleanup`, `temp_cleanup`, `skill_store::default_db_path`).
- Behaviour notes: if `dirs::home_dir()` is `None`, install commands now fail early ("failed to resolve home directory") instead of silently proceeding with installedness=false — unreachable on supported OSes. `get_project_tool_status` no longer has a swallowed-error branch for the AgentsStandard group (the new call is infallible).

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
