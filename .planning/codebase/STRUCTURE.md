# Codebase Structure

**Analysis Date:** 2026-04-16

## Directory Layout

```text
skills-hub/
├── .github/                 # CI, release, and automation workflows
├── .planning/               # GSD planning artifacts and generated codebase docs
├── docs/                    # Project documentation outside runtime code
├── public/                  # Static frontend assets served by Vite
├── scripts/                 # Versioning and build helper scripts
├── src/                     # React frontend application
│   ├── components/          # UI components and feature subtrees
│   │   ├── projects/        # Project distribution feature UI and hook
│   │   └── skills/          # Skills management UI and modal components
│   ├── i18n/                # Translation bootstrap and resource definitions
│   ├── pages/               # Dormant router-style pages, not active app entrypoints
│   ├── App.tsx              # Active frontend shell and orchestration root
│   ├── App.css              # Global component-level styles
│   ├── index.css            # Global theme variables and base styles
│   └── main.tsx             # React bootstrap entrypoint
├── src-tauri/               # Native Tauri app, Rust backend, packaging config
│   ├── src/
│   │   ├── commands/        # Tauri IPC command handlers and command tests
│   │   ├── core/            # Business logic, persistence, sync, integrations, tests
│   │   ├── lib.rs           # Tauri builder, state registration, command registration
│   │   └── main.rs          # Native binary entrypoint
│   ├── Cargo.toml           # Rust crate manifest
│   ├── Cargo.lock           # Rust lockfile
│   ├── build.rs             # Tauri build script
│   └── tauri.conf.json      # Tauri app/runtime/bundling configuration
├── dist/                    # Built frontend artifacts
├── package.json             # JS scripts and dependency manifest
├── package-lock.json        # npm lockfile
├── tsconfig.json            # TypeScript project references root
├── tsconfig.app.json        # Frontend TypeScript compiler config
├── tsconfig.node.json       # Node/Vite TypeScript compiler config
├── vite.config.ts           # Vite configuration
├── eslint.config.js         # ESLint flat config
└── featured-skills.json     # Packaged featured skills data snapshot
```

## Directory Purposes

**`src/`:**

- Purpose: Hold the active React desktop UI.
- Contains: `src/App.tsx`, shared CSS, i18n setup, feature component trees, and a small dormant `pages/` subtree.
- Key files: `src/App.tsx`, `src/main.tsx`, `src/index.css`, `src/App.css`, `src/i18n/index.ts`, `src/i18n/resources.ts`

**`src/components/skills/`:**

- Purpose: Hold the active skills-management presentation layer.
- Contains: Header, list/detail/explore/settings screens, loading UI, and modal components under `src/components/skills/modals/`.
- Key files: `src/components/skills/Header.tsx`, `src/components/skills/SkillsList.tsx`, `src/components/skills/SkillCard.tsx`, `src/components/skills/SkillDetailView.tsx`, `src/components/skills/SettingsPage.tsx`, `src/components/skills/types.ts`

**`src/components/projects/`:**

- Purpose: Hold the per-project distribution feature UI.
- Contains: Feature-local hook, project list, assignment matrix, CRUD/configuration modals, and DTO definitions.
- Key files: `src/components/projects/ProjectsPage.tsx`, `src/components/projects/useProjectState.ts`, `src/components/projects/AssignmentMatrix.tsx`, `src/components/projects/ProjectList.tsx`, `src/components/projects/types.ts`

**`src/components/`:**

- Purpose: Hold shared or feature-root components.
- Contains: The active feature folders plus the dormant router layout file.
- Key files: `src/components/Layout.tsx`, `src/components/projects/`, `src/components/skills/`

**`src/i18n/`:**

- Purpose: Centralize localization bootstrap and resource data.
- Contains: i18n initialization and translation dictionaries.
- Key files: `src/i18n/index.ts`, `src/i18n/resources.ts`

**`src/pages/`:**

- Purpose: Hold alternate router-oriented pages not wired into the active shell.
- Contains: Placeholder page components.
- Key files: `src/pages/Dashboard.tsx`

**`src-tauri/src/commands/`:**

- Purpose: Expose Tauri IPC commands.
- Contains: General commands in `src-tauri/src/commands/mod.rs`, project-specific commands in `src-tauri/src/commands/projects.rs`, and command tests under `src-tauri/src/commands/tests/`.
- Key files: `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/projects.rs`, `src-tauri/src/commands/tests/commands.rs`

**`src-tauri/src/core/`:**

- Purpose: Hold backend business logic modules.
- Contains: Installer logic, project sync logic, SQLite store, tool adapter registry, content hashing, onboarding, search/download helpers, cleanup helpers, and tests in `src-tauri/src/core/tests/`.
- Key files: `src-tauri/src/core/installer.rs`, `src-tauri/src/core/project_ops.rs`, `src-tauri/src/core/project_sync.rs`, `src-tauri/src/core/sync_engine.rs`, `src-tauri/src/core/skill_store.rs`, `src-tauri/src/core/tool_adapters/mod.rs`

**`src-tauri/src/core/tests/`:**

- Purpose: Hold Rust backend tests that mirror core modules.
- Contains: Test files such as `src-tauri/src/core/tests/installer.rs`, `src-tauri/src/core/tests/project_sync.rs`, and `src-tauri/src/core/tests/skill_store.rs`.
- Key files: `src-tauri/src/core/tests/installer.rs`, `src-tauri/src/core/tests/project_ops.rs`, `src-tauri/src/core/tests/project_sync.rs`, `src-tauri/src/core/tests/sync_engine.rs`, `src-tauri/src/core/tests/tool_adapters.rs`

**`src-tauri/`:**

- Purpose: Package the desktop backend and app metadata.
- Contains: Rust crate files, Tauri config, icons/resources, and build settings.
- Key files: `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`

**`scripts/`:**

- Purpose: Hold JS build/release helper scripts.
- Contains: Version and packaging helpers referenced from `package.json`.
- Key files: `scripts/version.mjs`

**`.planning/codebase/`:**

- Purpose: Hold generated codebase mapping documents consumed by GSD commands.
- Contains: This architecture/structure documentation set.
- Key files: `.planning/codebase/ARCHITECTURE.md`, `.planning/codebase/STRUCTURE.md`

## Key File Locations

**Entry Points:**

- `src/main.tsx`: React bootstrap entry.
- `src/App.tsx`: Active frontend shell and view switcher.
- `src-tauri/src/main.rs`: Native Rust binary entrypoint.
- `src-tauri/src/lib.rs`: Tauri app builder and command registration root.

**Configuration:**

- `package.json`: JS scripts and frontend dependency manifest.
- `vite.config.ts`: Vite build/dev configuration.
- `eslint.config.js`: Frontend lint rules.
- `tsconfig.json`: TypeScript project references root.
- `tsconfig.app.json`: Frontend TypeScript settings.
- `tsconfig.node.json`: Node/Vite TypeScript settings.
- `src-tauri/Cargo.toml`: Rust dependency and crate configuration.
- `src-tauri/tauri.conf.json`: Tauri runtime, bundle, and updater configuration.

**Core Logic:**

- `src-tauri/src/core/skill_store.rs`: SQLite schema, migrations, and CRUD.
- `src-tauri/src/core/installer.rs`: Skill import/update workflows.
- `src-tauri/src/core/project_ops.rs`: Project registration, DTO shaping, and cleanup helpers.
- `src-tauri/src/core/project_sync.rs`: Project assignment sync, resync, staleness, and cleanup.
- `src-tauri/src/core/sync_engine.rs`: Symlink/junction/copy primitives.
- `src-tauri/src/core/tool_adapters/mod.rs`: Tool registry and path conventions.

**Testing:**

- `src-tauri/src/core/tests/`: Backend/core tests.
- `src-tauri/src/commands/tests/`: Tauri command tests.
- `package.json`: Test command entry via `npm run rust:test`.

## Naming Conventions

**Files:**

- React component files use PascalCase: `src/components/skills/SkillCard.tsx`, `src/components/projects/ProjectsPage.tsx`.
- React feature hooks use camelCase `use*` filenames: `src/components/projects/useProjectState.ts`.
- Frontend support/setup files use lowercase names: `src/main.tsx`, `src/i18n/index.ts`, `src/i18n/resources.ts`.
- Rust backend modules use snake_case: `src-tauri/src/core/project_sync.rs`, `src-tauri/src/core/skill_store.rs`, `src-tauri/src/commands/projects.rs`.
- Rust module index files use `mod.rs`: `src-tauri/src/commands/mod.rs`, `src-tauri/src/core/mod.rs`, `src-tauri/src/core/tool_adapters/mod.rs`.
- Rust tests mirror the covered module name under `src-tauri/src/core/tests/`: `src-tauri/src/core/tests/project_sync.rs`, `src-tauri/src/core/tests/skill_store.rs`.

**Directories:**

- Frontend feature directories are noun-based and lowercase: `src/components/skills/`, `src/components/projects/`.
- Backend responsibilities are split by technical layer under lowercase directories: `src-tauri/src/commands/`, `src-tauri/src/core/`.
- Test directories sit under the backend layer they validate: `src-tauri/src/core/tests/`, `src-tauri/src/commands/tests/`.

## Where to Add New Code

**New Feature:**

- Primary frontend orchestration: `src/App.tsx` if the feature changes top-level navigation, global app state, or shared modal/workflow wiring.
- Primary frontend feature UI: create or extend a focused subtree under `src/components/skills/` or `src/components/projects/`, depending on whether the feature belongs to global skill management or project distribution.
- Primary backend logic: add a focused module under `src-tauri/src/core/`, then expose it through `src-tauri/src/commands/mod.rs` or `src-tauri/src/commands/projects.rs`.
- Tests: add Rust tests under `src-tauri/src/core/tests/` or `src-tauri/src/commands/tests/` using the mirrored module name.

**New Component/Module:**

- Skills UI component: `src/components/skills/`.
- Skills modal: `src/components/skills/modals/`.
- Projects UI component: `src/components/projects/`.
- Shared frontend DTO additions: update `src/components/skills/types.ts` or `src/components/projects/types.ts`, then mirror the backend DTO in `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/projects.rs`, or `src-tauri/src/core/project_ops.rs`.
- New backend core module: `src-tauri/src/core/`, and export it from `src-tauri/src/core/mod.rs`.
- New Tauri command: add the function to `src-tauri/src/commands/mod.rs` or `src-tauri/src/commands/projects.rs`, then register it in `src-tauri/src/lib.rs` inside `generate_handler!`.

**Utilities:**

- Frontend feature-local helpers: keep them in the owning component file unless reused broadly, then move them into the same feature directory such as `src/components/projects/` or `src/components/skills/`.
- Backend reusable helpers: place them under `src-tauri/src/core/` near the owning domain, such as `src-tauri/src/core/content_hash.rs` or `src-tauri/src/core/skill_files.rs`.
- Build/release utilities: `scripts/` for Node-based automation.

## Special Directories

**`src/components/projects/`:**

- Purpose: Isolate the newer project-distribution feature from the legacy `src/App.tsx` state mass.
- Generated: No.
- Committed: Yes.

**`src/pages/`:**

- Purpose: Hold unused router-style page components.
- Generated: No.
- Committed: Yes.

**`src/components/Layout.tsx`:**

- Purpose: Hold an unused router shell built around `react-router-dom`.
- Generated: No.
- Committed: Yes.

**`src-tauri/src/core/tests/`:**

- Purpose: Keep backend tests close to the business logic they validate.
- Generated: No.
- Committed: Yes.

**`dist/`:**

- Purpose: Store built frontend output.
- Generated: Yes.
- Committed: Yes, currently present in the repository root.

**`.planning/`:**

- Purpose: Store project planning, debug notes, phase artifacts, and generated codebase maps.
- Generated: Mixed; many files are workflow-generated.
- Committed: Yes.

---

_Structure analysis: 2026-04-16_
