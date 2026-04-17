# Codebase Structure

**Analysis Date:** 2026-04-07

## Directory Layout

```text
skills-hub/
├── src/                     # React frontend application
│   ├── components/          # Presentational UI and modal components
│   ├── i18n/                # i18next initialization and translation resources
│   ├── pages/               # Secondary page experiments / unused route pages
│   ├── App.tsx              # Main application controller and state container
│   ├── App.css              # Shared component-level styling
│   ├── index.css            # Theme variables and Tailwind entry CSS
│   └── main.tsx             # Frontend bootstrap
├── src-tauri/               # Tauri desktop shell and Rust backend
│   ├── src/
│   │   ├── commands/        # Tauri IPC commands and command tests
│   │   ├── core/            # Business logic, persistence, sync, search, adapters
│   │   ├── lib.rs           # Tauri runtime setup and command registration
│   │   └── main.rs          # Native binary entry point
│   ├── capabilities/        # Tauri capability manifests
│   ├── Cargo.toml           # Rust dependencies and crate settings
│   └── tauri.conf.json      # Tauri app/build/bundle configuration
├── scripts/                 # Versioning and icon helper scripts
├── public/                  # Static assets copied by Vite
├── docs/                    # Project docs, plans, release design notes, logs
├── .planning/codebase/      # Generated architecture/quality/planning reference docs
├── package.json             # Frontend package manifest and npm scripts
├── vite.config.ts           # Vite dev/build configuration
├── tsconfig.json            # Root TypeScript project references
└── featured-skills.json     # Bundled featured-skill catalog fallback
```

## Directory Purposes

**`src/`:**

- Purpose: Hold the entire React frontend.
- Contains: `src/App.tsx`, global CSS, i18n setup, feature components under `src/components/skills/`, and dormant route-style files such as `src/components/Layout.tsx` and `src/pages/Dashboard.tsx`.
- Key files: `src/App.tsx`, `src/main.tsx`, `src/index.css`, `src/App.css`, `src/i18n/resources.ts`.

**`src/components/`:**

- Purpose: Hold reusable UI pieces.
- Contains: Active skill-management UI in `src/components/skills/` and an inactive alternate shell in `src/components/Layout.tsx`.
- Key files: `src/components/skills/Header.tsx`, `src/components/skills/SkillsList.tsx`, `src/components/skills/SkillCard.tsx`, `src/components/skills/ExplorePage.tsx`, `src/components/skills/SkillDetailView.tsx`, `src/components/skills/SettingsPage.tsx`.

**`src/components/skills/modals/`:**

- Purpose: Hold transient workflow dialogs.
- Contains: Add/import/delete/pick/sync confirmation modals.
- Key files: `src/components/skills/modals/AddSkillModal.tsx`, `src/components/skills/modals/ImportModal.tsx`, `src/components/skills/modals/GitPickModal.tsx`, `src/components/skills/modals/LocalPickModal.tsx`, `src/components/skills/modals/DeleteModal.tsx`, `src/components/skills/modals/NewToolsModal.tsx`, `src/components/skills/modals/SharedDirModal.tsx`.

**`src/i18n/`:**

- Purpose: Centralize localization setup.
- Contains: i18next initialization and translation dictionaries.
- Key files: `src/i18n/index.ts`, `src/i18n/resources.ts`.

**`src/pages/`:**

- Purpose: Contain route-oriented page components for an alternate layout approach.
- Contains: `src/pages/Dashboard.tsx`.
- Key files: `src/pages/Dashboard.tsx`.

**`src-tauri/src/commands/`:**

- Purpose: Define the frontend/backend command boundary.
- Contains: Tauri command functions, DTO structs, and command tests.
- Key files: `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/tests/commands.rs`.

**`src-tauri/src/core/`:**

- Purpose: Hold backend domain logic.
- Contains: Persistence, installation, onboarding, sync, search, adapter registry, cleanup helpers, and unit/integration-style tests under `src-tauri/src/core/tests/`.
- Key files: `src-tauri/src/core/skill_store.rs`, `src-tauri/src/core/installer.rs`, `src-tauri/src/core/onboarding.rs`, `src-tauri/src/core/sync_engine.rs`, `src-tauri/src/core/tool_adapters/mod.rs`, `src-tauri/src/core/skill_files.rs`, `src-tauri/src/core/mod.rs`.

**`src-tauri/src/core/tests/`:**

- Purpose: Keep backend module tests close to the corresponding domain code.
- Contains: One test file per major backend module.
- Key files: `src-tauri/src/core/tests/installer.rs`, `src-tauri/src/core/tests/skill_store.rs`, `src-tauri/src/core/tests/onboarding.rs`, `src-tauri/src/core/tests/sync_engine.rs`, `src-tauri/src/core/tests/tool_adapters.rs`.

**`scripts/`:**

- Purpose: Hold project-specific maintenance scripts.
- Contains: Version sync/check helpers and icon generation helpers referenced by `package.json` scripts.
- Key files: `scripts/version.mjs`, `scripts/tauri-icon-desktop.mjs`.

**`public/`:**

- Purpose: Hold static web assets served/copied by Vite.
- Contains: Public files such as logos and images used by the React UI.
- Key files: `public/` contents are served at the app root during frontend runtime.

**`docs/`:**

- Purpose: Hold long-form project documentation and planning artifacts.
- Contains: Release plans, system design notes, changelog translations, and conversation logs.
- Key files: `docs/releases/v0.1-v0.2/system-design.md`, `docs/README.zh.md`, `docs/plans/2026-04-02-skills-hub-fork-design.md`.

## Key File Locations

**Entry Points:**

- `src/main.tsx`: Frontend bootstrap; mounts `src/App.tsx`.
- `src/App.tsx`: Actual active application shell and state orchestrator.
- `src-tauri/src/main.rs`: Native executable entry that calls `app_lib::run()`.
- `src-tauri/src/lib.rs`: Tauri runtime setup, plugin registration, shared-state injection, and command registration.

**Configuration:**

- `package.json`: npm scripts, frontend dependencies, and Tauri CLI dependency.
- `vite.config.ts`: Vite plugins and development server port.
- `tsconfig.json`: TypeScript project references.
- `tsconfig.app.json`: Frontend TypeScript compiler options.
- `tsconfig.node.json`: Node/Vite TypeScript compiler options.
- `src-tauri/Cargo.toml`: Rust crate metadata and dependencies.
- `src-tauri/tauri.conf.json`: Desktop bundle metadata, frontend hooks, updater endpoint.
- `eslint.config.js`: ESLint flat config.

**Core Logic:**

- `src/components/skills/types.ts`: Shared frontend DTO definitions.
- `src-tauri/src/commands/mod.rs`: Tauri command contract.
- `src-tauri/src/core/skill_store.rs`: SQLite persistence and migrations.
- `src-tauri/src/core/installer.rs`: Local/git import and update workflows.
- `src-tauri/src/core/sync_engine.rs`: Filesystem sync strategy.
- `src-tauri/src/core/onboarding.rs`: Existing-skill discovery/import planning.
- `src-tauri/src/core/tool_adapters/mod.rs`: Supported tool registry and path resolution.
- `src-tauri/src/core/skill_files.rs`: Skill file listing and safe reading for the detail view.

**Testing:**

- `src-tauri/src/commands/tests/commands.rs`: Command-layer tests.
- `src-tauri/src/core/tests/*.rs`: Backend domain tests.
- Not detected: active frontend test directory or frontend test runner config.

## Naming Conventions

**Files:**

- Frontend React component files use PascalCase: `src/components/skills/SkillCard.tsx`, `src/components/skills/SettingsPage.tsx`.
- Frontend bootstrap and non-component files use lowercase or framework-default names: `src/main.tsx`, `src/index.css`, `src/App.tsx`.
- Rust module files use snake_case: `src-tauri/src/core/skill_store.rs`, `src-tauri/src/core/github_search.rs`.
- Generated planning docs use uppercase filenames: `.planning/codebase/ARCHITECTURE.md`, `.planning/codebase/STRUCTURE.md`.

**Directories:**

- Frontend feature directories are lowercase and nested by feature: `src/components/skills/`, `src/components/skills/modals/`.
- Rust backend directories are layer-oriented and lowercase: `src-tauri/src/commands/`, `src-tauri/src/core/`, `src-tauri/src/core/tests/`.
- Documentation directories are lowercase and topic-oriented: `docs/plans/`, `docs/releases/`, `docs/conversation-logs/`.

## Where to Add New Code

**New Feature:**

- Primary frontend orchestration: `src/App.tsx` when the feature needs global state, command sequencing, or cross-screen coordination.
- New presentational UI: `src/components/skills/` if the feature belongs to the active skills-management experience.
- Backend command surface: `src-tauri/src/commands/mod.rs`.
- Backend business logic: `src-tauri/src/core/` in a dedicated module, then export it from `src-tauri/src/core/mod.rs`.
- Tests: `src-tauri/src/core/tests/` for backend logic and `src-tauri/src/commands/tests/` for command behavior.

**New Component/Module:**

- Implementation: `src/components/skills/` for active feature UI, or `src/components/skills/modals/` for dialog-based flows.
- Shared props/types: `src/components/skills/types.ts` only when the type is a backend DTO or reused across multiple frontend components.

**Utilities:**

- Frontend helper logic that is local to the app shell: keep near `src/App.tsx` unless it becomes reusable enough to justify extraction.
- Backend shared helpers: add to `src-tauri/src/core/` and export in `src-tauri/src/core/mod.rs`.
- Tool-specific path/detection logic: extend `src-tauri/src/core/tool_adapters/mod.rs` instead of scattering tool rules elsewhere.

## Special Directories

**`.planning/codebase/`:**

- Purpose: Hold generated codebase analysis documents consumed by other GSD commands.
- Generated: Yes.
- Committed: Yes, when the orchestrator chooses to commit planning artifacts.

**`src-tauri/src/core/tests/`:**

- Purpose: Keep backend tests parallel to domain modules.
- Generated: No.
- Committed: Yes.

**`docs/releases/`:**

- Purpose: Store release-specific design notes and implementation plans.
- Generated: No.
- Committed: Yes.

**`public/`:**

- Purpose: Serve static assets directly through Vite/Tauri web assets.
- Generated: No.
- Committed: Yes.

**`dist/`:**

- Purpose: Vite/Tauri frontend build output referenced by `src-tauri/tauri.conf.json` as `frontendDist`.
- Generated: Yes.
- Committed: No by default; build artifact directory is not part of the current source listing.

**Dormant route-shell files:**

- Purpose: `src/components/Layout.tsx` and `src/pages/Dashboard.tsx` indicate a router-style shell that is not currently wired into `src/main.tsx` or `src/App.tsx`.
- Generated: No.
- Committed: Yes.

---

_Structure analysis: 2026-04-07_
