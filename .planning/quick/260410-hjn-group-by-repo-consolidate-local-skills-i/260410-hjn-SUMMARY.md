# Quick Task 260410-hjn: Summary

## Description

Group by repo: consolidate local skills, indent matrix skills, fix icon layout, hide All tools button

## Changes

### Task 1: Consolidate local skills under single group header

- **SkillsList.tsx**: Changed grouping logic to use `__local__` sentinel key for all non-git-repo skills, rendering them under a single "Local" group header instead of individual groups per local path
- **AssignmentMatrix.tsx**: Applied same `__local__` consolidation logic for consistency between My Skills and Project Assignment Matrix
- **resources.ts**: Added `localGroup: "Local"` i18n key for both EN and ZH

### Task 2: Matrix indent, inline icon layout, conditional all-tools

- **AssignmentMatrix.tsx**: Added `matrix-grid-grouped` CSS class when grouping is active for indented skill names; wrapped group header content in `matrix-group-label` span for inline-flex icon layout; added `showBulkAssign` prop to conditionally hide "All tools" button when only one tool is selected
- **App.css**: Added `.matrix-grid-grouped .matrix-skill-cell` indent rule, `.matrix-group-label` inline-flex styling, updated `.repo-group-icon` for flex-shrink

## Commits

- `1064008` feat(quick-260410-hjn): consolidate local skills under single group header
- `fd8add8` feat(quick-260410-hjn): matrix indent, inline icon layout, conditional all-tools

## Files Modified

- `src/components/skills/SkillsList.tsx`
- `src/components/projects/AssignmentMatrix.tsx`
- `src/App.css`
- `src/i18n/resources.ts`
