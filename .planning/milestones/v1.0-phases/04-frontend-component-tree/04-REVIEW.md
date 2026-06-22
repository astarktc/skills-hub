---
phase: 04-frontend-component-tree
reviewed: 2026-04-08T21:18:00Z
depth: standard
files_reviewed: 15
files_reviewed_list:
  - src-tauri/src/commands/projects.rs
  - src-tauri/src/lib.rs
  - src/App.css
  - src/App.tsx
  - src/components/projects/AddProjectModal.tsx
  - src/components/projects/AssignmentMatrix.tsx
  - src/components/projects/EditProjectModal.tsx
  - src/components/projects/ProjectList.tsx
  - src/components/projects/ProjectsPage.tsx
  - src/components/projects/RemoveProjectModal.tsx
  - src/components/projects/ToolConfigModal.tsx
  - src/components/projects/types.ts
  - src/components/projects/useProjectState.ts
  - src/components/skills/Header.tsx
  - src/i18n/resources.ts
findings:
  critical: 1
  warning: 5
  info: 3
  total: 9
status: issues_found
---

# Phase 04: Code Review Report

**Reviewed:** 2026-04-08T21:18:00Z
**Depth:** standard
**Files Reviewed:** 15
**Status:** issues_found

## Summary

Phase 04 adds a Projects tab with a well-structured component tree: `useProjectState` hook centralizes project-related state (keeping it out of App.tsx per project constraints), `ProjectsPage` orchestrates sub-components, and backend commands follow the established layering pattern. The code quality is generally good, with proper `memo()` usage, `useCallback`/`useMemo` where appropriate, stale-result discard via version counter, and consistent error handling.

Key concerns: one bug in the gitignore block removal logic that can corrupt file content, a stale-closure issue in the assignment matrix, a ToolConfigModal that resets selection state on re-open, and a few missing error handling paths.

## Critical Issues

### CR-01: Gitignore `remove_block` closure can corrupt file content when the block is at the end of a file

**File:** `src-tauri/src/commands/projects.rs:408-427`
**Issue:** The `remove_block` closure detects the end of the Skills Hub block by looking for an empty line or the last line of the file. However, when the block immediately precedes non-empty content (e.g., other gitignore patterns with no blank line separator), the end detection at line 417 never triggers for lines that are non-empty and not at the end of the file. This means the block removal silently fails (no drain occurs), leaving the file unchanged.

More critically, when `start` is found and the marker line itself is the `i > s` condition trigger on the same line (impossible since `i > s` requires at least one line gap), the logic on line 417 checks `i > s` -- but the marker line sets `start` at line 413 and immediately enters the `end` check at line 416 on the same iteration. Since `i == s` (not `i > s`), the end check is skipped for the marker line. If the next line is a pattern like `/.claude/skills/` (non-empty), and it is followed by more non-empty lines until EOF, then `end` is never set and `drain` never runs.

Consider a gitignore like:

```
# Some existing content
# Skills Hub -- managed skill directories
/.claude/skills/
/.cursor/skills/
```

Here `start = 1` (line index of marker, or 0 if previous line is blank). The loop sees lines 2 and 3 as non-empty and not at EOF. Line 3 (`/.cursor/skills/`) IS the last line (`i == lines.len() - 1`), so `end = Some(4)`. This works. But if the file ends:

```
# Skills Hub -- managed skill directories
/.claude/skills/
more-stuff
```

Line 2 (`more-stuff`) is the last line, so `end = Some(3)`. This drains the entire block including `more-stuff` -- which is unrelated content that happened to follow the block without a blank separator.

**Fix:** Use an explicit block end sentinel (e.g., a closing comment like `# /Skills Hub`) instead of relying on blank-line heuristics. Alternatively, count only the marker line plus immediately following lines that start with `/`:

```rust
let remove_block = |content: &str| -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut start = None;
    let mut end = None;
    for (i, line) in lines.iter().enumerate() {
        if line.contains(marker) {
            // Include preceding blank line if present
            start = Some(if i > 0 && lines[i - 1].trim().is_empty() { i - 1 } else { i });
        }
        if start.is_some() && end.is_none() && i > start.unwrap() {
            // Block continues while lines look like gitignore patterns for our dirs
            if line.trim().is_empty() || !line.starts_with('/') {
                end = Some(i);
                break;
            }
        }
    }
    // If we found start but not end, the block runs to EOF
    if start.is_some() && end.is_none() {
        end = Some(lines.len());
    }
    // ... drain and rejoin
};
```

## Warnings

### WR-01: `toggleAssignment` reads stale `assignments` array from closure

**File:** `src/components/projects/useProjectState.ts:179`
**Issue:** The `toggleAssignment` callback captures `assignments` in its dependency array (line 222), but the check on line 179 (`assignments.some(...)`) reads the `assignments` value from when the callback was last memoized. If two rapid checkbox toggles occur before the state update from the first toggle propagates, the second toggle sees stale data and may attempt to add an assignment that already exists (or remove one that was already removed). The backend has a guard (`ASSIGNMENT_EXISTS`), but the user sees an error toast for what should be an idempotent operation.

**Fix:** Use a functional state update or a ref to track the latest assignments, or disable the checkbox while `pendingCells` contains the key (the component already shows a spinner, but the `onChange` handler on line 259 of AssignmentMatrix.tsx can still fire since `disabled={isPending}` is only set when `isPending` is true -- but the checkbox is rendered conditionally so this is partially mitigated). To fully fix, add a guard at the top of `toggleAssignment`:

```typescript
const toggleAssignment = useCallback(
  async (skillId: string, tool: string) => {
    if (!selectedProjectId) return;
    const key = `${skillId}:${tool}`;
    // Prevent double-toggle while a pending operation is in flight
    if (pendingCells.has(key)) return;
    // ... rest of logic
  },
  [selectedProjectId, assignments, loadProjects, pendingCells],
);
```

### WR-02: `ToolConfigModal` loses selection state when re-opened for an existing project

**File:** `src/components/projects/ToolConfigModal.tsx:40-42`
**Issue:** `ToolConfigModalInner` initializes `selectedTools` via `useState(() => buildInitialSelection(toolStatus, currentTools))`. The `useState` initializer runs only once when the component mounts. Since the outer `ToolConfigModal` uses `if (!open) return null` (unmounting on close), re-opening the modal re-mounts the inner component -- which means the initial selection is rebuilt correctly. However, `buildInitialSelection` on lines 6-20 merges `toolStatus.installed` AND `currentTools`. This means when re-opening to configure an existing project that already has tools configured, ALL installed tools are pre-checked (even ones deliberately not added to this project). Users may accidentally add unwanted tools.

**Fix:** When `currentTools` is non-empty (editing an existing project), use only `currentTools` as the initial selection rather than merging with all installed tools:

```typescript
function buildInitialSelection(
  toolStatus: ToolStatusDto | null,
  currentTools: ProjectToolDto[],
): Set<string> {
  // If project already has tools configured, use those as baseline
  if (currentTools.length > 0) {
    return new Set(currentTools.map((ct) => ct.tool));
  }
  // For new projects with no tools yet, pre-select installed tools
  const initial = new Set<string>();
  if (toolStatus) {
    for (const key of toolStatus.installed) {
      initial.add(key);
    }
  }
  return initial;
}
```

### WR-03: `AddProjectModal` resets state after `handleSubmit` even if `onRegister` throws

**File:** `src/components/projects/AddProjectModal.tsx:46-51`
**Issue:** The `handleSubmit` function calls `await onRegister(path, ...)` and then unconditionally resets local state (`setPath("")`, etc.) on lines 48-50. If `onRegister` throws (e.g., the path does not exist, is not writable, etc.), the catch is in the parent `handleAddProject` which shows a toast, but the modal's path input is already cleared. The user loses their input and must re-type/re-browse the path.

**Fix:** Only reset state on success:

```typescript
const handleSubmit = async () => {
  try {
    await onRegister(path, { addToGitignore, addToExclude });
    setPath("");
    setAddToGitignore(false);
    setAddToExclude(false);
  } catch {
    // Parent handles the error toast; preserve input state
  }
};
```

### WR-04: `AssignmentMatrix` wraps itself in `memo()` but `project` prop is a new object reference on every render

**File:** `src/components/projects/ProjectsPage.tsx:216-219`
**Issue:** On line 216-219, the `project` prop is computed inline as `state.projects.find(p => p.id === state.selectedProjectId) ?? null`. The `find` call returns the same object reference from the array each time (since the array identity changes only after loadProjects), but the parent `AssignmentMatrix` wrapper at line 287 of AssignmentMatrix.tsx uses `memo()`. The `memo` is ineffective because the `pendingCells` prop is a `Set<string>` created via `new Set(prev)` on every state update (lines 173-177 of useProjectState.ts), which is always a new reference. This means every pending cell change re-renders the entire matrix (all rows), not just the affected cell.

**Fix:** Move `pendingCells` to a stable reference (e.g., using `useMemo` or converting to a serialized key string for comparison), or use a custom `areEqual` comparator with `memo` that compares `pendingCells` by contents rather than reference. Alternatively, pass pending state down through context so individual `MatrixRow` components can subscribe to just their slice.

### WR-05: `EditProjectModal` directly imports `invoke` from `@tauri-apps/api/core` instead of going through `invokeTauri`

**File:** `src/components/projects/EditProjectModal.tsx:2`
**Issue:** The `EditProjectModal` imports `invoke` directly from `@tauri-apps/api/core` at the module level. The existing codebase convention (see App.tsx lines 129-138) uses a lazy dynamic import of `invoke` wrapped in `invokeTauri` to support non-Tauri environments gracefully. While `EditProjectModal` is only rendered inside the Tauri app, this breaks the established pattern and will cause a module-level import error if the component is ever loaded outside a Tauri context (e.g., in a future test harness or Storybook).

The same issue exists in `ProjectsPage.tsx` (line 3) and `useProjectState.ts` (line 2).

**Fix:** Either pass `invokeTauri` from the parent (via props or context) consistent with the App.tsx pattern, or accept the direct import as a deliberate convention for the projects subtree since it always runs inside Tauri. If accepting, add a brief comment documenting the decision.

## Info

### IN-01: Chinese translations missing for `navProjects` and the `projects` namespace

**File:** `src/i18n/resources.ts`
**Issue:** The `zh` translation block has no `navProjects` key and no `projects` namespace. When the UI language is Chinese, i18next will fall back to English strings. Per the project constraints ("i18n: English strings only -- Chinese deferred"), this is expected behavior for phase 04. Flagging for tracking so it is not forgotten.

**Fix:** Add Chinese translations for the `projects` namespace when Chinese support is prioritized.

### IN-02: `ProjectsPage` wraps in `memo()` but has no props

**File:** `src/components/projects/ProjectsPage.tsx:288`
**Issue:** `ProjectsPage` is exported as `memo(ProjectsPage)` but accepts zero props -- its only external dependency is `useTranslation()`. Wrapping a propless component in `memo()` has no effect since `memo` compares props, and there are no props to compare. This is harmless but unnecessarily misleading.

**Fix:** Remove the `memo()` wrapper from `ProjectsPage` since it provides no benefit.

### IN-03: ToolConfigModal "(installed)" badge text is hardcoded in English

**File:** `src/components/projects/ToolConfigModal.tsx:95`
**Issue:** The string `" (installed)"` on line 95 is a hardcoded English literal rather than an i18n key. While Chinese translations for the projects feature are deferred, this should use `t('projects.installed')` or similar for consistency and future-proofing.

**Fix:** Replace with `t('status.installed')` (which already exists in the i18n resources) or a dedicated key:

```tsx
<span className="pick-item-badge"> ({t("status.installed")})</span>
```

---

_Reviewed: 2026-04-08T21:18:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
