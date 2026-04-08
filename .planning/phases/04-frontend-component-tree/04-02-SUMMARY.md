---
phase: 04-frontend-component-tree
plan: 02
subsystem: frontend-projects
tags: [react, modals, css, tauri-command, gitignore]
dependency_graph:
  requires: [04-01]
  provides:
    [
      ProjectList,
      AddProjectModal,
      ToolConfigModal,
      RemoveProjectModal,
      update_project_gitignore,
    ]
  affects: [ProjectsPage, App.css, lib.rs]
tech_stack:
  added: []
  patterns: [inner-component-mount-for-state-init, useRef-for-cross-modal-state]
key_files:
  created:
    - src/components/projects/ProjectList.tsx
    - src/components/projects/AddProjectModal.tsx
    - src/components/projects/ToolConfigModal.tsx
    - src/components/projects/RemoveProjectModal.tsx
  modified:
    - src/components/projects/ProjectsPage.tsx
    - src/App.css
    - src-tauri/src/commands/projects.rs
    - src-tauri/src/lib.rs
decisions:
  - "ToolConfigModal uses Inner component mount pattern instead of useEffect+setState to satisfy react-hooks/set-state-in-effect lint rule"
  - "Gitignore options stored in useRef across modal transition (not useState) since they never drive re-renders"
metrics:
  duration: 7m
  completed: "2026-04-08T19:04:00Z"
  tasks_completed: 3
  tasks_total: 3
---

# Phase 4 Plan 2: Project List, Modals, and Gitignore Command Summary

Project list panel with three modal dialogs (Add, ToolConfig, Remove), full CSS, and backend update_project_gitignore command with end-to-end D-13 gitignore flow wired through ProjectsPage.

## What Was Built

### ProjectList.tsx

Left panel component rendering project items with name, path, assignment count, and sync status dot. Includes skeleton loading (3 shimmer rows), error state, and listbox accessibility (`role="listbox"`, `role="option"`, `aria-selected`). Remove button per item with hover reveal. Does NOT render empty state (that is in ProjectsPage per D-03).

### AddProjectModal.tsx

Registration modal with folder picker (dynamic import of `@tauri-apps/plugin-dialog`), manual path input, inline duplicate validation (D-14), and gitignore checkbox section (D-13). The `onRegister` callback accepts gitignore options as a second parameter, which the parent stores in a ref for deferred use.

### ToolConfigModal.tsx

Tool selection modal showing all known tools with checkboxes. Installed tools are pre-checked via `useState` initializer that runs on component mount. Uses Inner component pattern: outer component guards `if (!open) return null`, inner component mounts fresh each time with correct initial state. This satisfies the `react-hooks/set-state-in-effect` lint rule.

### RemoveProjectModal.tsx

Confirmation dialog following the existing DeleteModal pattern exactly: `TriangleAlert` icon, warning message with project name interpolation, two warning list items, cancel and danger-solid remove buttons.

### ProjectsPage.tsx (rewritten)

Full wiring of all components with three handler callbacks:

- `handleAddProject`: register -> store gitignore prefs in ref -> auto-select -> open ToolConfigModal (D-11)
- `handleToolConfigConfirm`: persist tools via addTools/removeTools -> invoke `update_project_gitignore` using stored ref (D-13 full delivery)
- `handleRemoveProject`: confirm -> cleanup -> success toast

D-03 full-width empty state renders outside the split-panel layout when zero projects exist, with FolderOpen icon and CTA button.

### update_project_gitignore backend command

Rust command accepting `projectId`, `addToGitignore`, `addToExclude`. Looks up project path and configured tools, derives gitignore patterns from tool adapter `relative_skills_dir`, writes to `.gitignore` and/or `.git/info/exclude` with idempotency via `# Skills Hub` marker comment. Creates `.git/info/` directory if it does not exist.

### CSS (App.css)

All project styles appended under `/* === Projects Tab === */` comment: projects-page layout, split-panel, project-list sidebar (250px), project-item with selected state and hover, status dots (synced/stale/error), skeleton shimmer rows, empty state, matrix panel, gitignore section, field-error, tool pick list.

## Commits

| Task | Name                                                       | Commit  | Key Files                                                                                  |
| ---- | ---------------------------------------------------------- | ------- | ------------------------------------------------------------------------------------------ |
| 1    | Create ProjectList, three modals, and all CSS              | 1195e0c | ProjectList.tsx, AddProjectModal.tsx, ToolConfigModal.tsx, RemoveProjectModal.tsx, App.css |
| 2    | Wire components into ProjectsPage with D-13 gitignore flow | a32a605 | ProjectsPage.tsx, ToolConfigModal.tsx                                                      |
| 3    | Implement update_project_gitignore backend command         | eab6ff4 | commands/projects.rs, lib.rs                                                               |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed ToolConfigModal lint error: react-hooks/set-state-in-effect**

- **Found during:** Task 1 verification (lint run)
- **Issue:** Original implementation used `useEffect` + `setState` to initialize tool selection when modal opens. ESLint `react-hooks/set-state-in-effect` rule rejects setState inside useEffect body.
- **Fix:** Refactored to Inner component mount pattern -- outer component handles `if (!open) return null`, inner component uses `useState` initializer function that computes initial selection on mount. Component remounts each time modal opens, resetting state naturally.
- **Files modified:** src/components/projects/ToolConfigModal.tsx
- **Commit:** a32a605

## Decisions Made

1. **ToolConfigModal Inner component pattern**: Instead of useEffect+setState (which violates react-hooks lint), used a wrapper/inner component split where the outer handles open guard and the inner mounts fresh with a useState initializer. This is cleaner than useRef hacks and naturally resets state on each open.

2. **useRef for gitignore options**: Gitignore checkbox selections from AddProjectModal are stored in `pendingGitignoreRef` (not useState) because they are consumed once in handleToolConfigConfirm and never drive re-renders.

## Verification

- `npm run build` passes (tsc + vite)
- `npm run lint` passes (ESLint, 0 errors)
- `cargo fmt --check` passes
- `cargo clippy -- -D warnings` passes (0 warnings)
- `cargo test` passes (119 tests, 0 failures)

## Self-Check: PASSED

All 5 created/modified files verified present. All 3 commit hashes verified in git log.
