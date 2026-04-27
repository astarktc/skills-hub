# Skills Hub - Project Rules

## Overview

Skills Hub is a cross-platform desktop app (Tauri 2 + React 19) for managing AI Agent Skills and syncing them to 47+ AI coding tools. Core concept: "Install once, sync everywhere."

## Tech Stack - read .planning/codebase/STACK.md

## Common Commands

```bash
npm run dev              # Vite dev server (port 5173)
npm run tauri:dev        # Tauri dev window (frontend + backend)
npm run build            # tsc + vite build
npm run check            # Full check: lint + build + rust:fmt:check + rust:clippy + rust:test
npm run lint             # ESLint (flat config v9)
npm run rust:test        # cargo test
npm run rust:clippy      # Rust lint
npm run rust:fmt         # Rust format
npm run rust:fmt:check   # Rust format check
```

Always run `npm run check` before committing to ensure all checks pass.

## Codebase and Directory Structure - read .planning/codebase/STRUCTURE.md

## Architecture - read .planning/codebase/ARCHITECTURE.md

### Frontend ↔ Backend Communication

- Uses Tauri IPC (`invoke`) to call backend commands
- Frontend call pattern: `const result = await invoke('command_name', { param })`
- Backend commands are defined in `commands/mod.rs` and registered in `lib.rs` via `generate_handler!`
- New commands must be registered in both places

### Frontend State Management

- **No state management library** — all state is centralized in `App.tsx` via `useState`
- Passed to child components via props drilling (modals receive many props)
- Data refresh pattern: call `invoke('get_managed_skills')` after operations to re-fetch the list

### Backend Layering

- `commands/` layer: Tauri command definitions, DTO conversions, error formatting (no business logic)
- `core/` layer: Pure business logic, independently testable
- Async commands use `tauri::async_runtime::spawn_blocking` to wrap synchronous operations
- Shared state injected via `app.manage(store)` + `State<'_, SkillStore>`

### Error Handling

- Backend uses `anyhow::Result<T>`, converted to string via `format_anyhow_error()` for the frontend
- Special error prefixes for frontend identification: `MULTI_SKILLS|`, `TARGET_EXISTS|`, `TOOL_NOT_INSTALLED|`
- Frontend catches with try-catch and displays errors via sonner toast

## External Integrations - read .planning/codebase/INTEGRATIONS.md

## Coding Conventions - read .planning/codebase/CONVENTIONS.md

## Development Workflow

1. **Before implementing**: Briefly describe the approach and list the files to be modified. Wait for confirmation before writing code.
2. **Implement completely**: For features involving both frontend and backend, modify both sides in one pass — including Tauri command registration, DTO types, i18n translations (both EN and ZH), and UI.
3. **Verify after changes**: Always run `npm run check` after implementation to ensure lint, build, and all Rust checks pass. Fix any errors before presenting the result.
4. **Keep changes minimal**: Only modify what is necessary for the requirement. Do not refactor, add comments, or "improve" unrelated code.

## Worktree Safety

Parallel worktrees that branch from a stale base can silently revert changes merged to main after the branch point. This caused a major incident in v1.1.4 where 14 commits were lost.

**Before merging a worktree branch back to main:**

1. Rebase the worktree branch onto current main first: `git rebase main` from the worktree branch
2. After rebase, review the full diff against main: `git diff main...HEAD` — scan for unexpected deletions in files the worktree didn't intentionally modify
3. Pay special attention to shared files modified by multiple worktrees: `tool_adapters/mod.rs`, `installer.rs`, `App.tsx`, `commands/mod.rs`, `project_sync.rs`

**After merging a worktree branch:**

1. Run `git diff <pre-merge-commit>..HEAD -- src-tauri/src src/` and verify no unintended reverts
2. Run `npm run check` immediately — a passing build does not guarantee feature completeness, but a failing build catches structural damage early
3. Spot-check key functions that other worktrees recently added (grep for function names from recent commits)

**When resolving merge conflicts in worktree merges:**

- For files the worktree did NOT intentionally modify, prefer the main branch version
- Never accept the worktree version wholesale for a file with conflicts unless you verify every hunk

## Testing - read .planning/codebase/TESTING.md

## Important Notes

- Path handling must support `~` expansion (backend has `expand_home_path()`)
- Sync strategy uses triple fallback: symlink → junction (Windows) → copy
- Git uses vendored-openssl, HTTP uses rustls-tls — avoids system SSL issues
- Version numbers must stay in sync between `package.json` and `src-tauri/tauri.conf.json` (validate with `npm run version:check`)
- Rust crate is named `app_lib` (not the default package name) — use `app_lib::...` for imports
- Database has a schema migration mechanism (`migrate_legacy_db_if_needed`) — consider migrations when modifying table structures
- Tool adapter list is in `tool_adapters/mod.rs` — adding a new AI tool requires both a `ToolId` enum variant and an adapter instance

<!-- GSD:project-start source:PROJECT.md -->

## Project

**Skills Hub — Per-Project Skill Distribution**

Skills Hub is a cross-platform desktop app (Tauri 2 + React 19) for managing AI Agent Skills and syncing them to 47+ AI coding tools. This milestone adds per-project skill distribution: register project directories, assign specific skills to specific projects, and sync via symlinks from `~/.skillshub/<skill>` to `<project>/.claude/skills/<skill>` (or equivalent tool path).

**Core Value:** Any skill assigned to a project is immediately available in that project's tool directory via symlink, so AI tools only load the skills that matter for that project — not the entire library.

### Constraints

- **Tech stack**: Tauri 2 + React 19 + Rust + SQLite — no new frameworks
- **Sync engine**: Reuse existing `sync_engine.rs` primitives — do not duplicate or modify
- **App.tsx**: Minimize changes — new feature state stays in Projects component tree
- **i18n**: English strings only — Chinese deferred
- **Platform**: Must work on WSL2 (primary dev environment), macOS, and Linux
- **Backward compat**: Existing global sync must continue working alongside project sync
<!-- GSD:project-end -->

<!-- GSD:stack-start source:codebase/STACK.md -->

## Technology Stack

## High-Level Summary

- **Frontend**: React 19 + TypeScript 5.9 (strict) + Vite 7 + Tailwind CSS 4
- **Backend**: Rust (Edition 2021, MSRV 1.77.2) + Tauri 2
- **Database**: SQLite (rusqlite, bundled)
- **Git**: libgit2 (git2 crate, vendored-openssl)
- **HTTP**: reqwest (rustls-tls, blocking)
- **i18n**: i18next (English / Chinese bilingual)
- **Notifications**: sonner (toast)
- **Icons**: lucide-react

## Languages

- TypeScript 5.9.x - React desktop UI in `src/**/*.ts` and `src/**/*.tsx`, configured by `package.json`, `tsconfig.json`, `tsconfig.app.json`, and `vite.config.ts`
- Rust 2021 edition (MSRV 1.77.2) - Tauri backend, native app shell, filesystem/database/network logic in `src-tauri/src/**/*.rs`, configured by `src-tauri/Cargo.toml`
- CSS - Global styling and theme variables in `src/App.css` and `src/index.css`
- JSON - App/package configuration in `package.json`, `src-tauri/tauri.conf.json`, `featured-skills.json`, and GitHub workflow files under `.github/workflows/`
- JavaScript (Node.js scripts) - Release/version automation in `scripts/*.mjs` referenced by `package.json` and `.github/workflows/*.yml`
- YAML - CI/CD automation in `.github/workflows/ci.yml`, `.github/workflows/release.yml`, and `.github/workflows/update-featured-skills.yml`

## Runtime

- Node.js 20 in CI and release workflows, declared in `.github/workflows/ci.yml` and `.github/workflows/release.yml`
- Node.js 18+ for local development, with 20+ recommended in `README.md`
- Rust stable toolchain with minimum supported version 1.77.2 in `src-tauri/Cargo.toml`
- Tauri 2 desktop runtime, combining the web UI and native Rust backend in `src-tauri/src/lib.rs` and `src-tauri/tauri.conf.json`
- npm - scripts and install flow are defined in `package.json` and `README.md`
- Lockfile: present via `package-lock.json`
- Cargo - Rust dependency manager for `src-tauri/Cargo.toml`
- Lockfile: present via `src-tauri/Cargo.lock`

## Frameworks

- React 19.2.x - UI framework for the desktop frontend in `src/App.tsx`, `src/components/**/*.tsx`, and `src/pages/**/*.tsx`
- Tauri 2.9.x - Desktop application framework and IPC bridge in `src-tauri/src/lib.rs`, `src-tauri/src/commands/mod.rs`, and `src-tauri/tauri.conf.json`
- Vite 7.3.x - Frontend dev server and production bundler in `vite.config.ts` and `package.json`
- React Router DOM 7.12.x - Client-side routing for page navigation, imported from frontend files such as `src/pages/*.tsx`
- i18next 25.x + react-i18next 16.x - bilingual UI localization in `src/i18n/index.ts`, `src/i18n/resources.ts`, and frontend components importing `TFunction` or `useTranslation`
- Tailwind CSS 4.1.x - utility styling via Vite plugin in `vite.config.ts` and dependency declarations in `package.json`
- Rust built-in test harness (`cargo test`) - backend/unit integration tests under `src-tauri/src/core/tests/` and `src-tauri/src/commands/tests/`
- Mockito 1.x - HTTP mocking for Rust tests in `src-tauri/src/core/tests/github_search.rs`, `src-tauri/src/core/tests/skills_search.rs`, and `src-tauri/Cargo.toml`
- Tempfile 3.x - temporary filesystem fixtures for Rust tests in `src-tauri/src/core/tests/*.rs` and `src-tauri/Cargo.toml`
- No dedicated frontend test runner is detected in `package.json` or root config files
- `@vitejs/plugin-react` 5.1.x - React integration for Vite in `vite.config.ts`
- `@tailwindcss/vite` 4.1.x - Tailwind Vite integration in `vite.config.ts`
- ESLint 9.39.x with flat config - frontend linting in `eslint.config.js`
- TypeScript project references - split app/node TS configs via `tsconfig.json`, `tsconfig.app.json`, and `tsconfig.node.json`
- `tauri-build` 2.5.3 - Rust-side build integration in `src-tauri/Cargo.toml` and `src-tauri/build.rs`

## Key Dependencies

- `@tauri-apps/api` 2.9.1 - frontend access to Tauri APIs from files such as `src/App.tsx` and `src/components/skills/SettingsPage.tsx`
- `@tauri-apps/plugin-dialog` 2.5.3 - native directory/file pickers used from frontend flows and registered in `src-tauri/src/lib.rs`
- `@tauri-apps/plugin-opener` 2.5.3 - shell/open integration registered in `src-tauri/src/lib.rs`
- `@tauri-apps/plugin-updater` 2.5.3 - auto-update checks and installation used in `src/App.tsx`, `src/components/skills/SettingsPage.tsx`, `src-tauri/src/lib.rs`, and `src-tauri/tauri.conf.json`
- `react-markdown` 10.1.0 + `remark-frontmatter` 5.0.0 + `remark-gfm` 4.0.1 - Markdown skill file rendering in frontend detail views under `src/components/skills/`
- `react-syntax-highlighter` 16.1.1 - code block rendering in skill detail UI under `src/components/skills/`
- `sonner` 2.0.7 - toast notifications used in `src/App.tsx` and `src/components/skills/SkillCard.tsx`
- `lucide-react` 0.562.0 - icon system used broadly across `src/components/**/*.tsx`
- `rusqlite` 0.31 with `bundled` feature - embedded SQLite storage in `src-tauri/src/core/skill_store.rs`
- `reqwest` 0.12 with `blocking`, `json`, and `rustls-tls` - outbound HTTP for GitHub, skills.sh, and featured skills fetches in `src-tauri/src/core/github_search.rs`, `src-tauri/src/core/github_download.rs`, `src-tauri/src/core/skills_search.rs`, and `src-tauri/src/core/featured_skills.rs`
- `git2` 0.19 with `vendored-openssl` - Git clone/pull support in `src-tauri/src/core/git_fetcher.rs` and installer tests
- `uuid` 1.x with v4 - identifier generation in `src-tauri/src/commands/mod.rs` and `src-tauri/src/core/installer.rs`
- `walkdir` 2.5 - recursive filesystem traversal in `src-tauri/src/core/content_hash.rs` and `src-tauri/src/core/skill_files.rs`
- `sha2` 0.10 + `hex` 0.4 - directory content hashing in `src-tauri/src/core/content_hash.rs`
- `dirs` 5.0 - home/app-data path resolution in `src-tauri/src/core/central_repo.rs`, `src-tauri/src/core/tool_adapters/mod.rs`, and `src-tauri/src/commands/mod.rs`
- `junction` 1.1 - Windows junction fallback used by sync logic in `src-tauri/src/core/sync_engine.rs`
- `clsx` 2.1.1 + `tailwind-merge` 3.4.0 - class composition utilities available to the frontend from `package.json`

## Configuration

- Runtime configuration is primarily app-internal, persisted in SQLite settings via `src-tauri/src/core/skill_store.rs` rather than loaded from dotenv files
- `.env.example` exists at `/home/alexwsl/skills-hub/.env.example`, but no dotenv loader is detected in `package.json`, `vite.config.ts`, or `src-tauri/src/**/*.rs`
- GitHub API authentication is optional and stored through `get_github_token` / `set_github_token` in `src-tauri/src/commands/mod.rs`, with the token consumed by `src-tauri/src/core/github_search.rs`, `src-tauri/src/core/github_download.rs`, and `src-tauri/src/core/installer.rs`
- Tauri updater endpoints and signing metadata are configured in `src-tauri/tauri.conf.json`
- Version synchronization between frontend and Tauri config is enforced by `scripts/version.mjs` and the `version:*` scripts in `package.json`
- Frontend build: `vite.config.ts`, `tsconfig.json`, `tsconfig.app.json`, `tsconfig.node.json`
- Linting: `eslint.config.js`
- Tauri/native build: `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`
- CI verification: `.github/workflows/ci.yml`
- Release packaging and updater artifact assembly: `.github/workflows/release.yml`

## Platform Requirements

- Node.js 18+ with npm, documented in `README.md`
- Rust stable toolchain, with MSRV 1.77.2 from `src-tauri/Cargo.toml`
- Tauri OS dependencies, documented in `README.md`; Linux CI installs GTK/WebKit-related packages in `.github/workflows/ci.yml`
- Desktop environment capable of running Tauri apps; the app is not configured as a web-only deployment target
- Packaged desktop binaries produced by Tauri for macOS, Windows, and Linux via scripts in `package.json` and pipelines in `.github/workflows/release.yml`
- Auto-update artifacts published through GitHub Releases, configured by `src-tauri/tauri.conf.json` and `.github/workflows/release.yml`
- Local persistent storage uses SQLite database file resolution from `src-tauri/src/core/skill_store.rs` and central repository filesystem storage from `src-tauri/src/core/central_repo.rs`
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->

## Conventions

## High-Level Summary

### TypeScript

- Strict mode: `noUnusedLocals` and `noUnusedParameters` are enabled — unused variables/params cause compile errors
- Component files: PascalCase (`SkillCard.tsx`)
- Props types: `ComponentNameProps` (`SkillCardProps`)
- CSS class names: kebab-case (`modal-backdrop`, `skill-card`)
- Modal conditional rendering: `if (!open) return null` (full unmount, not display:none)
- Wrap presentational components with `memo()`
- All user-visible text must use i18n (`t('key')`), translation keys defined in `src/i18n/resources.ts`
- When adding new text, always provide both English and Chinese translations
- DTO types are defined in `src/components/skills/types.ts` and must stay in sync with the Rust DTOs in `commands/mod.rs`

### Rust

- Functions/methods: snake_case
- Constants: SCREAMING_SNAKE_CASE
- Tauri command parameters use camelCase (to match frontend JS calling convention)
- Use `anyhow::Context` to add context to errors
- New core modules must be exported in `core/mod.rs`
- Tests use `tempfile` crate for temp directories and `mockito` for HTTP mocking

### Styling

- Component styles go in `src/App.css` (not CSS Modules), using semantic CSS class names
- Theming via CSS variables + `[data-theme="dark"]` selector, variables defined in `src/index.css`
- Tailwind utility classes and custom CSS classes can be mixed

## Naming Patterns

- React component files use PascalCase filenames in `src/components/skills/` such as `src/components/skills/SkillCard.tsx`, `src/components/skills/SettingsPage.tsx`, and `src/components/skills/modals/AddSkillModal.tsx`.
- Frontend non-component support files use lowercase names when they represent infrastructure or setup, such as `src/main.tsx`, `src/i18n/index.ts`, and `src/i18n/resources.ts`.
- Rust production modules use snake_case filenames such as `src-tauri/src/core/skill_store.rs`, `src-tauri/src/core/github_search.rs`, and `src-tauri/src/commands/mod.rs`.
- Rust test files mirror the module name they cover under `src-tauri/src/core/tests/`, such as `src-tauri/src/core/tests/installer.rs` and `src-tauri/src/core/tests/skill_store.rs`.
- TypeScript component functions use PascalCase when they define components, for example `SkillCard` in `src/components/skills/SkillCard.tsx` and `AddSkillModal` in `src/components/skills/modals/AddSkillModal.tsx`.
- TypeScript helper functions use camelCase, for example `formatRelative`, `getSkillSourceLabel`, and `getGithubInfo` in `src/App.tsx`, plus `formatCount` in `src/components/skills/ExplorePage.tsx`.
- Rust functions and methods use snake_case throughout, for example `format_anyhow_error` in `src-tauri/src/commands/mod.rs`, `ensure_schema` in `src-tauri/src/core/skill_store.rs`, and `hash_dir` tested from `src-tauri/src/core/tests/content_hash.rs`.
- Frontend local variables and state use camelCase, including state setters from `useState` in `src/App.tsx` like `managedSkills`, `searchResults`, `showAddModal`, and `updateAvailableVersion`.
- Boolean state names prefer `is*`, `show*`, `can*`, or `has*`, as seen in `src/App.tsx`, `src/components/skills/ExplorePage.tsx`, and `src/components/skills/SettingsPage.tsx`.
- Rust locals use snake_case, such as `user_version`, `db_path`, and `newly_installed` in `src-tauri/src/core/skill_store.rs` and `src-tauri/src/commands/mod.rs`.
- Type aliases and prop types use PascalCase with descriptive suffixes, such as `SkillCardProps` in `src/components/skills/SkillCard.tsx`, `SettingsPageProps` in `src/components/skills/SettingsPage.tsx`, and DTO aliases in `src/components/skills/types.ts`.
- Rust structs use PascalCase and DTO suffixes for IPC types, for example `ToolInfoDto`, `ToolStatusDto`, and `InstallResultDto` in `src-tauri/src/commands/mod.rs`.
- Shared frontend DTOs are centralized in `src/components/skills/types.ts` and mirror backend command DTOs from `src-tauri/src/commands/mod.rs`; keep names and fields aligned across both files.

## Code Style

- Frontend code uses a Prettier-like style with single quotes, no semicolons, and trailing commas in multiline structures, as shown in `src/App.tsx`, `src/components/skills/SkillCard.tsx`, and `src/i18n/index.ts`.
- React code prefers destructured props with a typed props object followed by an arrow function definition, for example in `src/components/skills/modals/AddSkillModal.tsx` and `src/components/skills/SettingsPage.tsx`.
- JSX keeps one prop per line once elements become non-trivial, especially in modal and list components under `src/components/skills/`.
- CSS lives in global stylesheets, primarily `src/App.css` and `src/index.css`, and uses semantic kebab-case class names such as `.skill-card`, `.modal-backdrop`, and `.explore-card`.
- Rust is formatted with `cargo fmt`, enforced by the `rust:fmt:check` script in `package.json`.
- ESLint flat config is defined in `eslint.config.js` and applies to `**/*.{ts,tsx}`.
- The frontend extends `@eslint/js`, `typescript-eslint`, `eslint-plugin-react-hooks`, and `eslint-plugin-react-refresh` via `eslint.config.js`.
- TypeScript strictness is enforced in `tsconfig.app.json` and `tsconfig.node.json` with `strict`, `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`, and `noUncheckedSideEffectImports` enabled.
- Use TypeScript’s strict null handling and explicit fallback expressions such as `?? ''`, `?? null`, and early returns, as seen in `src/App.tsx` and `src/components/skills/SkillDetailView.tsx`.

## Import Organization

- Not detected. Frontend imports use relative paths such as `./components/skills/ExplorePage` in `src/App.tsx` and `../types` in `src/components/skills/modals/AddSkillModal.tsx`.
- Rust modules use `crate::core::...` absolute crate paths in backend code such as `src-tauri/src/commands/mod.rs`.

## Error Handling

- Frontend async actions generally use `try/catch` with `err instanceof Error ? err.message : String(err)` normalization, as seen in `src/App.tsx` and `src/components/skills/SettingsPage.tsx`.
- Frontend converts backend wire-format errors into user-facing translation keys in `formatErrorMessage` inside `src/App.tsx`; preserve this pattern when adding new backend error prefixes.
- Toast notifications via `sonner` are the standard user-facing error and success channel in components like `src/App.tsx`, `src/components/skills/SkillCard.tsx`, and `src/components/skills/SkillDetailView.tsx`.
- Backend command handlers return `Result<T, String>` at the Tauri boundary and map internal `anyhow::Error` values through `format_anyhow_error` in `src-tauri/src/commands/mod.rs`.
- Backend business logic adds context with `anyhow::Context`, for example in `expand_home_path` and `ensure_schema` code paths in `src-tauri/src/commands/mod.rs` and `src-tauri/src/core/skill_store.rs`.
- Long-running synchronous backend work is wrapped in `tauri::async_runtime::spawn_blocking` in `src-tauri/src/commands/mod.rs`; use this when exposing blocking filesystem, git, or SQLite work to the frontend.

## Logging

- Frontend uses `sonner` toasts for user-visible operational feedback rather than console logging, with examples in `src/App.tsx` and `src/components/skills/SkillCard.tsx`.
- Backend uses the `log` crate with `tauri-plugin-log` initialization in `src-tauri/src/lib.rs`.
- Backend setup and cleanup emit `log::info!` for best-effort maintenance events in `src-tauri/src/lib.rs`.
- Direct `console.log` usage is not detected in the frontend source under `src/`; follow the existing pattern and surface UI feedback through state or toast instead.
- Silent failures are occasionally intentional for non-critical browser or storage operations, such as `catch {}` in `src/components/skills/SkillCard.tsx`, `src/components/skills/SettingsPage.tsx`, and `src/i18n/index.ts`.

## Comments

- Comments are sparse and used to explain non-obvious logic boundaries, not routine code. Examples include section comments in `src/components/skills/SkillDetailView.tsx` and lifecycle/setup comments in `src-tauri/src/lib.rs`.
- Use comments for behavior constraints, safety assumptions, and algorithm notes, such as the cleanup safety bullets in `src-tauri/src/lib.rs` and migration notes in `src-tauri/src/core/skill_store.rs`.
- Avoid redundant comments on self-explanatory JSX or straightforward assignments; most component files omit them.
- Not generally used in TypeScript component files under `src/components/skills/`.
- Rust doc comments are minimal but present for selected internals, for example `CancelToken` in `src-tauri/src/core/cancel_token.rs`.

## Function Design

- Presentational components are kept moderate and focused, often one component per file, such as `src/components/skills/SkillCard.tsx`, `src/components/skills/ExplorePage.tsx`, and modal files under `src/components/skills/modals/`.
- `src/App.tsx` is the notable orchestration exception: it centralizes application state, view switching, and command orchestration. New global stateful workflows currently belong there unless the surrounding architecture is deliberately changed.
- Rust command and core modules favor many small functions over monolithic logic, with helper functions and DTO mappers split inside `src-tauri/src/commands/mod.rs` and `src-tauri/src/core/*.rs`.
- React components receive explicit prop objects with typed callbacks and data dependencies, as seen in `src/components/skills/modals/AddSkillModal.tsx` and `src/components/skills/SettingsPage.tsx`.
- Event handlers usually pass primitive values upward rather than DOM events, for example `onLocalPathChange(event.target.value)` in `src/components/skills/modals/AddSkillModal.tsx`.
- Backend command parameters use camelCase for frontend compatibility, while internal Rust functions stay snake_case in `src-tauri/src/commands/mod.rs`.
- Frontend components use early null returns for conditional mounting, for example `if (!open) return null` in `src/components/skills/modals/DeleteModal.tsx`, `src/components/skills/modals/AddSkillModal.tsx`, and `src/components/skills/LoadingOverlay.tsx`.
- Frontend helpers often return normalized primitives or nullable objects, such as `getGithubInfo` in `src/App.tsx` and `isInstalled` in `src/components/skills/ExplorePage.tsx`.
- Backend functions return `Result<T>` internally and serialize DTO structs at the command boundary in `src-tauri/src/commands/mod.rs`.

## Module Design

- Frontend component files usually define a single component and default-export `memo(Component)` for presentational modules, as shown in `src/components/skills/SkillCard.tsx`, `src/components/skills/ExplorePage.tsx`, `src/components/skills/SettingsPage.tsx`, and modal components under `src/components/skills/modals/`.
- Frontend shared DTOs use named `export type` declarations from `src/components/skills/types.ts`.
- Backend command aggregation uses a single module file `src-tauri/src/commands/mod.rs` with public command functions and DTO structs.
- Backend core modules expose focused APIs through per-file modules under `src-tauri/src/core/` and are re-exported via `src-tauri/src/core/mod.rs`.
- Not used on the frontend; components are imported directly from their file paths in `src/App.tsx`.
- Rust module indexing is handled through `mod.rs` files such as `src-tauri/src/core/mod.rs` and `src-tauri/src/commands/mod.rs`.

## Prescriptive Patterns to Follow

- Put new user-visible strings in `src/i18n/resources.ts` and consume them through `t('key')`; do not inline English text in JSX.
- Keep new frontend DTO fields synchronized between `src/components/skills/types.ts` and `src-tauri/src/commands/mod.rs`.
- Wrap new presentational React components in `memo()` when they primarily render props, matching `src/components/skills/SkillCard.tsx` and `src/components/skills/ExplorePage.tsx`.
- Use early-return modal rendering (`if (!open) return null`) for overlay components, matching files in `src/components/skills/modals/`.
- Put global app-level orchestration, shared fetch/reload flows, and Tauri command wiring in `src/App.tsx` unless the architecture is intentionally reworked.
- For backend commands, keep business logic in `src-tauri/src/core/` and make `src-tauri/src/commands/mod.rs` responsible for Tauri command wrappers, DTOs, and error formatting.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->

## Architecture

## Pattern Overview

- Use `src/App.tsx` as the single frontend orchestration layer for view state, modal state, tool status, onboarding state, settings state, and async action flows.
- Use Tauri IPC as the only frontend/backend boundary: React calls `invoke` through `invokeTauri()` in `src/App.tsx`, backend commands live in `src-tauri/src/commands/mod.rs`, and command registration lives in `src-tauri/src/lib.rs`.
- Keep business logic out of Tauri command functions: the command layer in `src-tauri/src/commands/mod.rs` converts inputs/outputs and delegates to pure core modules under `src-tauri/src/core/`.

## Layers

- Purpose: Start the React app and load global styling and i18n.
- Location: `src/main.tsx`, `src/index.css`, `src/i18n/index.ts`
- Contains: React root creation, Tailwind/global CSS entry, i18next initialization.
- Depends on: React DOM, `src/App.tsx`, translation resources.
- Used by: Vite frontend startup and Tauri webview bootstrap configured in `src-tauri/tauri.conf.json`.
- Purpose: Coordinate the entire visible application state and every user-triggered workflow.
- Location: `src/App.tsx`
- Contains: View switching (`myskills` / `explore` / `detail` / `settings`), modal visibility, skill list state, tool detection state, onboarding import state, update checks, settings persistence, and all Tauri command calls.
- Depends on: `src/components/skills/*.tsx`, `src/components/skills/modals/*.tsx`, `src/components/skills/types.ts`, `@tauri-apps/api/core`, Tauri plugins, i18n.
- Used by: `src/main.tsx` only.
- Purpose: Render UI sections without owning the application-wide state model.
- Location: `src/components/skills/Header.tsx`, `src/components/skills/FilterBar.tsx`, `src/components/skills/SkillsList.tsx`, `src/components/skills/SkillCard.tsx`, `src/components/skills/ExplorePage.tsx`, `src/components/skills/SkillDetailView.tsx`, `src/components/skills/SettingsPage.tsx`, `src/components/skills/modals/*.tsx`
- Contains: Header tabs, search/sort bar, cards, detail viewer, settings form, pickers, import dialogs, delete dialog, shared-directory confirmation.
- Depends on: Props passed from `src/App.tsx`, shared DTO types in `src/components/skills/types.ts`, `react-i18next`, `lucide-react`, `sonner`, `react-markdown`, syntax highlighting.
- Used by: `src/App.tsx`.
- Purpose: Keep frontend type contracts aligned with backend command DTOs.
- Location: `src/components/skills/types.ts`
- Contains: `ManagedSkill`, `InstallResultDto`, `ToolStatusDto`, `OnboardingPlan`, `FeaturedSkillDto`, `OnlineSkillDto`, `SkillFileEntry`.
- Depends on: Backend response shapes defined in `src-tauri/src/commands/mod.rs`.
- Used by: Most components under `src/components/skills/` and `src/App.tsx`.
- Purpose: Start the desktop runtime, register plugins, initialize storage, and expose shared state to commands.
- Location: `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`
- Contains: `app_lib::run()`, Tauri builder setup, plugin registration, SQLite store creation, cancel token registration, background cleanup tasks, and `generate_handler!` command registration.
- Depends on: `src-tauri/src/commands/mod.rs`, `src-tauri/src/core/skill_store.rs`, `src-tauri/src/core/cache_cleanup.rs`, `src-tauri/src/core/temp_cleanup.rs`, Tauri plugins.
- Used by: Desktop executable entry point.
- Purpose: Define the IPC contract between the webview and the backend.
- Location: `src-tauri/src/commands/mod.rs`
- Contains: Tauri command functions, DTO structs, command-specific argument naming, error formatting, and `spawn_blocking` wrappers around synchronous core operations.
- Depends on: Shared state from `src-tauri/src/lib.rs`, all core modules under `src-tauri/src/core/`.
- Used by: Frontend `invokeTauri()` calls in `src/App.tsx` and detail view file readers in `src/components/skills/SkillDetailView.tsx`.
- Purpose: Implement skill management behavior independently from the desktop shell.
- Location: `src-tauri/src/core/`
- Contains: Install/update workflows in `src-tauri/src/core/installer.rs`, sync logic in `src-tauri/src/core/sync_engine.rs`, onboarding discovery in `src-tauri/src/core/onboarding.rs`, tool registry in `src-tauri/src/core/tool_adapters/mod.rs`, central repo resolution in `src-tauri/src/core/central_repo.rs`, local database access in `src-tauri/src/core/skill_store.rs`, online search in `src-tauri/src/core/github_search.rs` and `src-tauri/src/core/skills_search.rs`, featured catalog fetch in `src-tauri/src/core/featured_skills.rs`, file browsing in `src-tauri/src/core/skill_files.rs`, and cleanup/cancel helpers.
- Depends on: Filesystem, SQLite, HTTP, git/network helpers, home-directory resolution.
- Used by: `src-tauri/src/commands/mod.rs` and startup logic in `src-tauri/src/lib.rs`.
- Purpose: Persist managed skill metadata, sync targets, app settings, and schema migrations.
- Location: `src-tauri/src/core/skill_store.rs`
- Contains: SQLite schema creation, incremental migrations, CRUD for `skills`, `skill_targets`, and `settings`, legacy DB migration.
- Depends on: `rusqlite`, app data directory resolution via Tauri, filesystem.
- Used by: Nearly every backend workflow, especially `src-tauri/src/lib.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/core/installer.rs`, `src-tauri/src/core/onboarding.rs`, `src-tauri/src/core/featured_skills.rs`, and cache configuration helpers.

## Data Flow

- Use local React hooks in `src/App.tsx` as the only state container; there is no Redux, Zustand, Context store, or router-driven state model in active use.
- Persist durable settings in two places: browser `localStorage` for UI-only settings like theme/language behavior in `src/App.tsx`, and SQLite settings via backend commands for app-wide configuration such as central repo path, git cache values, and GitHub token in `src-tauri/src/core/skill_store.rs`.

## Key Abstractions

- Purpose: Represent one installed skill in the app-managed central repository.
- Examples: `src-tauri/src/core/skill_store.rs`, `src-tauri/src/commands/mod.rs`, `src/components/skills/types.ts`
- Pattern: Persist backend canonical records as `SkillRecord`, expose UI-safe DTOs as `ManagedSkillDto`, mirror them in TypeScript as `ManagedSkill`.
- Purpose: Represent one sync relationship between a managed skill and a tool directory.
- Examples: `src-tauri/src/core/skill_store.rs`, `src-tauri/src/commands/mod.rs`
- Pattern: Store `(skill_id, tool)` uniqueness in SQLite and materialize per-tool badges in `src/components/skills/SkillCard.tsx`.
- Purpose: Centralize supported AI tools, installation detection, and filesystem destinations.
- Examples: `src-tauri/src/core/tool_adapters/mod.rs`
- Pattern: Use `ToolAdapter` entries plus `ToolId` enum variants; resolve directories from the user home folder and group tools that share the same skills directory.
- Purpose: Decouple the app’s canonical managed copies from external tool directories.
- Examples: `src-tauri/src/core/central_repo.rs`, `src-tauri/src/core/installer.rs`, `src-tauri/src/commands/mod.rs`
- Pattern: Resolve a single base directory (`~/.skillshub` by default or a stored override), copy every imported skill into it, and sync outward from there.
- Purpose: Keep IPC payloads explicit and serializable.
- Examples: `src-tauri/src/commands/mod.rs`, `src/components/skills/types.ts`
- Pattern: Define Rust DTO structs next to commands, serialize them with `serde`, and mirror the shapes in `src/components/skills/types.ts`.
- Purpose: Allow long-running git/download operations to be canceled from the UI.
- Examples: `src-tauri/src/core/cancel_token.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/commands/mod.rs`, `src/App.tsx`
- Pattern: Register one shared `Arc<CancelToken>` in Tauri state, reset it before install operations, and expose `cancel_current_operation` for the loading overlay cancel button.

## Entry Points

- Location: `src-tauri/src/main.rs`
- Triggers: Native app startup.
- Responsibilities: Call `app_lib::run()` and keep the binary entry minimal.
- Location: `src-tauri/src/lib.rs`
- Triggers: `src-tauri/src/main.rs`.
- Responsibilities: Build the Tauri app, register plugins, initialize logging and persistence, schedule cleanup tasks, and register all frontend-callable commands.
- Location: `src/main.tsx`
- Triggers: Vite dev server or Tauri webview page load.
- Responsibilities: Mount React, global CSS, and i18n.
- Location: `src/App.tsx`
- Triggers: Every user interaction after initial render.
- Responsibilities: Orchestrate command calls, own all feature state, choose which top-level screen to render, and coordinate modal workflows.
- Location: `src/components/Layout.tsx`, `src/pages/Dashboard.tsx`
- Triggers: Not wired into `src/main.tsx` or `src/App.tsx`.
- Responsibilities: Provide an alternate router-style shell with sidebar navigation and a placeholder dashboard. Treat these files as dormant until they are explicitly connected.

## Error Handling

- Use `tauri::async_runtime::spawn_blocking` in `src-tauri/src/commands/mod.rs` for filesystem, database, and network work so commands can remain async without rewriting the core layer.
- Use prefixed errors such as `MULTI_SKILLS|`, `TARGET_EXISTS|`, `TOOL_NOT_INSTALLED|`, `TOOL_NOT_WRITABLE|`, and `SKILL_INVALID|...` in `src-tauri/src/commands/mod.rs` and `src-tauri/src/core/installer.rs` to drive frontend-specific flows.
- Use `format_anyhow_error()` in `src-tauri/src/commands/mod.rs` to preserve error chains and rewrite common GitHub/network failures into clearer messages.
- Use toast-based display in `src/App.tsx` with `formatErrorMessage()` for end-user translation and suppression of canceled operations.

## Cross-Cutting Concerns

<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->

## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, or `.github/skills/` with a `SKILL.md` index file.

<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->

## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:

- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.

<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->

## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.

<!-- GSD:profile-end -->
