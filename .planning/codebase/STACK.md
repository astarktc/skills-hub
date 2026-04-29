# Technology Stack

**Analysis Date:** 2026-04-29

## Languages

**Primary:**

- TypeScript 5.9.3 - React desktop UI in `src/App.tsx`, `src/main.tsx`, `src/components/**/*.tsx`, and frontend type contracts in `src/components/skills/types.ts`; configured by `package.json`, `tsconfig.json`, `tsconfig.app.json`, and `vite.config.ts`.
- Rust 2021 edition, MSRV 1.77.2 - Tauri backend, IPC command handlers, SQLite persistence, filesystem sync, Git/HTTP integrations, and tests under `src-tauri/src/**/*.rs`; configured by `src-tauri/Cargo.toml`.

**Secondary:**

- CSS - Global styling and theme variables in `src/App.css` and `src/index.css`.
- JavaScript / Node.js ESM - Maintenance and release scripts under `scripts/*.mjs`, especially `scripts/version.mjs`, `scripts/fetch-featured-skills-v2.mjs`, `scripts/extract-changelog.mjs`, and Tauri icon tooling.
- JSON - App metadata and catalogs in `package.json`, `src-tauri/tauri.conf.json`, `featured-skills.json`, and TypeScript config files.
- YAML - GitHub Actions automation in `.github/workflows/ci.yml`, `.github/workflows/release.yml`, and `.github/workflows/update-featured-skills.yml`.

## Runtime

**Environment:**

- Tauri 2.9.5 desktop runtime - Native shell and IPC bridge initialized in `src-tauri/src/main.rs` and `src-tauri/src/lib.rs`, with frontend dist/dev URL configured in `src-tauri/tauri.conf.json`.
- Browser/WebView runtime - React UI runs in the Tauri webview and uses dynamic `@tauri-apps/api` imports from `src/App.tsx` and `src/components/projects/*.tsx`.
- Node.js 18+ local development - Documented in `README.md`; Node 20 is used by CI and release workflows in `.github/workflows/ci.yml`, `.github/workflows/release.yml`, and `.github/workflows/update-featured-skills.yml`.
- Rust stable toolchain - CI uses `dtolnay/rust-toolchain@stable` in `.github/workflows/ci.yml` and `.github/workflows/release.yml`; crate declares `rust-version = "1.77.2"` in `src-tauri/Cargo.toml`.

**Package Manager:**

- npm - JavaScript package manager and script runner via `package.json`.
- Lockfile: present at `package-lock.json`.
- Cargo - Rust package manager for `src-tauri/Cargo.toml`.
- Lockfile: present at `src-tauri/Cargo.lock`.

## Frameworks

**Core:**

- React 19.2.3 - Main UI framework in `src/App.tsx`, `src/main.tsx`, and `src/components/**/*.tsx`.
- Tauri 2.9.5 - Cross-platform desktop framework in `src-tauri/src/lib.rs`, `src-tauri/src/main.rs`, and `src-tauri/tauri.conf.json`.
- Vite 7.3.1 - Frontend dev server and production bundler configured in `vite.config.ts`; dev server runs on port 5173 with `strictPort`.
- Tailwind CSS 4.1.18 - Styling utility framework enabled through `@tailwindcss/vite` in `vite.config.ts`.
- React Router DOM 7.12.0 - Present in `package.json`; dormant router-style files exist under `src/pages/` and `src/components/Layout.tsx`, while active navigation is state-driven in `src/App.tsx`.
- i18next 25.7.4 + react-i18next 16.5.3 - Localization initialized in `src/i18n/index.ts` with resources in `src/i18n/resources.ts`.

**Testing:**

- Rust built-in test harness - Backend tests are under `src-tauri/src/core/tests/*.rs` and `src-tauri/src/commands/tests/*.rs`; run with `npm run rust:test` or `cargo test` from `src-tauri/`.
- Mockito 1.x - HTTP mocking for Rust tests in `src-tauri/src/core/tests/github_search.rs`, `src-tauri/src/core/tests/skills_search.rs`, and other HTTP-focused tests.
- Tempfile 3.x - Temporary filesystem fixtures in Rust tests under `src-tauri/src/core/tests/`.
- Playwright 1.59.1 - Script dependency for `scripts/fetch-featured-skills-v2.mjs` and scheduled featured-skills scraping; not configured as the primary app test runner.
- No dedicated frontend unit test runner is detected in `package.json` or root test config files.

**Build/Dev:**

- TypeScript project references - `tsconfig.json` references `tsconfig.app.json` and `tsconfig.node.json`; `npm run build` runs `tsc -b && vite build`.
- ESLint 9.39.2 flat config - `eslint.config.js` applies `@eslint/js`, `typescript-eslint`, `eslint-plugin-react-hooks`, and `eslint-plugin-react-refresh` to `**/*.{ts,tsx}`.
- `@vitejs/plugin-react` 5.1.2 - React plugin in `vite.config.ts`.
- `@tailwindcss/vite` 4.1.18 - Tailwind Vite plugin in `vite.config.ts`.
- `@tauri-apps/cli` 2.9.6 - Tauri dev/build CLI used by scripts in `package.json`.
- `tauri-build` 2.5.3 - Rust build integration via `src-tauri/build.rs` and `src-tauri/Cargo.toml`.
- GitHub Actions - CI in `.github/workflows/ci.yml`, release packaging in `.github/workflows/release.yml`, and featured catalog refresh in `.github/workflows/update-featured-skills.yml`.

## Key Dependencies

**Critical:**

- `@tauri-apps/api` 2.10.1 - Frontend IPC and app/webview APIs used in `src/App.tsx`, `src/components/projects/ProjectsPage.tsx`, `src/components/projects/useProjectState.ts`, and `src/components/projects/EditProjectModal.tsx`.
- `@tauri-apps/plugin-dialog` 2.7.0 - Native directory picker integration used in `src/App.tsx`, `src/components/projects/ProjectsPage.tsx`, and `src/components/projects/AddProjectModal.tsx`; registered in `src-tauri/src/lib.rs`.
- `@tauri-apps/plugin-updater` 2.10.1 - In-app update checks and installs used in `src/App.tsx` and `src/components/skills/SettingsPage.tsx`; configured in `src-tauri/tauri.conf.json` and registered in `src-tauri/src/lib.rs`.
- `@tauri-apps/plugin-opener` 2.5.3 - Native opener integration registered in `src-tauri/src/lib.rs`.
- `rusqlite` 0.31 with `bundled` - Embedded SQLite persistence in `src-tauri/src/core/skill_store.rs`; stores skills, sync targets, projects, project tools, project assignments, and settings.
- `reqwest` 0.12 with `blocking`, `json`, and `rustls-tls` - HTTP client for GitHub, skills.sh, and featured catalog requests in `src-tauri/src/core/github_search.rs`, `src-tauri/src/core/github_download.rs`, `src-tauri/src/core/skills_search.rs`, and `src-tauri/src/core/featured_skills.rs`.
- `git2` 0.19 with `vendored-openssl` - libgit2 fallback for Git fetch/clone workflows in `src-tauri/src/core/git_fetcher.rs`; system Git is preferred when available.
- `serde` / `serde_json` - Serialization and DTO payloads across `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/projects.rs`, and core HTTP parsers.
- `anyhow` 1.0 - Backend error propagation and context in `src-tauri/src/core/*.rs` and `src-tauri/src/commands/*.rs`.

**Infrastructure:**

- `dirs` 5.0 - Home, data, and app-directory resolution in `src-tauri/src/core/skill_store.rs`, `src-tauri/src/core/central_repo.rs`, `src-tauri/src/core/tool_adapters/mod.rs`, and `src-tauri/src/commands/mod.rs`.
- `walkdir` 2.5 - Recursive file traversal in `src-tauri/src/core/sync_engine.rs`, `src-tauri/src/core/content_hash.rs`, `src-tauri/src/core/skill_files.rs`, and installer logic.
- `sha2` 0.10 + `hex` 0.4 - Content hashing in `src-tauri/src/core/content_hash.rs`.
- `junction` 1.1 - Windows junction fallback for sync in `src-tauri/src/core/sync_engine.rs`.
- `uuid` 1.x with `v4` - Identifier generation for skills, targets, projects, assignments, and operations in backend commands and core modules.
- `urlencoding` 2.1 - Query/path encoding for GitHub and skills.sh requests in `src-tauri/src/core/github_search.rs` and `src-tauri/src/core/skills_search.rs`.
- `tauri-plugin-log` 2 - Backend logging configured in `src-tauri/src/lib.rs` with log-dir and stdout targets.
- `react-markdown` 10.1.0 + `remark-gfm` 4.0.1 + `remark-frontmatter` 5.0.0 - Markdown and frontmatter rendering for skill detail/update notes in `src/App.tsx` and `src/components/skills/SkillDetailView.tsx`.
- `react-syntax-highlighter` 16.1.1 - Code block rendering in skill file previews under `src/components/skills/SkillDetailView.tsx`.
- `sonner` 2.0.7 - Toast notifications used in `src/App.tsx`, `src/components/skills/SkillCard.tsx`, and other UI flows.
- `lucide-react` 0.562.0 - Icon set used broadly by components under `src/components/**/*.tsx`.
- `clsx` 2.1.1 + `tailwind-merge` 3.4.0 - Class composition dependencies available to frontend components.

## Configuration

**Environment:**

- Runtime app configuration is mostly persisted in SQLite settings via `src-tauri/src/core/skill_store.rs`, accessed through commands such as `get_central_repo_path`, `set_central_repo_path`, `get_github_token`, `set_github_token`, `get_git_cache_ttl_secs`, and `set_git_cache_ttl_secs` in `src-tauri/src/commands/mod.rs`.
- UI-only preferences use browser `localStorage` in `src/App.tsx`, including `skills-language`, `skills-theme`, `skills-groupByRepo`, `skills-viewMode`, and `skills-ignored-update-version`.
- `.env.example` exists at `.env.example`; no dotenv runtime loader is detected for the app itself. `scripts/fetch-featured-skills-v2.mjs` optionally reads a local `.env` file for script-only variables such as `GITHUB_TOKEN`.
- Never read or commit `.env` files. Only note existence when present.
- Git command behavior can be tuned with environment variables read in `src-tauri/src/core/git_fetcher.rs`: `SKILLS_HUB_GIT_BIN`, `SKILLS_HUB_GIT_PATH`, `SKILLS_HUB_GIT_TIMEOUT_SECS`, `SKILLS_HUB_GIT_FETCH_TIMEOUT_SECS`, and `SKILLS_HUB_ALLOW_LIBGIT2_FALLBACK`.
- IO profiling can be enabled with `SKILLS_HUB_PROFILE_IO` in `src-tauri/src/core/sync_engine.rs`.
- Featured catalog refresh in `.github/workflows/update-featured-skills.yml` passes GitHub Actions `GITHUB_TOKEN` to `scripts/fetch-featured-skills-v2.mjs`.
- Release signing and packaging in `.github/workflows/release.yml` uses GitHub Actions secrets such as `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, and `KEYCHAIN_PASSWORD`.

**Build:**

- Frontend build config: `vite.config.ts`, `tsconfig.json`, `tsconfig.app.json`, `tsconfig.node.json`.
- Frontend lint config: `eslint.config.js`.
- Tauri build config: `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/build.rs`.
- App version is duplicated in `package.json` and `src-tauri/tauri.conf.json`; keep them synchronized with `scripts/version.mjs` and the `version:*` scripts in `package.json`.
- Tauri updater endpoint and public key are configured in `src-tauri/tauri.conf.json`.
- Bundled fallback featured catalog is committed as `featured-skills.json` and embedded by `src-tauri/src/core/featured_skills.rs`.
- CI verification runs separate web and Rust jobs in `.github/workflows/ci.yml`.
- Release packaging targets macOS, Windows, and Linux in `.github/workflows/release.yml`.

## Platform Requirements

**Development:**

- Node.js 18+ with npm; Node 20+ recommended by `README.md` and used in CI.
- Rust stable with MSRV 1.77.2 from `src-tauri/Cargo.toml`.
- Tauri OS dependencies; Linux CI installs GTK/WebKit/AppIndicator packages in `.github/workflows/ci.yml` and `.github/workflows/release.yml`.
- System Git is strongly preferred for clone/fetch flows in `src-tauri/src/core/git_fetcher.rs`; libgit2 fallback exists when `SKILLS_HUB_ALLOW_LIBGIT2_FALLBACK=1` or no usable Git binary is found.
- Local development commands are defined in `package.json`: `npm run dev`, `npm run tauri:dev`, `npm run build`, `npm run lint`, `npm run rust:test`, `npm run rust:clippy`, and `npm run check`.

**Production:**

- Deployment target is packaged desktop binaries produced by Tauri, not a hosted web app.
- Supported platforms are macOS, Windows, and Linux, documented in `README.md` and packaged through `.github/workflows/release.yml`.
- macOS builds produce `.app`/`.dmg` and updater `.tar.gz` artifacts; Windows builds produce NSIS installer `.exe`; Linux builds produce `.deb` and `.AppImage` artifacts.
- Auto-update artifacts are published through GitHub Releases using updater metadata generated in `.github/workflows/release.yml` and consumed by `@tauri-apps/plugin-updater` from `src/App.tsx` and `src/components/skills/SettingsPage.tsx`.
- Persistent data uses a SQLite database under the Tauri app data directory resolved by `src-tauri/src/core/skill_store.rs`, plus a configurable central skills repository that defaults to `~/.skillshub` via `src-tauri/src/core/central_repo.rs`.

---

_Stack analysis: 2026-04-29_
