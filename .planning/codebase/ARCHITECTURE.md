# Architecture

**Analysis Date:** 2026-04-07

## Pattern Overview

**Overall:** Desktop client with a monolithic React frontend and layered Tauri/Rust backend.

**Key Characteristics:**

- Use `src/App.tsx` as the single frontend orchestration layer for view state, modal state, tool status, onboarding state, settings state, and async action flows.
- Use Tauri IPC as the only frontend/backend boundary: React calls `invoke` through `invokeTauri()` in `src/App.tsx`, backend commands live in `src-tauri/src/commands/mod.rs`, and command registration lives in `src-tauri/src/lib.rs`.
- Keep business logic out of Tauri command functions: the command layer in `src-tauri/src/commands/mod.rs` converts inputs/outputs and delegates to pure core modules under `src-tauri/src/core/`.

## Layers

**Frontend bootstrap layer:**

- Purpose: Start the React app and load global styling and i18n.
- Location: `src/main.tsx`, `src/index.css`, `src/i18n/index.ts`
- Contains: React root creation, Tailwind/global CSS entry, i18next initialization.
- Depends on: React DOM, `src/App.tsx`, translation resources.
- Used by: Vite frontend startup and Tauri webview bootstrap configured in `src-tauri/tauri.conf.json`.

**Frontend application shell layer:**

- Purpose: Coordinate the entire visible application state and every user-triggered workflow.
- Location: `src/App.tsx`
- Contains: View switching (`myskills` / `explore` / `detail` / `settings`), modal visibility, skill list state, tool detection state, onboarding import state, update checks, settings persistence, and all Tauri command calls.
- Depends on: `src/components/skills/*.tsx`, `src/components/skills/modals/*.tsx`, `src/components/skills/types.ts`, `@tauri-apps/api/core`, Tauri plugins, i18n.
- Used by: `src/main.tsx` only.

**Frontend presentational feature layer:**

- Purpose: Render UI sections without owning the application-wide state model.
- Location: `src/components/skills/Header.tsx`, `src/components/skills/FilterBar.tsx`, `src/components/skills/SkillsList.tsx`, `src/components/skills/SkillCard.tsx`, `src/components/skills/ExplorePage.tsx`, `src/components/skills/SkillDetailView.tsx`, `src/components/skills/SettingsPage.tsx`, `src/components/skills/modals/*.tsx`
- Contains: Header tabs, search/sort bar, cards, detail viewer, settings form, pickers, import dialogs, delete dialog, shared-directory confirmation.
- Depends on: Props passed from `src/App.tsx`, shared DTO types in `src/components/skills/types.ts`, `react-i18next`, `lucide-react`, `sonner`, `react-markdown`, syntax highlighting.
- Used by: `src/App.tsx`.

**Frontend DTO boundary layer:**

- Purpose: Keep frontend type contracts aligned with backend command DTOs.
- Location: `src/components/skills/types.ts`
- Contains: `ManagedSkill`, `InstallResultDto`, `ToolStatusDto`, `OnboardingPlan`, `FeaturedSkillDto`, `OnlineSkillDto`, `SkillFileEntry`.
- Depends on: Backend response shapes defined in `src-tauri/src/commands/mod.rs`.
- Used by: Most components under `src/components/skills/` and `src/App.tsx`.

**Tauri bootstrap and dependency injection layer:**

- Purpose: Start the desktop runtime, register plugins, initialize storage, and expose shared state to commands.
- Location: `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`
- Contains: `app_lib::run()`, Tauri builder setup, plugin registration, SQLite store creation, cancel token registration, background cleanup tasks, and `generate_handler!` command registration.
- Depends on: `src-tauri/src/commands/mod.rs`, `src-tauri/src/core/skill_store.rs`, `src-tauri/src/core/cache_cleanup.rs`, `src-tauri/src/core/temp_cleanup.rs`, Tauri plugins.
- Used by: Desktop executable entry point.

**Tauri command layer:**

- Purpose: Define the IPC contract between the webview and the backend.
- Location: `src-tauri/src/commands/mod.rs`
- Contains: Tauri command functions, DTO structs, command-specific argument naming, error formatting, and `spawn_blocking` wrappers around synchronous core operations.
- Depends on: Shared state from `src-tauri/src/lib.rs`, all core modules under `src-tauri/src/core/`.
- Used by: Frontend `invokeTauri()` calls in `src/App.tsx` and detail view file readers in `src/components/skills/SkillDetailView.tsx`.

**Core domain layer:**

- Purpose: Implement skill management behavior independently from the desktop shell.
- Location: `src-tauri/src/core/`
- Contains: Install/update workflows in `src-tauri/src/core/installer.rs`, sync logic in `src-tauri/src/core/sync_engine.rs`, onboarding discovery in `src-tauri/src/core/onboarding.rs`, tool registry in `src-tauri/src/core/tool_adapters/mod.rs`, central repo resolution in `src-tauri/src/core/central_repo.rs`, local database access in `src-tauri/src/core/skill_store.rs`, online search in `src-tauri/src/core/github_search.rs` and `src-tauri/src/core/skills_search.rs`, featured catalog fetch in `src-tauri/src/core/featured_skills.rs`, file browsing in `src-tauri/src/core/skill_files.rs`, and cleanup/cancel helpers.
- Depends on: Filesystem, SQLite, HTTP, git/network helpers, home-directory resolution.
- Used by: `src-tauri/src/commands/mod.rs` and startup logic in `src-tauri/src/lib.rs`.

**Persistence layer:**

- Purpose: Persist managed skill metadata, sync targets, app settings, and schema migrations.
- Location: `src-tauri/src/core/skill_store.rs`
- Contains: SQLite schema creation, incremental migrations, CRUD for `skills`, `skill_targets`, and `settings`, legacy DB migration.
- Depends on: `rusqlite`, app data directory resolution via Tauri, filesystem.
- Used by: Nearly every backend workflow, especially `src-tauri/src/lib.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/core/installer.rs`, `src-tauri/src/core/onboarding.rs`, `src-tauri/src/core/featured_skills.rs`, and cache configuration helpers.

## Data Flow

**Initial app boot and state hydration:**

1. `src/main.tsx` mounts `src/App.tsx` after loading `src/index.css` and `src/i18n/index.ts`.
2. `src-tauri/src/main.rs` calls `app_lib::run()`, and `src-tauri/src/lib.rs` registers plugins, creates `SkillStore`, runs `ensure_schema()`, stores shared state with `app.manage(...)`, and registers IPC commands.
3. `src/App.tsx` boot effects call backend commands such as `get_managed_skills`, `get_onboarding_plan`, `get_tool_status`, `get_central_repo_path`, `get_git_cache_cleanup_days`, `get_git_cache_ttl_secs`, and `get_github_token`.
4. Presentational components render from the centralized state held in `src/App.tsx`.

**Skill install flow:**

1. User actions in `src/components/skills/ExplorePage.tsx`, `src/components/skills/modals/AddSkillModal.tsx`, `src/components/skills/modals/GitPickModal.tsx`, or `src/components/skills/modals/LocalPickModal.tsx` call handlers in `src/App.tsx`.
2. `src/App.tsx` invokes `install_local`, `install_local_selection`, `install_git`, or `install_git_selection` in `src-tauri/src/commands/mod.rs`.
3. The command layer delegates to `src-tauri/src/core/installer.rs`, which resolves the central repository via `src-tauri/src/core/central_repo.rs`, copies or downloads skill content, parses `SKILL.md`, computes hashes, and persists a `SkillRecord` through `src-tauri/src/core/skill_store.rs`.
4. `src/App.tsx` optionally invokes `sync_skill_to_tool` for each selected installed tool.
5. `src-tauri/src/core/sync_engine.rs` writes the target using symlink, junction, or copy, while `src-tauri/src/core/tool_adapters/mod.rs` resolves the tool directory and shared-directory groups.
6. `src/App.tsx` refreshes the list with `get_managed_skills`.

**Onboarding import flow:**

1. `src/App.tsx` requests `get_onboarding_plan` during startup or when the import banner is opened.
2. `src-tauri/src/core/onboarding.rs` scans home-directory tool paths from `src-tauri/src/core/tool_adapters/mod.rs`, filters already managed targets and the central repo, hashes results, and groups variants by skill name.
3. `src/components/skills/SkillsList.tsx` shows the import banner when `plan.total_skills_found > 0`.
4. Import confirmation in `src/App.tsx` calls `import_existing_skill`, then syncs the imported skill back to selected tool directories.

**Skill update flow:**

1. Update actions from `src/components/skills/SkillCard.tsx` call `handleUpdateManaged()` in `src/App.tsx`.
2. `src-tauri/src/commands/mod.rs` forwards `update_managed_skill` to `src-tauri/src/core/installer.rs`.
3. `src-tauri/src/core/installer.rs` reconstructs content from the original local path or git source, stages a replacement directory, swaps it into the central repo, and re-syncs existing targets.
4. `src/App.tsx` reloads managed skills and refreshes visible timestamps/status.

**Detail viewer file flow:**

1. `src/components/skills/SkillCard.tsx` opens a skill detail page through `onOpenDetail()`.
2. `src/components/skills/SkillDetailView.tsx` invokes `list_skill_files` and `read_skill_file`.
3. `src-tauri/src/core/skill_files.rs` walks the managed skill directory, blocks path traversal, enforces a 1 MB per-file read limit, and returns UTF-8 text.
4. The detail view renders Markdown or syntax-highlighted source inline.

**State Management:**

- Use local React hooks in `src/App.tsx` as the only state container; there is no Redux, Zustand, Context store, or router-driven state model in active use.
- Persist durable settings in two places: browser `localStorage` for UI-only settings like theme/language behavior in `src/App.tsx`, and SQLite settings via backend commands for app-wide configuration such as central repo path, git cache values, and GitHub token in `src-tauri/src/core/skill_store.rs`.

## Key Abstractions

**Managed skill record:**

- Purpose: Represent one installed skill in the app-managed central repository.
- Examples: `src-tauri/src/core/skill_store.rs`, `src-tauri/src/commands/mod.rs`, `src/components/skills/types.ts`
- Pattern: Persist backend canonical records as `SkillRecord`, expose UI-safe DTOs as `ManagedSkillDto`, mirror them in TypeScript as `ManagedSkill`.

**Skill target record:**

- Purpose: Represent one sync relationship between a managed skill and a tool directory.
- Examples: `src-tauri/src/core/skill_store.rs`, `src-tauri/src/commands/mod.rs`
- Pattern: Store `(skill_id, tool)` uniqueness in SQLite and materialize per-tool badges in `src/components/skills/SkillCard.tsx`.

**Tool adapter registry:**

- Purpose: Centralize supported AI tools, installation detection, and filesystem destinations.
- Examples: `src-tauri/src/core/tool_adapters/mod.rs`
- Pattern: Use `ToolAdapter` entries plus `ToolId` enum variants; resolve directories from the user home folder and group tools that share the same skills directory.

**Central repository:**

- Purpose: Decouple the app’s canonical managed copies from external tool directories.
- Examples: `src-tauri/src/core/central_repo.rs`, `src-tauri/src/core/installer.rs`, `src-tauri/src/commands/mod.rs`
- Pattern: Resolve a single base directory (`~/.skillshub` by default or a stored override), copy every imported skill into it, and sync outward from there.

**Command DTO contract:**

- Purpose: Keep IPC payloads explicit and serializable.
- Examples: `src-tauri/src/commands/mod.rs`, `src/components/skills/types.ts`
- Pattern: Define Rust DTO structs next to commands, serialize them with `serde`, and mirror the shapes in `src/components/skills/types.ts`.

**Cancellation token:**

- Purpose: Allow long-running git/download operations to be canceled from the UI.
- Examples: `src-tauri/src/core/cancel_token.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/commands/mod.rs`, `src/App.tsx`
- Pattern: Register one shared `Arc<CancelToken>` in Tauri state, reset it before install operations, and expose `cancel_current_operation` for the loading overlay cancel button.

## Entry Points

**Desktop executable entry point:**

- Location: `src-tauri/src/main.rs`
- Triggers: Native app startup.
- Responsibilities: Call `app_lib::run()` and keep the binary entry minimal.

**Tauri runtime entry point:**

- Location: `src-tauri/src/lib.rs`
- Triggers: `src-tauri/src/main.rs`.
- Responsibilities: Build the Tauri app, register plugins, initialize logging and persistence, schedule cleanup tasks, and register all frontend-callable commands.

**Frontend browser entry point:**

- Location: `src/main.tsx`
- Triggers: Vite dev server or Tauri webview page load.
- Responsibilities: Mount React, global CSS, and i18n.

**Frontend application controller:**

- Location: `src/App.tsx`
- Triggers: Every user interaction after initial render.
- Responsibilities: Orchestrate command calls, own all feature state, choose which top-level screen to render, and coordinate modal workflows.

**Unused/secondary UI shell:**

- Location: `src/components/Layout.tsx`, `src/pages/Dashboard.tsx`
- Triggers: Not wired into `src/main.tsx` or `src/App.tsx`.
- Responsibilities: Provide an alternate router-style shell with sidebar navigation and a placeholder dashboard. Treat these files as dormant until they are explicitly connected.

## Error Handling

**Strategy:** Convert backend `anyhow::Error` values into user-facing strings in `src-tauri/src/commands/mod.rs`, and let the frontend map known prefixes into translated UI messages in `src/App.tsx`.

**Patterns:**

- Use `tauri::async_runtime::spawn_blocking` in `src-tauri/src/commands/mod.rs` for filesystem, database, and network work so commands can remain async without rewriting the core layer.
- Use prefixed errors such as `MULTI_SKILLS|`, `TARGET_EXISTS|`, `TOOL_NOT_INSTALLED|`, `TOOL_NOT_WRITABLE|`, and `SKILL_INVALID|...` in `src-tauri/src/commands/mod.rs` and `src-tauri/src/core/installer.rs` to drive frontend-specific flows.
- Use `format_anyhow_error()` in `src-tauri/src/commands/mod.rs` to preserve error chains and rewrite common GitHub/network failures into clearer messages.
- Use toast-based display in `src/App.tsx` with `formatErrorMessage()` for end-user translation and suppression of canceled operations.

## Cross-Cutting Concerns

**Logging:** Use Tauri log plugin setup in `src-tauri/src/lib.rs`, `log::info!`/`log::warn!` in backend modules like `src-tauri/src/core/installer.rs`, and limited `console.warn` usage in `src/App.tsx` for non-fatal frontend failures.

**Validation:** Validate paths and names at the boundary. Examples: absolute path enforcement and `~` expansion in `src-tauri/src/commands/mod.rs`, path traversal checks and size caps in `src-tauri/src/core/skill_files.rs`, and skill-directory detection in `src-tauri/src/core/installer.rs` and `src-tauri/src/core/onboarding.rs`.

**Authentication:** Keep GitHub access token storage in SQLite settings through `get_github_token` and `set_github_token` in `src-tauri/src/commands/mod.rs`, then pass the token only to network helpers in `src-tauri/src/core/github_search.rs`, `src-tauri/src/core/featured_skills.rs`, and `src-tauri/src/core/installer.rs` download paths.

---

_Architecture analysis: 2026-04-07_
