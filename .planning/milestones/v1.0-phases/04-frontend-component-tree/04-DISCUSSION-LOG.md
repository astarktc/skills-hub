# Phase 4: Frontend Component Tree - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-08
**Phase:** 04-frontend-component-tree
**Areas discussed:** Tab layout structure, Matrix interactions, Project registration UX, State management hook

---

## Tab Layout Structure

| Option      | Description                                                                         | Selected |
| ----------- | ----------------------------------------------------------------------------------- | -------- |
| Split panel | Project list left (~250px), matrix right. Email-client pattern.                     | ✓        |
| Drill-down  | Full-width list → full-width matrix with back button. Matches existing nav pattern. |          |
| Accordion   | Collapsible inline expand per project. Good for few projects.                       |          |

**User's choice:** Split panel
**Notes:** None — selected recommended option.

---

### Project Row Content

| Option                 | Description                                              | Selected |
| ---------------------- | -------------------------------------------------------- | -------- |
| Card with status badge | Name, truncated path, assignment count, colored sync dot | ✓        |
| Minimal text rows      | Just name and path, no status                            |          |
| Card with progress bar | Name, path, mini progress bar for synced/total           |          |

**User's choice:** Card with status badge
**Notes:** None.

---

### Empty State (No Projects)

| Option                | Description                                                       | Selected |
| --------------------- | ----------------------------------------------------------------- | -------- |
| Illustration + CTA    | Full-width placeholder with icon, explanation, Add Project button | ✓        |
| Minimal hint + button | One-line hint text + button                                       |          |
| You decide            | Claude decides                                                    |          |

**User's choice:** Illustration + CTA
**Notes:** None.

---

### Right Panel Header

| Option               | Description                                                                   | Selected |
| -------------------- | ----------------------------------------------------------------------------- | -------- |
| Toolbar with actions | Project name, path, action buttons (Sync Project, Sync All, Add/Remove Tools) | ✓        |
| Name + overflow menu | Just name, actions in ⋯ dropdown                                              |          |

**User's choice:** Toolbar with actions
**Notes:** None.

---

### No Project Selected State

| Option           | Description                                    | Selected |
| ---------------- | ---------------------------------------------- | -------- |
| Placeholder text | "Select a project to manage skill assignments" | ✓        |
| Add-project CTA  | Same as empty state                            |          |
| Hide right panel | Collapse right panel                           |          |

**User's choice:** Placeholder text
**Notes:** None.

---

### Panel Sizing

| Option            | Description                      | Selected |
| ----------------- | -------------------------------- | -------- |
| Fixed width       | Left panel ~250px fixed          | ✓        |
| Resizable divider | Draggable divider between panels |          |

**User's choice:** Fixed width
**Notes:** None.

---

## Matrix Interactions

### Checkbox Toggle Behavior

| Option            | Description                                  | Selected |
| ----------------- | -------------------------------------------- | -------- |
| Optimistic toggle | Show synced immediately, revert on failure   |          |
| Wait for backend  | Show pending, wait for response, then update | ✓        |
| You decide        | Claude decides                               |          |

**User's choice:** Wait for backend
**Notes:** Avoids optimistic UI revert complexity flagged in STATE.md.

---

### Bulk Assign

| Option                 | Description                                                      | Selected |
| ---------------------- | ---------------------------------------------------------------- | -------- |
| Row button             | "All Tools" button per skill row, uses bulk_assign_skill command | ✓        |
| Column header checkbox | Toggle all skills for a tool                                     |          |
| Both row and column    | Both actions                                                     |          |

**User's choice:** Row button
**Notes:** None.

---

### Status Cell Display

| Option                  | Description                                                        | Selected |
| ----------------------- | ------------------------------------------------------------------ | -------- |
| Colored cell + checkbox | Background tint per status, checkbox always visible, hover tooltip | ✓        |
| Status icons            | Replace checkbox with colored icon per status                      |          |
| Checkbox + dot          | Plain checkbox with small colored dot                              |          |

**User's choice:** Colored cell + checkbox
**Notes:** None.

---

### Error Display

| Option             | Description                                 | Selected |
| ------------------ | ------------------------------------------- | -------- |
| Tooltip on hover   | Red cell, hover shows error, click to retry | ✓        |
| Error icon + toast | Red cell with icon, click for toast         |          |
| You decide         | Claude decides                              |          |

**User's choice:** Tooltip on hover
**Notes:** None.

---

## Project Registration UX

### Add Project Flow

| Option        | Description                                                   | Selected |
| ------------- | ------------------------------------------------------------- | -------- |
| Modal dialog  | Folder picker + manual input in modal (matches AddSkillModal) | ✓        |
| Inline input  | Input field appears in project list                           |          |
| Direct picker | OS picker fires directly, no modal                            |          |

**User's choice:** Modal dialog
**Notes:** None.

---

### After Registration

| Option                    | Description                                             | Selected |
| ------------------------- | ------------------------------------------------------- | -------- |
| Auto-select + tool setup  | Auto-select project, immediately show tool config modal | ✓        |
| Auto-select, manual setup | Auto-select, user clicks Add Tools manually             |          |
| Add to list only          | No auto-select                                          |          |

**User's choice:** Auto-select + tool setup
**Notes:** None.

---

### Tool Configuration

| Option                 | Description                                        | Selected |
| ---------------------- | -------------------------------------------------- | -------- |
| Checkbox modal         | Modal with all tools listed, installed pre-checked | ✓        |
| Inline tool picker     | Inline panel in matrix area                        |          |
| Grouped checkbox modal | Same modal but grouped by tool category            |          |

**User's choice:** Checkbox modal
**Notes:** None.

---

### Gitignore Handling

| Option       | Description                            | Selected |
| ------------ | -------------------------------------- | -------- |
| Info banner  | Non-blocking banner after registration |          |
| Modal prompt | Modal asking about .gitignore          |          |
| Skip         | No prompt                              |          |
| **Custom**   | Two checkboxes in registration modal   | ✓        |

**User's choice:** Custom — Registration modal includes two checkboxes (unchecked by default): "Add to project `.gitignore`" (shared/committed) and "Add to `.git/info/exclude`" (private/local). Backend inserts entries and creates files as needed.
**Notes:** User proposed this approach — integrates gitignore choice directly into the registration flow rather than as a separate prompt.

---

### Duplicate Handling

| Option                     | Description                                          | Selected |
| -------------------------- | ---------------------------------------------------- | -------- |
| Toast + highlight existing | Post-submit toast error, highlight duplicate in list |          |
| Toast error only           | Just toast error                                     |          |
| **Custom**                 | Inline validation in modal                           | ✓        |

**User's choice:** Custom — Inline validation in the registration modal. After path is entered/picked, immediately check against registered projects and show inline warning before save is allowed.
**Notes:** User proposed this — catches duplicates pre-submit rather than post-submit.

---

### Project Removal

| Option         | Description                                                  | Selected |
| -------------- | ------------------------------------------------------------ | -------- |
| Confirm dialog | Confirmation dialog explaining cleanup (matches DeleteModal) | ✓        |
| Undo toast     | Swipe/right-click delete with undo toast                     |          |

**User's choice:** Confirm dialog
**Notes:** None.

---

## State Management Hook

### Hook Architecture

| Option         | Description                                               | Selected |
| -------------- | --------------------------------------------------------- | -------- |
| Single hook    | useProjectState() owns all state, returns state + actions | ✓        |
| Multiple hooks | useProjects(), useAssignments(), useProjectTools()        |          |
| You decide     | Claude decides                                            |          |

**User's choice:** Single hook
**Notes:** None.

---

### Data Loading Strategy

| Option          | Description                                                                 | Selected |
| --------------- | --------------------------------------------------------------------------- | -------- |
| Fetch on demand | Load project list on mount, assignments on select, re-fetch after mutations | ✓        |
| Eager load all  | Load everything on tab mount                                                |          |
| You decide      | Claude decides                                                              |          |

**User's choice:** Fetch on demand
**Notes:** None.

---

### App.tsx Integration

| Option             | Description                                               | Selected |
| ------------------ | --------------------------------------------------------- | -------- |
| Self-contained     | ProjectsPage fetches own skills. Zero props from App.tsx. | ✓        |
| Pass managedSkills | App.tsx passes skills list as prop                        |          |

**User's choice:** Self-contained
**Notes:** User asked about App.tsx re-fetch frequency. After learning it re-fetches after every mutation (~15 call sites), confirmed self-contained is appropriate since the extra IPC call per tab switch is negligible.

---

### Loading States

| Option         | Description                                       | Selected |
| -------------- | ------------------------------------------------- | -------- |
| Inline loading | Skeleton/shimmer in matrix and project list areas | ✓        |
| Full overlay   | Reuse LoadingOverlay for big operations           |          |
| You decide     | Claude decides                                    |          |

**User's choice:** Inline loading
**Notes:** None.

---

### Component Granularity

| Option              | Description                                                            | Selected |
| ------------------- | ---------------------------------------------------------------------- | -------- |
| 3 components + hook | ProjectsPage, ProjectList, AssignmentMatrix + useProjectState + modals | ✓        |
| Fine-grained        | 6+ components with ProjectCard, MatrixToolbar, MatrixCell, etc.        |          |

**User's choice:** 3 components + hook
**Notes:** None.

---

## Claude's Discretion

- Exact CSS class names and styling details
- Internal hook helper decomposition
- Skeleton/shimmer implementation approach
- Whether to memoize matrix cells individually
- Exact modal form validation UX
- i18n key naming convention for project strings

## Deferred Ideas

None — discussion stayed within phase scope.
