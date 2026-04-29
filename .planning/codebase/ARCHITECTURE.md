<!-- refreshed: 2026-04-29 -->

# Architecture

**Analysis Date:** 2026-04-29

## System Overview

```text
┌─────────────────────────────────────────────────────────────┐
│                 React 19 Desktop Webview UI                 │
├──────────────────┬──────────────────┬───────────────────────┤
│  Skills Hub App  │  Projects UI     │ Settings / Details    │
│  `src/App.tsx`   │ `src/components/`│ `src/components/`     │
│                  │ `projects/`      │ `skills/`             │
└────────┬─────────┴────────┬─────────┴──────────┬────────────┘
         │                  │                     │
         │ Tauri IPC invoke │                     │
         ▼                  ▼                     ▼
┌─────────────────────────────────────────────────────────────┐
│                  Tauri Command Boundary                      │
│ `src-tauri/src/commands/mod.rs`                              │
│ `src-tauri/src/commands/projects.rs`                         │
└────────────────────────────┬────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                  Rust Core Business Layer                    │
│ `src-tauri/src/core/installer.rs`                            │
│ `src-tauri/src/core/project_ops.rs`                          │
│ `src-tauri/src/core/project_sync.rs`                         │
│ `src-tauri/src/core/sync_engine.rs`                          │
│ `src-tauri/src/core/skill_store.rs`                          │
└────────────────────────────┬────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│              SQLite + Filesystem + External Services         │
│ SQLite app DB via `src-tauri/src/core/skill_store.rs`        │
│ Central repo `~/.skillshub` via `central_repo.rs`            │
│ Tool/project skill dirs via `tool_adapters/mod.rs`           │
└─────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

| Component              | Responsibility                                                                                                                              | File                                                                                     |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| React entry point      | Mounts the React root, imports global CSS, initializes i18n, and renders `App`.                                                             | `src/main.tsx`                                                                           |
| App orchestrator       | Owns global skills UI state, view selection, modal state, settings state, update state, and most global Tauri IPC calls.                    | `src/App.tsx`                                                                            |
| Skills UI components   | Render skill list, filters, cards, detail view, explore view, settings page, and skill management modals.                                   | `src/components/skills/*.tsx`, `src/components/skills/modals/*.tsx`                      |
| Projects UI subtree    | Owns per-project UI state and flows separately from `App.tsx`, including project list, tool configuration, assignments, and resync actions. | `src/components/projects/ProjectsPage.tsx`, `src/components/projects/useProjectState.ts` |
| Frontend DTO types     | Defines TypeScript mirrors of Rust IPC DTOs for skills, tools, onboarding, project records, project tools, and project assignments.         | `src/components/skills/types.ts`, `src/components/projects/types.ts`                     |
| Desktop runtime        | Registers Tauri plugins, initializes SQLite storage, manages shared command state, runs cleanup tasks, and registers all IPC commands.      | `src-tauri/src/lib.rs`, `src-tauri/src/main.rs`                                          |
| Global command module  | Exposes skill, settings, search, sync, file browsing, and update-related Tauri commands; converts core errors to frontend strings.          | `src-tauri/src/commands/mod.rs`                                                          |
| Project command module | Exposes per-project registration, tool configuration, skill assignment, resync, and gitignore commands.                                     | `src-tauri/src/commands/projects.rs`                                                     |
| Skill installer        | Installs local/GitHub skills into the central repository, enriches metadata, parses skill content, and updates skill records.               | `src-tauri/src/core/installer.rs`                                                        |
| Project operations     | Implements project CRUD DTO construction, project cleanup, project path updates, and project-level coordination logic.                      | `src-tauri/src/core/project_ops.rs`                                                      |
| Project sync           | Implements assignment-to-filesystem sync, staleness detection, resync summaries, and assignment status updates.                             | `src-tauri/src/core/project_sync.rs`                                                     |
| Sync engine            | Provides low-level symlink/junction/copy primitives and safe target removal.                                                                | `src-tauri/src/core/sync_engine.rs`                                                      |
| Skill store            | Owns SQLite schema, migrations, settings, skills, global targets, projects, project tools, and assignments persistence.                     | `src-tauri/src/core/skill_store.rs`                                                      |
| Tool adapters          | Defines supported AI tools, keys, display names, global skill directories, detection directories, and project-relative skill paths.         | `src-tauri/src/core/tool_adapters/mod.rs`                                                |
| Dormant router shell   | Provides an alternate React Router layout/dashboard not wired into active app startup.                                                      | `src/components/Layout.tsx`, `src/pages/Dashboard.tsx`                                   |

## Pattern Overview

**Overall:** Tauri desktop app with a React orchestration shell, IPC command boundary, and Rust core-service layer over SQLite/filesystem primitives.

**Key Characteristics:**

- Use Tauri IPC as the only frontend/backend boundary. Frontend calls `invoke` from `src/App.tsx`, `src/components/projects/useProjectState.ts`, `src/components/projects/ProjectsPage.tsx`, and `src/components/skills/SkillDetailView.tsx`; backend commands are registered in `src-tauri/src/lib.rs`.
- Keep Rust business logic in `src-tauri/src/core/`. Tauri commands in `src-tauri/src/commands/mod.rs` and `src-tauri/src/commands/projects.rs` should wrap blocking work, convert DTOs, and format errors.
- Keep global skills workflows in `src/App.tsx`; keep per-project workflows inside `src/components/projects/` through `useProjectState()` and `ProjectsPage` handlers.
- Persist canonical state in SQLite via `src-tauri/src/core/skill_store.rs`; treat React state as an in-memory view model refreshed after operations.
- Use filesystem sync as a derived deployment artifact from central repo skill directories to global tool dirs or project tool dirs.

## Layers

**Frontend boot layer:**

- Purpose: Start React and global UI infrastructure.
- Location: `src/main.tsx`, `src/index.css`, `src/i18n/index.ts`, `src/i18n/resources.ts`
- Contains: React root creation, strict mode, CSS imports, i18next initialization, translation resources.
- Depends on: React DOM, `src/App.tsx`, i18n resources.
- Used by: Vite and the Tauri webview configured through `src-tauri/tauri.conf.json`.

**Frontend application orchestration layer:**

- Purpose: Coordinate top-level app views, global skills workflows, settings, updates, and cross-cutting UI state.
- Location: `src/App.tsx`
- Contains: `activeView`, `managedSkills`, onboarding/import state, install/update handlers, settings fetch/save handlers, theme/language persistence, zoom state, frontend error formatting, `invokeTauri()` lazy IPC wrapper.
- Depends on: `@tauri-apps/api/core`, Tauri plugins, `src/components/skills/`, `src/components/projects/ProjectsPage.tsx`, `src/components/skills/types.ts`, `sonner`, `react-i18next`.
- Used by: `src/main.tsx`.

**Skills presentation layer:**

- Purpose: Render reusable UI for managed skills, explore/search, detail browsing, filters, settings, loading overlays, and skill modals.
- Location: `src/components/skills/`, `src/components/skills/modals/`
- Contains: Presentational components such as `Header`, `FilterBar`, `SkillsList`, `SkillCard`, `ExplorePage`, `SkillDetailView`, `SettingsPage`, and modal components.
- Depends on: Props from `src/App.tsx`, `TFunction`, DTO types from `src/components/skills/types.ts`, icons, markdown rendering, syntax highlighting, and toast notifications.
- Used by: `src/App.tsx`.

**Projects feature layer:**

- Purpose: Isolate per-project skill distribution UI state and workflows from the large global app orchestrator.
- Location: `src/components/projects/`
- Contains: `ProjectsPage`, `useProjectState`, project list, assignment matrix, add/edit/tool/remove modals, project DTO types.
- Depends on: Direct `invoke` from `@tauri-apps/api/core`, directory picker from `@tauri-apps/plugin-dialog`, `ManagedSkill` and `ToolStatusDto` from `src/components/skills/types.ts`, project DTOs from `src/components/projects/types.ts`.
- Used by: `src/App.tsx` when `activeView === "projects"`.

**IPC command layer:**

- Purpose: Define the serialized contract between frontend TypeScript and Rust core logic.
- Location: `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/projects.rs`
- Contains: `#[tauri::command]` functions, DTO structs, camelCase command parameters, `spawn_blocking` wrappers, error prefix preservation, `format_anyhow_error()`.
- Depends on: Tauri `State`, `AppHandle`, `SkillStore`, `CancelToken`, `SyncMutex`, core modules under `src-tauri/src/core/`.
- Used by: `tauri::generate_handler!` in `src-tauri/src/lib.rs` and frontend `invoke` calls.

**Desktop runtime/setup layer:**

- Purpose: Construct the Tauri app, register plugins and commands, initialize persistence, and register shared mutable state.
- Location: `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`
- Contains: `app_lib::run()`, plugin registration, logger setup, database path resolution, schema migration, `app.manage(...)`, startup cleanup tasks, command registration.
- Depends on: Tauri runtime, Tauri plugins, `src-tauri/src/commands/`, `src-tauri/src/core/skill_store.rs`, cleanup modules.
- Used by: The packaged native executable.

**Rust core business layer:**

- Purpose: Implement app behavior without direct dependency on frontend rendering concerns.
- Location: `src-tauri/src/core/`
- Contains: installation, central repo resolution, onboarding discovery, global sync, project sync, project operations, GitHub/skills search, skill files, cleanup, content hashing, tool adapters, SQLite storage.
- Depends on: Filesystem, SQLite (`rusqlite`), Git (`git2`), HTTP (`reqwest`), hashing, home/app-data resolution, Tauri handles where app paths are required.
- Used by: Command layer and selected startup tasks in `src-tauri/src/lib.rs`.

**Persistence and filesystem layer:**

- Purpose: Store app metadata and materialize skills into central, global, and project destinations.
- Location: `src-tauri/src/core/skill_store.rs`, `src-tauri/src/core/central_repo.rs`, `src-tauri/src/core/sync_engine.rs`, `src-tauri/src/core/tool_adapters/mod.rs`
- Contains: SQLite schema versioning, settings, skills, targets, project tables, central repository path, symlink/junction/copy operations, tool directory resolution.
- Depends on: User app data directories, home directory, platform filesystem behavior.
- Used by: Installer, onboarding, sync, project operations, project sync, and settings commands.

## Data Flow

### Primary Managed Skill Install Path

1. User enters a local path or Git URL and submits through `src/App.tsx` handlers (`src/App.tsx:288`, `src/App.tsx:316`).
2. Frontend calls commands such as `install_local`, `list_local_skills_cmd`, `install_git`, or `install_git_selection` through `invokeTauri()` (`src/App.tsx:168`).
3. Backend command functions in `src-tauri/src/commands/mod.rs` wrap blocking work with `tauri::async_runtime::spawn_blocking` (`src-tauri/src/commands/mod.rs:127`).
4. Installer copies or downloads the skill into the central repository, parses metadata, computes hashes, and writes a `SkillRecord` (`src-tauri/src/core/installer.rs:29`).
5. SQLite persistence is updated through `SkillStore::upsert_skill()` (`src-tauri/src/core/skill_store.rs:265`).
6. Frontend refreshes the canonical skill list through `get_managed_skills` and updates `managedSkills` (`src/App.tsx:316`).

### Global Tool Sync Path

1. User toggles a tool target from skill UI in `src/App.tsx` or `src/components/skills/SkillCard.tsx`.
2. Frontend calls `sync_skill_to_tool`, `unsync_skill_from_tool`, `sync_skill_dir`, or related global sync commands registered in `src-tauri/src/lib.rs` (`src-tauri/src/lib.rs:116`).
3. Commands resolve tool metadata through `src-tauri/src/core/tool_adapters/mod.rs` and central source paths through SQLite records.
4. Low-level sync uses `sync_dir_for_tool_with_overwrite()` or `sync_dir_hybrid()` from `src-tauri/src/core/sync_engine.rs` (`src-tauri/src/core/sync_engine.rs:116`).
5. The sync engine attempts symlink, Windows junction, then copy fallback; Cursor is forced to copy mode (`src-tauri/src/core/sync_engine.rs:122`).
6. Target status is persisted in `skill_targets` via `src-tauri/src/core/skill_store.rs` and displayed in `ManagedSkill.targets` (`src/components/skills/types.ts:37`).

### Per-Project Assignment Path

1. User opens the Projects view rendered by `src/components/projects/ProjectsPage.tsx` and state is loaded by `useProjectState()` (`src/components/projects/useProjectState.ts:79`).
2. `useProjectState()` fetches projects, skills, tools, and assignments using `list_projects`, `get_managed_skills`, `list_project_tools`, and `list_project_skill_assignments` (`src/components/projects/useProjectState.ts:115`).
3. User toggles a skill/tool cell in `AssignmentMatrix`, which calls `toggleAssignment()` in `src/components/projects/useProjectState.ts` (`src/components/projects/useProjectState.ts:195`).
4. Frontend invokes `add_project_skill_assignment` or `remove_project_skill_assignment` with camelCase args (`src/components/projects/useProjectState.ts:211`).
5. Command layer serializes filesystem sync operations using `SyncMutex` (`src-tauri/src/commands/projects.rs:145`, `src-tauri/src/lib.rs:10`).
6. `project_sync::assign_and_sync()` inserts a pending assignment, resolves `<project>/<tool-skills-dir>/<skill-name>`, syncs from the central repo, and updates status (`src-tauri/src/core/project_sync.rs:29`).
7. Assignment records are reloaded through `list_project_skill_assignments`, which recalculates staleness/missing status (`src-tauri/src/core/project_sync.rs:225`).

### Project Registration and Tool Configuration Path

1. User selects a directory from `ProjectsPage` with `@tauri-apps/plugin-dialog` (`src/components/projects/ProjectsPage.tsx:206`).
2. Frontend invokes `register_project` or `update_project_path` (`src/components/projects/useProjectState.ts:170`).
3. `project_ops::register_project_path()` expands and canonicalizes paths, rejects duplicates with `DUPLICATE_PROJECT|`, and persists a `ProjectRecord` (`src-tauri/src/core/project_ops.rs:72`).
4. Tool selection invokes `add_project_tool` / `remove_project_tool` (`src/components/projects/useProjectState.ts:47`).
5. Removing a tool or project cleans up assignment artifacts through `project_ops::remove_tool_with_cleanup()` or `remove_project_with_cleanup()` (`src-tauri/src/core/project_ops.rs:102`).

### Detail File Browsing Path

1. User opens a skill detail view rendered by `src/components/skills/SkillDetailView.tsx`.
2. Frontend invokes `list_skill_files` and `read_skill_file` (`src/components/skills/SkillDetailView.tsx`).
3. Backend delegates to `src-tauri/src/core/skill_files.rs` for safe file listing and content reads.
4. Frontend renders Markdown through `react-markdown` and syntax-highlighted code blocks in `src/components/skills/SkillDetailView.tsx`.

**State Management:**

- Use React `useState`, `useMemo`, `useCallback`, and refs only; no Redux/Zustand/Context store is active.
- Keep global skills state in `src/App.tsx`; keep project feature state in `src/components/projects/useProjectState.ts`.
- Persist durable app data in SQLite through `src-tauri/src/core/skill_store.rs`; persist UI-only preferences such as theme, language, grouping, and view mode in `localStorage` from `src/App.tsx`.
- Refresh from backend after mutating operations rather than manually trusting optimistic state, especially `get_managed_skills`, `list_projects`, and `list_project_skill_assignments`.

## Key Abstractions

**ManagedSkill / SkillRecord:**

- Purpose: Represent one installed skill managed by Skills Hub.
- Examples: `src-tauri/src/core/skill_store.rs`, `src-tauri/src/commands/mod.rs`, `src/components/skills/types.ts`
- Pattern: Persist canonical `SkillRecord` rows in SQLite, convert them to command DTOs in `src-tauri/src/commands/mod.rs`, and mirror them as `ManagedSkill` in `src/components/skills/types.ts`.

**SkillTargetRecord:**

- Purpose: Represent one global tool sync relationship for an installed skill.
- Examples: `src-tauri/src/core/skill_store.rs`, `src-tauri/src/commands/mod.rs`, `src/components/skills/types.ts`
- Pattern: Store one `(skill_id, tool)` target in SQLite, materialize it under the tool's global skills directory, and expose target badges via `ManagedSkill.targets`.

**ProjectRecord / ProjectDto:**

- Purpose: Represent a registered project directory eligible for project-specific skill distribution.
- Examples: `src-tauri/src/core/skill_store.rs`, `src-tauri/src/core/project_ops.rs`, `src/components/projects/types.ts`
- Pattern: Store canonical project paths in SQLite, derive display name from path, and compute tool/skill/assignment counts when constructing DTOs.

**ProjectToolRecord:**

- Purpose: Represent an AI tool enabled for a registered project.
- Examples: `src-tauri/src/core/skill_store.rs`, `src-tauri/src/commands/projects.rs`, `src/components/projects/types.ts`
- Pattern: Store `(project_id, tool)` rows; resolve actual project-relative skill directories through `tool_adapters::project_relative_skills_dir()` and backend adapter definitions.

**ProjectSkillAssignmentRecord:**

- Purpose: Represent one skill assigned to one tool in one project.
- Examples: `src-tauri/src/core/skill_store.rs`, `src-tauri/src/core/project_sync.rs`, `src/components/projects/types.ts`
- Pattern: Insert pending row before filesystem sync, then update `status`, `mode`, `synced_at`, `content_hash`, and `last_error` after sync or failure.

**ToolAdapter / ToolId:**

- Purpose: Centralize supported AI tools, stable keys, labels, detection directories, and skill destination directories.
- Examples: `src-tauri/src/core/tool_adapters/mod.rs`
- Pattern: Add a `ToolId` variant, `as_key()` mapping, and `ToolAdapter` entry together; use adapter lookup in commands and sync code rather than hard-coding paths elsewhere.

**Central repository:**

- Purpose: Decouple the canonical managed copy of each skill from all global and project deployment targets.
- Examples: `src-tauri/src/core/central_repo.rs`, `src-tauri/src/core/installer.rs`, `src-tauri/src/core/project_sync.rs`
- Pattern: Install or import into a single central directory, then sync outward through `sync_engine` to global tool dirs and project tool dirs.

**SyncOutcome / SyncMode:**

- Purpose: Capture how a source directory was deployed to a target.
- Examples: `src-tauri/src/core/sync_engine.rs`, `src-tauri/src/core/project_sync.rs`
- Pattern: Prefer symlink, fall back to Windows junction, then copy; store `copy` content hashes for staleness checks.

**CancelToken:**

- Purpose: Allow long-running install/network operations to be canceled.
- Examples: `src-tauri/src/core/cancel_token.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/commands/mod.rs`, `src/App.tsx`
- Pattern: Manage one shared `Arc<CancelToken>` in Tauri state and check it from installer/GitHub/Git fetch paths.

## Entry Points

**Native executable:**

- Location: `src-tauri/src/main.rs`
- Triggers: Desktop application startup.
- Responsibilities: Call `app_lib::run()` and keep binary entry minimal.

**Tauri library runtime:**

- Location: `src-tauri/src/lib.rs`
- Triggers: Called by `src-tauri/src/main.rs`.
- Responsibilities: Register plugins, configure logging, initialize database schema, register shared state, run startup cleanup tasks, and expose commands through `generate_handler!`.

**React bootstrap:**

- Location: `src/main.tsx`
- Triggers: Webview page load or Vite dev server startup.
- Responsibilities: Import CSS/i18n and mount `<App />` under `#root`.

**Main frontend app:**

- Location: `src/App.tsx`
- Triggers: Initial React render and user interactions.
- Responsibilities: Own top-level UI state, call global skill/settings/update IPC commands, render skills/explore/detail/settings/projects views, and coordinate global modals.

**Projects feature:**

- Location: `src/components/projects/ProjectsPage.tsx`
- Triggers: `activeView === "projects"` from `src/App.tsx`.
- Responsibilities: Register projects, configure project tools, assign/unassign skills, resync project assignments, manage project-specific modals and toast feedback.

**Dormant router files:**

- Location: `src/components/Layout.tsx`, `src/pages/Dashboard.tsx`
- Triggers: Not wired from `src/main.tsx` or `src/App.tsx`.
- Responsibilities: Provide a React Router shell and dashboard placeholder only if a router-based app entry is intentionally introduced.

## Architectural Constraints

- **Threading:** Frontend runs in the browser/webview event loop. Backend command functions are async but wrap filesystem, SQLite, Git, and HTTP work in `tauri::async_runtime::spawn_blocking` from `src-tauri/src/commands/mod.rs` and `src-tauri/src/commands/projects.rs`.
- **Global state:** Shared backend state is registered with `app.manage(...)` in `src-tauri/src/lib.rs`: `SkillStore`, `Arc<CancelToken>`, and `SyncMutex`. Frontend global state is module-local inside `src/App.tsx`; project-specific state is local to `useProjectState()` in `src/components/projects/useProjectState.ts`.
- **Sync serialization:** Project sync mutations must acquire `SyncMutex` from `src-tauri/src/lib.rs` through `src-tauri/src/commands/projects.rs` to avoid concurrent filesystem writes into project tool directories.
- **Command registration:** New Tauri commands must be defined in `src-tauri/src/commands/mod.rs` or `src-tauri/src/commands/projects.rs` and registered in `tauri::generate_handler!` in `src-tauri/src/lib.rs`.
- **DTO synchronization:** New backend DTO fields must be mirrored in `src/components/skills/types.ts` or `src/components/projects/types.ts`.
- **Path expansion:** Frontend-provided paths can use `~`; commands should use `expand_home_path()` from `src-tauri/src/commands/mod.rs` or equivalent validated expansion before filesystem access.
- **Database migrations:** Schema changes belong in `src-tauri/src/core/skill_store.rs`; bump `SCHEMA_VERSION` and add incremental migration logic in `ensure_schema()`.
- **Tool paths:** Tool-specific directories must be resolved through `src-tauri/src/core/tool_adapters/mod.rs`; do not duplicate tool path strings in commands or frontend code.
- **Circular imports:** No TypeScript or Rust circular dependency chain is intentionally used. Keep `src-tauri/src/core/` modules callable from commands; do not import commands from core modules.

## Anti-Patterns

### Business logic inside Tauri commands

**What happens:** Commands in `src-tauri/src/commands/mod.rs` or `src-tauri/src/commands/projects.rs` grow filesystem/database workflows directly.
**Why it's wrong:** It makes core behavior harder to unit test and duplicates patterns already isolated in `src-tauri/src/core/installer.rs`, `src-tauri/src/core/project_ops.rs`, and `src-tauri/src/core/project_sync.rs`.
**Do this instead:** Put behavior in `src-tauri/src/core/*.rs`, export it from `src-tauri/src/core/mod.rs`, and make the command perform argument conversion, state cloning, `spawn_blocking`, DTO conversion, and `format_anyhow_error()` only.

### Hard-coded tool directories outside adapters

**What happens:** New code writes `.claude/skills`, `.cursor/skills`, or other tool-specific paths directly in frontend components or command handlers.
**Why it's wrong:** Tool keys, labels, global paths, project paths, and detection directories drift from the central registry.
**Do this instead:** Add or read tool data from `src-tauri/src/core/tool_adapters/mod.rs` and expose derived DTOs through `get_tool_status`, `list_project_tools`, or command-specific DTOs.

### Manual frontend state mutation after backend writes

**What happens:** A component mutates local arrays as if a backend write succeeded without reloading canonical state.
**Why it's wrong:** Backend sync may fail, return partial statuses, mark assignments stale/missing, or update hashes; optimistic frontend-only state can hide failed filesystem operations.
**Do this instead:** Re-fetch from commands such as `get_managed_skills`, `list_projects`, `list_project_tools`, and `list_project_skill_assignments`, matching `src/App.tsx` and `src/components/projects/useProjectState.ts`.

### Adding project feature state to `App.tsx`

**What happens:** Per-project modal, matrix, tool, and assignment state is added to the already-large global `src/App.tsx` state collection.
**Why it's wrong:** Projects are intentionally isolated in `src/components/projects/useProjectState.ts` to keep the feature maintainable.
**Do this instead:** Add project-specific state and actions to `src/components/projects/useProjectState.ts`, and keep `src/App.tsx` limited to selecting/rendering `ProjectsPage`.

### Bypassing project sync mutex

**What happens:** Assignment, resync, tool removal, or project removal code calls filesystem sync/cleanup without acquiring `SyncMutex`.
**Why it's wrong:** Parallel writes can corrupt symlinks, copied directories, or assignment status records.
**Do this instead:** Route project sync mutations through commands in `src-tauri/src/commands/projects.rs` that lock `SyncMutex` before calling `project_sync` or cleanup functions.

## Error Handling

**Strategy:** Rust core returns `anyhow::Result<T>` with context; Tauri commands convert errors to `Result<T, String>` and preserve known machine-readable prefixes; frontend normalizes strings and displays user-facing messages through toasts.

**Patterns:**

- Use `anyhow::Context` in core modules such as `src-tauri/src/core/skill_store.rs`, `src-tauri/src/core/installer.rs`, and `src-tauri/src/core/sync_engine.rs`.
- Use `format_anyhow_error()` in `src-tauri/src/commands/mod.rs` to preserve prefixes such as `MULTI_SKILLS|`, `TARGET_EXISTS|`, `TOOL_NOT_INSTALLED|`, `TOOL_NOT_WRITABLE|`, `SKILL_INVALID|`, `DUPLICATE_PROJECT|`, `ASSIGNMENT_EXISTS|`, and `NOT_FOUND|`.
- Use frontend translators `formatErrorMessage()` in `src/App.tsx` and `formatProjectError()` in `src/components/projects/useProjectState.ts` for prefixed backend errors.
- Use `sonner` toasts from `src/App.tsx`, `src/components/projects/ProjectsPage.tsx`, and skill components for user-visible failures/success.
- Store project assignment failures in `last_error` and `status` through `src-tauri/src/core/project_sync.rs` instead of throwing after the assignment row is created.

## Cross-Cutting Concerns

**Logging:** Backend initializes `tauri-plugin-log` in `src-tauri/src/lib.rs` and uses `log::info!` / `log::warn!` in startup cleanup, installer, project cleanup, and sync code. Frontend uses toast feedback and avoids direct console logging in active source files.

**Validation:** Path validation and canonicalization happen in Rust command/core code such as `expand_home_path()` in `src-tauri/src/commands/mod.rs` and `register_project_path()` in `src-tauri/src/core/project_ops.rs`. Skill installability and multi-skill detection happen in `src-tauri/src/core/installer.rs`.

**Authentication:** GitHub token storage is app-internal through settings commands in `src-tauri/src/commands/mod.rs` and SQLite settings in `src-tauri/src/core/skill_store.rs`; token use is in GitHub/search/download/install modules.

**Internationalization:** Active UI text should flow through `react-i18next` and resources in `src/i18n/resources.ts`; components receive `TFunction` props or call `useTranslation()` directly.

**Styling:** Active UI uses global CSS in `src/App.css` and `src/index.css` plus utility classes where present. The dormant router shell in `src/components/Layout.tsx` uses Tailwind utilities directly.

**Project Skills:** No project skills found under `.claude/skills/` or `.agents/skills/`; `.claude/skills/` exists but contains no skill subdirectories.

---

_Architecture analysis: 2026-04-29_
