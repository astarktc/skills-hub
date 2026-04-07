# Technology Stack

**Analysis Date:** 2026-04-07

## Languages

**Primary:**

- TypeScript 5.9.x - React desktop UI in `src/**/*.ts` and `src/**/*.tsx`, configured by `package.json`, `tsconfig.json`, `tsconfig.app.json`, and `vite.config.ts`
- Rust 2021 edition (MSRV 1.77.2) - Tauri backend, native app shell, filesystem/database/network logic in `src-tauri/src/**/*.rs`, configured by `src-tauri/Cargo.toml`

**Secondary:**

- CSS - Global styling and theme variables in `src/App.css` and `src/index.css`
- JSON - App/package configuration in `package.json`, `src-tauri/tauri.conf.json`, `featured-skills.json`, and GitHub workflow files under `.github/workflows/`
- JavaScript (Node.js scripts) - Release/version automation in `scripts/*.mjs` referenced by `package.json` and `.github/workflows/*.yml`
- YAML - CI/CD automation in `.github/workflows/ci.yml`, `.github/workflows/release.yml`, and `.github/workflows/update-featured-skills.yml`

## Runtime

**Environment:**

- Node.js 20 in CI and release workflows, declared in `.github/workflows/ci.yml` and `.github/workflows/release.yml`
- Node.js 18+ for local development, with 20+ recommended in `README.md`
- Rust stable toolchain with minimum supported version 1.77.2 in `src-tauri/Cargo.toml`
- Tauri 2 desktop runtime, combining the web UI and native Rust backend in `src-tauri/src/lib.rs` and `src-tauri/tauri.conf.json`

**Package Manager:**

- npm - scripts and install flow are defined in `package.json` and `README.md`
- Lockfile: present via `package-lock.json`
- Cargo - Rust dependency manager for `src-tauri/Cargo.toml`
- Lockfile: present via `src-tauri/Cargo.lock`

## Frameworks

**Core:**

- React 19.2.x - UI framework for the desktop frontend in `src/App.tsx`, `src/components/**/*.tsx`, and `src/pages/**/*.tsx`
- Tauri 2.9.x - Desktop application framework and IPC bridge in `src-tauri/src/lib.rs`, `src-tauri/src/commands/mod.rs`, and `src-tauri/tauri.conf.json`
- Vite 7.3.x - Frontend dev server and production bundler in `vite.config.ts` and `package.json`
- React Router DOM 7.12.x - Client-side routing for page navigation, imported from frontend files such as `src/pages/*.tsx`
- i18next 25.x + react-i18next 16.x - bilingual UI localization in `src/i18n/index.ts`, `src/i18n/resources.ts`, and frontend components importing `TFunction` or `useTranslation`
- Tailwind CSS 4.1.x - utility styling via Vite plugin in `vite.config.ts` and dependency declarations in `package.json`

**Testing:**

- Rust built-in test harness (`cargo test`) - backend/unit integration tests under `src-tauri/src/core/tests/` and `src-tauri/src/commands/tests/`
- Mockito 1.x - HTTP mocking for Rust tests in `src-tauri/src/core/tests/github_search.rs`, `src-tauri/src/core/tests/skills_search.rs`, and `src-tauri/Cargo.toml`
- Tempfile 3.x - temporary filesystem fixtures for Rust tests in `src-tauri/src/core/tests/*.rs` and `src-tauri/Cargo.toml`
- No dedicated frontend test runner is detected in `package.json` or root config files

**Build/Dev:**

- `@vitejs/plugin-react` 5.1.x - React integration for Vite in `vite.config.ts`
- `@tailwindcss/vite` 4.1.x - Tailwind Vite integration in `vite.config.ts`
- ESLint 9.39.x with flat config - frontend linting in `eslint.config.js`
- TypeScript project references - split app/node TS configs via `tsconfig.json`, `tsconfig.app.json`, and `tsconfig.node.json`
- `tauri-build` 2.5.3 - Rust-side build integration in `src-tauri/Cargo.toml` and `src-tauri/build.rs`

## Key Dependencies

**Critical:**

- `@tauri-apps/api` 2.9.1 - frontend access to Tauri APIs from files such as `src/App.tsx` and `src/components/skills/SettingsPage.tsx`
- `@tauri-apps/plugin-dialog` 2.5.3 - native directory/file pickers used from frontend flows and registered in `src-tauri/src/lib.rs`
- `@tauri-apps/plugin-opener` 2.5.3 - shell/open integration registered in `src-tauri/src/lib.rs`
- `@tauri-apps/plugin-updater` 2.5.3 - auto-update checks and installation used in `src/App.tsx`, `src/components/skills/SettingsPage.tsx`, `src-tauri/src/lib.rs`, and `src-tauri/tauri.conf.json`
- `react-markdown` 10.1.0 + `remark-frontmatter` 5.0.0 + `remark-gfm` 4.0.1 - Markdown skill file rendering in frontend detail views under `src/components/skills/`
- `react-syntax-highlighter` 16.1.1 - code block rendering in skill detail UI under `src/components/skills/`
- `sonner` 2.0.7 - toast notifications used in `src/App.tsx` and `src/components/skills/SkillCard.tsx`
- `lucide-react` 0.562.0 - icon system used broadly across `src/components/**/*.tsx`

**Infrastructure:**

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

**Environment:**

- Runtime configuration is primarily app-internal, persisted in SQLite settings via `src-tauri/src/core/skill_store.rs` rather than loaded from dotenv files
- `.env.example` exists at `/home/alexwsl/skills-hub/.env.example`, but no dotenv loader is detected in `package.json`, `vite.config.ts`, or `src-tauri/src/**/*.rs`
- GitHub API authentication is optional and stored through `get_github_token` / `set_github_token` in `src-tauri/src/commands/mod.rs`, with the token consumed by `src-tauri/src/core/github_search.rs`, `src-tauri/src/core/github_download.rs`, and `src-tauri/src/core/installer.rs`
- Tauri updater endpoints and signing metadata are configured in `src-tauri/tauri.conf.json`
- Version synchronization between frontend and Tauri config is enforced by `scripts/version.mjs` and the `version:*` scripts in `package.json`

**Build:**

- Frontend build: `vite.config.ts`, `tsconfig.json`, `tsconfig.app.json`, `tsconfig.node.json`
- Linting: `eslint.config.js`
- Tauri/native build: `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`
- CI verification: `.github/workflows/ci.yml`
- Release packaging and updater artifact assembly: `.github/workflows/release.yml`

## Platform Requirements

**Development:**

- Node.js 18+ with npm, documented in `README.md`
- Rust stable toolchain, with MSRV 1.77.2 from `src-tauri/Cargo.toml`
- Tauri OS dependencies, documented in `README.md`; Linux CI installs GTK/WebKit-related packages in `.github/workflows/ci.yml`
- Desktop environment capable of running Tauri apps; the app is not configured as a web-only deployment target

**Production:**

- Packaged desktop binaries produced by Tauri for macOS, Windows, and Linux via scripts in `package.json` and pipelines in `.github/workflows/release.yml`
- Auto-update artifacts published through GitHub Releases, configured by `src-tauri/tauri.conf.json` and `.github/workflows/release.yml`
- Local persistent storage uses SQLite database file resolution from `src-tauri/src/core/skill_store.rs` and central repository filesystem storage from `src-tauri/src/core/central_repo.rs`

---

_Stack analysis: 2026-04-07_
