# Quick Task 260416-nwg: Cherry-pick Upstream Commits - Research

**Researched:** 2026-04-16
**Domain:** Git cherry-pick conflict analysis, Rust codebase integration
**Confidence:** HIGH

## Summary

Cherry-picking 9 upstream commits will produce conflicts in exactly two files: `installer.rs` (commits 6e8e733 and 1826e2a) and `App.tsx` (commit 1826e2a). The remaining 7 commits (docs, Copaw adapter, featured-skills.json) should apply cleanly. The manual ports (Hermes, window size, overwriteIfSameContent, project_relative_skills_dir) are isolated and can be done after conflicts are resolved.

**Primary recommendation:** Cherry-pick the 5 clean doc/adapter commits first, then cherry-pick 6e8e733 and 1826e2a with `--no-commit` to resolve conflicts manually, cherry-pick effe079, and finally apply the 4 manual ports.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

- Theirs-first, ours-on-top conflict resolution for installer.rs
- Cherry-pick order per CONTEXT.md specifics section
- Manual port scope: Hermes, window size, overwriteIfSameContent, project_relative_skills_dir

### Claude's Discretion

- featured-skills.json: take latest only (effe079), not intermediate bot commits

### Deferred Ideas (OUT OF SCOPE)

- macOS close-to-hide behavior (eba7809)
  </user_constraints>

## 1. Conflict Analysis: installer.rs

### What 6e8e733 Changes (sparse git checkout)

- **imports:** Adds `clone_or_pull_sparse` import from `git_fetcher` [VERIFIED: git show]
- **imports:** Removes `fetch_branch_sha` import (no longer needed since sparse checkout replaces API-first path) [VERIFIED: git show]
- **install_git_skill fast path (lines ~113-260):** Completely restructures the subpath install flow. Instead of GitHub API download with git clone fallback, it now does sparse git checkout first, then GitHub API as fallback. This is the **heaviest conflict zone** -- the entire if-else block is rewritten. [VERIFIED: git show]
- **update_managed_skill_from_source (~line 640):** Adds subpath-aware caching -- if record has `source_subpath`, uses `clone_to_cache_subpath` instead of `clone_to_cache`. [VERIFIED: git show]
- **repo_cache_key:** Signature changes from `(clone_url, branch)` to `(clone_url, branch, subpath)`. [VERIFIED: git show]
- **clone_to_cache:** Updates call to `repo_cache_key` to pass `None` as third arg. [VERIFIED: git show]
- **NEW clone_to_cache_subpath function (~80 lines):** Entirely new function for sparse checkout caching. [VERIFIED: git show]
- **parse_skill_md_with_reason:** Rewritten to support YAML block scalars (`|` and `>`) in frontmatter, changes from iterator-based to index-based line processing. [VERIFIED: git show]
- **NEW clean_frontmatter_value function:** Strips quotes from frontmatter values. [VERIFIED: git show]

### What 1826e2a Changes (container path discovery)

- **install_git_skill name derivation (~line 95):** Adds `subpath == "."` guard for root-path skills. [VERIFIED: git show]
- **install_git_skill:** Adds `ensure_installable_skill_dir(&sub_src)?` calls at 3 points (sparse checkout result, fallback result, and standard git path). [VERIFIED: git show]
- **parse_github_url:** Calls `normalize_github_skill_subpath()` on extracted subpath. [VERIFIED: git show]
- **NEW normalize_github_skill_subpath function:** Strips `/SKILL.md` from subpaths (so `skills/foo/SKILL.md` becomes `skills/foo`). [VERIFIED: git show]
- **NEW ensure_installable_skill_dir function:** Validates a directory is a skill dir before install. [VERIFIED: git show]
- **NEW is_hidden_dir_name, is_known_root_scan_dir, is_skill_container_dir_name helpers.** [VERIFIED: git show]
- **Refactors count_skills_in_repo + scan_skill_candidates_in_dir** into a shared `collect_skill_dirs()`. Replaces ~100 lines of duplicated scanning with one function. [VERIFIED: git show]
- **list_git_skills:** Major simplification -- replaces manual scan bases + root-level scan + marketplace + recursive scan with `collect_skill_dirs()`. Also adds container-URL handling (when folder URL points to a container dir, scan it for skills). [VERIFIED: git show]
- **is_skill_dir:** Changes from `has_skill_md(p) || is_claude_skill_dir(p)` to `p.join("SKILL.md").exists() || is_claude_skill_dir(p)` (minor: case-sensitive now). [VERIFIED: git show]
- **install_git_skill_from_selection:** Adds `ensure_installable_skill_dir(&copy_src)?` call. [VERIFIED: git show]

### Our Branch's Unique Changes (must preserve)

- **skill_lock import and enrichment in install_local_skill:** Lines 17 (`use super::skill_lock::try_enrich_from_skill_lock`) and lines 66-80 (enrichment logic). These are in `install_local_skill`, which upstream does NOT touch. **No conflict.** [VERIFIED: our installer.rs]
- **Case-insensitive has_skill_md + find_skill_md:** We use `eq_ignore_ascii_case("skill.md")` instead of `p.join("SKILL.md").exists()`. Note: 1826e2a reverts `is_skill_dir` to case-sensitive. We should keep our case-insensitive version. [VERIFIED: our installer.rs]
- **Marketplace scanning (parse_marketplace_json, scan_marketplace_skills):** These functions exist in our code but are removed/simplified by 1826e2a's `collect_skill_dirs`. **Decision needed:** upstream's `collect_skill_dirs` does NOT include marketplace scanning or recursive deep scan. Our version catches more edge cases. [VERIFIED: diff analysis]
- **list_local_skills function (~60 lines):** Our version is richer with validity checking. Upstream 1826e2a does not touch this function. **No conflict.** [VERIFIED: diff analysis]
- **install_local_skill_from_selection:** Our version exists, upstream does not touch it. **No conflict.** [VERIFIED: our installer.rs]
- **SKILL_SCAN_BASES includes 5 entries** including `.claude/skills`. Upstream 1826e2a's `collect_skill_dirs` uses the same bases. **Compatible.** [VERIFIED: both codebases]

### Conflict Resolution Strategy

**For 6e8e733:**

1. Accept upstream's new sparse checkout flow in `install_git_skill` (the entire fast-path rewrite)
2. Accept `clone_or_pull_sparse` import addition
3. Accept removal of `fetch_branch_sha` import -- BUT we must check if we use it elsewhere (we do: line 169 in our current code). Since upstream removes the API-first path that called it, this is safe.
4. Accept new `clone_to_cache_subpath`, `repo_cache_key` signature change, `clean_frontmatter_value`, and rewritten `parse_skill_md_with_reason`
5. Preserve our `skill_lock` import and enrichment (in `install_local_skill`, no overlap)

**For 1826e2a:**

1. Accept `ensure_installable_skill_dir`, `normalize_github_skill_subpath`, container discovery helpers
2. Accept `collect_skill_dirs` refactor BUT merge it with our marketplace/recursive scanning. Upstream's `collect_skill_dirs` only scans: (a) known bases, (b) root-level skills, (c) root-level skill containers. It drops marketplace and deep recursive scan. We should add marketplace + recursive back into `collect_skill_dirs`.
3. Accept list_git_skills simplification but keep our case-insensitive `has_skill_md`
4. Our `is_skill_dir` should keep case-insensitive behavior (upstream reverts to case-sensitive in 1826e2a)
5. Accept `install_git_skill_from_selection` `ensure_installable_skill_dir` addition

### Other Files in These Commits

**6e8e733 also touches:**

- `git_fetcher.rs`: Adds entire `clone_or_pull_sparse` function (~180 lines). Our `git_fetcher.rs` does not have this. **Should apply cleanly.** [VERIFIED: git show]
- `tests/git_fetcher.rs`: New test for sparse checkout. **Should apply cleanly.** [VERIFIED: git show]
- `tests/installer.rs`: Test updates. May conflict with our added tests. [ASSUMED]
- `App.css`: Adds `vertical-align: top`, `overflow-wrap: anywhere`, `white-space: pre-wrap` to markdown table styles. **Should apply cleanly.** [VERIFIED: git show]
- `SkillDetailView.tsx`: Adds YAML block scalar parsing to frontmatter. **Should apply cleanly.** [VERIFIED: git show]

**1826e2a also touches:**

- `github_download.rs`: Adds `subpath == "."` guard in `parse_github_api_params`. **Should apply cleanly.** [VERIFIED: git show]
- `tests/installer.rs`: Test updates. Same conflict risk as above. [ASSUMED]
- `App.tsx`: Changes single `install_git` call to candidate-based flow. **Our code already has this** (from quick task 260416-hw6). Will conflict but we keep ours entirely. [VERIFIED: our App.tsx already has list_git_skills_cmd flow]
- `docs/releases/v0.4.3/bugfix-*.md`: New file, no conflict. [VERIFIED: git show]

## 2. project_relative_skills_dir Integration

### Current Bug

Our `project_sync.rs` uses `adapter.relative_skills_dir` (the global path) in 6 call sites (lines 57, 129, 250, 358 and the `resolve_project_sync_target` function at line 14). [VERIFIED: grep of project_sync.rs]

For ~12 tools, the global path differs from the correct project-local path. Example: Claude Code global is `.claude/skills` (correct for both), but Cursor global is `.cursor/rules` while project should be `.agents/skills`. [VERIFIED: upstream's project_relative_skills_dir function in commit 00c41cc]

### What Upstream Adds (commit 00c41cc, ported via 827b878)

A `project_relative_skills_dir(adapter: &ToolAdapter) -> &'static str` function with a 40-line match that maps each ToolId to its project-local skills dir. Also adds:

- `adapters_sharing_project_skills_dir(adapter)`
- `resolve_project_path(adapter, project_root)`
- `supports_project_scope(adapter)`

### Required Changes in Our Code

1. **Add `project_relative_skills_dir` function** to `tool_adapters/mod.rs` (copy from upstream, ~50 lines including Hermes). [VERIFIED: upstream code in 00c41cc and 827b878]
2. **Add `supports_project_scope`** -- returns false for HermesAgent (global-only tool). [VERIFIED: upstream 827b878]
3. **Add `resolve_project_path`** helper. [VERIFIED: upstream 00c41cc]
4. **Update `project_sync.rs`** -- change all 4 call sites from `adapter.relative_skills_dir` to `project_relative_skills_dir(&adapter)`:
   - Line 57: `assign_and_sync`
   - Line 129: `sync_single_assignment`
   - Line 250: `list_assignments_with_staleness`
   - Line 358: `unassign_and_cleanup`
5. **Update `resolve_project_sync_target` signature** -- change param from `relative_skills_dir: &str` to just take the adapter and compute internally, OR keep the param but pass `project_relative_skills_dir(&adapter)` at each call site. The latter is less invasive.

## 3. overwriteIfSameContent Implementation

### What Upstream Does (commit 8df1106)

Adds an `overwriteIfSameContent: Option<bool>` parameter to `sync_skill_to_tool` command. When true and the target directory exists with the same content hash as the source, it converts the overwrite flag to `true` -- allowing the sync engine to replace the target without user confirmation. [VERIFIED: git show 8df1106]

Key pieces:

- New param `overwriteIfSameContent: Option<bool>` on the Tauri command
- New helper `target_has_same_content(source, target) -> bool` that compares `hash_dir` of both
- Overwrite calculation becomes: `overwrite.unwrap_or(false) || (overwriteIfSameContent.unwrap_or(false) && target_has_same_content(...))`
- Requires `use crate::core::content_hash::hash_dir` import

### Porting to Our Codebase

Our `sync_skill_to_tool` at line 484 of `commands/mod.rs` has the same structure. The port is straightforward:

1. Add the parameter to the function signature (line 490, after `overwrite: Option<bool>`)
2. Add the `hash_dir` import
3. Add `target_has_same_content` helper function
4. Update the overwrite calculation at line 511
5. Frontend call sites that invoke `sync_skill_to_tool` need to pass `overwriteIfSameContent: true` where appropriate

**Note:** Upstream also passes `overwriteIfSameContent: true` from all sync call sites. We should do the same for global sync. For project sync, our `project_sync.rs` calls `sync_dir_for_tool_with_overwrite` directly (not through the command), so project sync is unaffected by this change.

## 4. Cherry-pick Order and Dependencies

### Dependency Chain

```
e58cb56 (Copaw) -- independent
99c9b9e (README) -- depends on e58cb56 for context but no code dep
c882fa0 (README.zh) -- depends on 99c9b9e for context but no code dep
6da08dc (docs v0.4.3) -- independent doc
97489f7 (docs contributor) -- independent doc
6e8e733 (sparse checkout) -- INDEPENDENT, touches installer.rs + git_fetcher.rs
a5b4ffe (docs bugfix notes) -- independent doc
1826e2a (container paths) -- DEPENDS ON 6e8e733 (uses clone_to_cache_subpath, parse changes)
effe079 (featured-skills) -- independent
```

**Critical finding:** 1826e2a is a direct descendant of 6e8e733 via intermediate commits. The diff in 1826e2a's installer.rs is computed against 6e8e733's result. They MUST be applied in order: 6e8e733 first, then 1826e2a. [VERIFIED: git log --parents shows 1826e2a -> a5b4ffe -> fabf493 -> 6e8e733]

However, between 6e8e733 and 1826e2a there is commit `fabf493` (refactor: move project rules to AGENTS.md) which we are NOT cherry-picking. This means 1826e2a's diff is against a state that includes fabf493. Since fabf493 only touches AGENTS.md/CLAUDE.md (no Rust/TS files), the code context lines in 1826e2a's installer.rs diff should still match after applying just 6e8e733. [VERIFIED: fabf493 only modifies docs files]

### Recommended Order

1. e58cb56 -- clean, Copaw adapter only
2. 99c9b9e -- clean, README only
3. c882fa0 -- clean, README.zh only
4. 6da08dc -- clean, docs only
5. 97489f7 -- clean, docs only
6. 6e8e733 -- **CONFLICTS**: installer.rs (heavy), possibly tests/installer.rs
7. a5b4ffe -- clean, docs only
8. 1826e2a -- **CONFLICTS**: installer.rs (medium, builds on 6e8e733), App.tsx (discard theirs, keep ours), github_download.rs (clean)
9. effe079 -- featured-skills.json (clean, our version is unchanged from baseline)

## 5. featured-skills.json

Our `featured-skills.json` has zero local modifications (`git diff HEAD -- featured-skills.json` produces 0 lines). The file exists and matches the merge-base version. Cherry-picking effe079 should apply cleanly with no conflicts. [VERIFIED: git diff]

The commit only updates star counts and the `updated_at` timestamp. Taking it wholesale is correct per CONTEXT.md discretion.

## Common Pitfalls

### Pitfall 1: is_skill_dir Case Sensitivity Regression

**What goes wrong:** 1826e2a changes `is_skill_dir` to `p.join("SKILL.md").exists()` (case-sensitive), overriding our case-insensitive `has_skill_md` approach.
**How to avoid:** After cherry-pick, verify `is_skill_dir` still uses case-insensitive check. Keep our `has_skill_md` function and use it in `is_skill_dir`.

### Pitfall 2: Marketplace/Recursive Scan Loss

**What goes wrong:** 1826e2a's `collect_skill_dirs` replaces our comprehensive scanning (marketplace + recursive up to depth 5) with a simpler 3-tier scan. Some edge-case repos (e.g., wshobson/agents) might not be discovered.
**How to avoid:** After accepting `collect_skill_dirs`, add marketplace scanning and recursive fallback back into it.

### Pitfall 3: App.tsx Double-Apply

**What goes wrong:** 1826e2a's App.tsx change adds the candidate-based flow that our branch already has. Cherry-pick will conflict.
**How to avoid:** Use `git checkout --ours src/App.tsx` during conflict resolution for 1826e2a.

### Pitfall 4: fetch_branch_sha Import Removal

**What goes wrong:** 6e8e733 removes the `fetch_branch_sha` import. Our code still uses it in the API download fast path. After applying 6e8e733, the API download becomes the fallback (not primary), and `fetch_branch_sha` is no longer called in that code path.
**How to avoid:** Verify that after applying 6e8e733, no remaining code references `fetch_branch_sha`. The sparse checkout path replaces it.

## Assumptions Log

| #   | Claim                                                                  | Section   | Risk if Wrong                                                         |
| --- | ---------------------------------------------------------------------- | --------- | --------------------------------------------------------------------- |
| A1  | tests/installer.rs may conflict during cherry-pick                     | Section 1 | Low -- tests can be manually merged                                   |
| A2  | fabf493 (skipped commit between 6e8e733 and 1826e2a) only touches docs | Section 4 | Medium -- if it touches Rust files, 1826e2a context lines won't match |

## Sources

### Primary (HIGH confidence)

- `git show 6e8e733` -- full diff of sparse checkout commit
- `git show 1826e2a` -- full diff of container path commit
- `git show 8df1106` -- full diff of overwriteIfSameContent commit
- `git show 827b878` -- full diff of Hermes adapter commit (includes project_relative_skills_dir update)
- `git show 00c41cc` -- full diff of project_relative_skills_dir introduction
- Current `installer.rs`, `project_sync.rs`, `tool_adapters/mod.rs`, `commands/mod.rs` -- our branch state
- `git log --parents` -- commit ancestry verification

## Metadata

**Confidence breakdown:**

- Conflict analysis: HIGH -- read full diffs of both branches
- project_relative_skills_dir: HIGH -- verified function source and all call sites
- overwriteIfSameContent: HIGH -- verified upstream implementation
- Cherry-pick ordering: HIGH -- verified parent chain
- featured-skills.json: HIGH -- verified zero local changes

**Research date:** 2026-04-16
**Valid until:** 2026-04-23 (upstream may add more commits)
