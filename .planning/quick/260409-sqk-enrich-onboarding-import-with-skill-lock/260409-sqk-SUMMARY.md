# Quick Task 260409-sqk: Summary

## What was done

Enriched the onboarding import flow so skills originally installed via `npx skills add` (which stores canonical copies in `~/.agents/skills/` and symlinks into tool directories) are imported with full git provenance instead of `source_type: "local"`.

## Changes

| File                                     | Change                                                                                                                        |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `src-tauri/src/core/skill_lock.rs`       | New module: parses `~/.agents/.skill-lock.json`, resolves symlinks into `~/.agents/skills/`, returns git provenance metadata  |
| `src-tauri/src/core/mod.rs`              | Registered `skill_lock` module                                                                                                |
| `src-tauri/src/core/installer.rs`        | Wired `try_enrich_from_skill_lock()` into `install_local_skill()` — enriches source_type/ref/subpath when symlink match found |
| `src-tauri/src/core/tests/skill_lock.rs` | 9 unit tests covering parsing, symlink resolution, subpath derivation, and error handling                                     |
| `src-tauri/src/core/tests/installer.rs`  | 1 new test verifying non-symlink paths remain `source_type: "local"`                                                          |

## How it works

1. When `install_local_skill()` is called with a source path, it first calls `try_enrich_from_skill_lock(source_path)`
2. The function checks if the path is a symlink via `read_link()`
3. If the symlink target resolves under `~/.agents/skills/<name>/`, it reads `~/.agents/.skill-lock.json`
4. Looks up `<name>` in the lock file's skills map
5. If found, returns `SkillLockEntry` with `source_url` and derived `source_subpath`
6. The installer uses these to set `source_type: "git"` with full repo URL instead of `"local"`

## Commits

- `ee7740a` feat(quick-260409-sqk): add skill_lock.rs module with lock file parser and enrichment lookup
- `74e6607` feat(quick-260409-sqk): wire skill lock enrichment into install_local_skill

## Stats

- 5 files changed, 399 insertions, 3 deletions
- 10 new tests (9 skill_lock + 1 installer)
