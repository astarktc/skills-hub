---
phase: quick-260416-nwg
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/core/installer.rs
  - src-tauri/src/core/git_fetcher.rs
  - src-tauri/src/core/tool_adapters/mod.rs
  - src-tauri/src/core/project_sync.rs
  - src-tauri/src/commands/mod.rs
  - src-tauri/src/core/tests/git_fetcher.rs
  - src-tauri/src/core/tests/installer.rs
  - src-tauri/src/core/github_download.rs
  - src/App.css
  - src/components/skills/SkillDetailView.tsx
  - src-tauri/tauri.conf.json
  - featured-skills.json
  - README.md
  - README.zh.md
  - docs/
autonomous: true
requirements: []
must_haves:
  truths:
    - "Copaw tool adapter is registered and detectable"
    - "Sparse git checkout works for subpath skill install"
    - "Container path discovery finds skills inside nested dirs"
    - "Case-insensitive SKILL.md detection is preserved from our branch"
    - "Hermes Agent adapter is registered"
    - "overwriteIfSameContent skips re-confirmation for identical content"
    - "project_sync uses project-relative skill dirs instead of global dirs"
    - "All existing Rust tests pass"
    - "npm run check passes (lint + build + rust checks)"
  artifacts:
    - path: src-tauri/src/core/git_fetcher.rs
      provides: "clone_or_pull_sparse function for sparse git checkout"
    - path: src-tauri/src/core/installer.rs
      provides: "Merged upstream sparse checkout + container discovery with our case-insensitive detection and marketplace scanning"
    - path: src-tauri/src/core/tool_adapters/mod.rs
      provides: "Copaw + Hermes adapters and project_relative_skills_dir function"
    - path: src-tauri/src/core/project_sync.rs
      provides: "Uses project_relative_skills_dir instead of adapter.relative_skills_dir"
    - path: src-tauri/src/commands/mod.rs
      provides: "overwriteIfSameContent param on sync_skill_to_tool"
  key_links:
    - from: src-tauri/src/core/installer.rs
      to: src-tauri/src/core/git_fetcher.rs
      via: "clone_or_pull_sparse import"
      pattern: "clone_or_pull_sparse"
    - from: src-tauri/src/core/project_sync.rs
      to: src-tauri/src/core/tool_adapters/mod.rs
      via: "project_relative_skills_dir function call"
      pattern: "project_relative_skills_dir"
    - from: src-tauri/src/commands/mod.rs
      to: src-tauri/src/core/content_hash.rs
      via: "hash_dir import for overwriteIfSameContent"
      pattern: "hash_dir"
---

<objective>
Cherry-pick 9 clean upstream commits from qufei1993/skills-hub and manually port 4 isolated improvements onto our branch.

Purpose: Integrate upstream v0.5.0 features (Copaw adapter, sparse git checkout, container path discovery, docs) while adding Hermes adapter, window size bump, overwriteIfSameContent, and project_relative_skills_dir mapping. Preserves our branch's unique improvements (case-insensitive SKILL.md, skill-lock enrichment, multi-skill detection).

Output: Fully merged codebase with upstream improvements and our local enhancements, passing all checks.
</objective>

<execution_context>
@/home/alexwsl/skills-hub/.claude/get-shit-done/workflows/execute-plan.md
@/home/alexwsl/skills-hub/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/quick/260416-nwg-cherry-pick-clean-upstream-commits-and-m/260416-nwg-CONTEXT.md
@.planning/quick/260416-nwg-cherry-pick-clean-upstream-commits-and-m/260416-nwg-RESEARCH.md
</context>

<tasks>

<task type="auto">
  <name>Task 1: Cherry-pick 9 upstream commits with conflict resolution</name>
  <files>
    src-tauri/src/core/installer.rs
    src-tauri/src/core/git_fetcher.rs
    src-tauri/src/core/tests/git_fetcher.rs
    src-tauri/src/core/tests/installer.rs
    src-tauri/src/core/github_download.rs
    src/App.css
    src/components/skills/SkillDetailView.tsx
    src-tauri/tauri.conf.json
    featured-skills.json
    README.md
    README.zh.md
    docs/
  </files>
  <action>
Cherry-pick upstream commits in this exact order. Fetch upstream first: `git fetch upstream`.

**Phase A -- 5 clean commits (should apply without conflicts):**

```
git cherry-pick e58cb56   # feat: support copaw
git cherry-pick 99c9b9e   # docs: add Copaw to README
git cherry-pick c882fa0   # docs: add Copaw to README.zh.md
git cherry-pick 6da08dc   # docs: v0.4.3 release notes
git cherry-pick 97489f7   # docs: contributor PR link
```

If any of these fail, investigate -- they should be clean. Commit each one individually (standard cherry-pick behavior).

**Phase B -- Commit 6e8e733 (sparse git checkout, HEAVY CONFLICTS in installer.rs):**

```
git cherry-pick --no-commit 6e8e733
```

Resolve conflicts in `installer.rs` with "theirs-first, ours-on-top" strategy:

- ACCEPT all upstream changes: new `clone_or_pull_sparse` import, removal of `fetch_branch_sha` import, rewritten `install_git_skill` fast path (sparse checkout first, API fallback), new `clone_to_cache_subpath` function, `repo_cache_key` signature change to `(clone_url, branch, subpath)`, updated `clone_to_cache` call with `None` third arg, rewritten `parse_skill_md_with_reason` (index-based line processing for YAML block scalars), new `clean_frontmatter_value` function
- PRESERVE our unique code: `use super::skill_lock::try_enrich_from_skill_lock` import, skill-lock enrichment in `install_local_skill` (lines ~66-80), `install_local_skill_from_selection`, `list_local_skills`, case-insensitive `has_skill_md` and `find_skill_md` functions
- Other files in this commit (git_fetcher.rs, tests/git_fetcher.rs, App.css, SkillDetailView.tsx) should apply cleanly
- For tests/installer.rs: accept upstream changes but keep any of our tests that don't overlap

After resolving all conflicts:

```
git add -A
git commit -m "fix: git skill install and frontmatter rendering (cherry-pick 6e8e733)"
```

**Phase C -- Commit 1826e2a (container path discovery, CONFLICTS in installer.rs + App.tsx):**

```
git cherry-pick --no-commit 1826e2a
```

Resolve conflicts:

- `installer.rs`: Accept upstream's new functions: `normalize_github_skill_subpath`, `ensure_installable_skill_dir`, `is_hidden_dir_name`, `is_known_root_scan_dir`, `is_skill_container_dir_name`, `collect_skill_dirs`. Accept `list_git_skills` simplification using `collect_skill_dirs`. BUT after accepting `collect_skill_dirs`, add back marketplace scanning (our `parse_marketplace_json`, `scan_marketplace_skills`) and recursive fallback scanning into or alongside `collect_skill_dirs`. Keep our case-insensitive `has_skill_md` in `is_skill_dir` instead of upstream's `p.join("SKILL.md").exists()`.
- `App.tsx`: Run `git checkout --ours src/App.tsx` -- our branch already has the candidate-based flow from quick task 260416-hw6
- `github_download.rs`: Should apply cleanly (adds `subpath == "."` guard)
- `tests/installer.rs`: Accept upstream changes, merge with any of our unique tests

After resolving:

```
git add -A
git commit -m "fix: git skill discovery for container paths (cherry-pick 1826e2a)"
```

**Phase D -- Last clean commit:**

```
git cherry-pick effe079   # chore: update featured-skills.json
```

Should apply cleanly (our featured-skills.json has zero local modifications).

**Post-cherry-pick verification:**

- `cargo test --manifest-path src-tauri/Cargo.toml` -- all Rust tests pass
- Verify `has_skill_md` still uses `eq_ignore_ascii_case` (case-insensitive)
- Verify `install_local_skill` still has skill-lock enrichment
- Verify no remaining `fetch_branch_sha` references in code (should be fully removed)
  </action>
  <verify>
  <automated>cd /home/alexwsl/skills-hub && cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20</automated>
  </verify>
  <done>
  All 9 upstream commits cherry-picked. Conflict resolution complete: installer.rs has upstream's sparse checkout + container discovery merged with our case-insensitive detection and marketplace scanning. App.tsx kept as ours. All Rust tests pass.
  </done>
  </task>

<task type="auto">
  <name>Task 2: Manual port -- Hermes adapter, window size, overwriteIfSameContent, project_relative_skills_dir</name>
  <files>
    src-tauri/src/core/tool_adapters/mod.rs
    src-tauri/tauri.conf.json
    src-tauri/src/commands/mod.rs
    src-tauri/src/core/project_sync.rs
  </files>
  <action>
Four isolated manual ports. Apply all, then commit together.

**Port 1: Hermes Agent adapter (tool_adapters/mod.rs)**

Add a new `ToolId::HermesAgent` variant to the ToolId enum. Add a new ToolAdapter entry for Hermes Agent in the `ALL_ADAPTERS` array:

- name: "Hermes Agent"
- tool_id: ToolId::HermesAgent
- relative_skills_dir: ".hermes/skills" (verify against upstream commit 827b878)
- detection: check for `.hermes/` directory in home

**Port 2: Window size bump (tauri.conf.json)**

Change window width from current value to 960 and height to 680.

**Port 3: overwriteIfSameContent (commands/mod.rs)**

In the `sync_skill_to_tool` command function (around line 484):

1. Add parameter `overwriteIfSameContent: Option<bool>` after the existing `overwrite: Option<bool>` parameter
2. Add import: `use crate::core::content_hash::hash_dir;`
3. Add helper function `target_has_same_content(source: &Path, target: &Path) -> bool` that compares `hash_dir(source)` and `hash_dir(target)` -- returns true only if both succeed and hashes match
4. Update the overwrite calculation: `let do_overwrite = overwrite.unwrap_or(false) || (overwriteIfSameContent.unwrap_or(false) && target_has_same_content(&source_path, &target_path));`
5. In the frontend App.tsx, find all `invoke('sync_skill_to_tool', ...)` call sites and add `overwriteIfSameContent: true` to each invocation

**Port 4: project_relative_skills_dir (tool_adapters/mod.rs + project_sync.rs)**

In `tool_adapters/mod.rs`, add a public function `project_relative_skills_dir(adapter: &ToolAdapter) -> &'static str` that maps each ToolId to the correct project-local skills directory. Use upstream commit 00c41cc/827b878 as reference. Key mappings where project dir differs from global dir:

- Cursor: global `.cursor/rules` -> project `.cursor/rules` (same, but verify)
- Windsurf: global `.codeium/windsurf/skills` -> project `.windsurf/skills`
- Cline/Roo/etc: global dirs may differ from project dirs
- HermesAgent: return `.hermes/skills` (global-only, but still define it)
- For tools where global == project, return the same `adapter.relative_skills_dir`

Also add `supports_project_scope(adapter: &ToolAdapter) -> bool` that returns false for HermesAgent (global-only).

In `project_sync.rs`, update these 4 call sites to use `project_relative_skills_dir(&adapter)` instead of `adapter.relative_skills_dir`:

- `assign_and_sync` (around line 57)
- `sync_single_assignment` (around line 129)
- `list_assignments_with_staleness` (around line 250)
- `unassign_and_cleanup` (around line 358)

Also update `resolve_project_sync_target` if it takes `relative_skills_dir` as a param -- pass `project_relative_skills_dir` at each call site instead.

Add the import at the top of `project_sync.rs`: `use super::tool_adapters::project_relative_skills_dir;`

Commit all 4 ports together:

```
git commit -m "feat: add Hermes adapter, window size, overwriteIfSameContent, project-relative skill dirs"
```

  </action>
  <verify>
    <automated>cd /home/alexwsl/skills-hub && cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20 && cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings 2>&1 | tail -10</automated>
  </verify>
  <done>
Hermes Agent adapter registered. Window size is 960x680. sync_skill_to_tool accepts overwriteIfSameContent param and frontend passes it. project_sync.rs uses project_relative_skills_dir for all tool path resolution. Cargo test and clippy pass.
  </done>
</task>

<task type="auto">
  <name>Task 3: Full verification and final commit</name>
  <files></files>
  <action>
Run the full project check suite to verify everything integrates cleanly:

```
npm run check
```

This runs: lint + build + rust:fmt:check + rust:clippy + rust:test

If any failures:

- Lint errors: fix TypeScript issues (likely from new params in invoke calls)
- Build errors: fix type mismatches in frontend DTOs if overwriteIfSameContent needs a type update
- Rust fmt: run `cargo fmt --manifest-path src-tauri/Cargo.toml` to fix formatting
- Clippy: address any warnings as errors
- Test failures: investigate and fix

After all checks pass, verify key behaviors manually via grep:

1. `grep -n "has_skill_md\|eq_ignore_ascii_case" src-tauri/src/core/installer.rs` -- case-insensitive detection preserved
2. `grep -n "try_enrich_from_skill_lock" src-tauri/src/core/installer.rs` -- skill-lock enrichment preserved
3. `grep -n "clone_or_pull_sparse" src-tauri/src/core/installer.rs` -- sparse checkout integrated
4. `grep -n "ensure_installable_skill_dir" src-tauri/src/core/installer.rs` -- container validation integrated
5. `grep -n "project_relative_skills_dir" src-tauri/src/core/project_sync.rs` -- project dirs used
6. `grep -n "HermesAgent\|Hermes" src-tauri/src/core/tool_adapters/mod.rs` -- Hermes adapter present
7. `grep -n "overwriteIfSameContent" src-tauri/src/commands/mod.rs` -- new param present

If `npm run check` passes and all greps confirm the expected integrations, no additional commit needed. If fixes were required, commit them:

```
git commit -m "fix: address lint/build/clippy issues from upstream merge"
```

  </action>
  <verify>
    <automated>cd /home/alexwsl/skills-hub && npm run check 2>&1 | tail -30</automated>
  </verify>
  <done>
`npm run check` passes clean. All upstream cherry-picks integrated. All manual ports applied. Our unique enhancements (case-insensitive SKILL.md, skill-lock enrichment, multi-skill detection) preserved. No regressions.
  </done>
</task>

</tasks>

<threat_model>

## Trust Boundaries

| Boundary             | Description                                     |
| -------------------- | ----------------------------------------------- |
| upstream git commits | Third-party code being merged into our codebase |

## STRIDE Threat Register

| Threat ID | Category  | Component              | Disposition | Mitigation Plan                                                                                                              |
| --------- | --------- | ---------------------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------- |
| T-nwg-01  | Tampering | Cherry-picked commits  | accept      | Commits are from a known collaborator's fork on GitHub; we review all diffs during conflict resolution                       |
| T-nwg-02  | Elevation | overwriteIfSameContent | mitigate    | Only auto-overwrites when content hash matches exactly (same content = no data loss); requires explicit opt-in via parameter |

</threat_model>

<verification>
1. `npm run check` passes (lint + build + rust:fmt:check + rust:clippy + rust:test)
2. Case-insensitive SKILL.md detection preserved in installer.rs
3. Skill-lock enrichment preserved in install_local_skill
4. Sparse git checkout path present in install_git_skill
5. Container path discovery helpers present
6. Hermes adapter registered in tool_adapters
7. project_relative_skills_dir used in all project_sync.rs call sites
8. overwriteIfSameContent param present in sync_skill_to_tool
9. Window size is 960x680 in tauri.conf.json
</verification>

<success_criteria>
All 9 upstream commits cherry-picked and 4 manual ports applied. `npm run check` passes. No regressions in our unique features (case-insensitive detection, skill-lock enrichment, multi-skill flow). Git log shows clean commit history.
</success_criteria>

<output>
After completion, create `.planning/quick/260416-nwg-cherry-pick-clean-upstream-commits-and-m/260416-nwg-SUMMARY.md`
</output>
