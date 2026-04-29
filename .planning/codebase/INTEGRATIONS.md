# External Integrations

**Analysis Date:** 2026-04-29

## APIs & External Services

**GitHub:**

- GitHub Search API - Repository search for skill sources.
  - Implementation: `src-tauri/src/core/github_search.rs` calls `https://api.github.com/search/repositories` through `reqwest::blocking::Client`.
  - SDK/Client: `reqwest` 0.12 with `blocking`, `json`, and `rustls-tls` features.
  - Auth: optional GitHub personal access token stored as SQLite setting `github_token`, read/written through `get_github_token` and `set_github_token` in `src-tauri/src/commands/mod.rs`.
  - Headers: `User-Agent: skills-hub`; `Authorization: Bearer <token>` when configured in `src-tauri/src/core/github_search.rs`.
- GitHub Contents API - Fast download of skill subdirectories and file contents without cloning whole repositories.
  - Implementation: `src-tauri/src/core/github_download.rs` calls `https://api.github.com/repos/{owner}/{repo}/contents/{path}?ref={branch}` recursively and downloads `download_url` file payloads.
  - SDK/Client: `reqwest` 0.12.
  - Auth: optional `github_token` setting; rate-limit responses are converted to `RATE_LIMITED|<minutes>` in `src-tauri/src/core/github_download.rs`.
- GitHub Commits API - Branch SHA lookup for GitHub refs.
  - Implementation: `fetch_branch_sha()` in `src-tauri/src/core/github_download.rs` calls `https://api.github.com/repos/{owner}/{repo}/commits/{branch}`.
  - SDK/Client: `reqwest` 0.12.
  - Auth: optional `github_token` setting.
- GitHub repository clone/fetch - Install/update skills from Git URLs.
  - Implementation: `src-tauri/src/core/git_fetcher.rs` prefers the system `git` CLI and falls back to `git2` when allowed; installer entry points are in `src-tauri/src/core/installer.rs`.
  - SDK/Client: system Git CLI, `git2` 0.19 with `vendored-openssl` fallback.
  - Auth: no interactive auth; `src-tauri/src/core/git_fetcher.rs` sets `GIT_TERMINAL_PROMPT=0` and `GIT_ASKPASS=echo`.
- GitHub Releases API - Fetch full release notes for update prompts.
  - Implementation: `src/App.tsx` fetches `https://api.github.com/repos/astarktc/skills-hub/releases/tags/v${update.version}` after the Tauri updater reports an update.
  - SDK/Client: browser `fetch` in the Tauri webview.
  - Auth: none.
- GitHub raw content - Featured skills catalog refresh inside the app.
  - Implementation: `src-tauri/src/core/featured_skills.rs` fetches `https://raw.githubusercontent.com/astarktc/skills-hub/main/featured-skills.json` and falls back to cached SQLite data or bundled `featured-skills.json`.
  - SDK/Client: `reqwest` 0.12.
  - Auth: none.

**skills.sh:**

- skills.sh Search API - Online Explore search results.
  - Implementation: `src-tauri/src/core/skills_search.rs` calls `https://skills.sh/api/search?q={query}&limit={limit}` and maps `source` values to GitHub URLs.
  - SDK/Client: `reqwest` 0.12.
  - Auth: none.
- skills.sh leaderboard pages - Scheduled featured catalog generation.
  - Implementation: `scripts/fetch-featured-skills-v2.mjs` uses Playwright Chromium to scrape `https://skills.sh/` and `https://skills.sh/hot`, then enriches entries through the GitHub API.
  - SDK/Client: Playwright `chromium`, Node.js `fetch`.
  - Auth: none for skills.sh; GitHub enrichment uses `GITHUB_TOKEN` in CI.

**Tauri Native Plugins:**

- Dialog plugin - Native directory pickers for central repo selection, local skill import, and project registration.
  - Implementation: Frontend imports `@tauri-apps/plugin-dialog` in `src/App.tsx`, `src/components/projects/ProjectsPage.tsx`, and `src/components/projects/AddProjectModal.tsx`; backend registers `tauri_plugin_dialog::init()` in `src-tauri/src/lib.rs`.
  - SDK/Client: `@tauri-apps/plugin-dialog` 2.7.0 and `tauri-plugin-dialog` 2.
  - Auth: not applicable.
- Updater plugin - In-app update check/download/install.
  - Implementation: `src/App.tsx` and `src/components/skills/SettingsPage.tsx` call `@tauri-apps/plugin-updater`; backend registers `tauri_plugin_updater::Builder::new().build()` in `src-tauri/src/lib.rs`; endpoint configured in `src-tauri/tauri.conf.json`.
  - SDK/Client: `@tauri-apps/plugin-updater` 2.10.1 and `tauri-plugin-updater` 2.
  - Auth: public updater endpoint and public key in `src-tauri/tauri.conf.json`.
- Opener plugin - Native open/shell integration.
  - Implementation: registered through `tauri_plugin_opener::init()` in `src-tauri/src/lib.rs`.
  - SDK/Client: `@tauri-apps/plugin-opener` 2.5.3 and `tauri-plugin-opener` 2.
  - Auth: not applicable.
- Log plugin - Backend log output to Tauri log directory and stdout.
  - Implementation: `tauri_plugin_log::Builder` in `src-tauri/src/lib.rs`.
  - SDK/Client: `tauri-plugin-log` 2.
  - Auth: not applicable.

**AI Coding Tool Filesystem Integrations:**

- Supported tool directory detection and sync targets - Skills Hub syncs managed skills into local tool-specific skill directories.
  - Implementation: tool registry in `src-tauri/src/core/tool_adapters/mod.rs`; global sync in `src-tauri/src/core/sync_engine.rs`; project sync in `src-tauri/src/core/project_sync.rs` and `src-tauri/src/core/project_ops.rs`.
  - SDK/Client: local filesystem via Rust stdlib, `walkdir`, and `junction` on Windows.
  - Auth: not applicable.
  - Supported global targets include Cursor `.cursor/skills`, Claude Code `.claude/skills`, Codex `.codex/skills`, OpenCode `.config/opencode/skills`, Amp/Kimi `.config/agents/skills`, GitHub Copilot `.copilot/skills`, Gemini CLI `.gemini/skills`, Windsurf `.codeium/windsurf/skills`, and many others defined in `src-tauri/src/core/tool_adapters/mod.rs`.
  - Supported project-scoped targets are resolved by `project_relative_skills_dir()` in `src-tauri/src/core/tool_adapters/mod.rs`, including Claude Code `.claude/skills` and shared `.agents/skills` locations for multiple tools.

## Data Storage

**Databases:**

- SQLite embedded database.
  - Connection: local app data file resolved by `default_db_path()` in `src-tauri/src/core/skill_store.rs`; database filename is `skills_hub.db`.
  - Client: `rusqlite` 0.31 with bundled SQLite.
  - Schema owner: `src-tauri/src/core/skill_store.rs` creates and migrates tables `skills`, `skill_targets`, `settings`, `discovered_skills`, `projects`, `project_tools`, and `project_skill_assignments`.
  - Migrations: `SCHEMA_VERSION` and incremental migration logic in `ensure_schema()` in `src-tauri/src/core/skill_store.rs`; legacy database migration from old Tauri identifiers is handled by `migrate_legacy_db_if_needed()`.
  - Secrets stored: optional GitHub token is stored in the `settings` table under key `github_token` via `src-tauri/src/commands/mod.rs`.

**File Storage:**

- Central skills repository on local filesystem.
  - Default location: `~/.skillshub`, documented in `README.md` and resolved by `src-tauri/src/core/central_repo.rs`.
  - Configurable location: stored in SQLite settings and surfaced through `get_central_repo_path` / `set_central_repo_path` in `src-tauri/src/commands/mod.rs`.
  - Contents: installed/imported skill directories, Git cache directories, and session-scoped explore cache cleanup referenced by `src-tauri/src/lib.rs`.
- Tool skill directories on local filesystem.
  - Global paths: defined by `relative_skills_dir` in `src-tauri/src/core/tool_adapters/mod.rs`.
  - Project paths: derived by `project_relative_skills_dir()` in `src-tauri/src/core/tool_adapters/mod.rs`.
  - Sync method: symlink first, Windows junction fallback, then copy fallback in `src-tauri/src/core/sync_engine.rs`; Cursor uses copy mode in `sync_dir_for_tool_with_overwrite()`.
- Bundled catalog file.
  - `featured-skills.json` is committed and embedded into `src-tauri/src/core/featured_skills.rs` as fallback data.

**Caching:**

- Featured skills cache.
  - Storage: SQLite `settings` table key `featured_skills_cache` in `src-tauri/src/core/featured_skills.rs`.
  - Fallback order: network fetch from GitHub raw URL, then SQLite cache, then bundled `featured-skills.json`.
- Git/explore cache.
  - Storage: central repo filesystem directories managed by installer/cache modules; cleanup is scheduled at startup in `src-tauri/src/lib.rs`.
  - Settings: `get_git_cache_cleanup_days`, `set_git_cache_cleanup_days`, `get_git_cache_ttl_secs`, and `set_git_cache_ttl_secs` commands in `src-tauri/src/commands/mod.rs`.
  - Cleanup implementation: `src-tauri/src/core/cache_cleanup.rs` and `src-tauri/src/core/temp_cleanup.rs`.
- Browser localStorage.
  - Storage: UI-only preferences in `src/App.tsx`, including language, theme, grouping, view mode, and ignored update version.

## Authentication & Identity

**Auth Provider:**

- Custom optional GitHub token configuration.
  - Implementation: `src/App.tsx` and `src/components/skills/SettingsPage.tsx` expose a GitHub token setting; `src-tauri/src/commands/mod.rs` persists it via `SkillStore`; `src-tauri/src/core/github_search.rs`, `src-tauri/src/core/github_download.rs`, and `src-tauri/src/core/installer.rs` consume it for GitHub API calls.
  - Purpose: raise GitHub API rate limits for search, content download, and metadata enrichment.
  - Auth header: `Authorization: Bearer <token>` in backend GitHub API clients.
- No user account system, OAuth flow, session cookies, or external identity provider is detected.
- Local tool detection is filesystem based and does not authenticate to those tools.

## Monitoring & Observability

**Error Tracking:**

- None detected. No Sentry, Bugsnag, Datadog, OpenTelemetry, or hosted telemetry integration is present in `package.json`, `src-tauri/Cargo.toml`, or source imports.

**Logs:**

- Backend logs use Rust `log` macros and `tauri-plugin-log` configured in `src-tauri/src/lib.rs` with log-directory and stdout targets.
- Git/network/cache operations emit `log::info!` and `log::warn!` messages in files such as `src-tauri/src/core/git_fetcher.rs`, `src-tauri/src/core/sync_engine.rs`, and `src-tauri/src/lib.rs`.
- Frontend uses `sonner` toasts for user-visible operational errors/success in `src/App.tsx`; direct browser console logging is minimal and non-critical.

## CI/CD & Deployment

**Hosting:**

- GitHub Releases host packaged binaries, updater artifacts, and `updater.json` generated by `.github/workflows/release.yml`.
- Tauri updater endpoint is `https://github.com/astarktc/skills-hub/releases/latest/download/updater.json` in `src-tauri/tauri.conf.json`.
- The app itself is distributed as desktop binaries for macOS, Windows, and Linux; no web hosting deployment is detected.

**CI Pipeline:**

- GitHub Actions CI in `.github/workflows/ci.yml`.
  - Web job: checks out code, sets up Node 20 with npm cache, runs `npm ci`, `npm run version:check`, `npm run lint`, and `npm run build`.
  - Rust job: installs Linux Tauri dependencies, sets up Rust stable, caches `src-tauri`, runs `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`.
- GitHub Actions release pipeline in `.github/workflows/release.yml`.
  - Trigger: `v*` tags and manual dispatch.
  - Targets: macOS x86_64/aarch64, Windows x64/arm64, Linux x86_64.
  - Output: Tauri bundles, updater signatures, assembled `updater.json`, and GitHub Release assets.
  - Signing/notarization-related inputs: Tauri signing key and optional Apple certificate secrets.
- GitHub Actions featured catalog pipeline in `.github/workflows/update-featured-skills.yml`.
  - Trigger: daily cron and manual dispatch.
  - Action: installs Playwright Chromium, runs `scripts/fetch-featured-skills-v2.mjs`, commits changed `featured-skills.json`, and pushes to the repository.

## Environment Configuration

**Required env vars:**

- App runtime: none required for basic local use; the app can run without a GitHub token and without external auth.
- Optional app runtime / debugging:
  - `SKILLS_HUB_GIT_BIN` or `SKILLS_HUB_GIT_PATH` - override Git binary path in `src-tauri/src/core/git_fetcher.rs`.
  - `SKILLS_HUB_GIT_TIMEOUT_SECS` - clone timeout in `src-tauri/src/core/git_fetcher.rs`.
  - `SKILLS_HUB_GIT_FETCH_TIMEOUT_SECS` - fetch/sparse checkout timeout in `src-tauri/src/core/git_fetcher.rs`.
  - `SKILLS_HUB_ALLOW_LIBGIT2_FALLBACK` - allow fallback to `git2` after system Git failure in `src-tauri/src/core/git_fetcher.rs`.
  - `SKILLS_HUB_PROFILE_IO` - enable sync copy profiling logs in `src-tauri/src/core/sync_engine.rs`.
- Optional user setting:
  - `github_token` SQLite setting - configured through the UI and used as a GitHub API bearer token.
- Script/CI:
  - `GITHUB_TOKEN` - used by `.github/workflows/update-featured-skills.yml` and `scripts/fetch-featured-skills-v2.mjs` for GitHub API enrichment.
  - `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` - release updater signing in `.github/workflows/release.yml`.
  - `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, and `KEYCHAIN_PASSWORD` - optional macOS signing in `.github/workflows/release.yml`.

**Secrets location:**

- User GitHub token is stored locally in SQLite through `src-tauri/src/core/skill_store.rs`; do not expose the value in logs or docs.
- CI/release secrets are stored in GitHub Actions secrets and referenced by `.github/workflows/release.yml` and `.github/workflows/update-featured-skills.yml`.
- `.env.example` exists in the repo. A local `.env` may be used by `scripts/fetch-featured-skills-v2.mjs`, but `.env` contents must never be read, copied, or committed.
- No `.npmrc`, credential file, or certificate/private-key file content is required for normal development.

## Webhooks & Callbacks

**Incoming:**

- None detected. The app exposes Tauri IPC commands only to its local webview through `tauri::generate_handler!` in `src-tauri/src/lib.rs`; no HTTP server or webhook endpoint is present.
- GitHub Actions workflows are triggered by repository events (`push`, `pull_request`, `schedule`, `workflow_dispatch`, and tag pushes) in `.github/workflows/*.yml`, not by application webhooks.

**Outgoing:**

- GitHub API requests from backend modules:
  - `src-tauri/src/core/github_search.rs` -> GitHub Search API.
  - `src-tauri/src/core/github_download.rs` -> GitHub Contents and Commits APIs.
  - `src-tauri/src/core/featured_skills.rs` -> GitHub raw content.
- GitHub API request from frontend:
  - `src/App.tsx` -> GitHub Releases API for update notes.
- GitHub repository network operations:
  - `src-tauri/src/core/git_fetcher.rs` and `src-tauri/src/core/installer.rs` -> Git clone/fetch/sparse checkout from user-supplied Git URLs, especially GitHub repositories.
- skills.sh requests:
  - `src-tauri/src/core/skills_search.rs` -> `https://skills.sh/api/search`.
  - `scripts/fetch-featured-skills-v2.mjs` -> `https://skills.sh/` and `https://skills.sh/hot`.
- Tauri updater requests:
  - `@tauri-apps/plugin-updater` in `src/App.tsx` and `src/components/skills/SettingsPage.tsx` -> updater endpoint configured in `src-tauri/tauri.conf.json`.

---

_Integration audit: 2026-04-29_
