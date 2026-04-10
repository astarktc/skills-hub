---
phase: quick-260409-udb
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/components/skills/FilterBar.tsx
  - src/components/skills/SkillsList.tsx
  - src/components/projects/AssignmentMatrix.tsx
  - src/App.tsx
  - src/App.css
  - src/i18n/resources.ts
autonomous: true
must_haves:
  truths:
    - "My Skills page shows sort dropdown with three options: Name A-Z, Last Updated, Date Added"
    - "My Skills page default sort is Name A-Z (not Updated)"
    - "My Skills page has group-by-repo checkbox that groups skills under repo headers"
    - "Projects assignment matrix has identical sort dropdown and group-by-repo checkbox"
    - "Ungrouped skills (no source_ref) appear under an Ungrouped header when grouping is active"
    - "Repo headers sort A-Z, skills within each group sort by the selected sort order"
  artifacts:
    - path: "src/components/skills/FilterBar.tsx"
      provides: "Extended sort type with 'added' option and groupByRepo checkbox"
    - path: "src/components/skills/SkillsList.tsx"
      provides: "Grouped rendering with repo section headers"
    - path: "src/components/projects/AssignmentMatrix.tsx"
      provides: "Sort dropdown + group-by-repo checkbox in matrix toolbar, grouped table rows"
    - path: "src/i18n/resources.ts"
      provides: "i18n keys for sortAdded, groupByRepo, ungrouped"
  key_links:
    - from: "src/App.tsx"
      to: "src/components/skills/FilterBar.tsx"
      via: "sortBy and groupByRepo state props"
      pattern: "sortBy.*groupByRepo"
    - from: "src/App.tsx"
      to: "src/components/skills/SkillsList.tsx"
      via: "visibleSkills computed with sort+group logic"
      pattern: "visibleSkills"
    - from: "src/components/projects/AssignmentMatrix.tsx"
      to: "skills prop"
      via: "local sortBy/groupByRepo state with useMemo sorting"
      pattern: "sortedSkills|groupedSkills"
---

<objective>
Add a three-option sort dropdown (Name A-Z, Last Updated, Date Added) and a group-by-repo
checkbox to the My Skills FilterBar and the Projects assignment matrix. Change the default
sort from "Updated" to "Name A-Z". When grouping is active, skills are grouped under
source_ref headers (repos sorted A-Z, skills within by selected sort order). Skills with
no source_ref appear under an "Ungrouped" header.

Purpose: Give users control over skill list ordering and visual grouping by repository
origin, on both the My Skills and Projects pages.

Output: Modified FilterBar, SkillsList, AssignmentMatrix, App.tsx state, CSS, and i18n keys.
</objective>

<execution_context>
@/home/alexwsl/skills-hub/.claude/get-shit-done/workflows/execute-plan.md
@/home/alexwsl/skills-hub/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src/components/skills/FilterBar.tsx
@src/components/skills/SkillsList.tsx
@src/components/skills/SkillCard.tsx
@src/components/skills/types.ts
@src/components/projects/AssignmentMatrix.tsx
@src/components/projects/ProjectsPage.tsx
@src/components/projects/types.ts
@src/App.tsx
@src/App.css
@src/i18n/resources.ts

<interfaces>
<!-- Key types the executor needs -->

From src/components/skills/types.ts:

```typescript
export type ManagedSkill = {
  id: string;
  name: string;
  description?: string | null;
  source_type: string;
  source_ref?: string | null;
  central_path: string;
  created_at: number;
  updated_at: number;
  last_sync_at?: number | null;
  status: string;
  targets: {
    tool: string;
    mode: string;
    status: string;
    target_path: string;
    synced_at?: number | null;
  }[];
};
```

Current sort type in App.tsx (line 106):

```typescript
const [sortBy, setSortBy] = useState<"updated" | "name">("updated");
```

Current FilterBar sort type:

```typescript
sortBy: "updated" | "name"
onSortChange: (value: "updated" | "name") => void
```

</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Extend FilterBar sort options, add group-by-repo, wire into App.tsx and SkillsList</name>
  <files>
    src/App.tsx
    src/components/skills/FilterBar.tsx
    src/components/skills/SkillsList.tsx
    src/i18n/resources.ts
    src/App.css
  </files>
  <action>
1. **Widen the sort type union** from `"updated" | "name"` to `"name" | "updated" | "added"` across App.tsx (state declaration, handleSortChange, visibleSkills memo) and FilterBar.tsx (props type, select options).

2. **Change default sort** in App.tsx from `useState<...>("updated")` to `useState<...>("name")`.

3. **Add groupByRepo state** in App.tsx: `const [groupByRepo, setGroupByRepo] = useState(false)`. Pass `groupByRepo` and `onGroupByRepoChange` as new props to FilterBar.

4. **FilterBar.tsx changes:**
   - Add `groupByRepo: boolean` and `onGroupByRepoChange: (value: boolean) => void` to `FilterBarProps`.
   - Add a third `<option value="added">{t("sortAdded")}</option>` to the sort select.
   - Update the display label logic: add `sortBy === "added" ? t("sortAdded") :` to the ternary.
   - Add a checkbox after the sort button (before search) for group-by-repo:
     ```jsx
     <label className="group-by-repo-toggle" title={t("groupByRepo")}>
       <input
         type="checkbox"
         checked={groupByRepo}
         onChange={(e) => onGroupByRepoChange(e.target.checked)}
       />
       <span className="group-by-repo-label">{t("groupByRepo")}</span>
     </label>
     ```
   - Update the memo comparison to include `groupByRepo` and `onGroupByRepoChange`.

5. **Update visibleSkills memo** in App.tsx to handle the new sort option:
   - `sortBy === "name"` -> `a.name.localeCompare(b.name)`
   - `sortBy === "added"` -> `(b.created_at ?? 0) - (a.created_at ?? 0)` (newest first)
   - default ("updated") -> `(b.updated_at ?? 0) - (a.updated_at ?? 0)` (existing logic)

6. **Pass `groupByRepo` to SkillsList** as a new prop.

7. **SkillsList.tsx changes:**
   - Add `groupByRepo: boolean` to `SkillsListProps`.
   - When `groupByRepo` is true, group `visibleSkills` by `source_ref` (using `?? ""` for null/undefined). Render each group with a header row showing the source_ref value (or the i18n key `ungrouped` for the empty-string key). Repo groups sorted A-Z, with the "Ungrouped" group at the end.
   - The grouped rendering wraps skills in `<div className="repo-group">` with a `<div className="repo-group-header">` containing the repo label and skill count. Skills render normally within each group.
   - When `groupByRepo` is false, render flat list as currently done (no behavioral change).

8. **i18n resources.ts** -- add to English `translation` object:
   - `sortAdded: "Date added"`
   - `groupByRepo: "Group by repo"`
   - `ungrouped: "Ungrouped"`
     Add to Chinese `translation` object:
   - `sortAdded: "Date added"`
   - `groupByRepo: "Group by repo"`
   - `ungrouped: "Ungrouped"`
     (English-only per project constraints; Chinese gets same English strings as placeholders.)

9. **App.css** -- add styles:
   ```css
   .group-by-repo-toggle {
     display: flex;
     align-items: center;
     gap: 6px;
     cursor: pointer;
     font-size: 13px;
   }
   .group-by-repo-label {
     color: var(--text-secondary);
     white-space: nowrap;
   }
   .repo-group {
     margin-bottom: 8px;
   }
   .repo-group-header {
     display: flex;
     align-items: center;
     gap: 8px;
     padding: 8px 0 4px 0;
     font-size: 13px;
     font-weight: 500;
     color: var(--text-secondary);
     font-family: var(--font-mono);
     border-bottom: 1px solid var(--border-subtle);
     margin-bottom: 4px;
   }
   .repo-group-header .repo-count {
     font-weight: 400;
     color: var(--text-tertiary);
     font-size: 12px;
   }
   ```
     </action>
     <verify>
       <automated>cd /home/alexwsl/skills-hub && npm run build</automated>
     </verify>
     <done>
       - FilterBar shows three sort options: Name A-Z, Most recent (Last Updated), Date added
       - Default sort is "name" (A-Z) instead of "updated"
       - Group-by-repo checkbox appears in FilterBar
       - When checked, SkillsList groups skills under repo headers sorted A-Z with skills sorted by selected order within each group
       - Skills with no source_ref appear under "Ungrouped" at the end
       - TypeScript compiles with no errors
     </done>
   </task>

<task type="auto">
  <name>Task 2: Add sort dropdown and group-by-repo to Projects AssignmentMatrix</name>
  <files>
    src/components/projects/AssignmentMatrix.tsx
    src/App.css
  </files>
  <action>
1. **Add local state** inside the AssignmentMatrix component (NOT in useProjectState -- this is UI-only view state):
   ```typescript
   const [sortBy, setSortBy] = useState<"name" | "updated" | "added">("name");
   const [groupByRepo, setGroupByRepo] = useState(false);
   ```

2. **Add a sort/group controls row** in the matrix toolbar, between the info section and the action buttons. Create a new `<div className="matrix-toolbar-filters">` containing:
   - A sort button matching FilterBar's pattern (button with overlaid select, using `ArrowUpDown` icon from lucide-react):
     ```jsx
     <button className="btn btn-secondary btn-sm sort-btn" type="button">
       <span className="sort-label">{t("filterSort")}:</span>
       {sortBy === "name" ? t("sortName") : sortBy === "added" ? t("sortAdded") : t("sortUpdated")}
       <ArrowUpDown size={12} />
       <select aria-label={t("filterSort")} value={sortBy} onChange={(e) => setSortBy(e.target.value as "name" | "updated" | "added")}>
         <option value="name">{t("sortName")}</option>
         <option value="updated">{t("sortUpdated")}</option>
         <option value="added">{t("sortAdded")}</option>
       </select>
     </button>
     ```
   - A group-by-repo checkbox:
     ```jsx
     <label className="group-by-repo-toggle">
       <input
         type="checkbox"
         checked={groupByRepo}
         onChange={(e) => setGroupByRepo(e.target.checked)}
       />
       <span className="group-by-repo-label">{t("groupByRepo")}</span>
     </label>
     ```

3. **Compute sortedSkills via useMemo** that sorts `skills` by `sortBy`:
   - `"name"` -> `a.name.localeCompare(b.name)`
   - `"added"` -> `(b.created_at ?? 0) - (a.created_at ?? 0)`
   - `"updated"` -> `(b.updated_at ?? 0) - (a.updated_at ?? 0)`

4. **When groupByRepo is true**, compute grouped skills from `sortedSkills`:
   - Build a `Map<string, ManagedSkill[]>` keyed on `skill.source_ref ?? ""`.
   - Sort map keys A-Z, but place the empty-string key last.
   - Render each group as a `<tr>` with a `<td colSpan={tools.length + 2}>` containing the repo header label (or `t("ungrouped")` for empty key), followed by `<MatrixRow>` entries for that group's skills.
   - The group header row uses class `matrix-group-header-row`.

5. **When groupByRepo is false**, render `sortedSkills` flat using `<MatrixRow>` as currently done -- just replace the `skills.map(...)` with `sortedSkills.map(...)`.

6. **Import ArrowUpDown** from lucide-react (add to existing import).

7. **App.css** -- add matrix-specific group styles:

   ```css
   .matrix-toolbar-filters {
     display: flex;
     align-items: center;
     gap: 12px;
   }
   .matrix-group-header-row td {
     padding: 10px 8px 4px;
     font-size: 13px;
     font-weight: 500;
     color: var(--text-secondary);
     font-family: var(--font-mono);
     border-bottom: 1px solid var(--border-subtle);
     background: var(--bg-panel);
   }
   ```

8. **Update the memo comparison** of the outer AssignmentMatrix memo: no changes needed because sortBy/groupByRepo are local state (inside the component, not props). However, ensure the component still compares `skills` by reference as it already does.
   </action>
   <verify>
   <automated>cd /home/alexwsl/skills-hub && npm run check</automated>
   </verify>
   <done> - AssignmentMatrix toolbar shows sort dropdown with Name A-Z, Most recent, Date added options - AssignmentMatrix toolbar shows group-by-repo checkbox - Matrix skill rows are sorted by the selected sort order - When group-by-repo is checked, matrix rows are grouped under repo header rows (A-Z, Ungrouped last) - Skills within each group are sorted by selected sort order - Full `npm run check` passes (lint + build + rust:fmt:check + rust:clippy + rust:test)
   </done>
   </task>

</tasks>

<threat_model>

## Trust Boundaries

No new trust boundaries introduced. This is a frontend-only UI change with no new IPC commands, no new data inputs, and no user-supplied data flowing to the backend.

## STRIDE Threat Register

| Threat ID  | Category                   | Component          | Disposition | Mitigation Plan                                                                                         |
| ---------- | -------------------------- | ------------------ | ----------- | ------------------------------------------------------------------------------------------------------- |
| T-quick-01 | I (Information Disclosure) | source_ref display | accept      | source_ref is already shown in skill cards; grouping headers expose the same data the user already sees |

</threat_model>

<verification>
1. `npm run check` passes (lint + build + Rust checks)
2. My Skills page: sort dropdown has 3 options, default is Name A-Z
3. My Skills page: group-by-repo checkbox groups skills under repo headers
4. Projects page: assignment matrix has matching sort dropdown + group-by-repo checkbox
5. Both pages: ungrouped skills appear under "Ungrouped" header when grouping is active
6. Both pages: repo headers sort A-Z, skills within by selected sort order
</verification>

<success_criteria>

- Three sort options (Name A-Z, Last Updated, Date Added) on both My Skills and Projects pages
- Default sort changed from "Updated" to "Name A-Z"
- Group-by-repo checkbox functional on both pages
- Grouped mode: repo headers A-Z, skills sorted within by selected order, null/empty source_ref under "Ungrouped" at end
- `npm run check` passes
  </success_criteria>

<output>
After completion, create `.planning/quick/260409-udb-implement-sorting-dropdown-and-group-by-/260409-udb-SUMMARY.md`
</output>
