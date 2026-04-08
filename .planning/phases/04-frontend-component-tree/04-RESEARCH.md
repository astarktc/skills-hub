# Phase 4: Frontend Component Tree - Research

**Researched:** 2026-04-08
**Domain:** React 19 component architecture, Tauri IPC integration, custom hooks, CSS-in-App.css styling
**Confidence:** HIGH

## Summary

Phase 4 is a frontend-only phase that builds the entire Projects tab UI on top of existing backend commands from Phases 1-3. The domain is well-constrained: React 19 components following existing patterns (modal structure, `memo()` wrapping, CSS classes in App.css), a custom `useProjectState` hook (the first in this project) that owns all state and calls Tauri IPC commands directly, and integration with the existing Header nav tabs and App.tsx view switching.

The technical risk is low because: (1) the backend API surface is complete and stable (12 project commands registered in lib.rs), (2) all UI patterns have established references in the codebase (modals, page components, checkboxes), and (3) the component tree is fully specified by CONTEXT.md decisions D-01 through D-21. The primary implementation challenge is the assignment matrix -- a dynamic grid of skill rows x tool columns with per-cell async state management (pending/synced/stale/error).

**Primary recommendation:** Build the `useProjectState` hook first as the central orchestration layer, then the three main components (ProjectsPage, ProjectList, AssignmentMatrix), then the three modals (AddProjectModal, ToolConfigModal, RemoveProjectModal), and finally integrate into App.tsx/Header.tsx with minimal changes.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Split-panel layout -- project list on the left (~250px fixed width), assignment matrix on the right. Click a project in the left panel to load its matrix.
- **D-02:** Each project row in the left panel shows: name (basename per Phase 1 D-01), truncated path as subtitle, assignment count, and a colored dot for aggregate sync status.
- **D-03:** When no projects are registered (first-time), show a full-width empty state with icon/illustration, brief explanation of what project distribution does, and a prominent "Add Project" button.
- **D-04:** Right panel header is a toolbar containing: project name, path, and action buttons (Sync Project, Sync All, Add/Remove Tools).
- **D-05:** When no project is selected, the right panel shows placeholder text: "Select a project to manage skill assignments".
- **D-06:** Wait-for-backend toggle -- click checkbox shows pending state on that cell, waits for backend response, then updates to synced/error. No optimistic UI revert logic.
- **D-07:** Bulk-assign via row button -- "All Tools" button on each skill row. Calls `bulk_assign_skill` backend command. All tool cells in that row go to pending, then update individually per result.
- **D-08:** Per-cell status via colored cell background + checkbox: green tint (synced), yellow tint (stale), red tint (error), gray tint (pending/unchecked). Checkbox always visible.
- **D-09:** Sync errors displayed via hover tooltip on red-tinted cells showing the error message. Click red cell to retry sync.
- **D-10:** Add project via modal dialog (matches existing AddSkillModal pattern). Modal contains folder picker button + manual path input field.
- **D-11:** After registration, auto-select the new project in the left panel and immediately show tool configuration modal with auto-detection of installed tools (TOOL-02).
- **D-12:** Tool configuration is a checkbox modal listing all known tools. Installed tools are pre-checked (auto-detected). User unchecks tools they don't want. Confirm adds selected tools as matrix columns.
- **D-13:** Registration modal includes gitignore section with two checkboxes (both unchecked by default).
- **D-14:** Inline duplicate validation in the registration modal -- after path is entered/picked, check against registered projects immediately and show inline warning before save is allowed.
- **D-15:** Project removal via confirmation dialog (matches existing DeleteModal pattern) explaining that all synced symlinks/copies will be removed.
- **D-16:** Single `useProjectState()` custom hook owns all state: project list, selected project ID, assignments, tools, loading flags. Returns state + action functions. Components below are stateless presentational.
- **D-17:** Fetch on demand -- load project list on tab mount, load assignments + tools when a project is selected, re-fetch after any mutation.
- **D-18:** ProjectsPage is fully self-contained -- fetches its own skills list via `invoke('get_managed_skills')`. Zero props from App.tsx (except `t` for translations).
- **D-19:** App.tsx changes limited to: add `'projects'` to `activeView` union type, add Projects nav tab in Header.tsx, import and render `ProjectsPage` in the view switch.
- **D-20:** Three main components + hook, all in `src/components/projects/`: ProjectsPage.tsx, ProjectList.tsx, AssignmentMatrix.tsx, useProjectState.ts. Plus modals: AddProjectModal.tsx, ToolConfigModal.tsx, RemoveProjectModal.tsx.
- **D-21:** Inline loading states (skeleton/shimmer) within the matrix area and project list. No full-screen overlay.

### Claude's Discretion

- Exact CSS class names and styling details
- Internal hook helper decomposition
- Skeleton/shimmer implementation approach
- Whether to memoize matrix cells individually
- Exact modal form validation UX (field error messages, disabled states)
- i18n key naming convention for project strings

### Deferred Ideas (OUT OF SCOPE)

None -- discussion stayed within phase scope.
</user_constraints>

<phase_requirements>

## Phase Requirements

| ID      | Description                                                                                      | Research Support                                                                                                    |
| ------- | ------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------- |
| UI-01   | Projects tab appears in main navigation alongside existing tabs                                  | Header.tsx nav-tab pattern documented, `activeView` union type extension point identified at line 91 of App.tsx     |
| UI-02   | Project list panel (left) with add/remove project actions                                        | ProjectList component with `list_projects` command, `register_project` and `remove_project` for mutations           |
| UI-03   | Assignment matrix panel (right) with checkbox grid for selected project                          | AssignmentMatrix component consuming `list_project_tools` + `list_project_skill_assignments` + `get_managed_skills` |
| UI-04   | Sync status bar (bottom) with last sync time and Sync Project / Sync All buttons                 | Toolbar in right panel header (D-04) with `resync_project` and `resync_all_projects` commands                       |
| UI-05   | Projects tab uses its own component tree and state (isolated from App.tsx)                       | `useProjectState` hook pattern documented, direct `invoke` calls from hook, zero App.tsx state                      |
| SYNC-01 | Each assignment cell shows status: synced (green), stale (yellow), missing (red), pending (gray) | Status colors from CSS variables documented, cell tint mapping from UI-SPEC verified                                |
| TOOL-02 | Tool column picker auto-detects installed tools and pre-selects them on first setup              | Existing `get_tool_status` command returns `ToolStatusDto` with `installed[]` array                                 |
| TOOL-03 | User can add or remove tool columns from a project at any time                                   | `add_project_tool` and `remove_project_tool` commands documented with signatures                                    |

</phase_requirements>

## Standard Stack

### Core (already in project -- no new dependencies)

| Library                   | Version           | Purpose                                                       | Why Standard                                                           |
| ------------------------- | ----------------- | ------------------------------------------------------------- | ---------------------------------------------------------------------- |
| React                     | ^19.2.3           | UI framework                                                  | Project standard [VERIFIED: package.json]                              |
| TypeScript                | ~5.9.3            | Type safety                                                   | Project standard [VERIFIED: package.json]                              |
| @tauri-apps/api           | 2.9.1             | IPC invoke calls                                              | Project standard for backend communication [VERIFIED: package.json]    |
| @tauri-apps/plugin-dialog | 2.5.3             | Folder picker dialog                                          | Used for project directory selection [VERIFIED: package.json]          |
| lucide-react              | ^0.562.0          | Icons (FolderKanban, Plus, RefreshCw, Settings, Trash2, etc.) | Project standard icon library [VERIFIED: package.json]                 |
| sonner                    | ^2.0.7            | Toast notifications                                           | Project standard for success/error feedback [VERIFIED: package.json]   |
| i18next + react-i18next   | ^25.7.4 / ^16.5.3 | Translations                                                  | Project standard for all user-visible strings [VERIFIED: package.json] |

### Supporting (no new dependencies needed)

This phase requires zero new npm or cargo dependencies. All functionality is built with existing libraries.

### Alternatives Considered

| Instead of                    | Could Use                      | Tradeoff                                                                                                           |
| ----------------------------- | ------------------------------ | ------------------------------------------------------------------------------------------------------------------ |
| Custom `useProjectState` hook | Zustand/Jotai state library    | Project convention is useState-based; adding a library contradicts D-16 and CLAUDE.md                              |
| CSS-in-App.css                | CSS Modules or Tailwind-only   | Project uses App.css with semantic class names; changing would break consistency                                   |
| Inline tooltips for errors    | Radix/headless tooltip library | Project has no headless UI library; CSS-only tooltip with `title` attribute or `::after` pseudo-element is simpler |

**Installation:** No `npm install` needed. All dependencies are already present.

## Architecture Patterns

### Recommended Project Structure

```
src/components/projects/
  types.ts              # Already exists -- ProjectDto, ProjectToolDto, etc.
  useProjectState.ts    # Custom hook: all state + IPC calls + action functions
  ProjectsPage.tsx      # Container: orchestrates layout, passes hook state to children
  ProjectList.tsx       # Presentational: left panel, project cards
  AssignmentMatrix.tsx  # Presentational: right panel, toolbar + checkbox grid
  AddProjectModal.tsx   # Modal: folder picker + path input + gitignore checkboxes
  ToolConfigModal.tsx   # Modal: tool checkbox list with auto-detection
  RemoveProjectModal.tsx # Modal: confirmation dialog
```

[VERIFIED: matches D-20 from CONTEXT.md]

### Pattern 1: Custom Hook as Single State Owner (D-16)

**What:** A `useProjectState()` hook that encapsulates all project-related state and exposes it as a typed return object. All IPC calls (`invoke`) happen inside the hook. Components receive state/actions via destructuring.

**When to use:** This is the first custom hook in the project. It exists because D-16 mandates isolation from App.tsx and D-18 mandates zero props from App.tsx.

**Example:**

```typescript
// Source: derived from D-16, D-17, D-18 decisions + existing App.tsx invokeTauri pattern
import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  ProjectDto,
  ProjectToolDto,
  ProjectSkillAssignmentDto,
  ResyncSummaryDto,
  BulkAssignResultDto,
} from "./types";
import type { ManagedSkill, ToolStatusDto } from "../skills/types";

export type ProjectState = {
  // Data
  projects: ProjectDto[];
  selectedProjectId: string | null;
  tools: ProjectToolDto[];
  assignments: ProjectSkillAssignmentDto[];
  skills: ManagedSkill[];
  toolStatus: ToolStatusDto | null;

  // Loading flags
  projectsLoading: boolean;
  matrixLoading: boolean;
  pendingCells: Set<string>; // "skillId:tool" keys for cells in-flight

  // Modal state
  showAddModal: boolean;
  showToolConfigModal: boolean;
  showRemoveModal: boolean;
  removeTargetId: string | null;

  // Actions
  loadProjects: () => Promise<void>;
  selectProject: (id: string) => void;
  registerProject: (path: string) => Promise<ProjectDto>;
  removeProject: (id: string) => Promise<void>;
  toggleAssignment: (skillId: string, tool: string) => Promise<void>;
  bulkAssign: (skillId: string) => Promise<void>;
  resyncProject: () => Promise<void>;
  resyncAll: () => Promise<void>;
  loadToolStatus: () => Promise<void>;
  addTools: (tools: string[]) => Promise<void>;
  removeTools: (tools: string[]) => Promise<void>;
  setShowAddModal: (v: boolean) => void;
  setShowToolConfigModal: (v: boolean) => void;
  setShowRemoveModal: (v: boolean) => void;
  setRemoveTargetId: (id: string | null) => void;
};
```

**Key design decisions:**

- Uses `invoke` directly from `@tauri-apps/api/core`, not the App.tsx `invokeTauri` wrapper. This is acceptable because ProjectsPage is self-contained (D-18). [VERIFIED: `invoke` is the standard Tauri API, App.tsx wrapper just adds isTauri guard]
- `pendingCells` Set tracks which cells have in-flight requests for visual pending state (D-06)
- All mutation actions re-fetch relevant data after completion (D-17)

### Pattern 2: Wait-for-Backend Cell Toggle (D-06)

**What:** Clicking a matrix checkbox immediately sets the cell to "pending" visual state, fires the backend command, then transitions to "synced" or "error" based on the response. No optimistic update, no revert logic.

**When to use:** Every matrix cell interaction (assign/unassign/retry).

**Example:**

```typescript
// Source: derived from D-06, D-08, D-09 decisions
const toggleAssignment = useCallback(
  async (skillId: string, tool: string) => {
    const cellKey = `${skillId}:${tool}`;
    setPendingCells((prev) => new Set(prev).add(cellKey));

    try {
      const existing = assignments.find(
        (a) => a.skill_id === skillId && a.tool === tool,
      );
      if (existing) {
        await invoke("remove_project_skill_assignment", {
          projectId: selectedProjectId,
          skillId,
          tool,
        });
      } else {
        await invoke("add_project_skill_assignment", {
          projectId: selectedProjectId,
          skillId,
          tool,
        });
      }
      // Re-fetch assignments after mutation (D-17)
      await reloadAssignments();
    } catch (err) {
      // Error is captured in assignment status by backend
      // Re-fetch to get error state
      await reloadAssignments();
    } finally {
      setPendingCells((prev) => {
        const next = new Set(prev);
        next.delete(cellKey);
        return next;
      });
    }
  },
  [selectedProjectId, assignments],
);
```

### Pattern 3: Presentational Component with memo() Export

**What:** Every component below ProjectsPage receives data and callbacks via props and is exported wrapped in `memo()`.

**When to use:** All components: ProjectList, AssignmentMatrix, AddProjectModal, ToolConfigModal, RemoveProjectModal.

**Example:**

```typescript
// Source: existing pattern from SkillCard.tsx, ExplorePage.tsx, DeleteModal.tsx
import { memo } from "react";
import type { TFunction } from "i18next";
import type { ProjectDto } from "./types";

type ProjectListProps = {
  projects: ProjectDto[];
  selectedProjectId: string | null;
  loading: boolean;
  onSelectProject: (id: string) => void;
  onAddProject: () => void;
  onRemoveProject: (id: string) => void;
  t: TFunction;
};

const ProjectList = ({
  projects,
  selectedProjectId /* ... */,
}: ProjectListProps) => {
  // rendering logic
};

export default memo(ProjectList);
```

### Pattern 4: Modal Early-Return Unmount

**What:** Modals use `if (!open) return null` for full unmount when not visible.

**When to use:** All three project modals. [VERIFIED: existing pattern in DeleteModal.tsx, AddSkillModal.tsx]

### Pattern 5: Direct invoke from Hook (not through App.tsx)

**What:** The `useProjectState` hook imports `invoke` directly from `@tauri-apps/api/core` instead of going through App.tsx's `invokeTauri` wrapper.

**Why:** D-18 mandates zero props from App.tsx. The `invokeTauri` wrapper in App.tsx only adds an `isTauri` environment check. Since the Projects tab only renders inside Tauri (not a web-only context), the direct `invoke` import is safe and avoids coupling.

**Example:**

```typescript
// In useProjectState.ts
import { invoke } from "@tauri-apps/api/core";

// Use directly:
const projects = await invoke<ProjectDto[]>("list_projects");
```

### Anti-Patterns to Avoid

- **Adding project state to App.tsx:** D-16/D-18/D-19 explicitly prohibit this. All state lives in `useProjectState`. App.tsx only adds the `'projects'` view case.
- **Optimistic updates for cell toggles:** D-06 explicitly chose wait-for-backend over optimistic UI. Do not add optimistic update/revert logic.
- **Creating new CSS files:** All CSS goes in `src/App.css` per project convention. No CSS Modules, no separate stylesheet per component.
- **Full-screen loading overlay for project operations:** D-21 mandates inline skeleton/shimmer loading states, not the existing `LoadingOverlay` pattern.
- **Prop drilling `invokeTauri` from App.tsx:** ProjectsPage fetches data independently. Do not pass `invokeTauri` as a prop.

## Don't Hand-Roll

| Problem                    | Don't Build              | Use Instead                                                             | Why                                                                                |
| -------------------------- | ------------------------ | ----------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| Folder picker              | Custom file browser UI   | `@tauri-apps/plugin-dialog` `open()`                                    | Native OS dialog, already used in App.tsx for local path and storage path picking  |
| Tool detection             | Manual tool scanning     | `invoke('get_tool_status')`                                             | Returns `ToolStatusDto` with `tools[]` and `installed[]` arrays                    |
| Tooltip library            | npm tooltip package      | CSS `title` attribute or CSS-only tooltip with `::after` pseudo-element | Project has no headless UI library; D-09 needs simple error tooltips on red cells  |
| Skeleton/shimmer animation | Complex skeleton library | CSS `@keyframes shimmer` already in App.css                             | Reuse existing animation, apply to placeholder divs with `--bg-element` background |
| UUID generation            | npm uuid package         | Backend generates IDs (all IPC commands return server-generated IDs)    | Frontend never needs to generate IDs                                               |

**Key insight:** This phase has zero new dependencies. Every piece of infrastructure is already in the project.

## Common Pitfalls

### Pitfall 1: Stale Closure in Cell Toggle Callbacks

**What goes wrong:** The `toggleAssignment` callback captures stale `assignments` state if not properly memoized, causing incorrect assign/unassign detection.
**Why it happens:** React closures capture state at render time. If many cells are toggled rapidly, earlier closures may reference stale assignment data.
**How to avoid:** Use functional state updates and re-fetch assignments from the backend after each mutation rather than computing the next state locally. The wait-for-backend pattern (D-06) inherently solves this by always re-fetching.
**Warning signs:** Cell showing wrong status after rapid clicking; "ASSIGNMENT_EXISTS" error from backend.

### Pitfall 2: Missing camelCase in invoke Arguments

**What goes wrong:** Backend Tauri commands use `#[allow(non_snake_case)]` with camelCase parameter names (e.g., `projectId`, `skillId`). If the frontend passes snake_case keys, the command silently receives empty/default values.
**Why it happens:** Rust convention is snake_case, but Tauri IPC serialization uses the parameter names as-is, and the frontend must match exactly.
**How to avoid:** Always use camelCase in `invoke` argument objects: `{ projectId, skillId, tool }`. [VERIFIED: all backend project commands use camelCase params]
**Warning signs:** "project not found" or "skill not found" errors when IDs are clearly valid.

### Pitfall 3: Forgetting to Register Command Names

**What goes wrong:** New `invoke()` calls fail with "command not found" at runtime.
**Why it happens:** Backend commands must be registered in `lib.rs` `generate_handler!` macro.
**How to avoid:** All 12 project commands are already registered (verified in lib.rs lines 108-119). No new commands are needed for this phase. [VERIFIED: lib.rs]
**Warning signs:** Runtime error: "command [name] not found".

### Pitfall 4: Header.tsx activeView Type Mismatch

**What goes wrong:** Adding `'projects'` to the activeView union type in Header.tsx but not in App.tsx (or vice versa) causes TypeScript errors.
**Why it happens:** The `activeView` type is duplicated: once in App.tsx line 91 and once in Header.tsx line 8. Both must be updated.
**How to avoid:** Update both locations simultaneously. Consider whether to extract to a shared type (Claude's discretion area).
**Warning signs:** TypeScript build errors on `activeView` comparison.

### Pitfall 5: Async Race Conditions on Project Selection

**What goes wrong:** User clicks project A, then quickly clicks project B. Project A's data loads and overwrites project B's expected state.
**Why it happens:** Two concurrent `list_project_tools` + `list_project_skill_assignments` calls racing.
**How to avoid:** Track `selectedProjectId` in the fetch callback and discard results if the selection has changed. Or use an AbortController-like pattern with a version counter.
**Warning signs:** Matrix shows tools/assignments for wrong project after quick selection changes.

### Pitfall 6: Missing i18n Keys Causing Raw Key Display

**What goes wrong:** Component renders `projects.addProject` as literal text instead of "Add Project".
**Why it happens:** Forgetting to add translation keys to `src/i18n/resources.ts`.
**How to avoid:** Add all keys from the UI-SPEC copywriting contract table to resources.ts before building components. The UI-SPEC lists 30+ keys that need to be added.
**Warning signs:** UI shows dot-notation strings instead of English text.

### Pitfall 7: `npm run check` Failures from Unused Variables

**What goes wrong:** TypeScript strict mode (`noUnusedLocals`, `noUnusedParameters`) causes build failures.
**Why it happens:** During incremental development, props or variables may be defined but not yet used.
**How to avoid:** Prefix unused parameters with `_` (e.g., `_event`) or ensure all defined items are used before running check.
**Warning signs:** `npm run build` fails with "declared but its value is never read" errors.

## Code Examples

Verified patterns from existing source code:

### Folder Picker Pattern (from App.tsx)

```typescript
// Source: App.tsx lines 591-604 [VERIFIED: existing codebase]
const handlePickPath = async () => {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      directory: true,
      multiple: false,
      title: t("projects.pathLabel"),
    });
    if (!selected || Array.isArray(selected)) return;
    setPath(selected);
  } catch (err) {
    toast.error(err instanceof Error ? err.message : String(err));
  }
};
```

### View Switching Integration (App.tsx render)

```typescript
// Source: App.tsx lines 1847-1916 [VERIFIED: existing codebase]
// Add this branch to the conditional chain:
// activeView === 'projects' ? (
//   <ProjectsPage t={t} />
// ) :
```

### Header Nav Tab (Header.tsx)

```typescript
// Source: Header.tsx lines 33-48 [VERIFIED: existing codebase]
// Add between existing explore tab and closing </nav>:
// <button
//   className={`nav-tab${activeView === 'projects' ? ' active' : ''}`}
//   type="button"
//   onClick={() => onViewChange('projects')}
// >
//   <FolderKanban size={16} />
//   {t('navProjects')}
// </button>
```

### Modal Pattern (from DeleteModal.tsx)

```typescript
// Source: DeleteModal.tsx [VERIFIED: existing codebase]
const RemoveProjectModal = ({ open, loading, projectName, onRequestClose, onConfirm, t }: Props) => {
  if (!open) return null
  return (
    <div className="modal-backdrop" onClick={onRequestClose}>
      <div className="modal modal-delete" onClick={e => e.stopPropagation()} role="dialog" aria-modal="true">
        <div className="modal-body delete-body">
          {/* Content follows DeleteModal pattern */}
        </div>
        <div className="modal-footer space-between">
          <button className="btn btn-secondary" onClick={onRequestClose} disabled={loading}>
            {t('cancel')}
          </button>
          <button className="btn btn-danger-solid" onClick={onConfirm} disabled={loading}>
            {t('projects.removeConfirm')}
          </button>
        </div>
      </div>
    </div>
  )
}
export default memo(RemoveProjectModal)
```

### CSS Skeleton/Shimmer Pattern

```css
/* Source: App.css lines 948-955 [VERIFIED: existing codebase] */
/* Existing shimmer keyframe -- reuse for skeleton loading */
@keyframes shimmer {
  0% {
    background-position: 100% 0;
  }
  100% {
    background-position: -100% 0;
  }
}

/* New skeleton pattern (matches existing --bg-element background): */
.skeleton-row {
  height: 48px;
  background: var(--bg-element);
  border-radius: var(--radius-md);
  background: linear-gradient(
    90deg,
    var(--bg-element) 0%,
    var(--bg-element-hover) 50%,
    var(--bg-element) 100%
  );
  background-size: 200% 100%;
  animation: shimmer 1.5s infinite linear;
}
```

### Matrix Cell Status Styling

```css
/* Source: derived from UI-SPEC color contract + existing CSS variable system */
.matrix-cell {
  background: var(--bg-element);
  transition: background-color 0.15s;
}
.matrix-cell.synced {
  background: var(--success-soft-bg);
}
.matrix-cell.stale {
  background: var(--warning-soft-bg);
}
.matrix-cell.error {
  background: var(--danger-soft-bg);
  cursor: pointer;
}
.matrix-cell.pending {
  background: var(--bg-element);
  opacity: 0.7;
}
.matrix-cell input[type="checkbox"] {
  width: 18px;
  height: 18px;
  accent-color: var(--accent-primary);
}
```

## State of the Art

| Old Approach                         | Current Approach                                                             | When Changed              | Impact                                                                               |
| ------------------------------------ | ---------------------------------------------------------------------------- | ------------------------- | ------------------------------------------------------------------------------------ |
| React.createContext for shared state | Custom hooks with useState (this project's pattern)                          | N/A -- project convention | No context providers needed; hook returns state directly                             |
| useEffect for data fetching          | Still standard in React 19 (React Server Components not applicable in Tauri) | N/A                       | Use `useEffect` for mount-time fetching, `useCallback` for action-triggered fetching |
| CSS Modules                          | App.css with semantic classes (project convention)                           | N/A                       | All new CSS goes in App.css                                                          |

**Note:** React 19 does not change the fundamental hooks API used in this phase. The `use()` hook and React Server Components are not applicable in a Tauri desktop context with client-side rendering. [ASSUMED]

## Backend Command Reference

Complete IPC contract for the hook. All commands verified in `src-tauri/src/commands/projects.rs` and registered in `src-tauri/src/lib.rs`.

| Command Name                      | Parameters (camelCase)                                 | Returns                       | Used By                         |
| --------------------------------- | ------------------------------------------------------ | ----------------------------- | ------------------------------- |
| `list_projects`                   | (none)                                                 | `ProjectDto[]`                | Hook: mount + after mutations   |
| `register_project`                | `{ path: string }`                                     | `ProjectDto`                  | AddProjectModal submit          |
| `remove_project`                  | `{ projectId: string }`                                | `void`                        | RemoveProjectModal confirm      |
| `list_project_tools`              | `{ projectId: string }`                                | `ProjectToolDto[]`            | Hook: on project selection      |
| `add_project_tool`                | `{ projectId: string, tool: string }`                  | `void`                        | ToolConfigModal confirm         |
| `remove_project_tool`             | `{ projectId: string, tool: string }`                  | `void`                        | ToolConfigModal confirm         |
| `list_project_skill_assignments`  | `{ projectId: string }`                                | `ProjectSkillAssignmentDto[]` | Hook: on project selection      |
| `add_project_skill_assignment`    | `{ projectId: string, skillId: string, tool: string }` | `ProjectSkillAssignmentDto`   | Matrix cell toggle (assign)     |
| `remove_project_skill_assignment` | `{ projectId: string, skillId: string, tool: string }` | `void`                        | Matrix cell toggle (unassign)   |
| `bulk_assign_skill`               | `{ projectId: string, skillId: string }`               | `BulkAssignResultDto`         | Matrix "All Tools" row button   |
| `resync_project`                  | `{ projectId: string }`                                | `ResyncSummaryDto`            | Toolbar "Sync Project" button   |
| `resync_all_projects`             | (none)                                                 | `ResyncSummaryDto[]`          | Toolbar "Sync All" button       |
| `get_tool_status`                 | (none)                                                 | `ToolStatusDto`               | ToolConfigModal for auto-detect |
| `get_managed_skills`              | (none)                                                 | `ManagedSkill[]`              | Hook: for matrix skill rows     |

[VERIFIED: src-tauri/src/commands/projects.rs, src-tauri/src/lib.rs lines 108-119]

## Integration Points (Minimal App.tsx Changes)

Per D-19, App.tsx changes are strictly limited to:

### 1. activeView Union Type (App.tsx line 91)

```typescript
// Current:
const [activeView, setActiveView] = useState<
  "myskills" | "explore" | "detail" | "settings"
>("myskills");
// Change to:
const [activeView, setActiveView] = useState<
  "myskills" | "explore" | "detail" | "settings" | "projects"
>("myskills");
```

### 2. Header.tsx HeaderProps Type (Header.tsx line 8)

```typescript
// Current:
activeView: "myskills" | "explore" | "detail" | "settings";
// Change to:
activeView: "myskills" | "explore" | "detail" | "settings" | "projects";
```

### 3. Header.tsx onViewChange Prop Type (Header.tsx line 11)

```typescript
// Current:
onViewChange: (view: 'myskills' | 'explore') => void
// Change to:
onViewChange: (view: 'myskills' | 'explore' | 'projects') => void
```

### 4. App.tsx handleViewChange (line 637)

```typescript
// Current:
const handleViewChange = useCallback((view: 'myskills' | 'explore') => { ... })
// Change to:
const handleViewChange = useCallback((view: 'myskills' | 'explore' | 'projects') => { ... })
```

### 5. App.tsx Render Switch (line 1847)

Add `activeView === 'projects' ? <ProjectsPage t={t} /> :` branch.

### 6. App.tsx Import

Add `import ProjectsPage from './components/projects/ProjectsPage'`.

[VERIFIED: All line numbers and types confirmed from codebase reading]

## Assumptions Log

| #   | Claim                                                                                                          | Section                           | Risk if Wrong                                                                                         |
| --- | -------------------------------------------------------------------------------------------------------------- | --------------------------------- | ----------------------------------------------------------------------------------------------------- |
| A1  | React 19's `use()` hook and Server Components are not applicable in Tauri client-side rendering                | State of the Art                  | LOW -- if applicable, it would be an optimization opportunity, not a blocker                          |
| A2  | CSS `title` attribute provides sufficient tooltip UX for error messages on red cells (D-09)                    | Don't Hand-Roll                   | MEDIUM -- if title tooltip is too limited, may need a CSS-only tooltip with `::after` pseudo-element  |
| A3  | Direct `invoke` import from `@tauri-apps/api/core` works without the isTauri guard in the Tauri window context | Architecture Patterns (Pattern 5) | LOW -- ProjectsPage only renders inside Tauri; the guard exists in App.tsx for SSR/test compatibility |

## Open Questions

1. **Error tooltip approach for D-09**
   - What we know: D-09 requires hover tooltip on red cells showing error message. Clicking retries sync.
   - What's unclear: Whether `title` attribute provides sufficient UX or if a CSS-only tooltip (positioned element) is needed for better styling.
   - Recommendation: Start with CSS-only tooltip (positioned `::after` pseudo-element) to control styling. `title` attribute has inconsistent browser rendering and no style control.

2. **Shared activeView type vs duplicated**
   - What we know: The `activeView` union type is currently duplicated in App.tsx (line 91) and Header.tsx (line 8). Adding 'projects' requires updating both.
   - What's unclear: Whether to extract a shared type (e.g., `type ActiveView = ...` in a shared types file).
   - Recommendation: Extract to avoid future duplication. But this is minor refactoring that could be deferred.

3. **Duplicate project detection for D-14**
   - What we know: D-14 requires inline validation checking if a path is already registered. The hook has the `projects` list.
   - What's unclear: Whether path comparison should be case-sensitive on all platforms or case-insensitive on macOS/Windows.
   - Recommendation: Do a normalized comparison (trailing slash removal, home path expansion) on the frontend using the existing project paths from `list_projects`. The backend `register_project` command also validates for duplicates as a safety net.

## Validation Architecture

### Test Framework

| Property           | Value                                                          |
| ------------------ | -------------------------------------------------------------- |
| Framework          | Rust: `cargo test`, Frontend: none (no Vitest/Jest configured) |
| Config file        | src-tauri/Cargo.toml (test deps: tempfile, mockito)            |
| Quick run command  | `npm run rust:test`                                            |
| Full suite command | `npm run check`                                                |

### Phase Requirements -> Test Map

| Req ID  | Behavior                                       | Test Type   | Automated Command                                           | File Exists? |
| ------- | ---------------------------------------------- | ----------- | ----------------------------------------------------------- | ------------ |
| UI-01   | Projects tab in navigation                     | manual-only | Visual verification in Tauri dev window                     | N/A          |
| UI-02   | Project list panel with add/remove             | manual-only | Visual verification in Tauri dev window                     | N/A          |
| UI-03   | Assignment matrix with checkbox grid           | manual-only | Visual verification in Tauri dev window                     | N/A          |
| UI-04   | Sync status + Sync Project/All buttons         | manual-only | Visual verification in Tauri dev window                     | N/A          |
| UI-05   | Isolated component tree/state                  | manual-only | `npm run build` (TypeScript compilation confirms isolation) | N/A          |
| SYNC-01 | Cell status indicators (green/yellow/red/gray) | manual-only | Visual verification with test data                          | N/A          |
| TOOL-02 | Auto-detect installed tools                    | manual-only | `npm run tauri:dev`, add project, verify tool config modal  | N/A          |
| TOOL-03 | Add/remove tool columns                        | manual-only | Visual verification in Tauri dev window                     | N/A          |

**Justification for manual-only:** This phase is entirely frontend UI with no backend logic changes. There is no frontend test framework configured in this project (`package.json` has no test runner). All verification requires the Tauri runtime window.

### Sampling Rate

- **Per task commit:** `npm run check` (lint + build + rust checks)
- **Per wave merge:** `npm run check` + manual Tauri dev verification
- **Phase gate:** Full `npm run check` green + visual verification of all 8 requirements

### Wave 0 Gaps

- No frontend test framework (Vitest/Jest) -- this is a project-wide gap, not specific to this phase. Not adding one per project constraints (no new frameworks).
- Backend tests for project commands exist from Phase 3 and do not need changes.

## Security Domain

This phase is frontend-only with no new attack surface. All data flows through existing Tauri IPC commands that were implemented and security-reviewed in Phases 1-3.

### Applicable ASVS Categories

| ASVS Category         | Applies | Standard Control                                                                    |
| --------------------- | ------- | ----------------------------------------------------------------------------------- |
| V2 Authentication     | no      | N/A -- local desktop app                                                            |
| V3 Session Management | no      | N/A -- no sessions                                                                  |
| V4 Access Control     | no      | N/A -- single user                                                                  |
| V5 Input Validation   | yes     | Backend validates paths in `register_project`; frontend does inline duplicate check |
| V6 Cryptography       | no      | N/A -- no crypto in this phase                                                      |

### Known Threat Patterns

| Pattern                                | STRIDE    | Standard Mitigation                                                     |
| -------------------------------------- | --------- | ----------------------------------------------------------------------- |
| Path traversal in project registration | Tampering | Backend `expand_home_path` + validation already implemented in Phase 1  |
| XSS via project name display           | Tampering | React's JSX auto-escapes; project name is basename from filesystem path |

## Sources

### Primary (HIGH confidence)

- Project source code: `src/App.tsx`, `src/App.css`, `src/index.css`, `src/components/skills/Header.tsx`, `src/components/skills/modals/AddSkillModal.tsx`, `src/components/skills/modals/DeleteModal.tsx`, `src/components/skills/ExplorePage.tsx`, `src/components/skills/LoadingOverlay.tsx` -- all patterns verified by direct reading
- `src-tauri/src/commands/projects.rs` -- all 12 command signatures verified
- `src-tauri/src/lib.rs` lines 108-119 -- command registration verified
- `src/components/projects/types.ts` -- DTO types verified
- `package.json` -- all dependency versions verified
- `.planning/phases/04-frontend-component-tree/04-CONTEXT.md` -- all 21 decisions
- `.planning/phases/04-frontend-component-tree/04-UI-SPEC.md` -- design contract
- `src/i18n/resources.ts` -- translation key patterns verified

### Secondary (MEDIUM confidence)

- lucide-react icon availability (`FolderKanban`, `Plus`, etc.) -- verified via runtime `require('lucide-react')` enumeration

### Tertiary (LOW confidence)

- None. All claims verified against source code.

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH -- all libraries verified in package.json, no new dependencies
- Architecture: HIGH -- all patterns extracted from existing codebase, all decisions locked
- Pitfalls: HIGH -- derived from direct code analysis and Tauri IPC conventions

**Research date:** 2026-04-08
**Valid until:** 2026-05-08 (30 days -- stable frontend-only phase with locked decisions)
