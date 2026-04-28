# Quick Task 260428-tf: Bump search limit to 50 and write v2 featured-skills scraper - Context

**Gathered:** 2026-04-28
**Status:** Ready for planning

<domain>
## Task Boundary

Two changes from the skills.sh investigation:

1. **Bump search limit from 20 to 50** in `src/App.tsx:906` — the `search_skills_online` call passes `limit: 20` but the backend supports up to 50. Prolific repos like mattpocock/skills have 28+ results and the low limit cuts off real skills while stale ghost entries occupy slots in the top 20.

2. **Write a v2 of `scripts/fetch-featured-skills.mjs`** that scrapes the skills.sh leaderboard page instead of using a hardcoded curated repo list sorted by GitHub stars. The current approach results in 282/300 featured skills from a single repo.

</domain>

<decisions>
## Implementation Decisions

### Scraping Approach

- Use headless browser (Playwright) to render skills.sh pages and extract data via DOM queries
- Purely scripted — no LLM integration, just standard DOM querying of rendered pages
- Playwright added as a dev dependency

### Featured List Composition

- Produce **separate lists**: `featured` from All Time tab, `trending` from Hot/Trending tab
- Two distinct arrays in the output JSON
- Frontend can later show tabs or sections (but frontend changes are NOT in scope for this task)

### Collapsed Repos Handling

- Expand all "+N more from repo" collapsed entries to get full coverage
- Click each collapse button and wait for content to appear before extracting

### Claude's Discretion

- Output JSON schema: maintain compatibility with existing `featured-skills.json` shape where possible, add a `trending` key alongside `skills`
- Max entries: keep 300 for featured (All Time), ~100 for trending

</decisions>

<specifics>
## Specific Ideas

- Script name: `scripts/fetch-featured-skills-v2.mjs` (leave original intact)
- All Time leaderboard URL: `https://skills.sh/`
- Hot tab URL: `https://skills.sh/hot`
- Each entry has: rank, name, source (owner/repo), install count
- No public JSON API exists for the leaderboard — scraping is the only option

</specifics>

<canonical_refs>

## Canonical References

- Memory: `project_skillsh_stale_data_investigation.md` — full investigation context
- Backend search limit: `src-tauri/src/core/skills_search.rs` clamps to max 50
- Current v1 script: `scripts/fetch-featured-skills.mjs`

</canonical_refs>
