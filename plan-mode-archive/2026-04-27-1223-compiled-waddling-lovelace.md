# Plan: Restore Lost Worktree Changes + Re-tag v1.1.4

## Context

During parallel worktree development, commit `68d6101` (perf: cache source hashes, from the `r00` worktree) overwrote changes from earlier worktrees when it was merged. A partial recovery was done in `e90d554`, but many changes remain missing. We need to restore everything, add CLAUDE.md worktree guidance, then delete/re-tag v1.1.4.

## Verified Ground Truth (what's actually missing in HEAD)

| # | Feature | Files Affected | Source Commit |
|---|---------|---------------|---------------|
| 1 | Copaw adapter | `tool_adapters/mod.rs` | `7532f21` |
| 2 | Hermes adapter + `project_relative_skills_dir()` + helpers | `tool_adapters/mod.rs` | `071a1cf` |
| 3 | `overwriteIfSameContent` param on sync_skill_to_tool | `commands/mod.rs`, `App.tsx` (8 call sites) | `071a1cf` |
| 4 | `clone_or_pull_sparse()` + sparse git checkout | `git_fetcher.rs` (~175 lines) | `6d72c08` |
| 5 | `normalize_github_skill_subpath()` + `ensure_installable_skill_dir()` | `installer.rs`, `github_download.rs` | `6d72c08` |
| 6 | Container path skill discovery (`collect_skill_dirs` etc) | `installer.rs` | `b6fc41f` |
| 7 | Frontmatter `blockLines` YAML rendering fix | `SkillDetailView.tsx` | `6d72c08` |
| 8 | Path traversal security fix (`canonicalize` + `starts_with`) | `installer.rs` | `88f2aa9` |
| 9 | Project-relative delete path | `commands/mod.rs` | `cd9d962` |
| 10 | `handleSyncSkillToAllTools` + link/unlink toggle | `App.tsx`, `SkillCard.tsx`, `SkillsList.tsx`, `resources.ts` | `be0b104` |
| 11 | Copy-mode project re-sync in `update_managed_skill_from_source` | `installer.rs` | `74add86` |
| 12 | Re-sync test | `tests/installer.rs` | `457c052` |
| 13 | Backend hash caching (`update_skill_content_hash`, skill_cache) | `skill_store.rs`, `project_sync.rs` | `68d6101` |
| 14 | `flex: 1` on `.matrix-toolbar-info` | `App.css` | `f65bf45` |
| 15 | Markdown table CSS | `App.css` | `6d72c08` |
| 16 | Dead code `#[allow]` attrs + test path updates | `tool_adapters/mod.rs`, `tests/*.rs` | `5d77fe9` |
| 17 | groupByRepo localStorage (already fixed in this session) | `App.tsx`, `AssignmentMatrix.tsx` | `b8ece0e`, `66a521e` |
| 18 | Assignment Map O(1) optimization | `AssignmentMatrix.tsx` | `6ec5564` |

## Restoration Strategy

Apply patches via `git show <hash> | git apply` in chronological order. For conflicts, manually apply from commit diffs. Run `npm run check` after each wave.

### Wave 1: Core Rust features (tool adapters, installer, git_fetcher)

**Order matters** — these modify overlapping files.

1. `7532f21` — Copaw adapter (CLEAN) → `tool_adapters/mod.rs`
2. `6d72c08` — git skill install/frontmatter (CONFLICT in SkillDetailView.tsx) → `git_fetcher.rs`, `installer.rs`, `github_download.rs`, `SkillDetailView.tsx`, `App.css`, tests
3. `b6fc41f` — container path discovery (CLEAN) → `installer.rs`, tests
4. `071a1cf` — Hermes, project_relative_skills_dir, overwriteIfSameContent (CLEAN) → `tool_adapters/mod.rs`, `commands/mod.rs`, `project_sync.rs`, `tauri.conf.json`, `App.tsx`
5. `88f2aa9` — path traversal security fix (CLEAN) → `installer.rs`
6. `5d77fe9` — dead code warnings, test paths (CONFLICT) → `tool_adapters/mod.rs`, `installer.rs`, tests
7. `cd9d962` — project-relative delete path (CLEAN) → `commands/mod.rs`
8. `74add86` — copy-mode project re-sync (CONFLICT) → `installer.rs`
9. `457c052` — re-sync test (CLEAN) → `tests/installer.rs`

**Checkpoint:** `cargo clippy`, `cargo test`, `cargo fmt --check`

### Wave 2: Frontend features

10. `6ec5564` — assignment Map O(1) (CLEAN) → `AssignmentMatrix.tsx`
11. `68d6101` — backend hash caching (CONFLICT, many files) → Extract only the NEW additions: `update_skill_content_hash` in `skill_store.rs`, `skill_cache` HashMap in `project_sync.rs`. Skip the reverts that caused the original damage.
12. `be0b104` — link/unlink toggle (CLEAN) → `App.tsx`, `SkillCard.tsx`, `SkillsList.tsx`, `resources.ts`
13. `f65bf45` — flex: 1 toolbar (CLEAN) → `App.css`

**Checkpoint:** `npm run build`, `npm run lint`

### Wave 3: Full verification

- `npm run check` (lint + build + rust:fmt:check + rust:clippy + rust:test)

### Wave 4: CLAUDE.md worktree safety guidance

Add a "## Worktree Safety" section to `CLAUDE.md` with rules to prevent this class of bug from recurring.

### Wave 5: Commit, re-tag, push

1. Commit restorations: `fix: restore changes lost during worktree merges`
2. Commit CLAUDE.md: `docs: add worktree safety guidance to CLAUDE.md`
3. Delete remote+local v1.1.4 tag, re-create, push main + tag

## Key Risk

Commit `68d6101` is the problematic one — it was a performance optimization worktree that also touched 37 files, silently reverting features from other worktrees. When extracting its *additions* (hash caching), we must be careful not to re-apply its *deletions*.

## Verification Checklist

- [ ] `npm run check` passes
- [ ] Copaw and HermesAgent in tool adapter list
- [ ] `project_relative_skills_dir()` exists and called from project_sync.rs
- [ ] `overwriteIfSameContent` param on sync_skill_to_tool
- [ ] Path traversal security: `canonicalize` + `starts_with` in parse_marketplace_json
- [ ] Link/unlink toggle on SkillCard
- [ ] groupByRepo persists across navigation
- [ ] `flex: 1` on `.matrix-toolbar-info`
- [ ] Release workflow triggers correctly with tag `v1.1.4`
