---
phase: quick-260416-nwg
plan: 01
subsystem: core
tags: [cherry-pick, upstream-merge, tool-adapters, sync-engine]
dependency_graph:
  requires: []
  provides:
    - sparse-git-checkout
    - container-path-discovery
    - copaw-adapter
    - hermes-adapter
    - overwrite-if-same-content
    - project-relative-skill-dirs
  affects:
    - installer
    - sync-engine
    - tool-adapters
    - project-sync
    - commands
tech_stack:
  added: []
  patterns:
    - content-hash-comparison-for-overwrite
    - project-relative-tool-path-mapping
key_files:
  created: []
  modified:
    - src-tauri/src/core/installer.rs
    - src-tauri/src/core/git_fetcher.rs
    - src-tauri/src/core/tool_adapters/mod.rs
    - src-tauri/src/core/project_sync.rs
    - src-tauri/src/commands/mod.rs
    - src-tauri/src/core/github_download.rs
    - src-tauri/src/core/tests/installer.rs
    - src-tauri/src/core/tests/git_fetcher.rs
    - src-tauri/src/core/tests/project_sync.rs
    - src-tauri/src/core/tests/project_ops.rs
    - src/App.tsx
    - src/App.css
    - src/components/skills/SkillDetailView.tsx
    - src-tauri/tauri.conf.json
    - featured-skills.json
    - README.md
    - README.zh.md
decisions:
  - Used content hash comparison (hash_dir) for overwriteIfSameContent to avoid false overwrites
  - Applied #[allow(dead_code)] on future-use functions rather than removing them
  - Updated test expectations to use project-relative paths (.agents/skills for cursor)
metrics:
  duration: 9m
  completed: "2026-04-16T23:30:00Z"
  tasks_completed: 3
  tasks_total: 3
---

# Quick Task 260416-nwg: Cherry-pick Clean Upstream Commits and Manual Ports Summary

Integrated 9 upstream commits from qufei1993/skills-hub (sparse git checkout, container path discovery, Copaw adapter, docs) plus 4 manual ports (Hermes adapter, window resize, overwriteIfSameContent, project-relative skill dirs) while preserving all local enhancements.

## Completed Tasks

| #   | Task                           | Commit(s)                                                                       | Key Changes                                                                         |
| --- | ------------------------------ | ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| 1   | Cherry-pick 9 upstream commits | 7532f21, 25deacf, c825f49, 0cb20e5, 0878b2e, 6d72c08, 9ba0a6f, b6fc41f, 3757f63 | Copaw adapter, sparse checkout, container discovery, docs, featured-skills          |
| 2   | Manual ports                   | 071a1cf                                                                         | Hermes adapter, 960x680 window, overwriteIfSameContent, project_relative_skills_dir |
| 3   | Full verification and fixes    | 5d77fe9                                                                         | Dead code suppression, test path updates, cargo fmt                                 |

## What Was Done

### Task 1: Cherry-pick Upstream (9 commits)

- **Phase A (5 clean):** Copaw adapter, README docs (EN + ZH), release notes, contributor links
- **Phase B (heavy conflicts):** Sparse git checkout via `clone_or_pull_sparse` in installer.rs. Resolved installer.rs conflicts preserving our case-insensitive detection, skill-lock enrichment, marketplace scanning, and recursive scanning
- **Phase B.5:** v0.4.3 bugfix docs
- **Phase C (conflicts):** Container path discovery with `collect_skill_dirs`, `ensure_installable_skill_dir`, `normalize_github_skill_subpath`. Preserved App.tsx candidate flow
- **Phase D:** Updated featured-skills.json

### Task 2: Manual Ports (4 changes)

1. **HermesAgent adapter:** New ToolId variant, adapter entry with `.hermes/skills` directory
2. **Window size:** 800x600 to 960x680 in tauri.conf.json
3. **overwriteIfSameContent:** New `Option<bool>` parameter on `sync_skill_to_tool` with `target_has_same_content` helper using `hash_dir` comparison. All 8 frontend call sites pass `overwriteIfSameContent: true`
4. **project_relative_skills_dir:** project_sync.rs updated at all 4 call sites to use project-relative paths instead of global adapter paths

### Task 3: Verification and Fixes

- Fixed 4 dead code warnings: `fetch_branch_sha`, `adapters_sharing_project_skills_dir`, `resolve_project_path`, `supports_project_scope`
- Fixed 5 test path expectations from `.cursor/skills` to `.agents/skills` (cursor project-relative mapping)
- Fixed `cargo fmt` formatting in installer tests
- All 163 lib + 16 integration tests pass
- `npm run check` fully clean (lint + build + fmt + clippy + test)

## Preserved Invariants

All local enhancements confirmed intact via grep:

- Case-insensitive SKILL.md detection (`eq_ignore_ascii_case`)
- Skill-lock enrichment in `install_local_skill` (`try_enrich_from_skill_lock`)
- Marketplace scanning (`parse_marketplace_json`, `scan_marketplace_skills`)
- Recursive scanning (`find_skill_dirs_recursive`)
- Multi-skill candidate flow in App.tsx (`GitSkillCandidate`, `LocalSkillCandidate`)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Dead code compiler errors**

- **Found during:** Task 3
- **Issue:** `fetch_branch_sha` (leftover from upstream removal) and 3 future-use project adapter functions triggered `-D warnings` errors
- **Fix:** Added `#[allow(dead_code)]` annotations to preserve the functions without compilation errors
- **Files modified:** `src-tauri/src/core/github_download.rs`, `src-tauri/src/core/tool_adapters/mod.rs`

**2. [Rule 1 - Bug] Test path expectations incorrect after project_relative_skills_dir change**

- **Found during:** Task 3
- **Issue:** 3 tests (assign_stores_hash_for_copy, bulk_assign_to_multiple_tools, remove_tool_with_cleanup_leaves_other_tools_intact) expected `.cursor/skills` but cursor's project-relative path is now `.agents/skills`
- **Fix:** Updated 5 path expectations across project_sync and project_ops test files
- **Files modified:** `src-tauri/src/core/tests/project_sync.rs`, `src-tauri/src/core/tests/project_ops.rs`

**3. [Rule 3 - Blocking] Rust format check failed**

- **Found during:** Task 3
- **Issue:** `cargo fmt --check` failed on installer tests (long assert lines)
- **Fix:** Ran `cargo fmt` to auto-format
- **Files modified:** `src-tauri/src/core/tests/installer.rs`, `src-tauri/src/core/installer.rs`

## Threat Flags

None -- no new network endpoints or auth paths introduced beyond what upstream already defined.

## Self-Check: PASSED

All 8 key files verified present. All 11 commits verified in git log.
