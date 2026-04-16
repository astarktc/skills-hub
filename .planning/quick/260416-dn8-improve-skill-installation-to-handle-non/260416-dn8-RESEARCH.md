# Quick Task 260416-dn8: Improve Skill Installation - Research

**Researched:** 2026-04-16
**Domain:** Skill discovery, registry integration, update tracking
**Confidence:** HIGH

## Summary

The root cause of Skills Hub failing on repos like `wshobson/agents` is a shallow scan depth. The repo has 149 SKILL.md files, ALL at depth 4: `plugins/<plugin>/skills/<skill>/SKILL.md`. Skills Hub only scans depth 1 (`skills/*`) and depth 2 (`skills/.curated/*`), completely missing the nested structure.

The `npx skills` CLI (v1.5.0, vercel-labs/skills) solves this by: (1) scanning plugin manifests (`.claude-plugin/marketplace.json`) for declared skill paths, and (2) falling back to recursive `findSkillDirs()` up to depth 5. Skills Hub needs both strategies.

The skills.sh API provides search but NO repo-level listing endpoint. The search API (`/api/search?q=wshobson&limit=50`) returns skills with their `source` field (e.g., `wshobson/agents`) but maxes out at 50 results and doesn't give subpaths -- so it cannot replace local discovery. Registry-first lookup is useful for validation and enrichment but NOT for complete discovery.

**Primary recommendation:** Add recursive SKILL.md scanning (depth 5) as the primary discovery strategy, with `.claude-plugin/marketplace.json` parsing for plugin-manifest-aware repos. Store source commit SHA at install time for future update checking.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Implementation Decisions

- **Registry-first lookup** via skills.sh API to get repo structure/skill manifest
- **Deep SKILL.md scan as fallback** when repo is not in registry or for direct URL installs
- Research phase will investigate exactly how skills.sh and npx skills CLI handle discovery to align our approach

### Skill Identity Model

- Primary: SKILL.md presence in directory
- Fallback: skills.sh registry authority -- if the registry lists a directory as a skill for that repo, treat it as valid even without SKILL.md
- Registry becomes the authority for repos that don't follow SKILL.md convention

### Update Architecture

- Store repo commit SHA + per-skill content hash at install time
- Update check flow: compare repo SHA first (cheap), then re-hash specific skill subdirectory only if repo changed
- Provides precise per-skill update detection even in mono-repos with 50+ skills
- Schema must include: source_url, source_subpath, installed_commit, content_hash
  </user_constraints>

## Key Finding: Context Decision Adjustment Needed

The CONTEXT.md states "registry-first lookup" but research reveals the skills.sh API **cannot serve as a repo skill listing**:

1. The search API maxes at 50 results per query and uses fuzzy text matching, not exact repo filtering
2. No repo-level API endpoint exists (`/api/repo/`, `/api/repos/`, `/api/skills/` all return 404 HTML pages) [VERIFIED: direct HTTP probing]
3. The `npx skills` CLI itself does NOT use skills.sh for discovery -- it clones the repo and scans locally [VERIFIED: vercel-labs/skills source code]

**Revised recommendation:** Deep SKILL.md scan should be PRIMARY (not fallback). The skills.sh search API remains useful for the Explore page but is not a discovery mechanism for install flows.

## wshobson/agents Repo Structure

[VERIFIED: GitHub API direct probing]

```
wshobson/agents/
  .claude-plugin/
    marketplace.json          # Declares 75+ plugins with sources
  plugins/
    accessibility-compliance/
      skills/
        screen-reader-testing/
          SKILL.md            # Actual skill
        wcag-audit-patterns/
          SKILL.md
    backend-development/
      skills/
        api-design-principles/
          SKILL.md
          assets/
          references/
    frontend-mobile-development/
      skills/
        tailwind-design-system/
          SKILL.md
          references/
    ...76 plugin directories total
  CLAUDE.md
  README.md
```

- **149 SKILL.md files** total, ALL at path depth 4: `plugins/*/skills/*/SKILL.md` [VERIFIED: recursive git tree API]
- **No SKILL.md at root** or in standard `skills/` directory
- The `.claude-plugin/marketplace.json` declares plugins with `"source": "./plugins/<name>"` paths
- The `skills.sh` registry lists 50+ skills from this repo with IDs like `wshobson/agents/tailwind-design-system`

## How npx skills CLI Discovers Skills

[VERIFIED: vercel-labs/skills source code on GitHub, v1.5.0]

The `discoverSkills()` function in `src/skills.ts` uses a three-tier approach:

### Tier 1: Direct SKILL.md check

If the target path has a `SKILL.md`, parse it and return (unless `--full-depth` is set).

### Tier 2: Priority search directories

Scans these directories for child dirs containing `SKILL.md`:

- Root directory itself
- `skills/`, `skills/.curated/`, `skills/.experimental/`, `skills/.system/`
- 25+ agent-specific dirs: `.agents/skills/`, `.claude/skills/`, `.cursor/skills/`, etc.
- **Plugin manifest paths** from `getPluginSkillPaths()` -- reads `.claude-plugin/marketplace.json` and `plugin.json`

### Tier 3: Recursive fallback

If nothing found in priority dirs, calls `findSkillDirs()` with recursive directory walking up to **depth 5**. This is what catches deeply nested structures.

**Key insight for wshobson/agents:** The marketplace.json parsing (via `getPluginSkillPaths()`) extracts `plugins/<name>` as search directories. The recursive scan in Tier 3 then finds `skills/<skill>/SKILL.md` inside each plugin dir. Both mechanisms contribute.

### SKILL.md Parsing

The CLI requires SKILL.md to have YAML frontmatter with both `name` and `description` fields as strings. Skills without both are silently skipped.

## Current Skills Hub Implementation Gaps

### Gap 1: Insufficient scan depth

`SKILL_SCAN_BASES` only covers 5 paths at depth 1-2:

```rust
const SKILL_SCAN_BASES: [&str; 5] = [
    "skills",
    "skills/.curated",
    "skills/.experimental",
    "skills/.system",
    ".claude/skills",
];
```

The `count_skills_in_repo()` and `list_git_skills()` functions scan these bases + root-level subdirs with SKILL.md. Nothing goes deeper than 2 levels.

### Gap 2: No marketplace.json awareness

The codebase reads `plugin.json` for descriptions (`read_plugin_description`) but never parses `.claude-plugin/marketplace.json` to discover plugin directories.

### Gap 3: No recursive scan fallback

When the standard bases find nothing, the code gives up. The npx skills CLI falls back to recursive `findSkillDirs()` up to depth 5.

### Gap 4: Multi-skill install UX

When `count_skills_in_repo() >= 2`, the installer bails with `MULTI_SKILLS|` error asking user to pick a specific URL. For repos like wshobson/agents with 149 skills, users need a selection UI, not an error.

## Schema Analysis for Update Tracking

[VERIFIED: skill_store.rs source code]

The existing `skills` table already has the columns needed:

| Column            | Current State                               | Needed for Update Tracking |
| ----------------- | ------------------------------------------- | -------------------------- |
| `source_ref`      | Stores full URL string                      | Already suitable           |
| `source_subpath`  | Stores relative path in repo                | Already suitable           |
| `source_revision` | Stores `"api-download-{branch}"` or git SHA | **Needs real commit SHA**  |
| `content_hash`    | SHA256 of installed dir                     | Already suitable           |

**No schema migration needed.** The `source_revision` column already exists and can store the commit SHA. The current code stores `clone_or_pull()` return value which IS the git HEAD SHA. The only gap is the API download path which stores `"api-download-{branch}"` instead of the actual SHA.

The `clone_to_cache()` function already stores and returns the HEAD commit SHA via `.skills-hub-cache.json`. The update check flow described in CONTEXT.md (compare SHA first, then content hash) is already architecturally supported.

## Architecture Patterns

### Recommended Discovery Strategy

```
install_git_skill() or list_git_skills():
  1. Clone/cache repo (existing)
  2. Parse .claude-plugin/marketplace.json if present
     -> Extract plugin source dirs (e.g., ./plugins/api-scaffolding)
  3. Scan priority directories (existing SKILL_SCAN_BASES + marketplace dirs)
  4. If nothing found: recursive findSkillDirs(repo_dir, depth=0, max_depth=5)
  5. For each found dir with SKILL.md: parse frontmatter for name+description
```

### Recommended File Changes

| File                 | Change                                                                                                         | Scope  |
| -------------------- | -------------------------------------------------------------------------------------------------------------- | ------ |
| `installer.rs`       | Add `parse_marketplace_json()`, expand `SKILL_SCAN_BASES` with dynamic dirs, add recursive `find_skill_dirs()` | Medium |
| `installer.rs`       | Fix `source_revision` for API download path to store real SHA                                                  | Small  |
| `github_download.rs` | Return branch SHA alongside downloaded content                                                                 | Small  |
| `commands/mod.rs`    | No changes needed -- `list_git_skills` already returns candidates                                              | None   |

### marketplace.json Parsing Pattern

```rust
// Source: verified from wshobson/agents/.claude-plugin/marketplace.json
#[derive(Deserialize)]
struct MarketplaceManifest {
    plugins: Option<Vec<MarketplacePlugin>>,
}

#[derive(Deserialize)]
struct MarketplacePlugin {
    name: Option<String>,
    source: Option<String>,  // e.g., "./plugins/api-scaffolding"
    description: Option<String>,
}

fn parse_marketplace_json(repo_dir: &Path) -> Vec<PathBuf> {
    let manifest_path = repo_dir.join(".claude-plugin/marketplace.json");
    // Parse, extract source dirs, resolve relative to repo_dir
    // Return list of directories to scan for skills
}
```

### Recursive Scan Pattern

```rust
fn find_skill_dirs_recursive(dir: &Path, depth: usize, max_depth: usize) -> Vec<PathBuf> {
    if depth > max_depth { return vec![]; }
    let mut results = Vec::new();
    if dir.join("SKILL.md").exists() {
        results.push(dir.to_path_buf());
    }
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if !p.is_dir() { continue; }
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "node_modules" || name == ".git" || name == "dist" || name == "build" {
                continue;
            }
            results.extend(find_skill_dirs_recursive(&p, depth + 1, max_depth));
        }
    }
    results
}
```

## Common Pitfalls

### Pitfall 1: API download path bypasses git clone

**What goes wrong:** When using the GitHub Contents API fast path, there's no git clone and thus no commit SHA.
**How to avoid:** Before the API download, make a lightweight API call to get the branch HEAD SHA: `GET /repos/{owner}/{repo}/commits/{branch}` returns the SHA without cloning.

### Pitfall 2: Performance on large repos

**What goes wrong:** Recursive scanning to depth 5 on a repo with thousands of directories would be slow.
**How to avoid:** Scan marketplace.json first (gives exact paths). Only use recursive scan as a last resort when priority dirs find nothing. The `findSkillDirs` in npx skills has the same max_depth=5 guard.

### Pitfall 3: skills.sh rate limiting

**What goes wrong:** skills.sh has no documented rate limits, but it's a Vercel-hosted Next.js app.
**How to avoid:** Don't depend on skills.sh for install-time discovery. Use it only for the Explore page search (already the case). The discovery must work offline via local file scanning.

### Pitfall 4: Duplicate skill detection

**What goes wrong:** A recursive scan might find the same SKILL.md through different paths (e.g., symlinks or if marketplace.json paths overlap with recursive scan).
**How to avoid:** The existing `dedup_by` on subpath handles this. Ensure all discovered skills use repo-relative subpaths for dedup.

## Don't Hand-Roll

| Problem                  | Don't Build            | Use Instead                               | Why                                       |
| ------------------------ | ---------------------- | ----------------------------------------- | ----------------------------------------- |
| YAML frontmatter parsing | Custom regex parser    | Existing `parse_skill_md()` already works | Handles edge cases                        |
| Content hashing          | New hash function      | Existing `hash_dir()` in content_hash.rs  | Already tested, handles .git exclusion    |
| Git clone/cache          | New clone logic        | Existing `clone_to_cache()`               | Has TTL caching, corruption recovery      |
| Marketplace JSON         | Over-engineered parser | Simple serde_json deserialization         | Only need `source` field from each plugin |

## Assumptions Log

| #   | Claim                                                         | Section               | Risk if Wrong                                                                                      |
| --- | ------------------------------------------------------------- | --------------------- | -------------------------------------------------------------------------------------------------- |
| A1  | Recursive depth 5 is sufficient for all known repo structures | Architecture Patterns | Could miss extremely nested repos, but matches npx skills CLI behavior                             |
| A2  | No schema migration needed                                    | Schema Analysis       | If planner wants additional columns (e.g., `last_update_check_at`), a V7 migration would be needed |

## Open Questions

1. **Multi-skill selection UI for git install**
   - What we know: Current code errors with `MULTI_SKILLS|` when >1 skill found
   - What's unclear: Should the planner address multi-skill selection UX in this task, or is that a separate task?
   - Recommendation: The `list_git_skills` command already returns candidates. The frontend multi-skill picker is needed but may be a separate task.

2. **skills.sh registry as validation source**
   - What we know: The API returns skill IDs like `wshobson/agents/tailwind-design-system`
   - What's unclear: Should we cross-reference discovered skills with registry data for enrichment (install counts, etc.)?
   - Recommendation: Not in this task. The Explore page already uses skills.sh for browsing. Install flow should be registry-independent.

## Sources

### Primary (HIGH confidence)

- GitHub API: `api.github.com/repos/wshobson/agents/git/trees/main?recursive=1` -- full repo tree, 149 SKILL.md files confirmed
- skills.sh API: `skills.sh/api/search?q=wshobson&limit=50` -- search endpoint verified, returns skill IDs with source field
- npm registry: `registry.npmjs.org/skills/latest` -- confirmed v1.5.0, repo at vercel-labs/skills
- vercel-labs/skills source: `src/skills.ts`, `src/add.ts`, `src/plugin-manifest.ts`, `src/find.ts` -- discovery algorithm verified

### Secondary (MEDIUM confidence)

- wshobson/agents `.claude-plugin/marketplace.json` -- read directly, 75+ plugins with source paths

### Codebase (HIGH confidence)

- `src-tauri/src/core/installer.rs` -- full source read, `SKILL_SCAN_BASES`, `list_git_skills`, `count_skills_in_repo`
- `src-tauri/src/core/skill_store.rs` -- schema V6, all columns documented
- `src-tauri/src/core/content_hash.rs` -- `hash_dir()` function
- `src-tauri/src/core/skills_search.rs` -- skills.sh `/api/search` integration
- `src-tauri/src/core/github_download.rs` -- GitHub Contents API download
- `src-tauri/src/core/skill_lock.rs` -- lock file enrichment

## Metadata

**Confidence breakdown:**

- Discovery mechanism: HIGH - verified directly from npx skills source code and GitHub API
- Schema analysis: HIGH - read full skill_store.rs source
- Architecture recommendation: HIGH - aligns with proven npx skills CLI approach
- skills.sh API capabilities: HIGH - directly probed multiple endpoints

**Research date:** 2026-04-16
**Valid until:** 2026-05-16 (stable domain, vercel-labs/skills is actively maintained)
