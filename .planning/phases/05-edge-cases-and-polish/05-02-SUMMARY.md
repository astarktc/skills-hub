---
phase: 05-edge-cases-and-polish
plan: 02
subsystem: frontend
tags: [auto-sync, unsync, filterbar, skillcard, ui-controls]
dependency_graph:
  requires:
    [
      get_auto_sync_enabled,
      set_auto_sync_enabled,
      unsync_all_skills,
      unsync_skill,
    ]
  provides:
    [
      auto-sync-toggle-ui,
      bulk-unsync-button,
      per-skill-unlink-icon,
      conditional-install-sync,
    ]
  affects:
    [
      App.tsx,
      FilterBar.tsx,
      SkillCard.tsx,
      SkillsList.tsx,
      resources.ts,
      App.css,
    ]
tech_stack:
  added: []
  patterns:
    [auto-sync-conditional-guard, bulk-unsync-handler, per-skill-unsync-handler]
key_files:
  created: []
  modified:
    - src/App.tsx
    - src/components/skills/FilterBar.tsx
    - src/components/skills/SkillCard.tsx
    - src/components/skills/SkillsList.tsx
    - src/i18n/resources.ts
    - src/App.css
decisions:
  - "Auto-sync guard wraps all 6 sync blocks (5 install paths + handleSyncAllManagedToTools)"
  - "Unlink button uses secondary-action CSS class with tertiary color for visual hierarchy"
  - "SkillsList threads onUnsyncSkill prop from App.tsx to SkillCard (existing props-drilling pattern)"
metrics:
  duration_seconds: 517
  completed: "2026-04-09T00:30:00Z"
  tasks_completed: 2
  tasks_total: 2
  tests_added: 0
  tests_passing: 124
  files_modified: 6
---

# Phase 5 Plan 2: My Skills Tab Frontend Controls Summary

Auto-sync toggle in FilterBar, conditional sync guards on all install paths, bulk unsync button, per-skill Unlink icon on SkillCard, and CSS for new UI elements and missing matrix cell status.

## Completed Tasks

| Task | Name                                                                         | Commit  | Key Changes                                                                                                                                                                             |
| ---- | ---------------------------------------------------------------------------- | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1    | Auto-sync toggle state, conditional install sync, and bulk unsync in App.tsx | 3bfdbdb | autoSyncEnabled state + mount load, handleAutoSyncToggle, 6 sync paths guarded with if (autoSyncEnabled), handleUnsyncAll, handleUnsyncSkill, new FilterBar/SkillsList props, i18n keys |
| 2    | FilterBar auto-sync toggle and unsync button, SkillCard Unlink icon, CSS     | b90b33c | FilterBar gains checkbox toggle + unsync button, SkillCard gains Unlink icon (disabled when no targets), CSS for .auto-sync-toggle, .matrix-cell.missing, .card-btn.secondary-action    |

## Implementation Details

### App.tsx Changes

1. **Auto-sync state**: `autoSyncEnabled` useState(true), loaded from backend `get_auto_sync_enabled` on mount
2. **Toggle handler**: `handleAutoSyncToggle` calls `set_auto_sync_enabled` then updates local state
3. **Conditional sync guards**: All 6 sync blocks wrapped with `if (autoSyncEnabled)`:
   - Single local install (install_local_selection)
   - Folder URL git install (install_git)
   - Single git selection (install_git_selection)
   - Auto-match from online search (install_git_selection)
   - Batch local import (install_local_selection)
   - Batch git import (install_git_selection via handleInstallSelectedCandidates)
4. **handleSyncAllManagedToTools**: Early return when `!autoSyncEnabled`, added to dependency array
5. **Bulk unsync**: `handleUnsyncAll` calls `unsync_all_skills`, shows toast with count, refreshes skill list
6. **Per-skill unsync**: `handleUnsyncSkill` calls `unsync_skill`, refreshes skill list

### FilterBar Changes

- Three new props: `autoSyncEnabled`, `onAutoSyncChange`, `onUnsyncAll`
- Checkbox toggle with label using `t('autoSyncToggle')`
- "Uninstall from tool directories" button using `t('unsyncAll')`
- Both controls placed before the existing sort button in filter-actions

### SkillCard Changes

- New `onUnsync` prop in SkillCardProps type
- `Unlink` icon imported from lucide-react
- Unlink button between Update and Delete buttons
- Disabled when `skill.targets.length === 0` (no active sync targets)
- Uses `secondary-action` CSS class for neutral visual weight

### SkillsList Changes

- New `onUnsyncSkill` prop threaded through from App.tsx to SkillCard as `onUnsync`

### CSS Additions

- `.auto-sync-toggle` - Flex layout for checkbox + label
- `.auto-sync-label` - Secondary text color
- `.unsync-all-btn` - No-wrap button styling
- `.matrix-cell.missing` - Danger background for missing status
- `.card-btn.secondary-action` - Tertiary color with hover/disabled states

### i18n Keys Added

- `autoSyncToggle`: "Auto-sync to tool directories"
- `unsyncAll`: "Uninstall from tool directories"
- `unsyncAllComplete`: "Removed {{count}} tool directory deployments"
- `unsyncSkill`: "Uninstall from tool directories"
- `unsyncSkillTooltip`: "Remove this skill from all tool directories"

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added onUnsyncSkill to SkillsList component**

- **Found during:** Task 1
- **Issue:** Plan specified passing onUnsync directly to SkillCard in App.tsx, but SkillCard is rendered inside SkillsList, not directly in App.tsx
- **Fix:** Added onUnsyncSkill prop to SkillsListProps type, destructured it, and passed it as onUnsync to SkillCard
- **Files modified:** src/components/skills/SkillsList.tsx
- **Commit:** 3bfdbdb

**2. [Rule 1 - Bug] Used --text-tertiary instead of --text-muted for secondary-action CSS**

- **Found during:** Task 2
- **Issue:** Plan CSS referenced `var(--text-muted)` which does not exist in the theme variables
- **Fix:** Used `var(--text-tertiary)` which is the actual CSS variable for muted text
- **Files modified:** src/App.css
- **Commit:** b90b33c

## Threat Surface Scan

No new threat surface introduced. All new functionality uses existing Tauri IPC commands (get/set_auto_sync_enabled, unsync_all_skills, unsync_skill) that were added in Plan 05-01. No new network endpoints, auth paths, or file access patterns.

## Self-Check: PASSED

All 6 modified files exist. Both commit hashes (3bfdbdb, b90b33c) verified. All acceptance criteria confirmed via grep. Lint and build pass clean. No Rust changes in this plan (frontend-only).
