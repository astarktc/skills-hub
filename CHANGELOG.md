# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

## [1.2.2] - 2026-09-03

A second architecture round (11 tickets): the operations that put skills into tools and take them out again each became one backend operation with one report. See "Internal/architecture" below.

### Added

- **Refresh and Update report what happened**: refreshing skills now reports the outcome of every skill and of every tool it synced to, instead of a bare success/failure. A failed fetch is named per skill and leaves that skill's synced copies untouched.
- **Assignment matrix tells you when statuses are stale**: sync operations run one at a time, so a matrix loaded while one is running shows a notice that the statuses were not re-checked against disk, rather than presenting them as verified.
- **Unsync reports every path it could not remove**, per tool, instead of a count.

### Changed

- **Refreshing many skills is faster**: sources are fetched in parallel (four at a time) and applied one at a time; progress counts completions, not positions. Cancelling stops fetching and applies nothing.
- **Failed removals stay visible and retryable**: when a skill's folder cannot be removed from a tool (permissions, a locked file), the entry is kept and marked `error` instead of being dropped. This now holds everywhere — unsync, delete, unassign, removing a project's tool, removing a project — and deleting a skill whose removal partly failed keeps the skill so the retry can find every artifact.
- **Onboarding import is one operation**: choosing variants and applying them is a single run with progress and a per-group report. With auto-sync off, an original is removed only when it is byte-identical to the imported copy; a same-named but divergent sibling is kept and reported.
- **Installing from GitHub uses the API fast path** (already used by Explore preview) for install and update, recording the real commit; a clone is the fallback. A skill that does not exist in the repository, or a hit rate limit, is now reported as such on install and update instead of a generic git failure.
- **Refusal to delete outside a tool directory** is now a dedicated, localized message (`PATH_OUTSIDE_TOOL_DIRS`).
- **Shared skills directory confirmation** is now an in-app dialog (previously a native browser confirm) and reads the same in both places it appears.

### Fixed

- **Refresh/install from a GitHub repository whose default branch is not `main`** no longer fails with "skill not found": the API fast path falls back to a clone when its assumed branch does not exist.
- **Refreshing a skill can no longer leave a drifting copy**: a copy on a tool that supports symlinks is re-materialised as a link, and the recorded sync mode matches what is on disk.
- **Repository links in the My Skills grouping**: a non-GitHub git source no longer produces a bogus `github.com` link, and the same skill now shows the same repository label on My Skills, the assignment matrix and Explore.

### Internal/architecture

- One process-wide **mutation guard** (`core/mutation_guard.rs`): every operation that materialises or removes a sync target serialises itself at its entry point; the command tier carries no lock state.
- **Propagation** (`core/propagation.rs`) is the one way a changed skill reaches its targets in both scopes; **Refresh** (`core/refresh.rs`) is one batch command (`refresh_managed_skills`) with streamed progress, replacing the deleted per-skill `update_managed_skill`.
- **Artifact removal** (`core/artifact_removal.rs`) is one module with seven scopes, one presence rule and one settlement rule (`docs/adr/0002-keep-row-with-error-on-failed-artifact-removal.md`); every project- and skill-scope removal caller plans over it.
- **Onboarding import** (`core/onboarding_import.rs`) replaces the deleted `import_existing_skill` / `remove_skill_source` commands; the Managed-skill catalog is assembled in core (`core/skill_catalog.rs`).
- **Project mutations return the affected project's view** (`ProjectViewDto`), so the project world applies one result instead of re-reading; per-project counts come from one aggregate query.
- **Git**: one cache entry point with per-key locking and its first tests (`core/git_cache.rs`), and one acquisition module over the API and clone adapters (`core/git_acquisition.rs`).
- **Frontend**: pure `src/lib/skillPresentation.ts` (source labels, repo grouping, search/sort, relative time) and `src/lib/persistedPreference.ts`; formatting props are gone from components.

## [1.2.1] - 2026-09-02

### Added

- **Invocation-mode badge in My Skills**: skills whose `SKILL.md` frontmatter restricts who may invoke them (`disable-model-invocation: true` / `user-invocable: false`) now show a badge with an explanatory tooltip — *User only*, *Model only*, or *Not invocable*. Skills invocable by both (the default) show no badge.

### Changed

- **Cursor now syncs by symlink** like every other tool. Cursor IDE 2.5+ and the current Cursor CLI discover symlinked skill directories, so the copy-only mode is gone. Skills already synced to Cursor as copies are left in place; re-sync with overwrite to replace them with symlinks.

### Fixed

- **Missing skill source**: syncing a skill whose central directory no longer exists now fails with a clear error in symlink mode as well, instead of creating a dangling link and reporting success.

## [1.2.0] - 2026-09-02

A hardening release: the internal architecture was reworked over 36 review tickets (commands/core seam, typed errors, generated IPC bindings, per-world frontend hooks) with the user-visible fixes below. Test coverage grew to 324 Rust + 98 frontend tests.

### Added

- **Chinese coverage for every error and status message**: 61 previously English-only strings now have `zh` translations, and a parity test keeps the `en`/`zh` catalogs in sync.
- **Keyboard-accessible modals**: every dialog is now labelled for screen readers, takes focus on open, and closes on `Escape`.
- **Precise error messages**: failures such as an unknown tool, a missing/invalid project path, a skill that already exists, GitHub rate limits, or a git timeout are reported with dedicated, localized messages instead of raw backend text.

### Fixed

- **Project tool configuration retry**: if configuring a project's tools failed, pressing Confirm again silently dropped the `.gitignore` choice; the intent is now kept until the save succeeds.
- **Import completion**: a failed skill-list refresh right after a successful import no longer hides the success toast or leaves the import dialog open.
- **Stale `.gitignore` blocks**: removing a project's last tool now strips the Skills Hub block from `.gitignore` / `.git/info/exclude` instead of leaving it behind.
- **`.gitignore` ordering** is now applied consistently by the backend regardless of how tools were toggled.
- **Settings write race**: rapid changes to settings no longer overwrite each other.
- **Sync-engine errors** (target already exists, permission denied) are recognised by type rather than by matching message text, so they survive localisation and platform differences.

### Changed

- **Typed IPC end to end**: `tauri-specta` (`=2.0.0-rc.25`) replaces `ts-rs` as the single generator of `src/bindings/index.ts`, which now carries every DTO plus one typed function per command; the frontend seam `invokeTauri(name, ...args)` is generic over that table, so a wrong command name or argument fails the build.
- **Structured error contract**: commands return a tagged `CommandError` enum instead of prefixed strings; all user-facing copy lives in the frontend catalog (see `docs/adr/0001-tagged-command-error-contract.md`).
- **Backend-owned sync fan-out**: syncing skills to tools is one batch command with streamed progress and per-target results, replacing the previous per-pair loops.
- **Settings** are served by a typed policy module (defaults, bounds and clamping live in one place); fourteen get/set commands collapsed to two.
- **Tool catalog**: each supported tool is one registry record carrying its directories, group membership and symlink capability; Cursor's copy-only mode is now a registry fact rather than a special case.
- **Global Tool selection** persisted under the settings module; legacy values migrate transparently.
- Backend comments and diagnostics are English-only; all locales live in the frontend.

## [1.1.9] - 2026-07-12

### Added

- **Global Configure Tools modal**: Configure tool-level deployment for all skills from a single modal on the My Skills page.
- **Assignment matrix reflects global deployment**: The project assignment matrix now shows skills deployed globally at the tool level.

### Fixed

- **My Skills search**: Search now supports wildcard matching on skill and repo names.

### Changed

- **Dependency upgrades**: TypeScript 6 (with TS7 RC for builds), ESLint 10, Vite 8, i18next 26, lucide-react 1.x, rusqlite 0.39, git2 0.21, plus transitive security fixes.

## [1.0.0] - 2026-04-09

### Added

- **Per-project skill distribution**: Register project directories, assign specific skills to specific projects, and sync via symlinks so AI tools only load relevant skills per project.
- **Project management UI**: Full project CRUD with assignment matrix, tool configuration, and sync status.
- **Linux x86_64 release**: `.deb` and `.AppImage` installers with auto-update support.

### Changed

- **App identifier**: Rebranded from `com.qufei1993.skillshub` to `com.skillshub.app` (fork-friendly, generic). Existing databases auto-migrate via legacy identifier detection.
- **Upstream URLs**: All functional references (updater endpoint, release notes, featured skills catalog) now point to `astarktc/skills-hub`.
- **Updater signing key**: New signing keypair for release artifact verification.

## [0.4.2] - 2026-04-06

### Fixed

- **New tools modal style**: "New tools detected" dialog now uses consistent header/footer structure (`modal-header` + `modal-footer`) matching all other modals, fixing missing padding and border separators ([#46](https://github.com/qufei1993/skills-hub/issues/46)).
- **Git skill name derivation**: Installing a Git skill from a repo root (subpath `"."`) now correctly derives the name from the repository URL instead of using `"."` as the display name.

## [0.4.1] - 2026-03-21

### Added

- **Frontmatter metadata table**: Markdown files with YAML frontmatter now render a GitHub-style metadata table at the top of the skill detail view.

## [0.4.0] - 2026-03-20

### Added

- **In-app update check**: Check for updates directly within Settings, download and install without leaving the app ([#33](https://github.com/qufei1993/skills-hub/issues/33)).
- **QoderWork tool adapter**: Support for QoderWork desktop AI agent (`~/.qoderwork/skills/`) ([#34](https://github.com/qufei1993/skills-hub/issues/34)).

### Changed

- **Settings promoted to full page**: Settings moved from a modal dialog to a dedicated page view, consistent with My Skills / Explore navigation pattern.
- **Curated skills aggregation**: Explore page now sources skills from a curated list of 7 high-quality repositories.

### Fixed

- Language toggle briefly flashing "Installing Skills..." loading overlay on Explore page.

## [0.3.0] - 2026-03-15

### Added

- **Explore page**: Explore promoted from a modal tab to an independent page with My Skills / Explore top-level navigation.
- **Featured skills**: Explore page displays curated skills from ClawHub API (updated daily via GitHub Actions) with frontend filtering and one-click install.
- **Online skill search**: Real-time search via skills.sh API (triggered at 2+ characters, 500ms debounce), results deduplicated against the featured list and shown in separate sections.
- **Skill detail view**: Click a skill name to browse its files with a file tree, Markdown rendering (GFM + frontmatter stripping), and syntax highlighting (40+ languages, light/dark theme adaptive).
- **Skill description field**: Description extracted from SKILL.md frontmatter at install time, stored in database, and displayed on My Skills cards.
- **GitHub Token setting**: Optional GitHub Token input in settings to increase API rate limit from 60 to 5,000 requests/hour.
- **MoltBot tool adapter**: Added standalone MoltBot tool support after OpenClaw rename/split.

### Fixed

- Git install deriving skill name as "skills" when URL points to a `skills/` subdirectory, causing duplicated sync paths ([#28](https://github.com/qufei1993/skills-hub/issues/28)).
- GitHub API rate-limit errors now display the exact reset time instead of a generic message.
- Windows "Access Denied" OS error 5 when syncing to tools ([#20](https://github.com/qufei1993/skills-hub/issues/20)).
- Git repo directory structures not correctly recognized as skills ([#18](https://github.com/qufei1993/skills-hub/issues/18), [#8](https://github.com/qufei1993/skills-hub/issues/8)).
- Repos using `.claude/skills/` directory format not detected ([#27](https://github.com/qufei1993/skills-hub/issues/27)).
- OpenClaw path updated from `.moltbot/skills` to `.openclaw/skills` ([#29](https://github.com/qufei1993/skills-hub/issues/29)).

### Changed

- My Skills list: tool badges now only show synced tools, collapsing to `+N more` beyond 5.
- Manual Add modal simplified to Local Directory / Git Repository tabs only (Explore tab removed).
- Multi-skill repo online install now auto-matches target skill (exact → unique-contains → fallback to manual picker).

## [0.2.0] - 2026-02-01

### Added

- **Windows platform support**: Full support for Windows build and release (thanks @jrtxio [PR#6](https://github.com/qufei1993/skills-hub/pull/6)).
- Support and display for many new tools (e.g., Kimi Code CLI, Augment, OpenClaw, Cline, CodeBuddy, Command Code, Continue, Crush, Junie, iFlow CLI, Kiro CLI, Kode, MCPJam, Mistral Vibe, Mux, OpenClaude IDE, OpenHands, Pi, Qoder, Qwen Code, Trae/Trae CN, Zencoder, Neovate, Pochi, AdaL).
- UI confirmation and linked selection for tools that share the same global skills directory.
- Local import multi-skill discovery aligned with Git rules, with a selection list and invalid-item reasons.
- New local import commands for listing candidates and installing a selected subpath with SKILL.md validation.

### Changed

- Antigravity global skills directory updated to `~/.gemini/antigravity/global_skills`.
- OpenCode global skills directory corrected to `~/.config/opencode/skills`.
- Tool status now includes `skills_dir`; frontend tool list/sync is driven by backend data and deduped by directory.
- Sync/unsync now updates records across tools sharing a skills directory to avoid duplicate filesystem work and inconsistent state.
- Local import flow now scans candidates first; single valid candidate installs directly, multi-candidate opens selection.

## [0.1.1] - 2026-01-26

### Changed

- GitHub Actions release workflow for macOS packaging and uploading `updater.json` (`.github/workflows/release.yml`).
- Cursor sync now always uses directory copy due to Cursor not following symlinks when discovering skills: https://forum.cursor.com/t/cursor-doesnt-follow-symlinks-to-discover-skills/149693/4
- Managed skill update now re-syncs copy-mode targets using copy-only overwrite, and forces Cursor targets to copy to avoid accidental relinking.

## [0.1.0] - 2026-01-25

### Added

- Initial release of Skills Hub desktop app (Tauri + React).
- Central repository for Skills; sync to multiple AI coding tools (symlink/junction preferred, copy fallback).
- Local import from folders.
- Git import via repository URL or folder URL (`/tree/<branch>/<path>`), with multi-skill selection and batch install.
- Sync and update: copy-mode targets can be refreshed; managed skills can be updated from source.
- Migration intake: scan existing tool directories, import into central repo, and one‑click sync.
- New tool detection and optional sync.
- Basic settings: storage path, language, and theme.
- Git cache with cleanup (days) and freshness window (seconds).

### Build & Release

- Local packaging scripts for macOS (dmg), Windows (msi/nsis), Linux (deb/appimage).
- GitHub Actions build validation and tag-based draft releases (release notes pulled from `CHANGELOG.md`).

### Performance

- Git import and batch install optimizations: cached clones reduce repeated fetches; timeouts and non‑interactive git improve stability.
