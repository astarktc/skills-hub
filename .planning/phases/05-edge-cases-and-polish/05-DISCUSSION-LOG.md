# Phase 5: Edge Cases and Polish - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-08
**Phase:** 05-edge-cases-and-polish
**Areas discussed:** Auto-sync toggle, Per-skill unsync icon, Missing project detection, Orphaned assignments

---

## Auto-sync Toggle

| Option                    | Description                                                      | Selected |
| ------------------------- | ---------------------------------------------------------------- | -------- |
| Single global toggle      | ON = auto-sync to all tools on install. OFF = central repo only. | ✓        |
| Global + per-tool toggles | Master switch plus per-tool granularity                          |          |
| Per-tool only             | Only per-tool toggles, no master switch                          |          |

**User's choice:** Single global toggle
**Notes:** Simplest approach, matches the description. Per-tool granularity not needed.

| Option            | Description                                               | Selected |
| ----------------- | --------------------------------------------------------- | -------- |
| My Skills toolbar | Toggle in toolbar above skill list, next to unsync button | ✓        |
| Settings page     | Under Settings alongside other app settings               |          |
| Both locations    | Primary in toolbar, also in Settings                      |          |

**User's choice:** My Skills toolbar
**Notes:** Visible and discoverable where users manage their skills.

| Option                       | Description                                                     | Selected |
| ---------------------------- | --------------------------------------------------------------- | -------- |
| Remove all from all tools    | One operation, no confirmation. Deletes all SkillTargetRecords. | ✓        |
| Remove all with confirmation | Confirmation dialog listing affected tools/skills               |          |
| Selective tool picker        | Modal to pick which tools to unsync from                        |          |

**User's choice:** Remove all from all tools
**Notes:** Simple and reversible.

| Option                    | Description                                                    | Selected |
| ------------------------- | -------------------------------------------------------------- | -------- |
| Leave existing synced     | Toggle only affects new installs. Use bulk unsync to clean up. | ✓        |
| Auto-remove on toggle off | Immediately remove all from tool dirs when toggled off         |          |

**User's choice:** Leave existing synced
**Notes:** Non-destructive toggle behavior.

| Option         | Description                                | Selected |
| -------------- | ------------------------------------------ | -------- |
| ON by default  | Matches current behavior                   | ✓        |
| OFF by default | New installs start with auto-sync disabled |          |

**User's choice:** ON by default
**Notes:** No behavior change for existing users.

---

## Per-skill Unsync Icon

| Option                  | Description                                        | Selected |
| ----------------------- | -------------------------------------------------- | -------- |
| Unlink (broken chain)   | lucide-react Unlink icon — chain link being broken | ✓        |
| Link2Off (slashed link) | Link with slash through it                         |          |
| FolderMinus             | Folder with minus sign                             |          |

**User's choice:** Unlink (broken chain)
**Notes:** Clear metaphor for disconnecting from tool directories.

| Option                | Description                                                 | Selected |
| --------------------- | ----------------------------------------------------------- | -------- |
| Immediate, no confirm | Remove from all tools immediately. Tooltip explains action. | ✓        |
| Confirm first         | Small confirmation popover before removing                  |          |

**User's choice:** Immediate, no confirmation
**Notes:** Fast workflow, consistent with bulk button behavior.

| Option           | Description                                  | Selected |
| ---------------- | -------------------------------------------- | -------- |
| Always visible   | Show on every card, greyed out if no targets | ✓        |
| Only when synced | Show only when skill has active tool targets |          |
| Hover reveal     | Show on hover only                           |          |

**User's choice:** Always visible
**Notes:** Discoverable, consistent placement.

---

## Missing Project Detection

| Option                    | Description                                | Selected |
| ------------------------- | ------------------------------------------ | -------- |
| On list load              | Check paths on tab mount + after mutations | ✓        |
| List load + periodic poll | Check on load + every 60s while tab open   |          |
| On-demand only            | Check only on explicit user action         |          |

**User's choice:** On list load
**Notes:** Catches missing dirs on every visit, no overhead from polling.

| Option                        | Description                                            | Selected |
| ----------------------------- | ------------------------------------------------------ | -------- |
| Warning badge + disabled sync | Yellow triangle, "(not found)" subtitle, sync disabled | ✓        |
| Error badge + fully disabled  | Red badge, dimmed, not selectable                      |          |
| Toast + subtle indicator      | Toast notification + subtle row indicator              |          |

**User's choice:** Warning badge + disabled sync
**Notes:** Project remains selectable for inspection/removal but can't sync.

| Option                     | Description                                | Selected |
| -------------------------- | ------------------------------------------ | -------- |
| Remove only                | Just "Remove Project" button               |          |
| Remove + re-point path     | Remove and "Update Path" via folder picker | ✓        |
| Remove + re-point + ignore | Remove, re-point, and dismiss warning      |          |

**User's choice:** Remove + re-point path
**Notes:** Useful for moved directories — preserves assignment history.

---

## Orphaned Assignments

| Option                       | Description                                            | Selected |
| ---------------------------- | ------------------------------------------------------ | -------- |
| Keep CASCADE, no indicator   | Assignments auto-deleted with skill. No missing state. | ✓        |
| SET NULL + missing indicator | Orphaned assignments remain with NULL skill_id         |          |
| Pre-delete mark + CASCADE    | Mark missing before cascade                            |          |

**User's choice:** Keep CASCADE, no indicator
**Notes:** Simplest — assignments just disappear from matrix on refresh.

| Option                       | Description                                | Selected |
| ---------------------------- | ------------------------------------------ | -------- |
| CASCADE + leave files        | Cascade delete, leave files in tool dirs   |          |
| Clean up tool dirs on delete | Remove files from tool dirs before cascade | ✓        |

**User's choice:** Clean up tool directories on delete
**Notes:** Clean removal of deployed artifacts before DB cleanup.

| Option                        | Description                                           | Selected |
| ----------------------------- | ----------------------------------------------------- | -------- |
| Redefine as cleanup-on-delete | INFR-03 = clean up deployed artifacts on skill delete | ✓        |
| Keep original INFR-03 spec    | Implement SET NULL + missing indicators               |          |

**User's choice:** Redefine as cleanup-on-delete
**Notes:** CASCADE makes true orphaned assignments impossible. INFR-03 redefined to cover filesystem cleanup.

---

## Claude's Discretion

- Exact toolbar layout and spacing for auto-sync toggle + unsync button
- Backend command naming conventions for new commands
- CSS styling details for warning badges and disabled states
- Test coverage scope for new features

## Deferred Ideas

None — discussion stayed within phase scope.
