# Technology Stack: Per-Project Skill Distribution

**Project:** Skills Hub - Per-Project Skill Distribution Milestone
**Researched:** 2026-04-07

## Recommended Stack

No new frameworks or major dependencies. This milestone extends the existing stack with targeted patterns and zero new crate/npm additions beyond what's already present.

### Core Framework (unchanged)

| Technology | Version                    | Purpose               | Why                                                                                                                                                                                                                                                                                                                  |
| ---------- | -------------------------- | --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Tauri 2    | 2.9.5 (current in project) | Desktop runtime + IPC | Already in use. Latest is 2.10.3 but no breaking changes or required features for this milestone. Stay on 2.9.5 to avoid unnecessary risk. Upgrade can be a separate maintenance task.                                                                                                                               |
| React 19   | 19.2.x                     | Frontend UI           | Already in use. No newer major version exists.                                                                                                                                                                                                                                                                       |
| Rust       | Edition 2021 (MSRV 1.77.2) | Backend logic         | Already in use. No change needed.                                                                                                                                                                                                                                                                                    |
| rusqlite   | 0.31 (current in project)  | SQLite persistence    | Already in use. Latest is 0.39.0 but upgrading introduces breaking changes (callback lifetimes in 0.32, statement validation in 0.35, `ToSql`/`FromSql` defaults changed in 0.38). The existing PRAGMA user_version migration pattern works perfectly for Schema V4. **Stay on 0.31 to avoid a multi-file upgrade.** |

**Confidence: HIGH** -- all versions verified against crates.io API.

### Database Layer

| Technology                 | Version        | Purpose                                                           | Why                                                                                                                                        |
| -------------------------- | -------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| rusqlite                   | 0.31 (bundled) | New tables for projects, project_tools, project_skill_assignments | Existing crate. Schema V4 migration using established PRAGMA user_version pattern.                                                         |
| SQLite PRAGMA user_version | N/A            | Schema versioning                                                 | Already V3 in codebase. Extend to V4 with `if user_version < 4` block in `ensure_schema()`. Proven pattern, no migration framework needed. |

**Confidence: HIGH** -- verified by reading existing `skill_store.rs`.

### Frontend Libraries

| Library                   | Version | Purpose                                | Why                                                                                                                                                                               |
| ------------------------- | ------- | -------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| React (hooks)             | 19.2.x  | Projects tab state management          | Use `useState` + `useCallback` in a dedicated `ProjectsTab.tsx`. No new state library needed -- the existing pattern works, and this feature lives in an isolated component tree. |
| Tailwind CSS 4            | 4.1.x   | Checkbox matrix styling                | Already in use. Grid/table layouts via utility classes.                                                                                                                           |
| lucide-react              | 0.562.0 | Status icons in matrix cells           | Already in use. Has CheckCircle, AlertTriangle, XCircle, Clock icons for synced/stale/missing/pending states.                                                                     |
| sonner                    | 2.0.7   | Toast notifications for sync results   | Already in use.                                                                                                                                                                   |
| @tauri-apps/plugin-dialog | 2.5.3   | Folder picker for project registration | Already in use. `open({ directory: true })` for selecting project directories.                                                                                                    |

**Confidence: HIGH** -- all already present in `package.json`.

### Infrastructure (unchanged)

| Technology             | Version        | Purpose                               | Why                                                                                                                                                        |
| ---------------------- | -------------- | ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `sync_engine.rs`       | N/A (internal) | Symlink/junction/copy with fallback   | Reuse as-is. `sync_dir_for_tool_with_overwrite()` accepts generic `source: &Path, target: &Path`. Per-project sync = same function, different target path. |
| `tool_adapters/mod.rs` | N/A (internal) | Tool path resolution                  | Extend with `resolve_project_path()` that joins `project_root + relative_skills_dir` instead of `home_dir + relative_skills_dir`.                          |
| uuid                   | 1.x (v4)       | ID generation for new records         | Already in use for `skill_targets`. Reuse for `project_skill_assignments`.                                                                                 |
| sha2 + hex             | 0.10 + 0.4     | Content hash for staleness detection  | Already in use. Reuse `content_hash.rs` for copy-mode staleness checks in project sync.                                                                    |
| dirs                   | 5.0            | Home directory resolution             | Already in use. No change needed.                                                                                                                          |
| walkdir                | 2.5            | Directory traversal for copy fallback | Already in use via sync_engine.                                                                                                                            |

**Confidence: HIGH** -- verified by reading source files.

## What NOT to Add (and Why)

| Category            | Rejected Option                     | Why Not                                                                                                                                                                                                                                          |
| ------------------- | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| State management    | Zustand, Jotai, Redux               | Overkill. Projects tab is a self-contained component tree with ~5 state variables. `useState` in `ProjectsTab.tsx` is sufficient. The app's existing pattern is props-from-parent, and the new tab is isolated enough to own its own state.      |
| Data table library  | @tanstack/react-table (v8.21.3)     | The checkbox matrix is not a generic data table -- it's a fixed structure (skills x tools) with custom cell renderers. A hand-rolled `<table>` with Tailwind is simpler, more maintainable, and avoids a 15KB dependency for a single component. |
| Virtual scrolling   | react-window, @tanstack/virtual     | Only needed if users have 100+ skills. Defer until evidence of performance problems. The matrix will have a search/filter bar that reduces visible rows.                                                                                         |
| Migration framework | refinery, barrel, diesel_migrations | The existing PRAGMA user_version pattern is simple, proven, and already handles V1->V2->V3 migrations cleanly. Adding a migration framework for one more migration step is pure overhead.                                                        |
| ORM                 | diesel, sea-orm                     | The codebase uses raw rusqlite with hand-written SQL. This is correct for a desktop app with 6 tables. ORMs add compile-time cost and abstraction that doesn't pay off here.                                                                     |
| New Tauri plugins   | None needed                         | Dialog plugin (folder picker) already installed. No new native APIs required.                                                                                                                                                                    |
| CSS-in-JS           | styled-components, emotion          | Tailwind 4 already in use. Adding another styling approach creates inconsistency.                                                                                                                                                                |
| rusqlite upgrade    | 0.39 (latest)                       | Breaking changes in 0.32 (callback lifetimes), 0.35 (statement validation), 0.38 (ToSql/FromSql defaults). Upgrade is a standalone maintenance task, not a feature milestone dependency.                                                         |

**Confidence: HIGH** -- based on actual codebase analysis, not speculation.

## Specific Technology Decisions

### 1. SQLite Schema V4 Migration Pattern

**Decision:** Extend existing `ensure_schema()` with an `if user_version < 4` block.

**Why:** The codebase already has a proven incremental migration pattern (V1 base schema, V2 added `description` column, V3 added `source_subpath` column). V4 adds three new tables -- this is the same pattern at larger scale but still straightforward.

**Schema V4 migration SQL:**

```sql
-- V4: Per-project skill distribution tables
CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  path TEXT NOT NULL UNIQUE,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS project_tools (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  tool_key TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  UNIQUE(project_id, tool_key),
  FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS project_skill_assignments (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  skill_id TEXT NOT NULL,
  tool_key TEXT NOT NULL,
  target_path TEXT NOT NULL,
  mode TEXT NOT NULL,
  status TEXT NOT NULL,
  content_hash TEXT NULL,
  last_error TEXT NULL,
  synced_at INTEGER NULL,
  created_at INTEGER NOT NULL,
  UNIQUE(project_id, skill_id, tool_key),
  FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
  FOREIGN KEY(skill_id) REFERENCES skills(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_psa_project ON project_skill_assignments(project_id);
CREATE INDEX IF NOT EXISTS idx_psa_skill ON project_skill_assignments(skill_id);
CREATE INDEX IF NOT EXISTS idx_projects_path ON projects(path);
```

**Key design choices:**

- `projects.path` is `UNIQUE` -- same directory cannot be registered twice
- `project_tools` stores per-project tool column preferences (not a JSON array in `projects`)
- `project_skill_assignments` has a triple unique constraint `(project_id, skill_id, tool_key)` -- one assignment per skill-tool pair per project
- `content_hash` on assignments enables staleness detection for copy-mode targets
- `ON DELETE CASCADE` on both FKs means removing a project or skill cleans up assignments automatically

**Confidence: HIGH** -- directly extends proven pattern from existing codebase.

### 2. Tauri Command Module Organization

**Decision:** Split `commands/mod.rs` (985 lines) into submodules. Keep existing commands in `commands/skills.rs`, add new project commands in `commands/projects.rs`.

**Why:** The current `commands/mod.rs` is already 985 lines with 28 commands. Adding 10+ project commands would push it past 1200 lines. Tauri 2 supports `commands::submodule::command_name` in `generate_handler!` with the module prefix stripped at runtime -- frontend invocation names stay flat.

**Structure:**

```
src-tauri/src/commands/
  mod.rs          -- shared DTOs, format_anyhow_error(), re-exports
  skills.rs       -- existing skill commands (moved here)
  projects.rs     -- new project commands
```

**Registration in lib.rs:**

```rust
.invoke_handler(tauri::generate_handler![
    // Existing skill commands
    commands::skills::get_managed_skills,
    commands::skills::install_local,
    // ... all existing commands ...

    // New project commands
    commands::projects::register_project,
    commands::projects::remove_project,
    commands::projects::list_projects,
    commands::projects::get_project_assignments,
    commands::projects::set_project_tools,
    commands::projects::assign_skill_to_project,
    commands::projects::unassign_skill_from_project,
    commands::projects::sync_project,
    commands::projects::sync_all_projects,
    commands::projects::get_project_sync_status,
])
```

**Risk mitigation:** This refactoring can be done as Phase 1 (before any feature work) to establish clean module boundaries.

**Confidence: HIGH** -- verified Tauri 2 docs confirm this pattern works.

### 3. Frontend Component Architecture

**Decision:** Build `src/components/projects/` as an isolated component tree with its own state. Wire into `App.tsx` only via a tab switch.

**Why:** The PROJECT.md constraint is explicit: "App.tsx is 2087 lines with 50+ state variables. The Projects tab will be a fully separate component tree." This decision is already validated by the project owner.

**Component tree:**

```
src/components/projects/
  ProjectsTab.tsx          -- Tab root, owns all project state
  ProjectList.tsx          -- Left panel: registered projects list
  ProjectToolConfig.tsx    -- Tool column picker for selected project
  AssignmentMatrix.tsx     -- Checkbox matrix: skills (rows) x tools (columns)
  AssignmentCell.tsx       -- Single cell with checkbox + status indicator
  ProjectSearchBar.tsx     -- Filter bar for the matrix
  types.ts                 -- Project-specific TypeScript DTOs
```

**State in ProjectsTab.tsx:**

```typescript
const [projects, setProjects] = useState<Project[]>([]);
const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
const [assignments, setAssignments] = useState<ProjectAssignment[]>([]);
const [projectTools, setProjectTools] = useState<string[]>([]);
const [searchFilter, setSearchFilter] = useState("");
```

**Pattern:** Fetch-on-select. When user selects a project, call `invoke('get_project_assignments', { projectId })` and `invoke('get_project_tools', { projectId })` to populate the matrix. Toggle a cell calls `invoke('assign_skill_to_project', ...)` or `invoke('unassign_skill_from_project', ...)` followed by a state refresh.

**Confidence: HIGH** -- follows existing codebase patterns; no new patterns introduced.

### 4. Checkbox Matrix UI Pattern

**Decision:** Hand-rolled HTML `<table>` with Tailwind classes. No data table library.

**Why:** The matrix has a fixed structure:

- Rows: skills (filtered by search bar)
- Columns: user-configured tools (typically 2-5, max ~10)
- Cells: checkbox + colored status dot
- Header row: tool names with optional "All" toggle column

This is too simple for `@tanstack/react-table` (which adds headless table logic for sorting, pagination, column resizing -- none needed here) and too specialized (each cell has checkbox + status indicator + sync action).

**Cell status rendering:**

| Status     | Visual              | Color (Tailwind)  |
| ---------- | ------------------- | ----------------- |
| synced     | Filled check circle | `text-green-500`  |
| stale      | Warning triangle    | `text-yellow-500` |
| missing    | X circle            | `text-red-500`    |
| pending    | Clock               | `text-gray-400`   |
| unassigned | Empty checkbox      | `text-gray-300`   |

Use `lucide-react` icons which are already in the project.

**Confidence: HIGH** -- standard React pattern, no library dependencies.

### 5. Cross-Platform Symlink Handling

**Decision:** Reuse existing `sync_engine.rs` with project-local target paths. No changes to the sync engine itself.

**Why:** The sync engine already handles the full fallback chain: symlink -> junction (Windows) -> copy. The only change is WHERE the target path points. Instead of `~/.claude/skills/<skill_name>` (global), the target becomes `<project_path>/.claude/skills/<skill_name>` (per-project).

**Path resolution function (new):**

```rust
// In tool_adapters/mod.rs or a new project_resolver.rs
pub fn resolve_project_skill_path(
    project_path: &Path,
    adapter: &ToolAdapter,
    skill_name: &str,
) -> PathBuf {
    project_path
        .join(adapter.relative_skills_dir)
        .join(skill_name)
}
```

**Platform-specific considerations:**

| Platform                         | Symlink Behavior                                                                | Notes                                                                                                                                                                         |
| -------------------------------- | ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Linux native**                 | `std::os::unix::fs::symlink` works everywhere                                   | No issues. Standard POSIX symlinks.                                                                                                                                           |
| **macOS**                        | `std::os::unix::fs::symlink` works everywhere                                   | No issues. APFS fully supports symlinks.                                                                                                                                      |
| **WSL2 (project in Linux FS)**   | `std::os::unix::fs::symlink` works                                              | Project at `/home/user/myproject`. Source at `/home/user/.skillshub/skill`. Both in ext4. No issues.                                                                          |
| **WSL2 (project in Windows FS)** | Symlink MAY fail on `/mnt/c/...`                                                | Project at `/mnt/c/Users/user/myproject`. DrvFS mount may not support symlinks depending on mount options (metadata flag). Falls back to copy via existing `sync_dir_hybrid`. |
| **Windows native**               | `std::os::windows::fs::symlink_dir` needs Developer Mode or elevated privileges | Existing code already handles this with junction fallback, then copy fallback. No changes needed.                                                                             |

**Critical insight:** The existing sync engine already handles all these cases. `sync_dir_hybrid` tries symlink first, falls through to junction on Windows, then falls through to copy. The per-project feature just passes different paths -- the fallback logic is unchanged.

**The Cursor exception is already handled:** `sync_dir_for_tool_with_overwrite()` forces copy mode for Cursor. This applies equally to project-local paths.

**Confidence: HIGH** -- verified by reading sync_engine.rs source code. The abstraction is path-generic by design.

### 6. Project Path Validation

**Decision:** Validate project paths at registration time using filesystem checks.

**Validations needed:**

1. Path exists and is a directory
2. Path is absolute (no relative paths)
3. Path is not the home directory itself
4. Path is not inside `~/.skillshub/` (the central repo)
5. Path is not already registered (enforced by UNIQUE constraint on `projects.path`)
6. Path does not contain another registered project (no nesting)

**Graceful handling of removed directories:** When listing projects, check `path.exists()` and mark missing projects with a warning status. Do not auto-delete -- the user may have temporarily unmounted a drive.

**Confidence: HIGH** -- straightforward filesystem validation, standard patterns.

### 7. .gitignore Integration

**Decision:** Prompt-based, not automatic. After project registration, check if tool skill directories would be gitignored. If not, offer to append entries.

**Pattern:**

1. After registering a project, scan configured tools
2. For each tool, check if `<project>/<relative_skills_dir>/` is gitignored (read `.gitignore` and check patterns)
3. If any are not ignored, show a confirmation dialog listing the entries to add
4. On confirmation, append entries to `<project>/.gitignore` (create if needed)

**Why prompt, not auto:** Respects user's git workflow preferences. Some users may want skills committed; others may use `.git/info/exclude` instead.

**Confidence: MEDIUM** -- the pattern is clear but `.gitignore` parsing has edge cases (nested `.gitignore` files, negation patterns). A simple line-by-line check for the exact pattern is sufficient for MVP.

## Alternatives Considered

| Category                | Recommended                            | Alternative                   | Why Not                                                                                                                                         |
| ----------------------- | -------------------------------------- | ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Schema migration        | PRAGMA user_version (existing)         | refinery crate                | One migration step doesn't justify a new dependency. The existing pattern handles it cleanly.                                                   |
| State management        | useState in ProjectsTab                | Zustand                       | Isolated component tree with <10 state variables. Zustand adds indirection without benefit at this scale.                                       |
| Matrix UI               | Hand-rolled table                      | @tanstack/react-table v8      | Matrix cells have custom rendering (checkbox + status icon + sync action). A headless table library adds complexity for a fixed-structure grid. |
| Command organization    | Module split (skills.rs + projects.rs) | Keep single mod.rs            | 985 lines already. Adding 10+ commands crosses the maintainability threshold. Module split is a one-time cost that pays off permanently.        |
| rusqlite version        | Stay on 0.31                           | Upgrade to 0.39               | Multiple breaking changes across versions. Upgrade is a maintenance task, not a feature dependency.                                             |
| Per-project tool config | Separate `project_tools` table         | JSON array in `projects.path` | SQL JOINs are cleaner than JSON parsing. Separate table enables indexed queries like "which projects use Claude Code?"                          |

## Installation

No new packages to install. The existing `package.json` and `Cargo.toml` dependencies cover everything needed for this milestone.

```bash
# Nothing new -- all dependencies already present
npm install         # existing frontend deps
cargo build         # existing backend deps
```

## Version Verification Summary

| Dependency            | Project Version               | Latest Available | Action                                          | Confidence                                    |
| --------------------- | ----------------------------- | ---------------- | ----------------------------------------------- | --------------------------------------------- |
| Tauri                 | 2.9.5                         | 2.10.3           | Stay on 2.9.5 (no required features in 2.10)    | HIGH (verified crates.io API)                 |
| rusqlite              | 0.31                          | 0.39.0           | Stay on 0.31 (breaking changes in upgrade path) | HIGH (verified crates.io API, read changelog) |
| React                 | 19.2.x                        | 19.2.x           | Already current                                 | HIGH (npm registry)                           |
| @tanstack/react-table | Not used                      | 8.21.3           | Do not add                                      | HIGH (verified npm registry)                  |
| All other deps        | As in Cargo.toml/package.json | N/A              | No changes                                      | HIGH                                          |

## Sources

- rusqlite version: crates.io API (`https://crates.io/api/v1/crates/rusqlite`) -- 0.39.0 latest
- rusqlite changelog: GitHub releases (`https://github.com/rusqlite/rusqlite/releases`) -- breaking changes documented
- Tauri version: crates.io API (`https://crates.io/api/v1/crates/tauri`) -- 2.10.3 latest
- Tauri command organization: Official docs (`https://v2.tauri.app/develop/calling-rust/`)
- Tauri state management: Official docs (`https://v2.tauri.app/develop/state-management/`)
- Tauri dialog plugin: Official docs (`https://v2.tauri.app/plugin/dialog/`)
- @tanstack/react-table: npm registry (`https://registry.npmjs.org/@tanstack/react-table/latest`) -- 8.21.3
- WSL2 file permissions: Microsoft docs (`https://learn.microsoft.com/en-us/windows/wsl/file-permissions`)
- WSL2 filesystems: Microsoft docs (`https://learn.microsoft.com/en-us/windows/wsl/filesystems`)
- Existing codebase: Direct source code analysis of `skill_store.rs`, `sync_engine.rs`, `commands/mod.rs`, `tool_adapters/mod.rs`, `lib.rs`, `types.ts`
