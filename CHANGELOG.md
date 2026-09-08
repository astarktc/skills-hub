# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

## [1.2.6] - 2026-09-08

Updates that cannot half-succeed, git skills on branches with slashes in their name, a repair that leaves nothing behind, and an invocation badge on every skill.

### Added

- **Every skill card shows who can invoke it.** The invocation badge is now icon-only and always present: a person and a robot for the default (both), a person for user-only, a robot for model-only, a crossed eye for neither. Hover for the name and the explanation.
- **Branch names containing `/`.** A GitHub URL such as `…/tree/feature/x/skills/foo` is resolved against the repository's branches (longest match wins) instead of being split at the first slash; Refresh, Update and Re-point reuse the recorded path and make no extra request.

### Changed

- **Update, Restore and Re-point replace the library copy atomically.** The previous copy is kept aside until the new bytes and the record are both written; if either step fails the old copy is restored. If that restore itself fails, the error names the retained backup so it can be recovered by hand.
- **Re-point honours auto-sync like Update does**: with auto-sync on, a re-pointed skill is also synced to any installed tool it was missing from. Recorded in the glossary under *Re-point*.
- **Symlinks inside a skill folder are not content**: they are neither copied nor hashed, so a copy of a skill compares equal to its original. Onboarding import no longer reports such an original as divergent from its own copy.
- **Warning toasts can be closed**, and the *Re-point* dialog stays open when the URL is rejected so it can be corrected.

### Fixed

- **A "skill not found" panel action for a skill deleted since** now says the skill is no longer in the library instead of opening a dialog that fails.
- **Re-point of a skill moved deeper in a repository** (e.g. `foo` → `skills/foo`) no longer mis-reads the branch.
- **A URL whose branch consumes the whole path** (`…/tree/feature/x`) still discovers the skill by name, like `…/tree/main`.
- **The detail view of a skill removed underneath it** falls back to the library instead of an empty page.

## [1.2.5] - 2026-09-08

A skill whose upstream repository moved it is repaired in place instead of removed and re-added, and a skill published under several per-tool aliases previews and installs as the one skill it is.

### Added

- **Re-point a git skill at its new GitHub URL.** When a Refresh reports *Skill not found on GitHub*, the failure row (and its toast) offers **Re-point**; every git skill's card and detail view offer it too. Paste the skill's new `/tree/` or `/blob/` URL — the repository may differ as well as the folder — and the app fetches from it first; only when that succeeds does it change the recorded source and update every synced copy, so the record, its tool links and its project assignments survive a repository reshuffle. A typo, a 404 or an ambiguous repository leaves the skill exactly as it was.
- **Too deep a chain of upstream symlinks is its own message** instead of an internal error.

### Changed

- **Onboarding import decides "identical copy" against the finished library copy**, at the moment it syncs, rather than from the scan taken earlier. A copy edited between the scan and the import is kept and reported as divergent instead of being overwritten.
- **Glossary**: *Re-point* is one repair over two provenances (local: pick the folder; git: paste the URL); *Skill discovery* records that symlink aliases of one directory are one candidate; *Git acquisition* records that the API fast path follows an upstream link only at the end of the path.

### Fixed

- **Explore preview of a repository that publishes one skill under several aliases** (e.g. `skills/<name>` plus `.claude/skills/<name>` and `.codex/skills/<name>` symlinks to it) no longer fails with "this repository contains multiple skills": aliases collapse into the real directory, and a repository whose only skill sits in a subfolder previews that skill rather than the repository root.
- **A project assignment whose skill row could not be located** now reports a typed not-found error instead of crashing the command.

## [1.2.4] - 2026-09-05

Refresh failures you can act on: skills whose upstream publishes them as symlinks refresh again, skills taken over from a tool no longer pretend to have a source, and what the app cannot locate is shown on the card with the actions that fix it.

### Added

- **Unlocatable skills are marked on their card** the moment the list loads: *Source folder missing* (a local skill whose folder is gone) offers **Re-point**, **Detach** and **Remove**; *Central copy missing* offers **Restore** and **Remove**. Refresh (all) skips such skills and reports how many, as a warning with one row per skill in the notification history, instead of failing them forever.
- **Imported skills read "Managed here · Imported from <tool>"** and offer no Update: a skill taken over from a tool's skills directory has no external source — the Skills Hub library is its source of truth (ADR-0003).
- **Adding a local folder that already lives inside a tool's skills directory is refused** with a message pointing at Import, so a skill can no longer be created with a source the app would later overwrite.

### Changed

- **Skills published as in-repo symlinks install and refresh** (e.g. an aggregation bundle such as `plugins/<all>/skills/<name>` linking to the real skill). The link is followed at fetch time on both the sparse-clone and the GitHub API paths; the recorded subpath stays the alias you chose. A link that points outside the repository is refused.
- **Onboarding import takes over every byte-identical copy of a skill**, not only the one you picked: a real directory in one tool with another tool's symlink into it ends with both tools linked to the library copy. The default choice prefers a real directory over a link. Divergent copies are still left in place and reported.
- **Missing-path failures are their own messages**: a source folder, a central copy or a repository subpath that is not there is reported as such, with the path shown on its own line — no more raw errors carrying internal cache paths.
- **Notification history reads a batch top-down** in the order it happened; copying a path shows a toast but is no longer recorded in the history; up to five toasts are visible at once.
- **"Open log folder"** failures are reported with their own message. On Linux and Windows the log folder is now opened directly (macOS still reveals it, because its name ends in `.app`).

### Fixed

- **Skills imported from a tool in earlier versions are recognised on first launch** and reclassified as imported, so the rows that failed every Refresh ("source path not found") stop failing without any action. Only sources that were a tool's own skills directory (under your home, resolving into the library, or a migrated Windows/WSL path) are touched; a folder of your own is never reclassified, and every change is logged with the former path.
- **Database schema version is now recorded after incremental migrations** (it was not, so the previous migration re-ran on every launch) and the whole upgrade runs in one transaction.
- **A Propagation row whose tool was shadowed in tests produced no outcome**; a row's own tool is now always in its shared-directory group.
- **The Refresh overlap test no longer depends on wall-clock time** (it flaked under parallel load).

### Internal/architecture

- **Provenance** (git / local / imported) and **Unlocatable skill** join the glossary; ADR-0003 records that an imported skill has no external source. `provenance::refresh_eligibility` is the one Refresh membership rule (member / not a member / unlocatable → skipped); `is_refreshable` is its Update/listing half.
- **One subpath normaliser and one link resolver** (`core/repo_subpath.rs`) serve the git cache, the sparse fetcher and both acquisition adapters; `..` and backslash segments are refused before anything is read.
- **The registry answers "which tool holds this path"** (`tool_holding_path`), the inverse of its deletion rule; Add's refusal and legacy reclassification both ask it.
- **Reporter world**: `useNotificationHistory` owns the ring; one `copyToClipboard`; `notifyError`/`formatError` travel from the binder instead of components re-formatting errors; Tool labels and the imported-source line live once in `skillPresentation.ts`.
- **Project sync** resolves adapter and skill once per assignment (`AssignmentSyncContext`); a log-reveal rule is a pure, tested core function.

## [1.2.3] - 2026-09-04

A follow-up round: the outcome of every action is now readable after the fact, and the review-panel residue from 1.2.2 is settled.

### Added

- **Notification history**: a bell in the app header opens the session's notifications (errors, warnings, successes) with an unread badge for errors and warnings, per-entry copy and *Copy all*, and *Clear*. In memory for the session; earlier runs are in the backend log.
- **Open log folder** in Settings reveals the app's log file in the file manager.

### Changed

- **Error toasts stay until you close them**; warnings linger five seconds; successes and confirmations flash. A refresh, unsync or import that finished with failures is reported as a warning (it lingers and counts as unread), not a success, and every failure of a batch is its own row in the history even when the screen shows one toast.
- **Onboarding import always overwrites the source Tool's original**: with auto-sync on, the Tool the chosen variant was found in is synced even when it is deselected in the auto-sync selection, so the original becomes the managed copy instead of an untracked duplicate. The completion toast names the Tool and why.
- **Adding a skill from a non-GitHub repository clones it once**: the listing's clone serves the install within the cache window, so the second clone (and its wait) is gone.

### Fixed

- **Typed removal errors**: a failure while removing a skill's folder from a tool (unsync, delete, unassign, project removal) is reported with its own localized message instead of a generic error.
- **Import completion explanation is readable**: the "also synced to …" lines render as separate lines in the toast and the history rather than running into the title.
- **Project removal that fails** re-reads the project view, like the other project actions, so the list shows what is actually on disk.
- **`npm run version:set`** rewrites both lockfiles as well as the three manifests, and `version:check` verifies all five, so a bump no longer dirties the next `cargo test` or `npm ci`.

### Internal/architecture

- **Notification** joins the glossary; the reporter (`useStatusReporter`) is the single owner of toast lifetime and the only writer of the history — every direct toast call outside it is gone.
- **Git cache keyed by repository and ref**, never the subpath: the checkout shape is entry metadata and an entry is only ever widened (a clone without a readable record is treated as full); the key digest is pinned.
- **Removal reports carry the error chain** (`anyhow::Error`, as Propagation does) so typed signals survive to the command seam; callerless `*_unlocked` twins and blind bulk target deletes are deleted; project-side removal entry points are named after Artifact removal.
- **One rule each**: the shared-skills-dir group is the registry's answer everywhere; `assignment_artifact_name` is the one naming rule for a project assignment's artifact (the stored name, never the live one); one function syncs one assignment.

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
- **My Skills shows a sync target in error**: a tool pill whose target failed removal or update is now marked and explained, matching the assignment matrix (ADR-0002).
- **A synced copy Skills Hub cannot inspect is no longer forgotten**: if the check for whether a skill's folder is still on disk fails outright (an unreadable parent directory, an I/O error) rather than reporting it absent, the entry is now kept and marked `error` instead of being dropped as already-removed.
- **A refresh that cannot re-check auto-sync says so**: when re-asserting auto-sync fails for a skill, refresh now reports it per skill and counts it, instead of only writing to the log while presenting the skill as fully refreshed.
- **Parallel refresh no longer fails a skill on a busy database**: store connections wait for a concurrent write to finish instead of giving up immediately.

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
