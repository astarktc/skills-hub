---
phase: quick
plan: 260429-pc
subsystem: explore
tags: [ui, sqlite, explore, hide-skills]
dependency_graph:
  requires: []
  provides: [hidden_explore_skills_table, hide_unhide_commands]
  affects: [explore-page]
tech_stack:
  added: []
  patterns: [V7_migration, localStorage_persistence]
key_files:
  created: []
  modified:
    - src-tauri/src/core/skill_store.rs
    - src-tauri/src/commands/mod.rs
    - src-tauri/src/lib.rs
    - src/App.tsx
    - src/components/skills/ExplorePage.tsx
    - src/App.css
    - src/i18n/resources.ts
decisions:
  - "Used if/else in onClick handlers instead of ternary to satisfy no-unused-expressions lint rule"
metrics:
  duration: ~4min
  completed: "2026-04-29"
  tasks: 2
  files_modified: 7
---

# Quick Task 260429-pc: Explore Hide Skills + Preview Rename Summary

**One-liner:** DB-persisted hide/unhide for Explore skills with "Show hidden" checkbox and "View" renamed to "Preview"

## What Was Done

### Task 1: Backend - V7 migration + hide/unhide commands

- Bumped SCHEMA_VERSION from 6 to 7
- Added `hidden_explore_skills` table via V7 incremental migration (source_url TEXT PK, hidden_at INTEGER)
- Added `hide_explore_skill`, `unhide_explore_skill`, `list_hidden_explore_skills` methods to SkillStore
- Added 3 corresponding Tauri IPC commands and registered them in `generate_handler!`
- Commit: d793023

### Task 2: Frontend - state management, ExplorePage UI, i18n

- Renamed `exploreView` i18n key from "View" to "Preview" in both EN and ZH
- Added `exploreHide`, `exploreUnhide`, `exploreShowHidden` i18n keys
- Added `hiddenSkills` (Set<string>) and `showHidden` (boolean, localStorage-persisted) state in App.tsx
- Added `loadHiddenSkills` callback, invoked when switching to Explore view
- Added `handleHideSkill`/`handleUnhideSkill` callbacks with optimistic state updates
- Extended ExplorePageProps with new hide/show props
- Added `visibleFeatured` and `visibleSearchResults` memos that filter by hidden state
- Added "Show hidden" checkbox with count badge in explore-hero section
- Added hide/unhide button to each featured and search result card (Eye/EyeOff icons)
- Added CSS styles for `.explore-btn-hide`, `.explore-show-hidden`, `.explore-hidden-count`
- Commit: 6f05e5b

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Lint error: no-unused-expressions**

- **Found during:** Task 2
- **Issue:** Plan used ternary expression as statement in onClick handlers (`condition ? fnA() : fnB()`), which ESLint's `@typescript-eslint/no-unused-expressions` flags as error
- **Fix:** Replaced with proper if/else blocks
- **Files modified:** src/components/skills/ExplorePage.tsx

## Verification

- `npm run check` passes (lint + build + rust:fmt:check + rust:clippy + rust:test)
- 164 Rust tests pass
- TypeScript builds cleanly
- ESLint passes (only pre-existing warnings in vendored `.cjs` files)

## Self-Check: PASSED
