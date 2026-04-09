# Architecture Patterns

**Domain:** Per-project file distribution system integrated into existing Tauri 2 desktop app
**Researched:** 2026-04-07

## Recommended Architecture

The per-project skill distribution system extends the existing layered architecture without modifying the sync engine or global sync flows. It adds three new concerns: project management (CRUD), assignment management (skill-project-tool triples), and project-aware path resolution. Each concern maps to a distinct component boundary.

### System Overview

```
+--------------------------+     +---------------------------+
| Frontend (React 19)      |     | Backend (Rust/Tauri 2)    |
|                          |     |                           |
| App.tsx (existing)       |     | commands/mod.rs (existing)|
|   adds "Projects" tab    |     |   existing global sync    |
|   to Header navigation   |     |                           |
|                          |     | commands/projects.rs (NEW)|
| components/projects/ NEW |<--->|   project IPC commands    |
|   ProjectsTab.tsx        |     |                           |
|   ProjectList.tsx        |     | core/project_sync.rs (NEW)|
|   AssignmentMatrix.tsx   |     |   project path resolution |
|   SyncStatusBar.tsx      |     |   assignment sync logic   |
|   ProjectCard.tsx        |     |                           |
|   AssignmentCell.tsx     |     | core/skill_store.rs       |
|   useProjectState.ts     |     |   + project CRUD methods  |
|                          |     |   + Schema V4 migration   |
| components/projects/     |     |                           |
|   types.ts               |     | core/sync_engine.rs       |
|                          |     |   UNCHANGED (path-generic)|
+--------------------------+     +---------------------------+
                                          |
                                 +--------v--------+
                                 | SQLite Database  |
                                 | skills           |
                                 | skill_targets    |
                                 | projects     NEW |
                                 | project_tools NEW|
                                 | project_skill_   |
                                 |  assignments NEW |
                                 | settings         |
                                 +-----------------+
```

### Component Boundaries

| Component                                  | Responsibility                                                                                                                                                   | Communicates With                                            | New/Modified                |
| ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | --------------------------- |
| `App.tsx`                                  | Add "Projects" tab to Header navigation. No new state variables.                                                                                                 | Header, ProjectsTab (renders on activeView)                  | Modified (minimal ~5 lines) |
| `components/projects/ProjectsTab.tsx`      | Tab container. Owns all project-related state via `useProjectState` hook. Renders ProjectList, AssignmentMatrix, SyncStatusBar.                                  | App.tsx (rendered by), all child components (renders)        | NEW                         |
| `components/projects/useProjectState.ts`   | Custom hook encapsulating all project state and backend IPC calls. Single source of truth for the Projects feature.                                              | Tauri IPC via `invoke()`, ProjectsTab (consumed by)          | NEW                         |
| `components/projects/ProjectList.tsx`      | Left panel: lists registered projects, add/remove buttons, project selection.                                                                                    | ProjectsTab (parent), Tauri dialog plugin (folder picker)    | NEW                         |
| `components/projects/AssignmentMatrix.tsx` | Right panel: skill x tool checkbox grid. Renders column headers from project_tools, rows from skills list, cells from assignments.                               | ProjectsTab (parent), AssignmentCell (child)                 | NEW                         |
| `components/projects/AssignmentCell.tsx`   | Individual checkbox with status indicator (green/yellow/red/gray). Handles toggle with optimistic update.                                                        | AssignmentMatrix (parent), useProjectState (via callback)    | NEW                         |
| `components/projects/SyncStatusBar.tsx`    | Bottom bar: aggregate sync status, "Sync Project" and "Sync All" buttons.                                                                                        | ProjectsTab (parent)                                         | NEW                         |
| `components/projects/ProjectCard.tsx`      | Individual project list item with name, path, assignment count badge.                                                                                            | ProjectList (parent)                                         | NEW                         |
| `components/projects/types.ts`             | TypeScript DTOs mirroring backend response shapes for projects, assignments, sync status.                                                                        | All project components                                       | NEW                         |
| `commands/projects.rs`                     | Tauri IPC command handlers for all project operations. Thin layer: validates input, delegates to core, formats output.                                           | Frontend via IPC, core/project_sync.rs, core/skill_store.rs  | NEW                         |
| `core/project_sync.rs`                     | Project-aware path resolution and sync orchestration. Computes `project_path + relative_skills_dir + skill_name` targets. Calls existing sync_engine primitives. | core/sync_engine.rs, core/skill_store.rs, core/tool_adapters | NEW                         |
| `core/skill_store.rs`                      | Extended with project CRUD, project_tools CRUD, assignment CRUD, Schema V4 migration.                                                                            | SQLite database                                              | Modified                    |
| `core/sync_engine.rs`                      | UNCHANGED. Already accepts generic `source: &Path` and `target: &Path`.                                                                                          | Filesystem                                                   | Unchanged                   |
| `core/tool_adapters/mod.rs`                | UNCHANGED. `relative_skills_dir` field is reused for project-local path computation.                                                                             | Config data                                                  | Unchanged                   |

### Data Flow

**Project registration flow:**

```
User clicks "Add Project"
  |
  v
ProjectList.tsx -> Tauri dialog.open({ directory: true })
  |
  v
User selects /home/alex/Projects/BDA
  |
  v
ProjectsTab calls invoke('add_project', { name: "BDA", path: "/home/alex/Projects/BDA" })
  |
  v
commands/projects.rs::add_project()
  - Validates: path exists, is directory, not already registered
  - Generates UUID
  - Inserts into `projects` table
  - Returns ProjectDto
  |
  v
useProjectState refreshes project list via invoke('list_projects')
```

**Tool column configuration flow:**

```
User opens tool column picker for project "BDA"
  |
  v
ProjectsTab calls invoke('set_project_tools', { projectId, tools: ["claude_code", "cursor"] })
  |
  v
commands/projects.rs::set_project_tools()
  - Deletes existing project_tools rows for this project
  - Inserts new rows for each selected tool
  - Returns updated tool list
  |
  v
AssignmentMatrix re-renders with selected tools as columns
```

**Skill assignment flow (the core interaction):**

```
User checks "brainstorming" for "BDA" project, "Claude Code" tool
  |
  v
AssignmentCell.tsx -> optimistic UI update (checkbox checked, status = gray/pending)
  |
  v
useProjectState calls invoke('assign_skill_to_project', {
  projectId: "bda-uuid",
  skillId: "brainstorming-uuid",
  tool: "claude_code"
})
  |
  v
commands/projects.rs::assign_skill_to_project()
  |
  v
core/project_sync.rs::sync_skill_to_project()
  1. Look up skill central_path from skill_store
  2. Look up tool adapter by key -> get relative_skills_dir (".claude/skills")
  3. Compute target: /home/alex/Projects/BDA/.claude/skills/brainstorming
  4. Compute source: /home/alex/.skillshub/brainstorming
  5. Call sync_engine::sync_dir_for_tool_with_overwrite(tool, source, target, true)
     (EXISTING FUNCTION, ZERO CHANGES)
  6. INSERT into project_skill_assignments with status="synced", content_hash, synced_at
  7. Return SyncResult
  |
  v
useProjectState refreshes assignments for this project
AssignmentCell updates: status = green (synced)
```

**Skill unassignment flow:**

```
User unchecks "brainstorming" for "BDA" project, "Claude Code" tool
  |
  v
AssignmentCell.tsx -> optimistic UI update (checkbox unchecked)
  |
  v
useProjectState calls invoke('unassign_skill_from_project', {
  projectId: "bda-uuid",
  skillId: "brainstorming-uuid",
  tool: "claude_code"
})
  |
  v
commands/projects.rs::unassign_skill_from_project()
  |
  v
core/project_sync.rs::unsync_skill_from_project()
  1. Compute target path (same resolution as assign)
  2. Remove symlink/copy at target path
  3. DELETE from project_skill_assignments
  |
  v
useProjectState refreshes assignments
```

**Sync all projects flow:**

```
User clicks "Sync All"
  |
  v
SyncStatusBar.tsx -> useProjectState.syncAllProjects()
  |
  v
invoke('sync_all_projects')
  |
  v
commands/projects.rs::sync_all_projects()
  |
  v
core/project_sync.rs::sync_all_projects()
  1. Load all projects from DB
  2. For each project:
     a. Validate project path still exists (mark missing if not)
     b. Load all assignments for this project
     c. For each assignment:
        - Resolve source and target paths
        - Call sync_engine primitive
        - Update assignment status and content_hash
  3. Return SyncAllReport { succeeded, failed, missing_projects }
  |
  v
useProjectState refreshes all project data
SyncStatusBar shows results
```

**Staleness check flow (copy-mode targets):**

```
On project selection (or periodic refresh)
  |
  v
useProjectState calls invoke('get_project_sync_status', { projectId })
  |
  v
core/project_sync.rs::check_project_sync_status()
  For each assignment where mode="copy":
    1. Read stored content_hash from assignment record
    2. Read current source content_hash from skill record
    3. Compare: match = "synced", mismatch = "stale", target missing = "missing"
  For symlink assignments:
    1. Check symlink exists and points to correct source
    2. Valid = "synced", broken = "missing"
  |
  v
Return per-assignment status array
AssignmentMatrix renders cells with appropriate colors
```

### Data Model

**Schema V4 Migration (three new tables):**

```sql
-- Projects registered for per-project skill distribution
CREATE TABLE projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  path TEXT NOT NULL UNIQUE,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

-- Which tools are configured (visible columns) per project
-- Without this, the matrix would show all 42+ tools as columns
CREATE TABLE project_tools (
  project_id TEXT NOT NULL,
  tool TEXT NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY(project_id, tool),
  FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);

-- Which skills are assigned to which projects, for which tools
CREATE TABLE project_skill_assignments (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  skill_id TEXT NOT NULL,
  tool TEXT NOT NULL,
  mode TEXT NOT NULL DEFAULT 'symlink',
  status TEXT NOT NULL DEFAULT 'pending',
  synced_at INTEGER NULL,
  content_hash TEXT NULL,
  UNIQUE(project_id, skill_id, tool),
  FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
  FOREIGN KEY(skill_id) REFERENCES skills(id) ON DELETE CASCADE
);

CREATE INDEX idx_psa_project ON project_skill_assignments(project_id);
CREATE INDEX idx_psa_skill ON project_skill_assignments(skill_id);
```

**Why `project_tools` exists as a separate table (not in the fork design doc but in PROJECT.md):**

The user typically works with 2-3 AI tools, not 42+. Without configurable tool columns, the matrix would have 42+ columns, which is unusable. `project_tools` lets each project declare which tools appear as columns. This is better than a JSON array in the `projects` table because:

1. SQL JOINs for filtering assignments by configured tools
2. `ON DELETE CASCADE` cleanup is automatic
3. `sort_order` column allows user-defined column ordering
4. Normalized, consistent with the rest of the schema

**Entity relationship summary:**

```
projects 1---* project_tools (which tools are visible per project)
projects 1---* project_skill_assignments (skill-tool assignments)
skills   1---* project_skill_assignments
skills   1---* skill_targets (existing global sync, unchanged)
```

A skill can be synced globally (via `skill_targets`) AND to specific projects (via `project_skill_assignments`). The two systems coexist independently.

## Patterns to Follow

### Pattern 1: Encapsulated Feature State via Custom Hook

**What:** Extract all Projects tab state into a `useProjectState()` custom hook, not into App.tsx.

**When:** Building a new tab/feature in a React app where the root component is already monolithic (App.tsx at 2087 lines, 50+ state vars).

**Why:** The existing codebase uses `useState` in App.tsx for everything. Adding 15+ more state variables for the projects feature would push it past 2200 lines. A custom hook provides the same simplicity as useState but scopes all state to the feature boundary.

**Example:**

```typescript
// src/components/projects/useProjectState.ts
import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectDto, AssignmentDto, ProjectSyncStatusDto } from "./types";

interface ProjectState {
  projects: ProjectDto[];
  selectedProjectId: string | null;
  assignments: AssignmentDto[];
  syncStatus: ProjectSyncStatusDto | null;
  loading: boolean;
  error: string | null;
}

export function useProjectState(managedSkills: ManagedSkill[]) {
  const [state, setState] = useState<ProjectState>({
    projects: [],
    selectedProjectId: null,
    assignments: [],
    syncStatus: null,
    loading: false,
    error: null,
  });

  const refreshProjects = useCallback(async () => {
    const projects = await invoke<ProjectDto[]>("list_projects");
    setState((prev) => ({ ...prev, projects }));
  }, []);

  const selectProject = useCallback(async (projectId: string) => {
    setState((prev) => ({
      ...prev,
      selectedProjectId: projectId,
      loading: true,
    }));
    const [assignments, syncStatus] = await Promise.all([
      invoke<AssignmentDto[]>("get_project_assignments", { projectId }),
      invoke<ProjectSyncStatusDto>("get_project_sync_status", { projectId }),
    ]);
    setState((prev) => ({ ...prev, assignments, syncStatus, loading: false }));
  }, []);

  const assignSkill = useCallback(
    async (projectId: string, skillId: string, tool: string) => {
      // Optimistic update
      setState((prev) => ({
        ...prev,
        assignments: [
          ...prev.assignments,
          {
            projectId,
            skillId,
            tool,
            status: "pending",
            mode: "symlink",
            syncedAt: null,
            contentHash: null,
          },
        ],
      }));
      try {
        await invoke("assign_skill_to_project", { projectId, skillId, tool });
        // Refresh real data
        const assignments = await invoke<AssignmentDto[]>(
          "get_project_assignments",
          { projectId },
        );
        setState((prev) => ({ ...prev, assignments }));
      } catch (err) {
        // Revert optimistic update on failure
        setState((prev) => ({
          ...prev,
          assignments: prev.assignments.filter(
            (a) => !(a.skillId === skillId && a.tool === tool),
          ),
        }));
        throw err;
      }
    },
    [],
  );

  // ... unassignSkill, syncProject, syncAllProjects, addProject, removeProject

  useEffect(() => {
    refreshProjects();
  }, [refreshProjects]);

  return { ...state, refreshProjects, selectProject, assignSkill /* ... */ };
}
```

**Why not useReducer:** The state transitions here are straightforward (load, toggle, refresh). `useReducer` is valuable when transitions have complex interdependencies or when the same action triggers multiple state changes. Here, each user action maps to one state update plus one backend call. The extra boilerplate of actions/reducer is not justified. If the state grows more complex later (e.g., undo/redo, batch operations), migrating to `useReducer` is a natural next step.

**Why not Zustand/Redux/Context:** The codebase has zero external state management. Introducing one for a single feature creates an inconsistency (App.tsx uses useState, Projects uses Zustand). The custom hook pattern is the minimal-disruption approach that still achieves clean separation.

### Pattern 2: Project-Aware Path Resolution

**What:** Compute project-local sync targets by substituting the project path for the home directory in the existing `relative_skills_dir` pattern.

**When:** Resolving where to place a skill symlink/copy within a project directory.

**Why:** The tool adapter's `relative_skills_dir` field (e.g., `.claude/skills`) is the same path segment whether rooted at `~` (global) or at a project directory (project-local). This means zero changes to the tool adapter registry.

**Example:**

```rust
// src-tauri/src/core/project_sync.rs

/// Resolve the target path for a skill within a project directory.
///
/// Global:  ~/          + .claude/skills/ + brainstorming
/// Project: /path/to/BDA/ + .claude/skills/ + brainstorming
pub fn resolve_project_target(
    project_path: &Path,
    tool_adapter: &ToolAdapter,
    skill_name: &str,
) -> PathBuf {
    project_path
        .join(tool_adapter.relative_skills_dir)
        .join(skill_name)
}
```

### Pattern 3: Thin Command Layer Delegation

**What:** New project commands go in a separate `commands/projects.rs` file, following the same pattern as the existing `commands/mod.rs`.

**When:** Adding a batch of related IPC commands to a Tauri app where the existing commands file is already ~985 lines.

**Why:** The existing `commands/mod.rs` is already large. Adding 8+ new commands (each ~30-50 lines) would push it past 1200 lines. Splitting into a dedicated file keeps each file focused. The commands still get registered in `lib.rs` via `generate_handler!`.

**Example:**

```rust
// src-tauri/src/commands/projects.rs
use serde::Serialize;
use tauri::State;
use crate::core::skill_store::SkillStore;

#[derive(Debug, Serialize)]
pub struct ProjectDto {
    pub id: String,
    pub name: String,
    pub path: String,
    pub assignment_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[tauri::command]
pub async fn add_project(
    store: State<'_, SkillStore>,
    name: String,
    path: String,
) -> Result<ProjectDto, String> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        // validate, insert, return
        todo!()
    })
    .await
    .map_err(|err| err.to_string())?
    .map_err(super::format_anyhow_error)
}

// ... other project commands following the same pattern
```

**Registration in lib.rs:**

```rust
// src-tauri/src/lib.rs
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    commands::projects::add_project,
    commands::projects::remove_project,
    commands::projects::list_projects,
    commands::projects::get_project_assignments,
    commands::projects::assign_skill_to_project,
    commands::projects::unassign_skill_from_project,
    commands::projects::sync_project,
    commands::projects::sync_all_projects,
    commands::projects::get_project_sync_status,
    commands::projects::set_project_tools,
    commands::projects::get_project_tools,
])
```

### Pattern 4: Schema Migration via PRAGMA user_version

**What:** Continue the existing incremental migration pattern in `ensure_schema()`, bumping `SCHEMA_VERSION` from 3 to 4.

**When:** Adding new tables to the SQLite database.

**Why:** The codebase already has a working migration pattern (V1 base, V2 added description column, V3 added source_subpath). Extending it to V4 is the lowest-risk approach. No new migration framework needed.

**Example:**

```rust
// In skill_store.rs, inside ensure_schema()

const SCHEMA_VERSION: i32 = 4;

// Add to the migration ladder:
if user_version < 4 {
    conn.execute_batch(r#"
        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS project_tools (
            project_id TEXT NOT NULL,
            tool TEXT NOT NULL,
            sort_order INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY(project_id, tool),
            FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS project_skill_assignments (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            skill_id TEXT NOT NULL,
            tool TEXT NOT NULL,
            mode TEXT NOT NULL DEFAULT 'symlink',
            status TEXT NOT NULL DEFAULT 'pending',
            synced_at INTEGER NULL,
            content_hash TEXT NULL,
            UNIQUE(project_id, skill_id, tool),
            FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
            FOREIGN KEY(skill_id) REFERENCES skills(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_psa_project
            ON project_skill_assignments(project_id);
        CREATE INDEX IF NOT EXISTS idx_psa_skill
            ON project_skill_assignments(skill_id);
    "#)?;
}
```

### Pattern 5: Optimistic UI with Revert-on-Error

**What:** Update the UI immediately on user action (checkbox toggle), then confirm or revert based on backend response.

**When:** Checkbox matrix interactions where latency would make the UI feel sluggish.

**Why:** Each checkbox toggle calls the backend to create/remove a symlink. This typically takes <100ms but can take longer on copy-mode or networked filesystems. Optimistic updates make the matrix feel instant.

**Example in AssignmentCell.tsx:**

```typescript
function AssignmentCell({ assigned, onToggle, status }: AssignmentCellProps) {
  const [optimistic, setOptimistic] = useState<boolean | null>(null)

  const handleClick = async () => {
    const newValue = !(optimistic ?? assigned)
    setOptimistic(newValue)  // immediate visual feedback
    try {
      await onToggle(newValue)
      setOptimistic(null)    // real data takes over
    } catch {
      setOptimistic(null)    // revert to real state
      // toast error shown by parent
    }
  }

  const isChecked = optimistic ?? assigned
  // status drives the color indicator
  return (
    <button onClick={handleClick} className={statusColor(status)}>
      {isChecked ? <CheckIcon /> : <EmptyIcon />}
    </button>
  )
}
```

## Anti-Patterns to Avoid

### Anti-Pattern 1: Adding State to App.tsx

**What:** Putting project-related useState calls in App.tsx.

**Why bad:** App.tsx is 2087 lines with 50+ state variables. Every line added increases coupling and cognitive load for all future changes. The Projects tab has no state dependencies on App.tsx's existing state (skills list comes from a separate backend call, tool status is fetched independently).

**Instead:** All project state lives in `useProjectState()` custom hook, consumed only by `ProjectsTab.tsx`. App.tsx only needs to know the tab exists for navigation switching.

**The one exception:** App.tsx must pass `managedSkills` (or the ability to refresh them) to ProjectsTab if the matrix needs to display skill names. However, this can be done via a prop rather than adding state. The hook can also independently call `get_managed_skills` if needed.

### Anti-Pattern 2: Modifying sync_engine.rs

**What:** Adding project-awareness to the sync engine's functions.

**Why bad:** The sync engine is intentionally path-generic. `sync_dir_hybrid(source, target)` works for any pair of paths. Adding project-specific logic here would break its generality and create coupling between the path resolution concern and the filesystem operations concern.

**Instead:** Project-aware path resolution happens in `project_sync.rs`. It computes `source` and `target`, then calls the existing sync_engine functions unchanged.

### Anti-Pattern 3: Single Monolithic Commands File

**What:** Adding all project commands to the existing `commands/mod.rs`.

**Why bad:** The file is already 985 lines. Adding 8+ commands with their DTO structs would push it past 1200+ lines, making it harder to navigate and increasing merge conflicts.

**Instead:** Create `commands/projects.rs` for all project-related commands. Keep `commands/mod.rs` for existing global sync commands. Register both sets in `lib.rs`.

### Anti-Pattern 4: Shared Directory Group Handling in Assignment Layer

**What:** Replicating the "shared skills directory" logic (e.g., Amp and Kimi CLI share `.config/agents/skills/`) in the project assignment layer.

**Why bad:** In global sync, shared directory groups matter because the same filesystem target path maps to multiple tools. In project-local sync, this is less relevant -- the user explicitly configures which tools are columns per project, and each tool creates its own directory under the project. However, if tools share a `relative_skills_dir`, creating a symlink for one tool creates it for the other too (same path).

**Instead:** Handle shared directories at the sync level (which already happens in the existing sync_engine via `is_same_link` detection). The assignment table records per-tool, and the UI can inform the user that some tools share directories. Do not auto-create assignment records for shared-directory tools -- let the user explicitly manage their tool columns.

### Anti-Pattern 5: Full Table Scan for Status Checks

**What:** Checking sync status for all projects on every tab switch.

**Why bad:** With many projects and many assignments, scanning all project directories for symlink status on every render causes noticeable lag.

**Instead:** Check status lazily -- only for the currently selected project. Use the stored `status` and `content_hash` fields for quick checks without filesystem access. Only do filesystem verification on explicit "Sync" or "Refresh" actions.

## Scalability Considerations

| Concern           | At 5 projects, 20 skills | At 20 projects, 50 skills      | At 50 projects, 100+ skills          |
| ----------------- | ------------------------ | ------------------------------ | ------------------------------------ |
| DB query time     | Negligible (<1ms)        | Negligible (<5ms)              | Still fast, indexes handle it        |
| Matrix render     | 5x2 = 10 cells           | 50x3 = 150 cells               | Needs virtualization or pagination   |
| Sync All time     | <1s (mostly symlinks)    | 5-10s                          | 30s+, needs progress indicator       |
| Status check      | Instant (lazy)           | Add per-project status caching | Batch check with progress bar        |
| Memory (frontend) | Trivial                  | ~100 assignment objects        | ~5000 assignment objects, still fine |

For the near term (Alex's 57+ skills, likely 5-10 projects, 2-3 tools each): the simple approach works. Virtualization and pagination are deferred enhancements, not requirements for MVP.

## Suggested Build Order (Dependencies)

The following order respects component dependencies -- each phase can be built and tested before the next begins.

### Phase 1: Backend Data Layer (no frontend dependency)

**Components:**

1. Schema V4 migration in `skill_store.rs` (new tables)
2. Project CRUD methods in `skill_store.rs` (add/remove/list projects, CRUD project_tools, CRUD assignments)
3. Unit tests for all new store methods

**Dependencies:** Only SQLite. No other component needs to exist yet.

**Verification:** `cargo test` passes. New tables created on app startup.

### Phase 2: Backend Sync Logic (depends on Phase 1)

**Components:**

1. `core/project_sync.rs` module (path resolution, sync/unsync/sync-all/staleness check)
2. Unit tests for path resolution and sync orchestration (using tempdir fixtures)

**Dependencies:** Phase 1 (needs store methods), existing sync_engine (unchanged), existing tool_adapters (unchanged).

**Verification:** `cargo test` passes. Can programmatically sync a skill to a temp project directory.

### Phase 3: Backend IPC Commands (depends on Phase 2)

**Components:**

1. `commands/projects.rs` (all IPC command handlers)
2. Registration in `lib.rs`
3. Integration tests (optional at this stage, but commands should be callable from frontend dev console)

**Dependencies:** Phase 2 (needs project_sync module), Phase 1 (needs store methods).

**Verification:** `npm run tauri:dev`, open browser devtools, invoke commands manually via `__TAURI_INTERNALS__`.

### Phase 4: Frontend Component Tree (depends on Phase 3)

**Components:**

1. `components/projects/types.ts` (TypeScript DTOs)
2. `components/projects/useProjectState.ts` (custom hook with all state + IPC)
3. `components/projects/ProjectsTab.tsx` (container)
4. `components/projects/ProjectList.tsx` (left panel)
5. `components/projects/AssignmentMatrix.tsx` (right panel)
6. `components/projects/AssignmentCell.tsx` (individual cell)
7. `components/projects/SyncStatusBar.tsx` (bottom bar)
8. `components/projects/ProjectCard.tsx` (project list item)
9. Minimal App.tsx change: add "Projects" to Header navigation, render ProjectsTab

**Dependencies:** Phase 3 (IPC commands must be available), existing components (Header for tab navigation).

**Verification:** Full end-to-end: register project, configure tools, assign skills via checkboxes, see symlinks appear in project directory, uncheck to remove.

### Phase 5: Edge Cases and Polish (depends on Phase 4)

**Components:**

1. .gitignore prompt on project registration
2. Missing/renamed project directory handling
3. Orphaned assignment detection (skill removed from library)
4. Content hash staleness detection for copy-mode
5. Cross-platform symlink testing

**Dependencies:** Phase 4 (need working UI to test edge cases).

**Verification:** Edge case matrix passes. All status indicators accurate.

### Build Order Rationale

The ordering is bottom-up: data layer first, then logic, then IPC surface, then UI. This means:

1. **Each phase is independently testable** via Rust unit tests (Phases 1-3) or functional testing (Phase 4-5).
2. **The sync engine remains untouched** -- validating this assumption early (Phase 2) de-risks the entire project.
3. **Frontend depends on all backend phases** -- building backend first means no integration surprises.
4. **Phase 4 (frontend) is the largest single phase** but has clear internal component boundaries.
5. **Phase 5 (polish) is isolated** -- edge cases don't require architectural changes.

## Key Architectural Decisions

| Decision                                        | Rationale                                                                                                                                                                                                                     | Confidence |
| ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| Reuse sync_engine unchanged                     | Already path-generic. Verified by reading `sync_dir_for_tool_with_overwrite` -- it takes `source: &Path` and `target: &Path` with no home-directory assumptions.                                                              | HIGH       |
| Separate `commands/projects.rs`                 | Existing commands file is 985 lines. Adding 8+ commands would exceed maintainability threshold. Tauri supports multiple command module files via `generate_handler!`.                                                         | HIGH       |
| Custom hook instead of state management library | No existing state library in codebase. useProjectState() encapsulates all state cleanly. Adding Zustand/Redux for one feature creates inconsistency.                                                                          | HIGH       |
| Three new tables (not two)                      | PROJECT.md specifies `project_tools` for configurable matrix columns. The fork design doc only shows `projects` and `project_skill_assignments`. The `project_tools` table is necessary because 42+ tool columns is unusable. | HIGH       |
| Schema V4 via existing migration ladder         | Codebase already has V1->V2->V3 incremental migrations in `ensure_schema()`. Extending to V4 is the natural, low-risk approach.                                                                                               | HIGH       |
| Optimistic UI for checkbox interactions         | Matrix interactions need to feel instant. Backend sync operations (symlink creation) are fast (<100ms) but not instant. Optimistic updates with revert-on-error match common UI patterns.                                     | MEDIUM     |
| Lazy status checking (selected project only)    | Checking all projects on tab switch would be slow at scale. Checking only the selected project keeps the common case fast.                                                                                                    | MEDIUM     |

## Sources

- Codebase analysis: `src-tauri/src/core/sync_engine.rs` (verified path-generic interface)
- Codebase analysis: `src-tauri/src/core/skill_store.rs` (verified migration pattern V1-V3)
- Codebase analysis: `src-tauri/src/commands/mod.rs` (verified command patterns and `format_anyhow_error`)
- Codebase analysis: `src-tauri/src/core/tool_adapters/mod.rs` (verified `relative_skills_dir` reuse)
- Codebase analysis: `src/App.tsx` lines 1-100 (verified 50+ state variables, no useReducer/context usage)
- Design document: `docs/plans/2026-04-02-skills-hub-fork-design.md` (data model, component layout, sync flow)
- Project requirements: `.planning/PROJECT.md` (constraints, decisions, `project_tools` table decision)
- Architecture analysis: `.planning/codebase/ARCHITECTURE.md` (layer boundaries, data flow patterns)

---

_Architecture analysis: 2026-04-07_
