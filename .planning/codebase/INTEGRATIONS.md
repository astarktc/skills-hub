# External Integrations

**Analysis Date:** 2026-04-07

## APIs & External Services

**Code hosting and repository metadata:**

- GitHub REST API - repository search and release metadata lookup
  - SDK/Client: `reqwest` in `src-tauri/src/core/github_search.rs`; browser `fetch` in `src/App.tsx`
  - Auth: optional `github_token` value persisted through `src-tauri/src/commands/mod.rs`
- GitHub Contents API - direct download of subdirectories from GitHub repositories without full clone
  - SDK/Client: `reqwest` in `src-tauri/src/core/github_download.rs`
  - Auth: optional `github_token` value persisted through `src-tauri/src/commands/mod.rs`
- GitHub repository clone/pull - install/update flows for Git-based skills
  - SDK/Client: `git2` in `src-tauri/src/core/git_fetcher.rs` with process/libgit2 fallback handling
  - Auth: no app-specific credential manager detected; optional token is used for GitHub API paths, not as a general Git credential helper

**Skill discovery sources:**

- `skills.sh` - online skill search used by the Explore page
  - SDK/Client: `reqwest` in `src-tauri/src/core/skills_search.rs`
  - Auth: none detected
- Raw GitHub content for featured skills - downloads curated `featured-skills.json`
  - SDK/Client: `reqwest` in `src-tauri/src/core/featured_skills.rs`
  - Auth: none detected
  - Source URL: `https://raw.githubusercontent.com/qufei1993/skills-hub/main/featured-skills.json`

**Application distribution:**

- GitHub Releases - desktop updater endpoint and release asset hosting
  - SDK/Client: Tauri updater plugin in `src-tauri/src/lib.rs`, `src-tauri/tauri.conf.json`, `src/App.tsx`, and `src/components/skills/SettingsPage.tsx`
  - Auth: public updater endpoint for clients; release publishing uses GitHub Actions secrets in `.github/workflows/release.yml`

**Tool ecosystem integration:**

- Local AI coding tool directories - install, scan, and sync operations target per-tool skill directories discovered by adapters in `src-tauri/src/core/tool_adapters/mod.rs`
  - SDK/Client: custom filesystem integration in `src-tauri/src/core/onboarding.rs`, `src-tauri/src/core/sync_engine.rs`, and `src-tauri/src/core/installer.rs`
  - Auth: local filesystem access only

## Data Storage

**Databases:**

- SQLite (embedded, local only)
  - Connection: no external connection string; file path is resolved in `src-tauri/src/core/skill_store.rs` via `default_db_path()`
  - Client: `rusqlite` in `src-tauri/src/core/skill_store.rs`
  - Schema includes `skills`, `skill_targets`, `settings`, and `discovered_skills` tables in `src-tauri/src/core/skill_store.rs`

**File Storage:**

- Local filesystem only
  - Central repository path resolves to `~/.skillshub` by default in `src-tauri/src/core/central_repo.rs`
  - Skill sync targets are tool-specific directories managed by `src-tauri/src/core/sync_engine.rs` and adapter definitions in `src-tauri/src/core/tool_adapters/mod.rs`
  - Temporary git download/cache directories are managed by `src-tauri/src/core/temp_cleanup.rs` and `src-tauri/src/core/cache_cleanup.rs`

**Caching:**

- SQLite-backed settings cache for featured skills JSON in `src-tauri/src/core/featured_skills.rs`
- Filesystem Git cache with configurable TTL/cleanup values in `src-tauri/src/core/cache_cleanup.rs`, surfaced through commands in `src-tauri/src/commands/mod.rs` and UI state in `src/App.tsx`
- In-memory updater state in frontend components `src/App.tsx` and `src/components/skills/SettingsPage.tsx`

## Authentication & Identity

**Auth Provider:**

- No end-user identity provider detected
  - Implementation: the app runs as a local desktop application without sign-in flows in `src/**/*.tsx` or `src-tauri/src/**/*.rs`

**API authentication:**

- Optional GitHub token for higher-rate GitHub API access and private/limited access scenarios
  - Implementation: token is saved in SQLite settings via `set_github_token` in `src-tauri/src/commands/mod.rs`, read via `get_github_token`, and attached as `Authorization: Bearer ...` in `src-tauri/src/core/github_search.rs` and `src-tauri/src/core/github_download.rs`
  - UI entry point: token field on `src/components/skills/SettingsPage.tsx` with persistence triggered from `src/App.tsx`

## Monitoring & Observability

**Error Tracking:**

- None detected for external SaaS error tracking

**Logs:**

- Tauri log plugin writes logs to the app log directory and stdout on desktop, configured in `src-tauri/src/lib.rs`
- Frontend surfaces operational failures through toast notifications in `src/App.tsx` and `src/components/skills/SkillCard.tsx`
- Backend error strings are returned over Tauri IPC from `src-tauri/src/commands/mod.rs`

## CI/CD & Deployment

**Hosting:**

- GitHub Releases hosts distributable binaries and updater metadata, assembled in `.github/workflows/release.yml`
- Featured skills source data is also distributed via repository content (`featured-skills.json`) and raw GitHub URLs referenced by `src-tauri/src/core/featured_skills.rs`

**CI Pipeline:**

- GitHub Actions
  - CI checks in `.github/workflows/ci.yml`
  - Release packaging and updater publication in `.github/workflows/release.yml`
  - Scheduled featured-skills refresh in `.github/workflows/update-featured-skills.yml`

## Environment Configuration

**Required env vars:**

- No required local runtime environment variables are detected in the application code paths under `src/` and `src-tauri/src/`
- Optional user-provided GitHub token is stored internally, not read from process environment, through `src-tauri/src/commands/mod.rs`
- CI/release environment uses GitHub Actions secrets in `.github/workflows/release.yml`:
  - `GITHUB_TOKEN`
  - `TAURI_SIGNING_PRIVATE_KEY`
  - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
  - `APPLE_CERTIFICATE`
  - `APPLE_CERTIFICATE_PASSWORD`
  - `APPLE_SIGNING_IDENTITY`
  - `KEYCHAIN_PASSWORD`

**Secrets location:**

- User GitHub token is stored in the local SQLite `settings` table managed by `src-tauri/src/core/skill_store.rs`
- CI/release secrets are stored in GitHub Actions repository secrets, referenced by `.github/workflows/release.yml` and `.github/workflows/update-featured-skills.yml`
- `.env.example` is present at `/home/alexwsl/skills-hub/.env.example`, but no dotenv-based runtime secret loading is detected

## Webhooks & Callbacks

**Incoming:**

- None detected

**Outgoing:**

- GET requests to `https://skills.sh/api/search` from `src-tauri/src/core/skills_search.rs`
- GET requests to `https://api.github.com/search/repositories` from `src-tauri/src/core/github_search.rs`
- GET requests to `https://api.github.com/repos/{owner}/{repo}/contents/...` and file download URLs from `src-tauri/src/core/github_download.rs`
- GET requests to `https://raw.githubusercontent.com/qufei1993/skills-hub/main/featured-skills.json` from `src-tauri/src/core/featured_skills.rs`
- GET requests to `https://api.github.com/repos/qufei1993/skills-hub/releases/tags/v{version}` from `src/App.tsx`
- Tauri updater polling against `https://github.com/qufei1993/skills-hub/releases/latest/download/updater.json` configured in `src-tauri/tauri.conf.json`

---

_Integration audit: 2026-04-07_
