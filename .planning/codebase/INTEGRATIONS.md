# External Integrations

**Analysis Date:** 2026-04-16

## APIs & External Services

**Source Discovery and Search:**

- GitHub REST API - repository search, release metadata lookup, branch SHA lookup, and repository contents download
  - SDK/Client: `reqwest` in `src-tauri/src/core/github_search.rs` and `src-tauri/src/core/github_download.rs`; browser `fetch()` in `src/App.tsx`
  - Auth: optional `github_token` stored through `src-tauri/src/commands/mod.rs`; workflow `GITHUB_TOKEN` in `.github/workflows/update-featured-skills.yml` and `.github/workflows/release.yml`
- GitHub repository clone endpoints - git-based installs and updates from skill repositories
  - SDK/Client: `git2` in `src-tauri/src/core/git_fetcher.rs` and installer flows in `src-tauri/src/core/installer.rs`
  - Auth: optional `github_token` for API-backed paths; git clone auth behavior is not separately configured in the inspected files
- skills.sh search API - online skill search from the Explore page
  - SDK/Client: `reqwest` in `src-tauri/src/core/skills_search.rs`
  - Auth: none detected
- Raw GitHub content delivery - featured skills catalog refresh
  - SDK/Client: `reqwest` in `src-tauri/src/core/featured_skills.rs`
  - Auth: none required in runtime fetch path

**Application Distribution:**

- GitHub Releases updater feed - Tauri auto-updates download `updater.json` and release artifacts
  - SDK/Client: `@tauri-apps/plugin-updater` in `src/App.tsx`; Tauri updater plugin in `src-tauri/src/lib.rs`
  - Auth: none detected for client-side update checks

## Data Storage

**Databases:**

- SQLite (embedded local database)
  - Connection: no environment variable; opened from app data paths in `src-tauri/src/core/skill_store.rs`
  - Client: `rusqlite` in `src-tauri/src/core/skill_store.rs`
  - Stored data includes skills, skill targets, settings, discovered skills, projects, project tools, and project assignments in `src-tauri/src/core/skill_store.rs`

**File Storage:**

- Local filesystem only for managed skills, sync targets, caches, and temp directories
  - Central repository defaults to `~/.skillshub`, described in `README.md` and resolved by backend modules under `src-tauri/src/core/`
  - Project sync writes into tool-specific project directories through backend project modules in `src-tauri/src/core/project_sync.rs` and `src-tauri/src/core/project_ops.rs`

**Caching:**

- SQLite-backed cache for featured skills JSON using the `featured_skills_cache` setting in `src-tauri/src/core/featured_skills.rs`
- Local git cache directories managed by cleanup settings exposed from `src/App.tsx` and backend cleanup modules in `src-tauri/src/core/cache_cleanup.rs`
- Browser `localStorage` for UI-only preferences and update dismissal state in `src/i18n/index.ts` and `src/App.tsx`

## Authentication & Identity

**Auth Provider:**

- Custom token handling for GitHub API access
  - Implementation: user enters a personal token in `src/components/skills/SettingsPage.tsx`; frontend saves it through `set_github_token` in `src/App.tsx`; backend persists it in SQLite settings through `src-tauri/src/commands/mod.rs` and `src-tauri/src/core/skill_store.rs`
- No OAuth, SSO, or third-party identity provider is detected in `src/`, `src-tauri/src/`, or workflow configuration

## Monitoring & Observability

**Error Tracking:**

- None detected as an external SaaS integration

**Logs:**

- Native application logs use `tauri-plugin-log` configured in `src-tauri/src/lib.rs`, writing to the Tauri log directory and stdout on desktop builds
- User-visible operational feedback uses `sonner` toasts in `src/App.tsx` and components under `src/components/skills/`
- GitHub Actions logs provide CI/release visibility in `.github/workflows/ci.yml`, `.github/workflows/release.yml`, and `.github/workflows/update-featured-skills.yml`

## CI/CD & Deployment

**Hosting:**

- GitHub Releases - packaged desktop binaries and updater metadata are published through `.github/workflows/release.yml`
- No server-side application hosting platform is detected; this is a packaged desktop app built by Tauri from `src-tauri/tauri.conf.json`

**CI Pipeline:**

- GitHub Actions - continuous integration in `.github/workflows/ci.yml`, release packaging in `.github/workflows/release.yml`, and featured catalog refresh in `.github/workflows/update-featured-skills.yml`

## Environment Configuration

**Required env vars:**

- Runtime desktop app: no required env vars detected for normal app startup in `src-tauri/src/lib.rs`, `src-tauri/src/commands/mod.rs`, or `src/App.tsx`
- Optional GitHub API token for desktop app features: stored as `github_token` in SQLite through `src-tauri/src/commands/mod.rs`
- Release workflow secrets: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `KEYCHAIN_PASSWORD`, and `GITHUB_TOKEN` in `.github/workflows/release.yml`
- Featured skills update workflow secret: `GITHUB_TOKEN` in `.github/workflows/update-featured-skills.yml`
- Script-local optional env file support: `GITHUB_TOKEN` read by `scripts/fetch-featured-skills.mjs`

**Secrets location:**

- Desktop app GitHub token is stored in the SQLite `settings` table via `src-tauri/src/core/skill_store.rs`
- CI/CD secrets are stored in GitHub Actions secrets and referenced from `.github/workflows/release.yml` and `.github/workflows/update-featured-skills.yml`
- `.env.example` is present at `/home/alexwsl/skills-hub/.env.example`; a local `.env` may be consumed by `scripts/fetch-featured-skills.mjs`, but its contents were not read

## Webhooks & Callbacks

**Incoming:**

- None detected in `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`, or `.github/workflows/*.yml`

**Outgoing:**

- GitHub REST API requests from `src-tauri/src/core/github_search.rs`, `src-tauri/src/core/github_download.rs`, `scripts/fetch-featured-skills.mjs`, and `src/App.tsx`
- skills.sh search requests from `src-tauri/src/core/skills_search.rs`
- Raw featured-skills JSON fetch from `src-tauri/src/core/featured_skills.rs`
- GitHub Releases updater checks from the Tauri updater configured in `src-tauri/tauri.conf.json` and invoked in `src/App.tsx`

---

_Integration audit: 2026-04-16_
