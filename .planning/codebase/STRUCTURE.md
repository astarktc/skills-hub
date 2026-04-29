# Codebase Structure

**Analysis Date:** 2026-04-29

## Directory Layout

```
skills-hub/
├── src/                         # React 19 + TypeScript frontend
│   ├── main.tsx                 # Frontend bootstrap
│   ├── App.tsx                  # Top-level app orchestration
│   ├── App.css                  # Active app component styles
│   ├── index.css                # Global CSS, theme variables, Tailwind entry
│   ├── i18n/                    # i18next setup and translation resources
│   ├── components/
│   │   ├── skills/              # Managed skills, explore, settings, and modals UI
│   │   ├── projects/            # Per-project distribution feature UI/state
│   │   └── Layout.tsx           # Dormant router-style layout shell
│   ├── pages/                   # Dormant router-style pages
│   └── tauri-plugin-dialog.d.ts # Tauri dialog plugin type shim
├── src-tauri/                   # Tauri 2 Rust backend and native app config
│   ├── src/
│   │   ├── main.rs              # Native binary entry point
│   │   ├── lib.rs               # Tauri builder, plugins, state, command registration
│   │   ├── commands/            # IPC command boundary
│   │   └── core/                # Backend business logic, persistence, sync, tests
│   ├── Cargo.toml               # Rust crate/dependency configuration
│   ├── Cargo.lock               # Rust dependency lockfile
│   ├── build.rs                 # Tauri build integration
│   └── tauri.conf.json          # Desktop packaging/runtime config
├── scripts/                     # Node release/version/catalog utility scripts
├── .github/workflows/           # CI, release, featured skills automation
├── .planning/                   # GSD planning and codebase mapping documents
├── .claude/                     # Claude/GSD commands, agents, hooks, local project config
├── .codex/                      # Codex/GSD commands, agents, hooks, skills distribution
├── .opencode/                   # OpenCode command/agent distribution artifacts
├── package.json                 # Frontend scripts and npm dependencies
├── package-lock.json            # npm lockfile
├── tsconfig.json                # TypeScript project references root
├── tsconfig.app.json            # Frontend TypeScript compiler settings
├── tsconfig.node.json           # Node/Vite TypeScript compiler settings
├── vite.config.ts               # Vite React/Tailwind config
├── eslint.config.js             # ESLint flat config
├── featured-skills.json         # Featured skills catalog data
└── README.md                    # Project overview and developer setup
```

## Directory Purposes

**`src/`:**

- Purpose: Active React/TypeScript frontend rendered inside the Tauri webview.
- Contains: App bootstrap, app orchestration, global CSS, i18n setup, active skills UI, active projects UI, dormant router UI.
- Key files: `src/main.tsx`, `src/App.tsx`, `src/App.css`, `src/index.css`, `src/i18n/index.ts`, `src/i18n/resources.ts`.

**`src/components/skills/`:**

- Purpose: UI for the original managed skills workflow: My Skills, Explore, detail view, settings, filters, loading overlay, skill cards, and modals.
- Contains: Mostly memoized presentational components plus shared TypeScript DTOs in `src/components/skills/types.ts`.
- Key files: `src/components/skills/SkillCard.tsx`, `src/components/skills/SkillsList.tsx`, `src/components/skills/ExplorePage.tsx`, `src/components/skills/SkillDetailView.tsx`, `src/components/skills/SettingsPage.tsx`, `src/components/skills/Header.tsx`, `src/components/skills/types.ts`.

**`src/components/skills/modals/`:**

- Purpose: Modal dialogs used by skills onboarding, installation, delete, and tool discovery flows.
- Contains: `AddSkillModal`, `DeleteModal`, `GitPickModal`, `ImportModal`, `LocalPickModal`, `NewToolsModal`, and `SharedDirModal`.
- Key files: `src/components/skills/modals/AddSkillModal.tsx`, `src/components/skills/modals/GitPickModal.tsx`, `src/components/skills/modals/LocalPickModal.tsx`.

**`src/components/projects/`:**

- Purpose: Per-project skill distribution UI and project-specific state management.
- Contains: Page container, custom state hook, assignment matrix, project list, add/edit/tool/remove modals, and project DTO types.
- Key files: `src/components/projects/ProjectsPage.tsx`, `src/components/projects/useProjectState.ts`, `src/components/projects/AssignmentMatrix.tsx`, `src/components/projects/ProjectList.tsx`, `src/components/projects/types.ts`.

**`src/i18n/`:**

- Purpose: Frontend localization setup and resources.
- Contains: i18next initialization and translation dictionaries.
- Key files: `src/i18n/index.ts`, `src/i18n/resources.ts`.

**`src/components/Layout.tsx` and `src/pages/`:**

- Purpose: Dormant React Router-style shell and page area.
- Contains: `Layout` with sidebar navigation and `Dashboard` placeholder.
- Key files: `src/components/Layout.tsx`, `src/pages/Dashboard.tsx`.

**`src-tauri/`:**

- Purpose: Rust/Tauri backend, native runtime setup, desktop packaging, and Rust dependency management.
- Contains: Rust source, Cargo manifests, Tauri config, icons/capabilities if present, and build script.
- Key files: `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`, `src-tauri/build.rs`.

**`src-tauri/src/`:**

- Purpose: Rust crate source for the native app.
- Contains: Minimal binary entry, Tauri library runtime, command modules, and core modules.
- Key files: `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/core/mod.rs`.

**`src-tauri/src/commands/`:**

- Purpose: Tauri IPC command boundary.
- Contains: Global commands in `mod.rs`, project-specific commands in `projects.rs`, and command tests.
- Key files: `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/projects.rs`, `src-tauri/src/commands/tests/commands.rs`.

**`src-tauri/src/core/`:**

- Purpose: Pure/backend business logic for persistence, installation, sync, search, cleanup, and tool registry behavior.
- Contains: Focused Rust modules exported by `src-tauri/src/core/mod.rs` and backend unit tests under `src-tauri/src/core/tests/`.
- Key files: `src-tauri/src/core/skill_store.rs`, `src-tauri/src/core/installer.rs`, `src-tauri/src/core/project_ops.rs`, `src-tauri/src/core/project_sync.rs`, `src-tauri/src/core/sync_engine.rs`, `src-tauri/src/core/tool_adapters/mod.rs`.

**`src-tauri/src/core/tool_adapters/`:**

- Purpose: Tool registry for supported AI coding tools.
- Contains: `ToolId`, `ToolAdapter`, default adapter list, adapter lookup, detection, and path resolution helpers.
- Key files: `src-tauri/src/core/tool_adapters/mod.rs`.

**`src-tauri/src/core/tests/`:**

- Purpose: Rust unit/integration-style tests for core modules.
- Contains: Tests mirroring core module names.
- Key files: `src-tauri/src/core/tests/project_sync.rs`, `src-tauri/src/core/tests/project_ops.rs`, `src-tauri/src/core/tests/skill_store.rs`, `src-tauri/src/core/tests/installer.rs`, `src-tauri/src/core/tests/sync_engine.rs`.

**`scripts/`:**

- Purpose: Node-based developer/release automation.
- Contains: Version synchronization, Tauri icon helper, featured skills fetch script.
- Key files: `scripts/version.mjs`, `scripts/fetch-featured-skills-v2.mjs`, `scripts/tauri-icon-desktop.mjs`.

**`.github/workflows/`:**

- Purpose: GitHub Actions automation.
- Contains: CI checks, release builds, and featured skills updates.
- Key files: `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/workflows/update-featured-skills.yml`.

**`.planning/`:**

- Purpose: GSD project planning, roadmap, phase, debug, research, and codebase map artifacts.
- Contains: Project state documents, phase plans, quick/debug/forensics notes, and codebase analysis docs.
- Key files: `.planning/PROJECT.md`, `.planning/ROADMAP.md`, `.planning/STATE.md`, `.planning/codebase/ARCHITECTURE.md`, `.planning/codebase/STRUCTURE.md`.

**`.claude/`, `.codex/`, `.opencode/`:**

- Purpose: Distributed agent command/skill/config artifacts for different AI coding tools.
- Contains: GSD commands, agents, hooks, settings, manifests, and tool-specific packaging files.
- Key files: `.claude/agents/gsd-codebase-mapper.md`, `.codex/skills/gsd-map-codebase/SKILL.md` if present, `.opencode/command/gsd-map-codebase.md`.

## Key File Locations

**Entry Points:**

- `src/main.tsx`: Frontend React bootstrap.
- `src/App.tsx`: Active top-level app component and primary frontend orchestrator.
- `src-tauri/src/main.rs`: Native binary entry point.
- `src-tauri/src/lib.rs`: Tauri runtime setup, plugin registration, backend state registration, command registration.

**Configuration:**

- `package.json`: npm scripts, frontend dependencies, Tauri CLI scripts.
- `vite.config.ts`: Vite plugins and dev server port.
- `tsconfig.json`: TypeScript project references.
- `tsconfig.app.json`: Strict frontend TypeScript compiler settings.
- `tsconfig.node.json`: TypeScript settings for Node/Vite config files.
- `eslint.config.js`: ESLint flat config.
- `src-tauri/Cargo.toml`: Rust crate, MSRV, backend dependencies.
- `src-tauri/tauri.conf.json`: Tauri app identifier, windows, bundle, updater, and security configuration.
- `.github/workflows/ci.yml`: CI check workflow.
- `.github/workflows/release.yml`: Cross-platform release packaging workflow.

**Core Logic:**

- `src-tauri/src/core/skill_store.rs`: SQLite schema, migrations, settings, skill records, global sync targets, project records, project tools, project assignments.
- `src-tauri/src/core/installer.rs`: Local and Git/GitHub skill install/update logic.
- `src-tauri/src/core/sync_engine.rs`: Symlink/junction/copy sync primitives and removal helper.
- `src-tauri/src/core/tool_adapters/mod.rs`: AI tool registry and path resolution.
- `src-tauri/src/core/project_ops.rs`: Project registration/update/removal and cleanup orchestration.
- `src-tauri/src/core/project_sync.rs`: Project assignment sync, resync, and staleness logic.
- `src-tauri/src/core/onboarding.rs`: Existing skills discovery and import plan construction.
- `src-tauri/src/core/central_repo.rs`: Central repository path resolution and creation.
- `src-tauri/src/core/git_fetcher.rs`: Git clone/pull and cache-aware fetch operations.
- `src-tauri/src/core/github_search.rs`: GitHub repository search.
- `src-tauri/src/core/github_download.rs`: GitHub contents download fallback.
- `src-tauri/src/core/skills_search.rs`: skills.sh-style online skill search.
- `src-tauri/src/core/featured_skills.rs`: Featured skills catalog fetch.
- `src-tauri/src/core/skill_files.rs`: Skill file listing and reading for detail view.
- `src-tauri/src/core/content_hash.rs`: Directory content hashing.
- `src-tauri/src/core/skill_lock.rs`: `~/.agents/.skill-lock.json` provenance enrichment.
- `src-tauri/src/core/cache_cleanup.rs`: Git cache cleanup settings and deletion.
- `src-tauri/src/core/temp_cleanup.rs`: Temporary clone directory cleanup.
- `src-tauri/src/core/cancel_token.rs`: Shared cancellation primitive.

**Frontend UI:**

- `src/components/skills/Header.tsx`: App tab/header UI.
- `src/components/skills/FilterBar.tsx`: Skills search/sort/view controls.
- `src/components/skills/SkillsList.tsx`: Managed skills list rendering.
- `src/components/skills/SkillCard.tsx`: Individual managed skill card and actions.
- `src/components/skills/ExplorePage.tsx`: Featured/search online skills UI.
- `src/components/skills/SkillDetailView.tsx`: Skill file tree/content detail view.
- `src/components/skills/SettingsPage.tsx`: Settings page UI and local setting handlers.
- `src/components/projects/ProjectsPage.tsx`: Projects feature container.
- `src/components/projects/useProjectState.ts`: Projects feature state and IPC calls.
- `src/components/projects/AssignmentMatrix.tsx`: Skill-to-tool assignment grid.
- `src/components/projects/ProjectList.tsx`: Registered projects sidebar/list.
- `src/components/projects/AddProjectModal.tsx`: Project registration modal.
- `src/components/projects/EditProjectModal.tsx`: Project configuration/edit modal.
- `src/components/projects/ToolConfigModal.tsx`: Project tool selection modal.
- `src/components/projects/RemoveProjectModal.tsx`: Project removal confirmation modal.

**IPC Contracts:**

- `src-tauri/src/commands/mod.rs`: Global command DTO structs and command functions.
- `src-tauri/src/commands/projects.rs`: Project command DTO structs and command functions.
- `src/components/skills/types.ts`: Frontend DTOs for skills, tools, onboarding, updates, featured/search results, and skill files.
- `src/components/projects/types.ts`: Frontend DTOs for projects, project tools, assignments, resync summaries, and bulk assignment results.

**Testing:**

- `src-tauri/src/core/tests/*.rs`: Core Rust tests.
- `src-tauri/src/commands/tests/commands.rs`: Command-layer Rust tests.
- No dedicated frontend test directory or frontend test runner config is detected in active root config.

## Naming Conventions

**Files:**

- React component files use PascalCase: `src/components/skills/SkillCard.tsx`, `src/components/projects/ProjectsPage.tsx`, `src/components/projects/AssignmentMatrix.tsx`.
- React hook files use camelCase with `use` prefix: `src/components/projects/useProjectState.ts`.
- Frontend DTO/type modules use lowercase `types.ts`: `src/components/skills/types.ts`, `src/components/projects/types.ts`.
- Frontend infrastructure files use lowercase names: `src/main.tsx`, `src/i18n/index.ts`, `src/i18n/resources.ts`.
- Rust module files use snake_case: `src-tauri/src/core/skill_store.rs`, `src-tauri/src/core/project_sync.rs`, `src-tauri/src/core/github_download.rs`.
- Rust module directories use `mod.rs` for module indexes: `src-tauri/src/commands/mod.rs`, `src-tauri/src/core/mod.rs`, `src-tauri/src/core/tool_adapters/mod.rs`.
- Rust tests mirror module names: `src-tauri/src/core/tests/project_sync.rs`, `src-tauri/src/core/tests/skill_store.rs`, `src-tauri/src/core/tests/tool_adapters.rs`.

**Directories:**

- Active frontend feature directories are plural and feature-oriented: `src/components/skills/`, `src/components/projects/`.
- Modal components are grouped under `modals/`: `src/components/skills/modals/`.
- Backend core code is grouped by domain under `src-tauri/src/core/`.
- Backend command code is grouped under `src-tauri/src/commands/`.
- GSD planning documents live under `.planning/`; codebase maps live under `.planning/codebase/`.

**Types and DTOs:**

- TypeScript DTOs use PascalCase with `Dto` suffix where mirrored from backend: `ProjectDto`, `ProjectToolDto`, `ProjectSkillAssignmentDto`, `ToolStatusDto`, `InstallResultDto`.
- Rust DTO structs use PascalCase and are colocated with command/core conversion code: `ToolInfoDto`, `ToolStatusDto`, `ProjectDto`, `ProjectSkillAssignmentDto`.
- Rust persistent records use `Record` suffix in `src-tauri/src/core/skill_store.rs`: `SkillRecord`, `SkillTargetRecord`, `ProjectRecord`, `ProjectToolRecord`, `ProjectSkillAssignmentRecord`.

## Where to Add New Code

**New global skills feature:**

- Primary frontend orchestration: `src/App.tsx`.
- Presentational UI: `src/components/skills/`.
- Modal UI: `src/components/skills/modals/`.
- Frontend DTOs: `src/components/skills/types.ts`.
- Backend command: `src-tauri/src/commands/mod.rs`.
- Backend business logic: a focused module under `src-tauri/src/core/`, exported from `src-tauri/src/core/mod.rs`.
- Command registration: `src-tauri/src/lib.rs` inside `tauri::generate_handler!`.
- Tests: `src-tauri/src/core/tests/<module>.rs` and, when command mapping matters, `src-tauri/src/commands/tests/commands.rs`.

**New per-project distribution feature:**

- Primary frontend orchestration/state: `src/components/projects/useProjectState.ts`.
- Page/handler glue: `src/components/projects/ProjectsPage.tsx`.
- Presentational UI: `src/components/projects/*.tsx`.
- Frontend DTOs: `src/components/projects/types.ts`.
- Backend command: `src-tauri/src/commands/projects.rs`.
- Backend business logic: `src-tauri/src/core/project_ops.rs` for project CRUD/cleanup or `src-tauri/src/core/project_sync.rs` for assignment/sync behavior.
- Persistence changes: `src-tauri/src/core/skill_store.rs` with `SCHEMA_VERSION` migration.
- Tests: `src-tauri/src/core/tests/project_ops.rs`, `src-tauri/src/core/tests/project_sync.rs`, and `src-tauri/src/core/tests/skill_store.rs`.

**New Tauri command:**

- Implementation: `src-tauri/src/commands/mod.rs` for global app commands, or `src-tauri/src/commands/projects.rs` for project-specific commands.
- Business logic: `src-tauri/src/core/<domain>.rs`.
- Registration: add to `tauri::generate_handler!` in `src-tauri/src/lib.rs`.
- Frontend call site: `src/App.tsx` for global skills/settings workflows, or `src/components/projects/useProjectState.ts` / `src/components/projects/ProjectsPage.tsx` for projects workflows.
- DTO mirror: `src/components/skills/types.ts` or `src/components/projects/types.ts`.

**New AI tool adapter:**

- Implementation: `src-tauri/src/core/tool_adapters/mod.rs`.
- Add: `ToolId` enum variant, `ToolId::as_key()` mapping, `ToolAdapter` entry in `default_tool_adapters()`, and tests in `src-tauri/src/core/tests/tool_adapters.rs` if behavior changes.
- Use existing path helpers rather than adding tool paths to frontend files.

**New database table or column:**

- Schema and migrations: `src-tauri/src/core/skill_store.rs`.
- Add or update `SCHEMA_VERSION`, initial schema/migration SQL, record struct, CRUD methods, and tests in `src-tauri/src/core/tests/skill_store.rs`.
- Expose through commands only after a core API exists.

**New settings UI:**

- Frontend UI: `src/components/skills/SettingsPage.tsx`.
- Global state/loading handlers: `src/App.tsx` if the setting affects top-level app behavior.
- Backend persistence commands: `src-tauri/src/commands/mod.rs`.
- Storage: `settings` table helpers in `src-tauri/src/core/skill_store.rs`; UI-only preferences may use `localStorage` in `src/App.tsx`.

**New skill detail/file browsing behavior:**

- Frontend UI: `src/components/skills/SkillDetailView.tsx`.
- Backend file operations: `src-tauri/src/core/skill_files.rs`.
- Commands: `list_skill_files` and `read_skill_file` patterns in `src-tauri/src/commands/mod.rs`.

**Utilities:**

- Shared frontend helper tied to app orchestration: keep near use in `src/App.tsx` unless reusable across multiple components.
- Shared frontend project helper: `src/components/projects/useProjectState.ts` or a new file under `src/components/projects/`.
- Shared backend helper: a focused module under `src-tauri/src/core/`, exported from `src-tauri/src/core/mod.rs`.
- Build/release utility: `scripts/*.mjs`.

## Special Directories

**`src-tauri/src/core/tests/`:**

- Purpose: Backend tests for core modules.
- Generated: No.
- Committed: Yes.

**`.planning/codebase/`:**

- Purpose: GSD codebase maps consumed by planning and execution commands.
- Generated: Yes, by codebase mapping workflow.
- Committed: Yes, when planning artifacts are committed.

**`.claude/`:**

- Purpose: Claude-specific GSD commands, agents, hooks, settings, and local workflow support.
- Generated: Partly; contains distributed GSD artifacts and local configuration.
- Committed: Mixed; inspect individual files before committing local settings such as `.claude/settings.local.json`.

**`.codex/`:**

- Purpose: Codex-specific GSD commands, agents, hooks, and skills distribution.
- Generated: Partly; contains tool distribution artifacts.
- Committed: Yes for distribution artifacts.

**`.opencode/`:**

- Purpose: OpenCode-specific GSD commands and agents distribution.
- Generated: Partly; contains tool distribution artifacts.
- Committed: Yes for distribution artifacts.

**`.github/workflows/`:**

- Purpose: GitHub Actions CI/release automation.
- Generated: No.
- Committed: Yes.

**`node_modules/`:**

- Purpose: npm dependencies.
- Generated: Yes.
- Committed: No.

**`dist/`:**

- Purpose: Vite frontend build output.
- Generated: Yes.
- Committed: No.

**`src-tauri/target/`:**

- Purpose: Rust build output.
- Generated: Yes.
- Committed: No.

---

_Structure analysis: 2026-04-29_
