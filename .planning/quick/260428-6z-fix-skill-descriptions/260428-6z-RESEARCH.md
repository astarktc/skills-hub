# Fix Skill Descriptions - Research

**Researched:** 2026-04-28
**Confidence:** HIGH

## Summary

The v2 featured-skills scraper (Playwright-based, skills.sh leaderboard) produces `summary: ''` for all 400 skills because skills.sh provides no description data. The v1 scraper (curated repos, GitHub API) fetched SKILL.md frontmatter for each skill and populated summaries for 100% of entries. The v2 switch broke descriptions. Separately, skills.sh search API returns no description field, so online search results have never had descriptions in the UI.

**Root cause:** The v2 scraper's `transformSkill` hardcodes `summary: ''` (line 88 of v2 script) because the RSC payload from skills.sh contains only: `skillId`, `name`, `installs`, `source`. No description anywhere in skills.sh data.

**Fix strategy:** Enrich the v2 scraper with GitHub SKILL.md fetching using the Trees API (reuse v1 patterns). For search results, descriptions would require runtime SKILL.md fetching in the Rust backend -- a separate, larger scope change.

## Finding 1: skills.sh API Has No Description Field

**[VERIFIED: curl probe]** The skills.sh search API (`/api/search`) returns exactly these fields per skill:

- `id`: `"owner/repo/skillId"` (full path)
- `skillId`: `"skill-name"` (just the skill directory name)
- `name`: same as skillId
- `installs`: number
- `source`: `"owner/repo"`

No `description`, `summary`, or `readme` field exists. The RSC leaderboard payload has the same fields. There is no way to get descriptions from skills.sh.

## Finding 2: v1 Had Full Descriptions via SKILL.md Fetching

**[VERIFIED: git history]** Pre-v2 featured-skills.json (commit `3757f63`) had 300/300 skills with non-empty summaries. The v1 script (`fetch-featured-skills.mjs`) achieved this via:

1. **`getRepoTree()`** -- fetched repo tree via GitHub Trees API (recursive), one call per repo
2. **`detectSkillsFromTree()`** -- found SKILL.md locations by matching directory structure
3. **`fetchSkillMdContent()`** -- fetched each SKILL.md via GitHub Contents API (base64 decode)
4. **`parseSkillMdFrontmatter()`** -- extracted `name` and `description` from YAML frontmatter

The v1 script also filtered out skills WITHOUT SKILL.md (line 367), ensuring data quality.

**Reusable functions from v1:** `parseSkillMdFrontmatter`, `fetchSkillMdContent`, `pMap`, `fetchJson` (with rate-limit handling).

## Finding 3: API Budget for Description Enrichment

**[VERIFIED: analysis of current featured-skills.json]**

| Metric                                      | Count |
| ------------------------------------------- | ----- |
| Total skills (skills + trending, pre-dedup) | 400   |
| Unique skill slugs                          | 324   |
| Unique repos                                | 61    |

**Two enrichment strategies:**

| Strategy                   | API Calls  | Pros                                                      | Cons                                       |
| -------------------------- | ---------- | --------------------------------------------------------- | ------------------------------------------ |
| **Per-repo Trees API**     | ~61 calls  | Efficient, gives all paths, enables local SKILL.md lookup | Still need Contents API for each SKILL.md  |
| **Per-skill Contents API** | ~324 calls | Simple, v1 already has the code                           | More calls, but well within 5,000/hr limit |

**Recommended: Hybrid approach** (what v1 already did)

1. Fetch Trees API per repo (~61 calls) to find SKILL.md paths
2. Fetch Contents API per skill's SKILL.md (~324 calls) for frontmatter
3. Total: ~385 calls -- well within 5,000/hr authenticated limit

**The key mapping:** skills.sh `source` = `"owner/repo"`, and the skill directory name = `skillId`. So the SKILL.md path is discoverable by fetching the repo tree and finding `*/skillId/SKILL.md`.

## Finding 4: Source URL Regression

**[VERIFIED: comparison of v1 vs v2 output]**

v1 source_urls had full skill paths:

```
https://github.com/anthropics/skills/tree/main/skills/algorithmic-art
```

v2 source_urls point to repo roots only:

```
https://github.com/anthropics/skills
```

This means the Explore page "View" button and install flow get a repo-level URL instead of a skill-specific URL. The install flow may still work (installer scans for skills), but it is less precise. Fix this alongside the description enrichment by constructing full source URLs from the tree data.

## Finding 5: Search Results (OnlineSkillDto) Have No Description

**[VERIFIED: code review]**

The full pipeline for search results:

- skills.sh API returns no description
- Rust `SkillsShItem` struct has: `name`, `installs`, `source`
- Rust `OnlineSkillResult` has: `name`, `installs`, `source`, `source_url`
- TS `OnlineSkillDto` has: `name`, `installs`, `source`, `source_url`
- Frontend renders no `explore-card-desc` div for search results (line 239 in ExplorePage.tsx -- compare to line 156 for featured skills)

Adding descriptions to search results would require either:

- **Option A:** Runtime SKILL.md fetching in Rust backend (new async endpoint, caching)
- **Option B:** Pre-built index (skills.sh would need to add descriptions)

**Recommendation:** Defer search result descriptions. Focus on fixing the featured skills regression first since it is a static JSON enrichment problem solvable in the scraper script.

## Action Plan

### Fix 1: Enrich v2 scraper with SKILL.md descriptions

Modify `fetch-featured-skills-v2.mjs` to add a post-scrape enrichment phase:

1. After scraping skills.sh, collect unique repos from `skill.source`
2. Fetch GitHub Trees API per repo (requires `GITHUB_TOKEN`)
3. For each skill, locate its SKILL.md by matching `skillId` to directory names in the tree
4. Fetch SKILL.md via Contents API, parse frontmatter for `description`
5. Set `summary` from description, and construct full `source_url` with `/tree/{branch}/{path}`

Port from v1: `fetchJson`, `pMap`, `parseSkillMdFrontmatter`, `fetchSkillMdContent`. The rate-limit retry logic is especially important to reuse.

The script should gracefully handle missing SKILL.md (keep `summary: ''`) and work without `GITHUB_TOKEN` (skip enrichment, warn).

### Fix 2: Fix source_url to include skill path

When tree data reveals the skill's directory path, construct:

```
https://github.com/{owner}/{repo}/tree/{default_branch}/{path_to_skill_dir}
```

### Fix 3 (optional, separate PR): Add descriptions to search results

Add a new Tauri command or extend `search_skills_online` to optionally fetch SKILL.md for each result. This requires caching to avoid repeated GitHub API calls. Defer to a follow-up task.

## Sources

- `scripts/fetch-featured-skills-v2.mjs` -- current v2 scraper (no SKILL.md fetching)
- `scripts/fetch-featured-skills.mjs` -- v1 scraper (has reusable SKILL.md patterns)
- `featured-skills.json` -- current output: 300/300 empty summaries
- `git show 3757f63:featured-skills.json` -- v1 output: 300/300 with summaries
- `curl https://skills.sh/api/search` -- live API probe confirming no description field
- `src-tauri/src/core/skills_search.rs` -- Rust search backend
- `src-tauri/src/core/featured_skills.rs` -- Rust featured skills backend
- `src/components/skills/ExplorePage.tsx` -- frontend rendering
- GitHub REST API docs: rate limits 5,000/hr authenticated [ASSUMED -- standard documented limit]
