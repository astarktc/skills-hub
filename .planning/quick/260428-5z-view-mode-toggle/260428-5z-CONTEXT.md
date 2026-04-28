# Quick Task 260428-5z: Add view mode toggle to My Skills page - Context

**Gathered:** 2026-04-28
**Status:** Ready for planning

<domain>
## Task Boundary

Add a view mode toggle (dropdown) to the My Skills page allowing users to switch between List (current full-width), Auto Grid (CSS grid auto-fill, ~300px min column), and Dense Grid (~180px min column). Reorganize the FilterBar toolbar into two rows. Persist the view mode selection in localStorage across sessions.

</domain>

<decisions>
## Implementation Decisions

### Grid Card Layout

- Full card, narrower: cards keep all existing content (description, tool badges, source, timestamps, action buttons) — just narrower. Action buttons remain stacked vertically on the right. This is a resizing/reflow exercise, not a content reduction.

### View Mode Options

- Three modes: **List** (current full-width), **Auto Grid** (CSS grid auto-fill with ~300px minimum column width), **Dense Grid** (CSS grid auto-fill with ~180px minimum column width)
- Auto columns adapt to container width rather than fixed column counts
- Sort order applies left-to-right, top-to-bottom in grid modes

### Toolbar Split

- **Row 1 (Actions):** Auto-sync checkbox, Unsync All button, Search box, Refresh button
- **Row 2 (Filters):** Sort dropdown, Group by Repo checkbox, View dropdown
- Both rows left-aligned with consistent spacing

### Claude's Discretion

- CSS implementation details (flex vs grid for toolbar rows)
- Exact min-width breakpoints for auto-fill if ~300px/~180px don't look right in practice
- View dropdown styling (match existing sort dropdown pattern)
- localStorage key naming convention (follow existing patterns)

</decisions>

<specifics>
## Specific Ideas

- View mode dropdown should follow the same visual pattern as the existing Sort dropdown (styled select with label)
- Persistence should use localStorage like the existing group-by-repo checkbox
- Grid view must work correctly both with and without group-by-repo enabled
- When grouped by repo, each group's skills should flow in the grid layout independently

</specifics>
