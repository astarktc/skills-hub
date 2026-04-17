# Architecture

**Analysis Date:** 2026-04-16

## Pattern Overview

**Overall:** Hybrid single-shell desktop architecture with a React orchestration frontend, Tauri IPC command boundary, and Rust core-services backend.

**Key Characteristics:**

- Use `src/App.tsx` as the active frontend composition root and application state coordinator.
- Use Tauri commands as the only supported frontend/backend boundary, with handlers registered in `src-tauri/src/lib.rs` and implemented in `src-tauri/src/commands/mod.rs` plus `src-tauri/src/commands/projects.rs`.
- Keep business logic in Rust core modules under `src-tauri/src/core/`, with the command layer handling DTO mapping, argument naming, async wrapping, and error formatting.

## Layers

**Frontend Bootstrap Layer:**

- Purpose: Start the React UI and load global styling and i18n.
- Location: `src/main.tsx`, `src/index.css`, `src/i18n/index.ts`
- Contains: React root creation, StrictMode mount, CSS entrypoints, translation initialization.
- Depends on: `src/App.tsx`, React DOM, translation resources.
- Used by: Vite startup and the Tauri webview.

**Frontend App Shell / Orchestration Layer:**

- Purpose: Own top-level app state, screen switching, async workflows, and all Tauri calls for the main skills experience.
- Location: `src/App.tsx`
- Contains: View state (`myskills`, `explore`, `detail`, `settings`, `projects`), onboarding state, managed skill state, modal visibility, theme/language preferences, updater handling, and command helpers such as `invokeTauri()`.
- Depends on: `src/components/skills/*.tsx`, `src/components/skills/modals/*.tsx`, `src/components/projects/ProjectsPage.tsx`, `src/components/skills/types.ts`, `@tauri-apps/api/core`, Tauri plugins, and `src/i18n/resources.ts`.
- Used by: `src/main.tsx` only.

**Skills UI Presentation Layer:**

- Purpose: Render the skills-management flows without owning global backend integration policy.
- Location: `src/components/skills/Header.tsx`, `src/components/skills/FilterBar.tsx`, `src/components/skills/SkillsList.tsx`, `src/components/skills/SkillCard.tsx`, `src/components/skills/ExplorePage.tsx`, `src/components/skills/SkillDetailView.tsx`, `src/components/skills/SettingsPage.tsx`, `src/components/skills/LoadingOverlay.tsx`, `src/components/skills/modals/*.tsx`
- Contains: Navigation header, filtering UI, list and card rendering, skill detail file browser, explore/install UI, settings UI, loading overlay, and modal workflows.
- Depends on: Props from `src/App.tsx`, DTOs from `src/components/skills/types.ts`, `react-i18next`, `sonner`, `react-markdown`, and `react-syntax-highlighter`.
- Used by: `src/App.tsx`.

**Projects Feature Layer:**

- Purpose: Implement the per-project skill distribution UI as a self-contained subtree.
- Location: `src/components/projects/ProjectsPage.tsx`, `src/components/projects/useProjectState.ts`, `src/components/projects/AssignmentMatrix.tsx`, `src/components/projects/ProjectList.tsx`, `src/components/projects/*.tsx`, `src/components/projects/types.ts`
- Contains: Project registration/edit/remove flows, project tool configuration, assignment matrix, project-specific state hook, and DTO mirrors for project commands.
- Depends on: Direct Tauri `invoke` calls, `src/components/skills/types.ts` for shared skill/tool DTOs, and translation strings from `src/i18n/resources.ts`.
- Used by: `src/App.tsx` when `activeView === "projects"`.

**Frontend DTO Contract Layer:**

- Purpose: Keep serialized frontend contracts explicit and aligned with Rust DTOs.
- Location: `src/components/skills/types.ts`, `src/components/projects/types.ts`
- Contains: `ManagedSkill`, `ToolStatusDto`, `OnboardingPlan`, `InstallResultDto`, `ProjectDto`, `ProjectToolDto`, `ProjectSkillAssignmentDto`, `ResyncSummaryDto`, and related frontend wire types.
- Depends on: Backend response shapes in `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/projects.rs`, and Rust core DTO wrappers in `src-tauri/src/core/project_ops.rs`.
- Used by: Most frontend files under `src/components/skills/`, `src/components/projects/`, and `src/App.tsx`.

**Tauri Runtime Layer:**

- Purpose: Start the native app, initialize shared state, register plugins, and expose commands.
- Location: `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`
- Contains: `app_lib::run()`, logger setup, store initialization, schema migration, cancel token registration, sync mutex registration, cleanup jobs, and `generate_handler!` command registration.
- Depends on: `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/projects.rs`, `src-tauri/src/core/skill_store.rs`, `src-tauri/src/core/cache_cleanup.rs`, `src-tauri/src/core/temp_cleanup.rs`, and Tauri plugins.
- Used by: The packaged desktop executable.

**Tauri Command Layer:**

- Purpose: Define the IPC surface between React and Rust.
- Location: `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/projects.rs`
- Contains: Tauri command functions, serializable DTOs, frontend-facing camelCase parameter names, `spawn_blocking` wrappers, and `format_anyhow_error()`.
- Depends on: Shared `State<'_, SkillStore>`, shared `Arc<CancelToken>`, `SyncMutex`, and business logic modules under `src-tauri/src/core/`.
- Used by: Frontend `invoke` and `invokeTauri` calls in `src/App.tsx`, `src/components/projects/useProjectState.ts`, and `src/components/projects/ProjectsPage.tsx`.

**Core Business Logic Layer:**

- Purpose: Implement app behavior independently from Tauri-specific concerns.
- Location: `src-tauri/src/core/`
- Contains: Skill installation in `src-tauri/src/core/installer.rs`, project registration/cleanup in `src-tauri/src/core/project_ops.rs`, project assignment sync in `src-tauri/src/core/project_sync.rs`, global sync primitives in `src-tauri/src/core/sync_engine.rs`, persistence in `src-tauri/src/core/skill_store.rs`, onboarding in `src-tauri/src/core/onboarding.rs`, tool registry in `src-tauri/src/core/tool_adapters/mod.rs`, and remote fetching modules such as `src-tauri/src/core/github_search.rs`, `src-tauri/src/core/github_download.rs`, and `src-tauri/src/core/featured_skills.rs`.
- Depends on: Filesystem APIs, SQLite via `rusqlite`, path resolution, HTTP, git, hashing, and platform-specific sync helpers.
- Used by: `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/projects.rs`, and startup logic in `src-tauri/src/lib.rs`.

**Persistence Layer:**

- Purpose: Store canonical application data and migrations.
- Location: `src-tauri/src/core/skill_store.rs`
- Contains: SQLite schema creation, incremental migrations, settings storage, global skill CRUD, target CRUD, project CRUD, project tool CRUD, project assignment CRUD, aggregate status queries, and legacy DB migration helpers.
- Depends on: `rusqlite`, Tauri app-data path resolution, filesystem.
- Used by: Nearly every backend workflow, especially `src-tauri/src/lib.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/projects.rs`, `src-tauri/src/core/installer.rs`, `src-tauri/src/core/project_ops.rs`, and `src-tauri/src/core/project_sync.rs`.

## Data Flow

**Global Skill Management Flow:**

1. `src/App.tsx` loads managed skills and onboarding state with commands such as `get_managed_skills` and `get_onboarding_plan`.
2. `src-tauri/src/commands/mod.rs` receives the IPC call, wraps blocking work in `tauri::async_runtime::spawn_blocking`, and delegates to modules such as `src-tauri/src/core/installer.rs`, `src-tauri/src/core/onboarding.rs`, and `src-tauri/src/core/skill_store.rs`.
3. `src-tauri/src/core/skill_store.rs` persists canonical skill records and sync target records in SQLite, while sync operations use `src-tauri/src/core/sync_engine.rs` and tool metadata from `src-tauri/src/core/tool_adapters/mod.rs`.
4. DTOs are serialized back through `src-tauri/src/commands/mod.rs` and consumed by `src/App.tsx`, which re-renders `src/components/skills/*.tsx`.

**Project Skill Distribution Flow:**

1. `src/components/projects/ProjectsPage.tsx` delegates stateful operations to `src/components/projects/useProjectState.ts`.
2. `src/components/projects/useProjectState.ts` invokes commands such as `register_project`, `list_project_tools`, `add_project_skill_assignment`, and `resync_project` directly through `@tauri-apps/api/core`.
3. `src-tauri/src/commands/projects.rs` validates the request, acquires `SyncMutex` for mutating sync operations, and calls `src-tauri/src/core/project_ops.rs` or `src-tauri/src/core/project_sync.rs`.
4. `src-tauri/src/core/project_sync.rs` resolves project-specific target directories using adapters from `src-tauri/src/core/tool_adapters/mod.rs`, syncs files via `src-tauri/src/core/sync_engine.rs`, and updates assignment status in `src-tauri/src/core/skill_store.rs`.
5. `src/components/projects/useProjectState.ts` re-fetches projects, tools, and assignments to keep the assignment matrix in `src/components/projects/AssignmentMatrix.tsx` consistent with SQLite state.

**Skill Detail File Browser Flow:**

1. `src/components/skills/SkillDetailView.tsx` requests file lists and file contents from the backend using the passed `invokeTauri` helper.
2. `src-tauri/src/commands/mod.rs` exposes `list_skill_files` and `read_skill_file`.
3. `src-tauri/src/core/skill_files.rs` traverses the managed skill directory and returns file metadata/content.
4. `src/components/skills/SkillDetailView.tsx` builds a tree, renders markdown/code, and keeps all file-view state local to the detail view.

**State Management:**

- Use React local state in `src/App.tsx` as the primary application state container for the active shell.
- Use the custom hook `src/components/projects/useProjectState.ts` as a feature-local state container for project distribution flows.
- Persist durable state in SQLite through `src-tauri/src/core/skill_store.rs`.
- Persist UI-only preferences such as theme, language choice, and ignored updater version in browser storage from `src/App.tsx`.

## Key Abstractions

**Managed Skill:**

- Purpose: Represent one canonical skill stored in the central repository.
- Examples: `src/components/skills/types.ts`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/core/skill_store.rs`
- Pattern: Store canonical rows as `SkillRecord` in SQLite, expose DTOs from `src-tauri/src/commands/mod.rs`, and mirror the shape as `ManagedSkill` in TypeScript.

**Global Sync Target:**

- Purpose: Represent one global tool sync relationship for a managed skill.
- Examples: `src-tauri/src/core/skill_store.rs`, `src-tauri/src/commands/mod.rs`
- Pattern: Store `(skill_id, tool)` uniqueness in `skill_targets`, then surface target badges/status into the frontend skill cards rendered by `src/components/skills/SkillCard.tsx`.

**Project:**

- Purpose: Represent one registered repository or workspace directory that can receive selected skills.
- Examples: `src-tauri/src/core/project_ops.rs`, `src-tauri/src/core/skill_store.rs`, `src/components/projects/types.ts`
- Pattern: Persist a canonical `ProjectRecord` with derived counts and sync summary, then expose it as `ProjectDto` for the project list UI.

**Project Tool:**

- Purpose: Represent one tool enabled for a specific project.
- Examples: `src-tauri/src/core/skill_store.rs`, `src-tauri/src/commands/projects.rs`, `src/components/projects/types.ts`
- Pattern: Store one row per `(project_id, tool)` in `project_tools`, and use those rows to drive both assignment matrix columns and gitignore pattern derivation.

**Project Skill Assignment:**

- Purpose: Represent one skill-to-project-tool distribution record with deployment status.
- Examples: `src-tauri/src/core/project_sync.rs`, `src-tauri/src/core/skill_store.rs`, `src/components/projects/types.ts`
- Pattern: Insert a pending row before filesystem sync, then update `status`, `mode`, `last_error`, `synced_at`, and `content_hash` based on sync outcome and staleness checks.

**Tool Adapter Registry:**

- Purpose: Centralize supported AI tools and their filesystem conventions.
- Examples: `src-tauri/src/core/tool_adapters/mod.rs`
- Pattern: Define a `ToolId` enum and `ToolAdapter` records with `relative_skills_dir` and `relative_detect_dir`; resolve both global and project-local target directories from this registry.

**Central Repository:**

- Purpose: Decouple imported skills from tool-specific directories.
- Examples: `src-tauri/src/core/central_repo.rs`, `src-tauri/src/core/installer.rs`, `src-tauri/src/commands/mod.rs`
- Pattern: Store a single canonical managed copy per skill under the resolved central repository, then sync outward to tool directories or project directories.

**Sync Engine:**

- Purpose: Provide the primitive filesystem deployment strategy for both global sync and project sync.
- Examples: `src-tauri/src/core/sync_engine.rs`
- Pattern: Attempt symlink first, fall back to junction on Windows, then copy; force copy for Cursor via `sync_dir_for_tool_with_overwrite()`.

**Cancellation and Serialization Controls:**

- Purpose: Prevent overlapping destructive operations and allow UI-triggered cancellation.
- Examples: `src-tauri/src/lib.rs`, `src-tauri/src/core/cancel_token.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/projects.rs`
- Pattern: Store one shared `Arc<CancelToken>` for long-running installs and one shared `SyncMutex` to serialize assignment/removal/resync operations that mutate the filesystem.

## Entry Points

**Native Binary Entry:**

- Location: `src-tauri/src/main.rs`
- Triggers: Desktop process startup.
- Responsibilities: Call `app_lib::run()` and keep the binary entry minimal.

**Tauri App Builder:**

- Location: `src-tauri/src/lib.rs`
- Triggers: `src-tauri/src/main.rs`
- Responsibilities: Configure plugins, initialize logging, migrate/open SQLite, register shared state, run best-effort cleanup jobs, and register the full IPC handler surface.

**React Entry:**

- Location: `src/main.tsx`
- Triggers: Vite frontend load or Tauri webview page load.
- Responsibilities: Mount `src/App.tsx` under `StrictMode` and load `src/index.css` plus `src/i18n/index.ts`.

**Active Frontend Screen Router:**

- Location: `src/App.tsx`
- Triggers: Every post-mount user interaction.
- Responsibilities: Decide which top-level screen renders, wire callbacks, launch backend operations, and host global modals and toasts.

**Dormant Alternate Router Shell:**

- Location: `src/components/Layout.tsx`, `src/pages/Dashboard.tsx`
- Triggers: Not triggered by `src/main.tsx` or `src/App.tsx`.
- Responsibilities: Provide a `react-router-dom` layout and placeholder dashboard that are currently not part of the active application path.

## Error Handling

**Strategy:** Use Rust `Result`-based core logic, convert errors to frontend-safe strings at the Tauri boundary, and map known prefixes to UX-specific messages in the frontend.

**Patterns:**

- Wrap blocking backend work in `tauri::async_runtime::spawn_blocking` in `src-tauri/src/commands/mod.rs` and `src-tauri/src/commands/projects.rs`.
- Normalize backend errors through `format_anyhow_error()` in `src-tauri/src/commands/mod.rs` so prefix-based flows such as `TARGET_EXISTS|`, `TOOL_NOT_INSTALLED|`, `DUPLICATE_PROJECT|`, `ASSIGNMENT_EXISTS|`, and `NOT_FOUND|` survive serialization.
- Use feature-specific frontend formatters in `src/App.tsx` and `src/components/projects/useProjectState.ts` to translate raw strings into user-facing messages.
- Keep non-fatal refresh failures silent in selected UI paths, then re-fetch authoritative state when possible, as shown in `src/components/projects/useProjectState.ts`.

## Cross-Cutting Concerns

**Logging:** Backend logging is initialized in `src-tauri/src/lib.rs` via `tauri_plugin_log`, and core modules use `log::info!` / `log::warn!` for best-effort operational events.

**Validation:** Path canonicalization and duplicate detection are handled in `src-tauri/src/core/project_ops.rs`; tool-key validation is enforced in `src-tauri/src/commands/projects.rs`; target existence checks and overwrite policy live in `src-tauri/src/core/sync_engine.rs`.

**Authentication:** No app-level user auth layer is present. External access is token-based where needed, with GitHub token settings exposed through `src-tauri/src/commands/mod.rs` and persisted through `src-tauri/src/core/skill_store.rs`.

**Persistence:** SQLite is the system of record for skills, targets, settings, projects, and project assignments in `src-tauri/src/core/skill_store.rs`.

**File Synchronization:** All filesystem deployment converges on `src-tauri/src/core/sync_engine.rs`, with project-specific target resolution in `src-tauri/src/core/project_sync.rs` and global target management in `src-tauri/src/commands/mod.rs`.

**Internationalization:** User-visible strings flow through i18n resources loaded by `src/i18n/index.ts` and consumed from `src/App.tsx`, `src/components/skills/*.tsx`, and `src/components/projects/*.tsx`.

---

_Architecture analysis: 2026-04-16_
