---
status: resolved
trigger: "Alex-LT25/skills-hub/docs/conversation-logs/2026-04/conversation-log-2026-04-29-1614.md There is a bug with the hide button on the skill cards. The other agent in the referenced conversation log was not able to figure it out, it seemed to be experiencing context rot and lost it's IQ. I think you'll be able to do better. Right now, as soon as you hide a skill, instead of just hiding the one skill, it hides every single skill that displays on the Explore page. While you are debugging this issue, also just overall code review the implementation of the hide feature in the first place because I need to make sure that the other agent didn't mess up. Regardless if I am looking at featured skills or searched skills, the hide function should apply regardless. If I have hidden a skill from a specific repo that has a specific name, that should be hidden regardless. of either it's displaying as a featured skill or as a skill that I searched."
created: 2026-04-29
updated: 2026-04-29
---

# Debug Session: hide-button-hides-all-skills

## Symptoms

- Expected behavior: Hiding a skill on the Explore page hides only that specific skill, identified by repo/name, and the same hidden identity applies whether the skill appears in featured skills or search results.
- Actual behavior: Clicking hide on one Explore skill hides many (or all) skills currently displayed on the Explore page.
- Error messages: None reported.
- Timeline: Reproduces currently after the recent hide-feature implementation.
- Reproduction: Open Explore page with featured skills visible, click the hide button on one skill card, observe multiple or all visible skills disappear.

## Current Focus

- hypothesis: CONFIRMED — hide identity uses raw source_url as key; many featured skills share the same source_url (base repo URL, no /tree/ path). Hiding one skill hides all others from the same repo with the same source_url.
- test: Confirmed via DB query: 1 hidden entry 'https://github.com/mattpocock/skills' (base URL); featured cache has 22 skills sharing 'https://github.com/pbakaus/impeccable', 7 sharing 'https://github.com/leonxlnx/taste-skill'.
- expecting: Fix uses composite key {name_lower}|{normalized_base_repo} as hide identity
- next_action: complete
- reasoning_checkpoint: 37 of 300 featured skills use base repo URLs with no /tree/ path specifics. Hiding any one of these hides all siblings with the same base URL. Additionally, search results always use base repo URLs while featured skills may use full /tree/ paths, making cross-context hide inconsistent.
- tdd_checkpoint:

## Evidence

- timestamp: 2026-04-29T investigation
  type: db_query
  finding: "hidden_explore_skills table: 1 row, url='https://github.com/mattpocock/skills' (base URL without /tree/ path, stored from a search result click)"

- timestamp: 2026-04-29T investigation
  type: data_analysis
  finding: "Cache analysis: pbakaus/impeccable has 22 skills all sharing URL https://github.com/pbakaus/impeccable; leonxlnx/taste-skill has 7 sharing same URL; 37 total skills use base repo URLs"

- timestamp: 2026-04-29T investigation
  type: code_review
  finding: "visibleFeatured: filteredSkills.filter((s) => !hiddenSkills.has(s.source_url)). When source_url is shared among multiple skills (same repo base URL), hiding one hides all"

- timestamp: 2026-04-29T investigation
  type: code_review
  finding: "search_skills_online_core: source_url = format!('https://github.com/{}', item.source) — always base repo URL. Featured skills have full /tree/ paths. These never match, so hiding a featured skill does NOT hide the search result for the same skill and vice versa."

## Eliminated

- DB migration issue (table creation fix was correct in 8e488dc)
- Frontend filter logic errors (Set.has() string comparison is correct)
- React memo/useMemo stale closure issues (dependencies correctly declared)
- Empty source_url causing universal match (no skills have empty source_url in data)

## Resolution

- root_cause: Hide identity uses raw source_url as unique key. Many featured skills from multi-skill repos share the same source_url (base repo URL with no per-skill path). Hiding one hides all siblings. Also featured vs search source_url formats differ (full path vs base URL), breaking cross-context hide.
- fix: Replace raw source_url with composite key {name.toLowerCase()}|{normalized_base_repo} in ExplorePage.tsx. normalizeRepo strips https://github.com/, .git, /tree/... path, lowercases. This key is unique per skill-name+repo combo and works identically whether skill appears in featured (full path URL) or search results (base URL). Backend Rust unchanged — it stores whatever string it receives.
- verification: `npm run check` passed: ESLint completed with pre-existing warnings only, TypeScript/Vite build passed, Rust fmt/clippy/test passed (180 Rust tests total). Code inspection confirmed App.tsx stores hidden entries as opaque strings, so the frontend composite key is compatible with the backend table. Browser automation could not be run because the configured agent-browser skill is unavailable in this session.
- files_changed: src/components/skills/ExplorePage.tsx
