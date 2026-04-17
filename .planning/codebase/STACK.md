# Technology Stack

**Analysis Date:** 2026-04-16

## Languages

**Primary:**

- TypeScript 5.9.3 - React desktop frontend in `src/**/*.ts` and `src/**/*.tsx`, configured by `package.json`, `tsconfig.json`, `tsconfig.app.json`, and `tsconfig.node.json`
- Rust 2021 edition (MSRV 1.77.2) - Tauri backend and native desktop runtime in `src-tauri/src/**/*.rs`, configured by `src-tauri/Cargo.toml`

**Secondary:**

- CSS - Global styling in `src/index.css` and `src/App.css`
- JSON - App, package, Tauri, and workflow configuration in `package.json`, `src-tauri/tauri.conf.json`, and `.github/workflows/*.yml`
- JavaScript (Node.js scripts) - Release and data-generation automation in `scripts/version.mjs`, `scripts/fetch-featured-skills.mjs`, and related scripts under `scripts/`
- YAML - CI/CD definitions in `.github/workflows/ci.yml`, `.github/workflows/release.yml`, and `.github/workflows/update-featured-skills.yml`

## Runtime

**Environment:**

- Node.js 20 in CI and release workflows, set in `.github/workflows/ci.yml` and `.github/workflows/release.yml`
- Node.js 18+ for local development, documented in `README.md`
- Rust stable toolchain with minimum supported version 1.77.2 in `src-tauri/Cargo.toml`
- Tauri 2 desktop runtime bootstrapped from `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, and `src-tauri/tauri.conf.json`

**Package Manager:**

- npm - scripts and dependency installation are defined in `package.json`
- Cargo - Rust dependency management in `src-tauri/Cargo.toml`
- Lockfile: present via `package-lock.json` and `src-tauri/Cargo.lock`

## Frameworks

**Core:**

- React 19.2.3 - UI framework for the desktop frontend in `src/App.tsx`, `src/components/**/*.tsx`, and `src/main.tsx`
- Tauri 2.9.5 / CLI 2.9.6 - desktop shell, IPC bridge, plugin host, and bundling pipeline in `src-tauri/src/lib.rs`, `src-tauri/src/commands/mod.rs`, and `src-tauri/tauri.conf.json`
- Vite 7.3.1 - frontend dev server and production bundler in `vite.config.ts` and `package.json`
- React Router DOM 7.12.0 - routed UI primitives are available in frontend files such as `src/components/Layout.tsx` and `src/pages/Dashboard.tsx`
- i18next 25.7.4 + react-i18next 16.5.3 - localization layer in `src/i18n/index.ts` and `src/i18n/resources.ts`
- Tailwind CSS 4.1.18 - utility styling via Vite integration in `vite.config.ts`

**Testing:**

- Rust built-in test harness (`cargo test`) - backend and core tests in `src-tauri/src/core/tests/*.rs` and `src-tauri/src/commands/tests/commands.rs`
- Mockito 1.x - HTTP mocking for Rust tests in `src-tauri/src/core/tests/github_search.rs`, `src-tauri/src/core/tests/skills_search.rs`, and `src-tauri/src/core/tests/featured_skills.rs`
- Tempfile 3.x - temporary filesystem fixtures for Rust tests in `src-tauri/src/core/tests/*.rs`
- No dedicated frontend test runner detected in `package.json`, project root config, or `src/`

**Build/Dev:**

- `@vitejs/plugin-react` 5.1.2 - React support in `vite.config.ts`
- `@tailwindcss/vite` 4.1.18 - Tailwind integration in `vite.config.ts`
- ESLint 9.39.2 with flat config - frontend linting in `eslint.config.js`
- TypeScript project references - app/node split configs in `tsconfig.json`, `tsconfig.app.json`, and `tsconfig.node.json`
- `tauri-build` 2.5.3 - Rust-side build integration in `src-tauri/Cargo.toml`

## Key Dependencies

**Critical:**

- `@tauri-apps/api` 2.10.1 - frontend access to Tauri IPC and native APIs from `src/App.tsx` and skill detail flows under `src/components/skills/`
- `@tauri-apps/plugin-dialog` 2.7.0 - native file and directory selection used in `src/App.tsx` and typed in `src/tauri-plugin-dialog.d.ts`
- `@tauri-apps/plugin-updater` 2.10.1 - auto-update check and installation used in `src/App.tsx` and enabled in `src-tauri/tauri.conf.json`
- `rusqlite` 0.31 with `bundled` feature - embedded SQLite persistence in `src-tauri/src/core/skill_store.rs`
- `reqwest` 0.12 with `blocking`, `json`, and `rustls-tls` - outbound HTTP for GitHub, skills.sh, and featured skills fetches in `src-tauri/src/core/github_search.rs`, `src-tauri/src/core/github_download.rs`, `src-tauri/src/core/skills_search.rs`, and `src-tauri/src/core/featured_skills.rs`
- `git2` 0.19 with `vendored-openssl` - Git clone and update flows in `src-tauri/src/core/git_fetcher.rs` and installer paths in `src-tauri/src/core/installer.rs`

**Infrastructure:**

- `tauri-plugin-log` 2 - native logging initialized in `src-tauri/src/lib.rs`
- `tauri-plugin-opener` 2 - OS open/shell integration registered in `src-tauri/src/lib.rs`
- `serde` / `serde_json` 1.0 - DTO serialization and config/data parsing throughout `src-tauri/src/commands/mod.rs` and `src-tauri/src/core/*.rs`
- `dirs` 5.0 - home and app-data path resolution in `src-tauri/src/core/central_repo.rs`, `src-tauri/src/core/tool_adapters/mod.rs`, and `src-tauri/src/commands/mod.rs`
- `walkdir` 2.5, `sha2` 0.10, and `hex` 0.4 - skill content hashing and filesystem traversal in `src-tauri/src/core/content_hash.rs`
- `junction` 1.1 - Windows junction fallback in `src-tauri/src/core/sync_engine.rs`
- `uuid` 1.x with v4 - identifier generation in `src-tauri/src/commands/mod.rs` and `src-tauri/src/core/installer.rs`
- `react-markdown` 10.1.0, `remark-frontmatter` 5.0.0, `remark-gfm` 4.0.1, and `react-syntax-highlighter` 16.1.1 - skill file rendering in `src/components/skills/SkillDetailView.tsx`
- `lucide-react` 0.562.0 - icon set used throughout `src/components/**/*.tsx`
- `sonner` 2.0.7 - toast notifications in `src/App.tsx` and presentational components under `src/components/skills/`
- `clsx` 2.1.1 and `tailwind-merge` 3.4.0 - frontend class composition utilities declared in `package.json`

## Configuration

**Environment:**

- Runtime app settings are persisted in SQLite through the `settings` table in `src-tauri/src/core/skill_store.rs`, not loaded through a dotenv runtime system
- `.env.example` exists at `/home/alexwsl/skills-hub/.env.example` - environment template present; contents not read
- A local `.env` file is optionally read only by the data-generation script `scripts/fetch-featured-skills.mjs` for `GITHUB_TOKEN`; no general app dotenv loader is detected in `package.json`, `vite.config.ts`, or `src-tauri/src/**/*.rs`
- GitHub token storage for the desktop app is handled by `get_github_token` and `set_github_token` in `src-tauri/src/commands/mod.rs`, backed by SQLite settings in `src-tauri/src/core/skill_store.rs`

**Build:**

- Frontend build config: `vite.config.ts`, `tsconfig.json`, `tsconfig.app.json`, `tsconfig.node.json`
- Lint config: `eslint.config.js`
- Native build config: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`
- CI verification: `.github/workflows/ci.yml`
- Release packaging and updater assembly: `.github/workflows/release.yml`
- Featured catalog refresh automation: `.github/workflows/update-featured-skills.yml` and `scripts/fetch-featured-skills.mjs`
- Version synchronization between web and Tauri packages: `scripts/version.mjs`, `package.json`, and `src-tauri/tauri.conf.json`

## Platform Requirements

**Development:**

- Node.js 18+ with npm, documented in `README.md`
- Rust stable toolchain, with MSRV 1.77.2 declared in `src-tauri/Cargo.toml`
- Tauri desktop system dependencies per OS; Linux CI installs GTK/WebKit packages in `.github/workflows/ci.yml`
- Desktop environment capable of running a Tauri app; no web-only deployment target is configured

**Production:**

- Packaged desktop binaries for macOS, Windows, and Linux are built through Tauri using `npm run tauri:build*` scripts from `package.json`
- GitHub Releases are the release distribution target, driven by `.github/workflows/release.yml`
- Auto-update metadata and artifacts are published for the Tauri updater configured in `src-tauri/tauri.conf.json`

---

_Stack analysis: 2026-04-16_
