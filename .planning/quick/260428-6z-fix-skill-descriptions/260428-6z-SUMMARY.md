---
phase: quick
plan: 260428-6z
subsystem: scripts
tags: [scraper, featured-skills, github-api, enrichment]
dependency_graph:
  requires: []
  provides: [enriched-featured-skills-data]
  affects: [explore-page-descriptions]
tech_stack:
  added: []
  patterns: [github-trees-api-enrichment, skill-md-frontmatter-parsing]
key_files:
  created: []
  modified:
    - scripts/fetch-featured-skills-v2.mjs
    - featured-skills.json
decisions:
  - Ported fetchJson, pMap, parseSkillMdFrontmatter, getRepoTree, fetchSkillMdContent from v1 scraper
  - Enrichment matches skills to directories by slug-to-directory-name comparison
  - 83% enrichment rate (250/300) due to slug-to-directory mismatch for some skills
metrics:
  duration: 6m 37s
  completed: 2026-04-28T20:00:53Z
---

# Quick Task 260428-6z: Fix Skill Descriptions Summary

GitHub SKILL.md enrichment phase added to v2 scraper, populating summaries, names, and full source_urls for 83% of featured skills.

## Tasks Completed

| Task | Name                                               | Commit  | Key Files                            |
| ---- | -------------------------------------------------- | ------- | ------------------------------------ |
| 1    | Add GitHub SKILL.md enrichment to v2 scraper       | 1fc60eb | scripts/fetch-featured-skills-v2.mjs |
| 2    | Regenerate featured-skills.json with enriched data | 131aeb2 | featured-skills.json                 |

## What Changed

### scripts/fetch-featured-skills-v2.mjs

Added a post-scrape GitHub enrichment phase. After Playwright scrapes skills.sh for the skill list and install counts, the script now:

1. Groups all scraped skills by their GitHub repo (63 unique repos)
2. Fetches default branch for each repo via GitHub Repos API
3. Fetches recursive tree for each repo via GitHub Trees API
4. Matches each skill's slug to a directory in the tree by last path segment
5. Fetches SKILL.md via GitHub Contents API for matched skills
6. Parses YAML frontmatter to extract `name` and `description`
7. Updates `summary`, `name`, and `source_url` (now includes `/tree/{branch}/{path}`)

Functions ported from v1 scraper: `fetchJson`, `pMap`, `sleep`, `parseSkillMdFrontmatter`, `getRepoTree`, `fetchSkillMdContent`. Added new `enrichWithGitHub` orchestrator function.

Graceful degradation: skips enrichment entirely if GITHUB_TOKEN is not set, and skips individual skills that cannot be matched to a directory.

### featured-skills.json

- Skills with non-empty summary: **250/300** (was 0/300)
- Skills with path source_url: **251/300** (was 0/300)
- Trending with non-empty summary: **84/100** (was 0/100)
- Human-readable names from SKILL.md frontmatter where available

## Deviations from Plan

None -- plan executed exactly as written.

## Known Limitations

49 skills (16%) could not be matched because their skills.sh slug does not match any directory name in their GitHub repo tree. Examples:

- `vercel-react-best-practices` (vercel-labs/agent-skills) -- directory name differs from slug
- `azure-cost-optimization` (microsoft/azure-skills) -- similar mismatch
- `polish`, `critique`, `audit` (pbakaus/impeccable) -- sub-skill names differ

The plan's aspirational target was 90%+ enrichment; actual result is 83%. All verification thresholds (>200 of 300 = >66%) are exceeded.

## Verification

- `npm run check` passes (lint + build + rust:fmt:check + rust:clippy + rust:test, 180 tests)
- Skills with summary: 250/300 (exceeds >200 threshold)
- Skills with path URL: 251/300 (exceeds >200 threshold)
- Trending with summary: 84/100 (exceeds >50 threshold)
- No frontend changes needed -- ExplorePage.tsx already renders `skill.summary`

## Self-Check: PASSED
