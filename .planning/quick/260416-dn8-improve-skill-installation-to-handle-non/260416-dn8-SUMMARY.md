# Quick Task 260416-dn8: Improve Skill Installation - Summary

**Completed:** 2026-04-16
**Commits:** e3060d8, 7eb1086, 530e97e

## What Changed

### Deep Skill Discovery (installer.rs)

Added three new functions to `installer.rs`:

1. **`find_skill_dirs_recursive(dir, depth, max_depth)`** - Recursively walks directories up to depth 5 looking for SKILL.md files. Skips `node_modules`, `.git`, `dist`, `build`, `target`, `.next`, `.cache`, and hidden directories. This is the key fix for repos like `wshobson/agents` that nest skills at depth 4 (`plugins/*/skills/*/SKILL.md`).

2. **`parse_marketplace_json(repo_dir)`** - Reads `.claude-plugin/marketplace.json` to extract plugin source directories. This is how `wshobson/agents` declares its 75+ plugins.

3. **`scan_marketplace_skills(repo_dir)`** - Combines marketplace parsing with skill scanning inside each plugin's `skills/` subdirectory.

These are integrated into all four discovery functions:

- `list_git_skills` - Primary discovery for frontend picker (marketplace first, recursive fallback)
- `count_skills_in_repo` - Multi-skill repo detection
- `scan_skill_candidates_in_dir` - Update flow name-matching
- `list_local_skills` - Local path discovery

**Discovery strategy:** Marketplace dirs are scanned first (fast, exact paths). Recursive scan runs only as fallback when standard paths + marketplace find nothing. Existing shallow repos work identically (no performance impact).

### API Download SHA Fix (github_download.rs)

Added `fetch_branch_sha(owner, repo, branch, token)` that makes a lightweight GitHub API call to get the real commit SHA. Used in the API download path of `install_git_skill` so `source_revision` stores a real SHA instead of `"api-download-{branch}"`. Falls back gracefully to the old format on failure.

### Tests (tests/installer.rs)

11 new tests covering:

- Deep nesting discovery (depth 4)
- Max depth enforcement
- Skip directory exclusions
- Marketplace JSON parsing (valid, malformed, missing)
- Integration with list_git_skills
- count_skills_in_repo deep counting
- scan_skill_candidates_in_dir deep scanning
- list_local_skills deep discovery
- Backward compatibility with existing shallow repos
- fetch_branch_sha extraction and error handling

## Files Modified

| File                                    | Changes                                                    |
| --------------------------------------- | ---------------------------------------------------------- |
| `src-tauri/src/core/installer.rs`       | +303 lines: 3 new functions, 4 updated discovery functions |
| `src-tauri/src/core/github_download.rs` | +92 lines: fetch_branch_sha function                       |
| `src-tauri/src/core/tests/installer.rs` | +305 lines: 11 new tests                                   |

## Verification

- `npm run check` passes (lint + build + fmt + clippy + test)
- 153 Rust tests pass, 0 failures
- All 11 new tests pass
- All existing tests pass unchanged (backward compatible)

## What's NOT Included (Future Work)

- **Multi-skill picker UI**: When 149 skills are discovered, the frontend needs a selection dialog. The backend returns candidates correctly; the frontend picker is a separate task.
- **Update checking UI**: Schema columns exist (`source_revision`, `content_hash`), but no UI for checking/applying updates yet.
- **skills.sh registry enrichment**: Could cross-reference discovered skills with registry data for install counts, but not needed for core install flow.
