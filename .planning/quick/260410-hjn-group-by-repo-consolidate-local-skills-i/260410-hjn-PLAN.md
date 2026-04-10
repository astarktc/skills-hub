---
phase: quick
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/components/skills/SkillsList.tsx
  - src/components/projects/AssignmentMatrix.tsx
  - src/App.css
  - src/i18n/resources.ts
autonomous: true
requirements: []
must_haves:
  truths:
    - "Local (non-git) skills are grouped under a single 'Local' header in both My Skills and Project Assignment Matrix when group-by-repo is on"
    - "Skill names are visually indented under group headers in the Project Assignment Matrix"
    - "Group header icons display inline-left of the repo name in the matrix, not stacked above"
    - "The 'All tools' button is hidden when only one tool is configured for the project"
  artifacts:
    - path: "src/components/skills/SkillsList.tsx"
      provides: "Consolidated local skill grouping"
    - path: "src/components/projects/AssignmentMatrix.tsx"
      provides: "Consolidated local grouping, indent, hidden all-tools button"
    - path: "src/App.css"
      provides: "Matrix indent and inline icon layout styles"
    - path: "src/i18n/resources.ts"
      provides: "New i18n key for Local group label"
  key_links:
    - from: "src/components/skills/SkillsList.tsx"
      to: "src/i18n/resources.ts"
      via: "t('localGroup') i18n key"
      pattern: "t\\('localGroup'\\)"
    - from: "src/components/projects/AssignmentMatrix.tsx"
      to: "src/i18n/resources.ts"
      via: "t('localGroup') i18n key"
      pattern: "t\\('localGroup'\\)"
---

<objective>
Fix 4 related UI issues in the "Group by repo" feature: consolidate local skills under one group, indent matrix skill rows, fix matrix group header icon layout, and hide the "All tools" button when only one tool is configured.

Purpose: Improve readability and usability of the group-by-repo presentation in both My Skills and Project Assignment Matrix views.
Output: Updated SkillsList, AssignmentMatrix, App.css, and i18n resources.
</objective>

<execution_context>
@/home/alexwsl/skills-hub/.claude/get-shit-done/workflows/execute-plan.md
@/home/alexwsl/skills-hub/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src/components/skills/SkillsList.tsx
@src/components/projects/AssignmentMatrix.tsx
@src/App.css
@src/i18n/resources.ts
@src/components/skills/types.ts
</context>

<tasks>

<task type="auto">
  <name>Task 1: Consolidate local skills grouping and add i18n key</name>
  <files>src/components/skills/SkillsList.tsx, src/components/projects/AssignmentMatrix.tsx, src/i18n/resources.ts</files>
  <action>
**i18n (src/i18n/resources.ts):**
Add a new key `localGroup: "Local"` in the English `translation` object, right after the existing `ungrouped` key (around line 39). Also add the same key in the Chinese section: `localGroup: "Local"` (English-only per project constraint for Chinese; use "Local" for both).

**SkillsList.tsx grouping logic (lines 47-73):**
In the `groups` useMemo, change the grouping key logic. Currently it groups by `skill.source_ref ?? ""`. Instead:

- If the skill's `source_ref` looks like a git URL (contains `github.com` or starts with `git+` or `https://`), use `source_ref` as the key (existing behavior).
- Otherwise (local files, empty source_ref, non-URL source_ref), use the fixed key `"__local__"` to consolidate all local/non-git skills into one group.

Replace the key derivation line `const key = skill.source_ref ?? "";` with:

```typescript
const ref = skill.source_ref ?? "";
const isGitUrl =
  ref.startsWith("git+") ||
  ref.startsWith("https://") ||
  ref.startsWith("http://") ||
  ref.includes("github.com");
const key = isGitUrl ? ref : "__local__";
```

Then in the label derivation (around line 64-68), change the label logic:

```typescript
const ghInfo = key !== "__local__" && key ? getGithubInfo(key) : null;
return {
  key,
  label:
    key === "__local__"
      ? t("localGroup")
      : ghInfo
        ? ghInfo.label
        : key || t("ungrouped"),
  href: ghInfo?.href ?? null,
  skills: map.get(key)!,
};
```

**AssignmentMatrix.tsx grouping logic (lines 96-121):**
Apply the same consolidation pattern in the `skillGroups` useMemo. Replace:

```typescript
const key = skill.source_ref ?? "";
```

with:

```typescript
const ref = skill.source_ref ?? "";
const isGitUrl =
  ref.startsWith("git+") ||
  ref.startsWith("https://") ||
  ref.startsWith("http://") ||
  ref.includes("github.com");
const key = isGitUrl ? ref : "__local__";
```

And in the label derivation (around line 115), update:

```typescript
const short = key !== "__local__" && key ? shortRepoLabel(key) : null;
return {
  key,
  label:
    key === "__local__" ? t("localGroup") : (short ?? key) || t("ungrouped"),
  skills: map.get(key)!,
};
```

  </action>
  <verify>
    <automated>cd /home/alexwsl/skills-hub && npm run build</automated>
  </verify>
  <done>All local/non-git skills consolidate into a single "Local" group in both SkillsList and AssignmentMatrix when group-by-repo is enabled. Git-sourced skills still group by their repository URL.</done>
</task>

<task type="auto">
  <name>Task 2: Fix matrix indent, icon layout, and conditional All-tools button</name>
  <files>src/components/projects/AssignmentMatrix.tsx, src/App.css</files>
  <action>
**CSS changes (src/App.css):**

1. **Indent skill names under group headers:** Add a new rule after the existing `.matrix-skill-cell` rule:

```css
.matrix-group-header-row
  ~ tr:not(.matrix-group-header-row)
  > .matrix-skill-cell {
  padding-left: 24px;
}
```

This uses the sibling combinator so that skill rows following a group header row get left-padding. However, since `~` would affect ALL subsequent rows not just those in the same group, a simpler approach is to add an explicit class. Instead, modify the `MatrixRow` component to accept a `grouped` boolean and conditionally add a class.

Actually, the cleanest approach: since ALL skill rows get indented when groupByRepo is active (every row is under some group), pass a prop to MatrixRow. Alternatively, add a parent class on the table. Simplest: add a class `matrix-grid-grouped` on the `matrix-grid` div when `groupByRepo` is true, and use CSS descendant selector:

```css
.matrix-grid-grouped .matrix-skill-cell {
  padding-left: 24px;
}
```

2. **Fix icon layout in group headers:** The current `.matrix-group-header-row td` does not enforce inline layout for its children. The `<GitBranch>` icon and text are direct children of `<td>`. Update the CSS to ensure inline-flex alignment:

```css
.matrix-group-header-row td {
  padding: 10px 8px 6px;
  font-size: 18px;
  font-weight: 1000;
  color: var(--text-primary);
  border-bottom: 2px solid var(--border-color);
  background: var(--bg-panel);
  display: flex;
  align-items: center;
  gap: 6px;
}
```

Wait -- `<td>` with `display: flex` can break table layout. Instead, wrap the icon + label inside a `<span>` with flex. Modify the JSX in AssignmentMatrix.tsx.

**JSX changes (src/components/projects/AssignmentMatrix.tsx):**

1. **Add grouped class to matrix-grid div (around line 276):** Change:

```tsx
<div className="matrix-grid">
```

to:

```tsx
<div className={`matrix-grid${groupByRepo ? ' matrix-grid-grouped' : ''}`}>
```

2. **Fix group header icon layout (around line 291-296):** Change the group header `<td>` content from:

```tsx
<td colSpan={tools.length + 2}>
  <GitBranch size={14} className="repo-group-icon" />
  {group.label}
</td>
```

to:

```tsx
<td colSpan={tools.length + 2}>
  <span className="matrix-group-label">
    <GitBranch size={14} className="repo-group-icon" />
    {group.label}
  </span>
</td>
```

3. **Hide "All tools" button when single tool:** Pass `tools.length` to `MatrixRow` (or better, a boolean `showBulkAssign`). Add `showBulkAssign: boolean` to `MatrixRowProps`. In the parent rendering of `MatrixRow`, add prop `showBulkAssign={tools.length > 1}`.

In the `MatrixRow` component, change the last `<td>` (around line 419-425):

```tsx
<td>
  {showBulkAssign && (
    <button
      className="btn btn-xs matrix-all-tools-btn"
      onClick={() => onBulkAssign(skill.id)}
      disabled={disabled}
    >
      {t("projects.allTools")}
    </button>
  )}
</td>
```

Update the `MatrixRowProps` type to include `showBulkAssign: boolean`.

Update the memo comparison function for `MatrixRow` to include `prev.showBulkAssign !== next.showBulkAssign`.

**CSS additions (src/App.css) -- add after the existing `.matrix-group-header-row .repo-group-icon` rule:**

```css
.matrix-grid-grouped .matrix-skill-cell {
  padding-left: 24px;
}

.matrix-group-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
```

Remove the now-redundant `.matrix-group-header-row .repo-group-icon` rule for `vertical-align` and `margin-right` since the flex container handles spacing. Replace it with:

```css
.matrix-group-header-row .repo-group-icon {
  color: var(--text-secondary);
  flex-shrink: 0;
}
```

  </action>
  <verify>
    <automated>cd /home/alexwsl/skills-hub && npm run check</automated>
  </verify>
  <done>
    - Skill names in the matrix are indented 24px under group headers when group-by-repo is on
    - Group header icons display inline-left of the label (flex layout), not stacked above
    - "All tools" button is hidden when only 1 tool is configured for the project
    - All lint, build, and Rust checks pass
  </done>
</task>

</tasks>

<verification>
1. `npm run check` passes (lint + build + rust checks)
2. Visual: Enable group-by-repo on My Skills page -- local skills appear under single "Local" group
3. Visual: Enable group-by-repo in Project Assignment Matrix -- local skills under single "Local" group, skill names indented, icons inline with group label
4. Visual: Configure a project with only 1 tool -- "All tools" button is not shown in matrix rows
5. Visual: Configure a project with 2+ tools -- "All tools" button appears normally
</verification>

<success_criteria>

- All 4 UI improvements implemented
- `npm run check` passes with zero errors
- No changes to backend/Rust code (frontend-only changes)
  </success_criteria>

<output>
After completion, create `.planning/quick/260410-hjn-group-by-repo-consolidate-local-skills-i/260410-hjn-SUMMARY.md`
</output>
