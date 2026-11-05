# Ticket 31 — Standards review

Reviewed the uncommitted diff on `t3/my-skills-anatomy` against `e177456`, including untracked `listColumns.ts`. Read-only except this report; no commit.

## Documented-standard breach

**S1 · P2 — `src/hooks/useListLens.ts:205–211`: new presentation fold outside its owner.**

> `groups.filter((group) => !collapsed.has(group.key)).flatMap((group) => group.skills)`

AGENTS.md § Ambiguity resolution says the list lens’s “folds stay pure in `skillPresentation.ts`”; that module already owns `displayOrder`. The new collapsed-group-to-visible-order fold instead lives inside the hook. Extract a pure fold beside `displayOrder`, test it there, and let the hook supply groups/collapse state. Persistence and transient-vs-stored state themselves follow the documented ownership and are clearly explained.

## Baseline smells — judgement calls, not hard violations

**S2 · P2 — `src/i18n/resources.test.ts:99–123`: possible Primitive Obsession.**

> `new RegExp(\`["'\`]${escaped}["'\`]\`).test(text)`
> `key.startsWith(prefix)`

Raw source text stands in for executable translation reads: a comment or unrelated quoted string counts as a read, and any matching template prefix blesses every descendant, without checking possible suffixes. Conversely, `t(prefix + leaf)` can be missed if neither a complete quoted key nor the recognised template appears. The synthetic self-test proves only those four accepted shapes, not these false-positive/negative boundaries. Prefer syntax-aware read extraction with explicit, justified dynamic-family coverage; at minimum document the heuristic and test comments, concatenation, and an unused sibling beneath a live prefix. No deleted-key runtime reference was found in this review.

**S3 · P3 — `src/components/skills/SkillsList.tsx:275–406`: possible Divergent Change.**

> `groups.map((group) => { … const state = groupState(group.skills); … return (<section …` 

The inline group block now owns selection, collapse semantics, source identity/link, summary, harness legend and Deploy all, closing over list-wide state. There is no group-header component/interface. Extract a private header component taking group, selection/collapse state, legend and action handle; keep list ordering/measurement in `SkillsList`. No new primitive colour/type/radius override was found.

## Checks and exclusions

Fresh targeted Vitest: **6 files, 249 tests passed**. `listColumns.ts` is the shared row/header template. Rust adds one project-deployment query, reuses `read_lifecycle`, and keeps command changes DTO-only. Fixture projects derive from assignment rows at read time, not a separately maintained skill snapshot. No App.css deletion occurred, so the unchanged ceiling and appended migrated file are consistent.

Actual catalog comparison found **56 deleted leaves**, not 55. Every deleted leaf name was grepped across `src/`. Hits: `subtitle`, `overridden`, `groupByRepo`, `sourceLabel`, `pathLabel`, `storagePath`, `confirm`, `files`, `toolCount`, `was`; all were CSS/code/comments or surviving differently-qualified keys, not reads of deleted paths. Other deleted leaf names had no hits.

## Files reviewed

- `DESIGN.md`, `index.html`, `scripts/styles-baseline.json`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/core/{skill_catalog,skill_store}.rs`, `src-tauri/src/core/tests/skill_catalog.rs`
- `src/App.tsx`, `src/bindings/index.ts`
- `src/components/projects/AssignmentMatrix.tsx`
- `src/components/skills/{ExplorePage,FilterBar,SkillRow,SkillsList,SyncedStrip}.tsx`, `src/components/skills/{types,listColumns}.ts`
- `src/components/skills/add/CandidatePicker.tsx`, `src/components/skills/inspector/HealthSection.tsx`, `src/components/ui/specimen.tsx`
- `src/fixtures/{backend.test,backend,model,scenarios}.ts`
- `src/hooks/{useListLens.test,useListLens,useSkillActions.test,useSkillLibrary.test}.ts`
- `src/i18n/{resources.test,resources}.ts`
- `src/lib/{bulkCounts.test,commandPalette.test,persistedPreference.test,persistedPreference,preferences.test,preferences,skillPresentation.test,skillPresentation}.ts`
- `.scratch/round19/issues/31-my-skills-row-and-group-anatomy.md` (scope context)

Summary: **3 findings — 1 documented-standard breach and 2 judgement calls; worst: S1, the new visible-order fold bypasses its documented presentation owner.**
