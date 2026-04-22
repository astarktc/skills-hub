# Quick Task 260422-ixr: Summary

## What Changed

Converted the static Unlink button on SkillCard into a toggle that syncs/unsyncs a skill to/from all installed tools, with a dynamic icon reflecting current deployment state.

## Files Modified

| File                                   | Change                                                                                |
| -------------------------------------- | ------------------------------------------------------------------------------------- |
| `src/i18n/resources.ts`                | Added `syncSkillTooltip` i18n key                                                     |
| `src/App.tsx`                          | Added `handleSyncSkillToAllTools` handler, passed as prop to SkillsList               |
| `src/components/skills/SkillsList.tsx` | Threaded `onSyncSkillToAllTools` prop through to SkillCard                            |
| `src/components/skills/SkillCard.tsx`  | Replaced static Unlink button with Link/Unlink toggle based on `skill.targets.length` |

## Behavior

- **Synced state** (targets > 0): Shows `Link` icon, tooltip says "Remove this skill from all tool directories", click calls `onUnsync`
- **Unsynced state** (targets = 0): Shows `Unlink` icon, tooltip says "Deploy this skill to all installed tools", click calls `onSyncToAllTools`
- Button is only disabled during loading — never permanently dead

## Verification

- `npm run check` passes (lint, TypeScript build, Rust fmt/clippy/test — 154 tests)
