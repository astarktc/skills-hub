# Phase 4: Frontend Component Tree - Context

**Gathered:** 2026-04-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Complete Projects tab UI: a new tab in the main navigation that lets users register projects, configure tool columns, assign skills via a checkbox matrix, and see per-cell sync status. The tab is a self-contained component tree with its own state hook, isolated from App.tsx. Backend commands from Phases 1-3 are called via Tauri IPC — no backend changes in this phase.

</domain>

<decisions>
## Implementation Decisions

### Tab Layout Structure

- **D-01:** Split-panel layout — project list on the left (~250px fixed width), assignment matrix on the right. Click a project in the left panel to load its matrix.
- **D-02:** Each project row in the left panel shows: name (basename per Phase 1 D-01), truncated path as subtitle, assignment count, and a colored dot for aggregate sync status.
- **D-03:** When no projects are registered (first-time), show a full-width empty state with icon/illustration, brief explanation of what project distribution does, and a prominent "Add Project" button.
- **D-04:** Right panel header is a toolbar containing: project name, path, and action buttons (Sync Project, Sync All, Add/Remove Tools).
- **D-05:** When no project is selected, the right panel shows placeholder text: "Select a project to manage skill assignments".

### Matrix Interactions

- **D-06:** Wait-for-backend toggle — click checkbox shows pending state on that cell, waits for backend response, then updates to synced/error. No optimistic UI revert logic.
- **D-07:** Bulk-assign via row button — "All Tools" button on each skill row. Calls `bulk_assign_skill` backend command. All tool cells in that row go to pending, then update individually per result.
- **D-08:** Per-cell status via colored cell background + checkbox: green tint (synced), yellow tint (stale), red tint (error), gray tint (pending/unchecked). Checkbox always visible.
- **D-09:** Sync errors displayed via hover tooltip on red-tinted cells showing the error message. Click red cell to retry sync.

### Project Registration UX

- **D-10:** Add project via modal dialog (matches existing AddSkillModal pattern). Modal contains folder picker button + manual path input field.
- **D-11:** After registration, auto-select the new project in the left panel and immediately show tool configuration modal with auto-detection of installed tools (TOOL-02).
- **D-12:** Tool configuration is a checkbox modal listing all known tools. Installed tools are pre-checked (auto-detected). User unchecks tools they don't want. Confirm adds selected tools as matrix columns.
- **D-13:** Registration modal includes gitignore section with two checkboxes (both unchecked by default):
  - "Add to project `.gitignore`" (shared, committed to repo)
  - "Add to `.git/info/exclude`" (private, local-only)
    When checked, the backend inserts tool skill directory entries (e.g., `.claude/skills/`) into the chosen file(s) on save. Creates `.gitignore` if it doesn't exist.
- **D-14:** Inline duplicate validation in the registration modal — after path is entered/picked, check against registered projects immediately and show inline warning before save is allowed.
- **D-15:** Project removal via confirmation dialog (matches existing DeleteModal pattern) explaining that all synced symlinks/copies will be removed.

### State Management

- **D-16:** Single `useProjectState()` custom hook owns all state: project list, selected project ID, assignments, tools, loading flags. Returns state + action functions. Components below are stateless presentational.
- **D-17:** Fetch on demand — load project list on tab mount, load assignments + tools when a project is selected, re-fetch after any mutation.
- **D-18:** ProjectsPage is fully self-contained — fetches its own skills list via `invoke('get_managed_skills')`. Zero props from App.tsx (except `t` for translations).
- **D-19:** App.tsx changes limited to: add `'projects'` to `activeView` union type, add Projects nav tab in Header.tsx, import and render `ProjectsPage` in the view switch.

### Component Structure

- **D-20:** Three main components + hook, all in `src/components/projects/`:
  - `ProjectsPage.tsx` — top-level, orchestrates layout
  - `ProjectList.tsx` — left panel, project cards
  - `AssignmentMatrix.tsx` — right panel, checkbox grid
  - `useProjectState.ts` — state hook
  - Plus modals: `AddProjectModal.tsx`, `ToolConfigModal.tsx`, `RemoveProjectModal.tsx`
- **D-21:** Inline loading states (skeleton/shimmer) within the matrix area and project list. No full-screen overlay.

### Claude's Discretion

- Exact CSS class names and styling details
- Internal hook helper decomposition
- Skeleton/shimmer implementation approach
- Whether to memoize matrix cells individually
- Exact modal form validation UX (field error messages, disabled states)
- i18n key naming convention for project strings

</decisions>

<canonical_refs>

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Existing UI Patterns (follow these)

- `src/components/skills/Header.tsx` — Nav tab pattern, activeView switching. Must add Projects tab here.
- `src/App.tsx` lines 91, 637-656, 1837-1915 — View switching logic, activeView union type, render switch
- `src/components/skills/modals/AddSkillModal.tsx` — Modal pattern with folder picker + manual input
- `src/components/skills/modals/DeleteModal.tsx` — Confirmation dialog pattern
- `src/components/skills/ExplorePage.tsx` — Self-contained page component with memo() export
- `src/components/skills/FilterBar.tsx` — Simple toolbar component reference
- `src/App.css` — Existing CSS class patterns, theming approach
- `src/index.css` — CSS variables, dark theme selectors

### Frontend DTO Contract (already created)

- `src/components/projects/types.ts` — ProjectDto, ProjectToolDto, ProjectSkillAssignmentDto, ResyncSummaryDto, BulkAssignResultDto, BulkAssignErrorDto

### Backend Commands (call via invoke)

- `src-tauri/src/commands/projects.rs` — All 12 project commands: register_project, remove_project, list_projects, get_project, add_project_tool, remove_project_tool, list_project_tools, add_project_skill_assignment, remove_project_skill_assignment, resync_project, resync_all_projects, bulk_assign_skill
- `src-tauri/src/commands/mod.rs` — get_managed_skills (for skill names in matrix rows), error prefix patterns

### i18n

- `src/i18n/resources.ts` — Translation key patterns, English strings (Chinese deferred)

### Project Docs

- `.planning/PROJECT.md` — Constraints: minimize App.tsx, separate component tree, English only
- `.planning/REQUIREMENTS.md` — UI-01, UI-02, UI-03, UI-04, UI-05, TOOL-02, TOOL-03, SYNC-01 definitions

### Prior Phase Context

- `.planning/phases/01-data-foundation/01-CONTEXT.md` — Schema design, project display name = basename (D-01)
- `.planning/phases/02-sync-logic/02-CONTEXT.md` — Sync atomicity (D-01/D-02), staleness detection (D-07/D-08/D-09), status lifecycle
- `.planning/phases/03-ipc-commands/03-CONTEXT.md` — Bulk-assign command (D-01), DTO file (D-04), error prefixes (D-06/D-07)

</canonical_refs>

<code_context>

## Existing Code Insights

### Reusable Assets

- `Header.tsx` nav-tabs: Extend with Projects tab button (Layers-like icon, "Projects" label)
- `AddSkillModal.tsx` pattern: Folder picker via `@tauri-apps/plugin-dialog`, manual path input, validation
- `DeleteModal.tsx` pattern: Confirmation dialog with description text
- `LoadingOverlay.tsx`: Reference for loading state patterns (though Projects uses inline loading)
- `ExplorePage.tsx`: Self-contained page with memo() export and own data fetching
- `FilterBar.tsx`: Simple toolbar/action bar reference

### Established Patterns

- View switching via `activeView` state + conditional rendering in App.tsx main section
- Components wrapped in `memo()` for presentational components
- Modals use `if (!open) return null` early return
- CSS classes in App.css with semantic kebab-case names
- Toast notifications via sonner for success/error feedback
- i18n via `useTranslation()` hook and `t('key')` calls
- Error handling: try/catch with `err instanceof Error ? err.message : String(err)`

### Integration Points

- `src/App.tsx` line 91: Add `'projects'` to activeView union
- `src/components/skills/Header.tsx` line 8: Add `'projects'` to HeaderProps activeView type
- `src/components/skills/Header.tsx` line 32-48: Add Projects nav tab button
- `src/App.tsx` ~line 1883: Add `activeView === 'projects'` branch in render switch

</code_context>

<specifics>
## Specific Ideas

- Gitignore: Two checkboxes in registration modal — project `.gitignore` (shared) and `.git/info/exclude` (private). Backend handles file creation and entry insertion.
- Duplicate detection: Inline validation in modal, not post-submit error. Catches duplicates before the user can click save.
- Tool setup: Immediately after registration, show tool config modal with installed tools pre-checked. Streamlines the "add project → configure tools → start assigning" flow.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

_Phase: 04-frontend-component-tree_
_Context gathered: 2026-04-08_
