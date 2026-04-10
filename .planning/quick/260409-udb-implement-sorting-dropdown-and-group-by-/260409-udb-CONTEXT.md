# Quick Task 260409-udb: Sorting dropdown + group-by-repo on My Skills & Projects - Context

**Gathered:** 2026-04-10
**Status:** Ready for planning

<domain>
## Task Boundary

Implement a sorting dropdown and group-by-repo checkbox on the My Skills page and the Projects assignment matrix. Default sort is A-Z by name. When grouped by repo, repos are sorted A-Z and skills sort by the selected sort order within each group.

</domain>

<decisions>
## Implementation Decisions

### Sort Menu Options

- Three sort options: Name (A-Z), Last Updated, Date Added
- Default sort: Name (A-Z) — changing current default from "Updated"
- Same dropdown on both My Skills FilterBar and Projects assignment matrix

### Group-by-Repo Definition

- Grouping key: `source_ref` field on ManagedSkill
- Skills with no `source_ref` (null/empty) go under an "Ungrouped" header
- When grouped: repo headers sorted A-Z, skills within each group sorted by selected sort order

### Projects Page Scope

- Sort/group controls apply to the **assignment matrix skill rows**, NOT the project list sidebar
- Full parity with My Skills: same 3 sort options + same group-by-repo checkbox

</decisions>

<specifics>
## Specific Ideas

- Existing FilterBar already has sort dropdown structure — extend it with third option + checkbox
- Assignment matrix needs a new toolbar area with sort dropdown + group-by-repo checkbox
- `source_ref` contains git URLs (e.g., `https://github.com/org/repo`) or local paths — use as-is for grouping label

</specifics>
