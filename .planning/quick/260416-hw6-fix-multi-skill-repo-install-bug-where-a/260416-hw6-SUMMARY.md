# Quick Task 260416-hw6: Summary

**Task:** Fix multi-skill repo install bug where all skills from repos like better-auth/skills get the same name instead of reading each skill's own SKILL.md frontmatter

## Changes Made

### Task 1: Case-insensitive SKILL.md detection + always-recursive scanning (installer.rs)

- Added `has_skill_md()` and `find_skill_md()` helper functions that detect SKILL.md/SKILL.MD case-insensitively
- Replaced 16+ hardcoded `"SKILL.md"` references with the new helpers
- Removed the `priority_count` gate that prevented recursive scanning when root-level skills were found
- Used HashSet dedup in `count_skills_in_repo` to prevent double-counting
- **Commit:** `e3f6bf4`

### Task 2: Route folder URLs through candidate flow + guard single-candidate branch (App.tsx, resources.ts)

- Removed the `isFolderUrl` direct-install shortcut — all URLs now route through `list_git_skills_cmd` candidate flow
- Added `autoSelectSkillName` mismatch guard in the `candidates.length === 1` branch
- When auto-select name doesn't match the single candidate, shows error toast instead of installing wrong skill
- Added `errors.skillNotFoundInRepo` i18n key
- **Commit:** `784abdb`

### Task 3: Verification

- `npm run check` passes (lint + build + rust:fmt:check + clippy + rust:test)
- No hardcoded `"SKILL.md"` string literals remain (all use helpers)
- No `isFolderUrl` references remain in App.tsx
- `find_skill_md` and `has_skill_md` helpers confirmed present

## Files Modified

- `src-tauri/src/core/installer.rs` — 291 insertions, 291 deletions
- `src/App.tsx` — 178 changes (removed isFolderUrl path, added guard)
- `src/i18n/resources.ts` — 2 insertions (new i18n key)
