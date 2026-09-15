# 34: Code residue from tickets 19–30

Status: resolved

Type: task
Source: review #3 (verified) — [verdict](../assets/review-3/verdict.md): S5, S6, S9, S12, S13, H5, J1–J8, J10

## What to build

Non-breaking polish, all verified. Group into one or two commits; none changes behaviour except where noted.

### Dead code / dead wire surface
- `SkillStore::set_onboarding_completed` (`core/skill_store.rs:356-362`) — zero production callers; it is also the last raw `set_setting` outside `core/settings.rs` (t24 acceptance criterion). Delete it (and the two tests that only exercise it) or route it through `settings`.
- `commands::install_local` / `commands::install_git` (`lib.rs:37,40`) — registered in `collect_commands!`, generated bindings, zero frontend call sites (both add flows use the `*Selection` commands). Delete both (+ regenerate bindings) unless a use is planned.

### Duplication / cycles
- `cache_cleanup.rs:11-14` `pub use super::settings::git_cache_ttl_secs as get_git_cache_ttl_secs` — stale "parallel change" comment; `installer.rs:7,778,857` should import `settings::git_cache_ttl_secs` directly. Delete the re-export. (3/3 reviewers.)
- Four private `now_ms()` copies (`core/central_repo.rs:53` new, `core/installer.rs:292`, `core/cache_cleanup.rs:95`, `commands/mod.rs:728`) and `install_finalize.rs:22` importing `installer::now_ms` back into the module extracted from it. One `core::clock::now_ms()` (or on `environment.rs`) breaks the cycle.
- `project_ops.rs:139-150, 245-257, 262-274` — three copies of "adapter_by_key → resolve_project_sync_target → if exists/symlink → remove_path_any + warn". Extract one helper.
- `tool_adapters/mod.rs` `adapter_by_key` / `adapters_sharing_skills_dir` still `.cloned()` out of the `static` registry; return `&'static ToolAdapter`.

### Interface honesty
- `commands/projects.rs` `update_project_gitignore(projectId, addToGitignore, addToExclude)` positional bools vs `configure_project_tools(…, gitignore: IgnoreUpdateOptions | null)` DTO — same clump, two shapes, converted camel→snake by hand at `ProjectsPage.tsx:26-29, 91-94` and unpacked again in `useProjectState.updateGitignore`. Take the DTO in both commands.
- `tool_adapters/catalog.rs:78 vs :82` — the project entry's `skills_dir` is the **global** dir while `shared_with` groups by the **project** dir; nothing in the frontend reads `skills_dir`. Either populate the project-relative dir or drop the field from the project DTO.
- `installer.rs:63-69, 88-96` passes a folder-derived name as `NameIntent::UserProvided`, sidestepping the SKILL.md preference the enum documents (behaviour deliberately preserved from base). Either add a third case (`FolderDerived` = "keep as-is") or document on the enum why local installs never rename.
- `sync_status.rs:74 / :181` `has_deployed_artifact` (Synced|Stale|Error) vs `has_deployed_target` (Synced|Stale|Missing) in separate `impl` blocks; the three cleanup gates (`project_ops.rs:137-147` orphan branch: none; removal: artifact; reconcile: target) differ with no comment. Co-locate and document why.
- `commands/mod.rs:194-206` `get_settings` calls `ensure_central_repo` (creates a dir) — a getter side effect now hit on mount by two hooks (`useSettingsState.ts:120`, `useSyncOrchestration.ts:132`). Same shape t23 removed from `get_tool_status`; make it an explicit step or move it to app setup.

### Small correctness guards
- `skill_matching.rs:80-83` — `target.contains("")` is true, so an empty candidate name is a tier-2 hit for every target. Guard `value.is_empty()` and add the blank-candidate row to the table test.
- `install_finalize.rs:111-124` `StagingDir::move_into` copy fallback: on mid-copy failure `Drop` removes staging but leaves the partial `dest` inside the central repo. Remove `dest` on copy failure before propagating.

### Frontend
- `useSettingsState.ts:63-70` `adoptSettings` overwrites all six fields on every single-setting write while the zoom hotkey (`:134`) writes without adopting — a stale snapshot can clobber a newer local value. Adopt only the written field's echo, or serialise writes.
- `useSettingsState.ts:55,57,58` retypes backend defaults (`1.0`, `30`, `60`) — put defaults on the `AppSettings` snapshot next to `bounds`, or accept and comment.
- `useCandidatePick.ts:181` — `install` is the only returned member not `useCallback`-memoised.
- Six direct `toast.*` calls bypass the reporter (`useSkillLibrary.ts:174`, `useExploreState.ts:83,100,128`, `useSyncOrchestration.ts:247,283`) — route through `setError`/`setSuccessToastMessage` or document why these are exempt.

### Pins / seams
- `src-tauri/Cargo.toml:41` `specta-typescript = "0.0.12"` → `"=0.0.12"` to match the AGENTS.md pin policy (semantically already exact for 0.0.x).
- `core/skill_store.rs:1165` `migrate_legacy_db_if_needed` calls `dirs::data_dir()` — pre-existing, but not among AGENTS.md's listed thin-adapter exceptions. Either pass the legacy root from `lib.rs` or add it to the exception list (ticket 35 covers the doc side).

## Acceptance criteria

- [ ] Each bullet either done or explicitly declined in the Answer with a reason.
- [ ] Zero `get_setting`/`set_setting` calls outside `core/settings.rs` (excluding its own tests).
- [ ] One `now_ms` in production code; `install_finalize.rs` does not import from `installer.rs`.
- [ ] Bindings regenerated if any command/DTO changed; `npm run version:check && npm run check` green.

## Answer

Branch `t34` (worktree `~/.worktrees/skills-hub-t34`), 4 commits:
`368f48d`, `36992b4`, `b3f1d71`, `e8f60d2`.

### Per-bullet outcome

| # | Bullet | Outcome |
|---|--------|---------|
| 1 | `SkillStore::set_onboarding_completed` | **done** — deleted with the two assertions that only exercised it; `get_setting`/`set_setting` narrowed to `pub(super)` so "only `core::settings` calls the raw adapter" is compiler-enforced, not documented |
| 2 | `install_local` / `install_git` commands | **done** — both deleted, bindings regenerated. Cascade: `installer::install_git_skill` became unreachable, so it went too, along with the now write-only `SkillFetchResult` (`fetch_skill_files` returns `Option<String>`). Its two MULTI_SKILLS tests were retargeted at the still-live shared fetch engine (`clone_for_explore_preview`); its SKILL.md-name test was dropped as duplicative of `install_git_skill_uses_skill_md_name_over_subpath_skills`, which covers the same rule on a reachable path |
| 3 | `cache_cleanup` `get_git_cache_ttl_secs` re-export | **done** — deleted; `installer.rs` imports `settings::git_cache_ttl_secs` directly |
| 4 | 4× `now_ms` + `install_finalize`↔`installer` cycle | **done** — one `core::clock::now_ms`; `install_finalize.rs` no longer names `installer` at all |
| 5 | 3× cleanup-walker shape in `project_ops.rs` | **done** — one `remove_project_artifact(project_path, tool, skill_name) -> bool`; it returns whether it removed anything so the "orphan may remain" warning keeps its single call site. Presence is now uniformly decided by `symlink_metadata` (broken symlinks are the usual orphan) |
| 6 | `.cloned()` out of the `static` registry | **done** — `adapter_by_key` / `adapters_sharing_skills_dir` return `&'static ToolAdapter`; `PlannedToolTarget.adapter` and `record_tools` borrow too |
| 7 | gitignore bool clump → DTO in both commands | **done** — `update_project_gitignore(projectId, gitignore: IgnoreUpdateOptions)`. Fixed end to end: both project modals emit the DTO, `ProjectsPage` no longer converts camel→snake by hand, `useProjectState.updateGitignore` passes it straight through |
| 8 | catalog project entry `skills_dir` | **done, by dropping the field** — it is absolute-under-home at global scope but project-relative at project scope, so one field cannot carry both honestly; the project entry was shipping the *global* dir while `shared_with` grouped by the project dir, and no consumer read it. Registry-level dir resolution keeps its own assertions in `tool_catalog.rs` |
| 9 | `NameIntent` folder-derived case | **done, by documenting** (third variant *declined*) — the variants name the policy ("honor as-is" vs "prefer SKILL.md"), not the provenance, so a folder name is legitimately `UserProvided`: for a directory on disk the folder name *is* the skill's identity. A `FolderDerived` variant behaving identically to `UserProvided` would widen the enum without changing behaviour |
| 10 | `has_deployed_artifact` / `has_deployed_target` | **done** — co-located in one `impl`, with a table of the two statuses where they differ (`Missing`, `Error`) and why, plus why the orphan branch needs neither gate |
| 11 | `get_settings` side effect | **done** — `ensure_central_repo` moved to app setup in `lib.rs`; the getter only reads |
| 12 | empty candidate name in `skill_matching.rs` | **done** — blank candidate names are excluded from containment; table test extended with blank/whitespace-named candidates (real match still wins, dir name still matchable) |
| 13 | partial `dest` on copy-fallback failure | **done** — `StagingDir::move_into` removes `dest` before propagating. Verified the test catches it: without the fix the assertion reports 3 leftover entries |
| 14 | `adoptSettings` overwrite race | **done** — one `writeSetting` seam adopts only the written field's echo (plus bounds); the zoom hotkey uses it instead of writing without adopting. Regression test drives two overlapping writes and fails on the old code |
| 15 | retyped backend defaults `1.0` / `30` / `60` | **option A declined, option B done** — shipping backend `defaults` over the wire would not remove the need for a pre-load value (the snapshot is async), so it would add a wire field the frontend still had to shadow: exactly the residue this ticket removes. Instead they are one named `PRE_LOAD_PLACEHOLDERS` const documenting that they are single-frame placeholders, never a second source of truth and never part of a write |
| 16 | `useCandidatePick.install` not memoised | **done** — `useCallback`, same as every other returned member |
| 17 | six direct `toast.*` calls | **done** — all six routed through `setError` / `setSuccessToastMessage`; `sonner` is no longer imported by any world hook (`useUpdateChecker` and the components were not in scope) |
| 18 | `specta-typescript = "=0.0.12"` | **done** |
| 19 | `dirs::data_dir()` in `migrate_legacy_db_if_needed` | **done** — takes the legacy data root, resolved from `app.path().data_dir()` at the wiring tier. Core reads no environment, and the adoption rule became testable (3 new tests: adopt, skip-when-target-populated, no-op) |

### Tests

- Rust: **317 → 320** (`cargo test --all`). +1 partial-dest guard, +3 legacy-DB migration, −1 test of the deleted whole-repo git install flow; the `skill_matching` table test and `tool_catalog` assertions grew in place.
- Frontend: **89 → 90** (`npm run test`), plus the reshaped `useProjectState` gitignore-DTO assertion.
- Two new guards were verified by temporarily reverting the fix and confirming the test fails.

### Gate

`npm run version:check` → OK (1.1.9). `npm run check` → exit 0 (lint + vitest + build + rustfmt + clippy + cargo test). `cd src-tauri && cargo test --all` → 320 passed. `git status` clean; `src/bindings/index.ts` regenerated and committed.

### Note for the merge

The branch is based on `06533fe`; `main` has since advanced to `3add0c1` (t32). Rebase before merging. Touched shared files: `commands/mod.rs`, `core/installer.rs`, `core/tool_adapters/*`, `src/hooks/*`.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
