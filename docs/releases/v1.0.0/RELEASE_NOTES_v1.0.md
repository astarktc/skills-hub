# Skills Hub v1.0.0 Release Notes

**Release date:** April 9, 2026
**Platforms:** macOS (Intel & Apple Silicon), Windows (x64), Linux (x86_64)

---

## What is Skills Hub?

Skills Hub is a cross-platform desktop app for managing AI Agent Skills and syncing them to 47+ AI coding tools. Install a skill once, and it's instantly available in every tool you use — via symlink, junction, or copy fallback.

v1.0.0 is the first release under the [astarktc/skills-hub](https://github.com/astarktc/skills-hub) fork, adding **per-project skill distribution** alongside the existing global sync.

---

## What's New in v1.0

### Per-Project Skill Distribution

The headline feature: register project directories, assign specific skills to specific projects, and sync them directly into each project's tool directory. AI tools only load the skills that matter for that project — not your entire library.

**Project Management**

- Register any directory as a managed project
- Remove projects with full filesystem cleanup (synced artifacts removed)
- Missing project detection — warning badge when a registered path no longer exists, with "Update Path" to re-point without losing assignment history
- `.gitignore` integration — optionally add Skills Hub entries to `.gitignore` (shared) or `.git/info/exclude` (private)

**Tool Configuration**

- Configure which AI tools are active per project (checkbox list)
- Auto-detect installed tools and pre-select them for new projects
- Tool removal cascade — removing a tool column cleans up all synced artifacts for that tool across all assigned skills

**Skill Assignment Matrix**

- Visual checkbox grid: skills as rows, configured tools as columns
- Per-cell status indicators: green (synced), yellow (stale), red (error/missing), gray (pending)
- "All Tools" bulk-assign button per skill row
- Click error cells to retry sync
- Hover tooltips showing error details
- Content hash staleness detection for copy-mode targets

**Sync Operations**

- "Sync Project" — re-sync all assignments for a single project
- "Sync All Projects" — re-sync every assignment across all projects
- Per-assignment retry for failed syncs
- Missing status detection — surfaces when source skill is deleted or target artifact is removed
- Auto-recovery when source reappears
- `SyncMutex` serialization prevents race conditions across concurrent sync operations

### Global Sync Enhancements

- **Auto-sync toggle** — global ON/OFF switch (persisted in settings). When OFF, new skill installs go to the central repo only without deploying to tool directories. Default: ON for backward compatibility.
- **Bulk unsync** — "Uninstall from tool directories" removes all skills from all tool directories in one operation. Skills remain in the central repo.
- **Per-skill unlink** — remove a single skill from all tool directories without deleting it from the central repo.

### Skill Deletion Cleanup

- Deleting a managed skill now removes all synced artifacts from both global tool directories and project directories before the database cascade delete. No orphaned symlinks or copies left behind.

### Linux Support

- First-class Linux x86_64 release artifacts: `.deb` and `.AppImage` packages
- Auto-updater support for Linux via AppImage
- `updater.json` includes `linux-x86_64` platform entry

### Rebranding

- App identifier changed from `com.qufei1993.skillshub` to `com.skillshub.app`
- Automatic database migration for existing installs (legacy identifier in migration path)
- Updater endpoint, featured skills catalog URL, and release notes API updated to `astarktc/skills-hub`

### CI/CD Hardening

- Linux added to release matrix (`ubuntu-22.04`)
- AppImage correctly used as Linux updater artifact (not tar.gz)
- `.deb` internals excluded from updater artifact search
- New signing keypair for release artifact verification

---

## Existing Features (from upstream)

These features were already present in the original codebase and continue to work in v1.0:

- **Central skills repository** at `~/.skillshub/` with sync to 47+ AI coding tools
- **Install from** local folder or Git URL (multi-skill repo selection, `.claude/skills/` directory support)
- **Explore page** — browse featured skills from ClawHub (updated daily) + search via skills.sh API
- **Skill detail view** — file tree browser with Markdown/GFM rendering and syntax highlighting (40+ languages)
- **Onboarding migration** — scan existing tool directories, import skills, one-click sync
- **In-app updates** — startup check with notification
- **Settings** — storage path, language, theme, GitHub token for API rate limits
- **Cross-platform** — macOS, Windows, Linux

---

## Architecture Highlights

- **3 new Rust modules** with clean separation:
  - `project_ops.rs` — business logic (registration, validation, cleanup)
  - `project_sync.rs` — sync orchestration (assign, unassign, resync, staleness detection)
  - `commands/projects.rs` — 13 new Tauri IPC commands
- **Self-contained frontend** — 9 new component files under `src/components/projects/` with a `useProjectState` custom hook. Zero state leaked into App.tsx.
- **3 schema migrations** (V4 → V5 → V6) with backward-compatible incremental DDL
- **Sync engine reused unchanged** — project sync delegates to the same `sync_engine.rs` primitives as global sync

---

## Quality

- 26/26 requirements satisfied
- 22 code review findings fixed across 6 phases (0 skipped)
- 9/9 security threats audited and closed (SQL injection, path traversal, symlink safety)
- All parameterized queries (zero string interpolation in SQL)
- Path canonicalization + directory validation in core layer
- Symlink-safe removal via `symlink_metadata`

---

## Stats

| Metric                  | Value           |
| ----------------------- | --------------- |
| Source files changed    | 30              |
| Lines added/removed     | +9,568 / -1,665 |
| New Rust source files   | 3 (1,270 lines) |
| New frontend files      | 9               |
| New Tauri IPC commands  | 13              |
| Schema migrations added | 3 (V4, V5, V6)  |
| Development timeline    | 2 days          |

---

## Supported AI Coding Tools (47)

Cursor, Claude Code, Codex, OpenCode, Antigravity, Amp, Kimi Code CLI, Augment, OpenClaw, Cline, CodeBuddy, Command Code, Continue, Crush, Junie, iFlow CLI, Kiro CLI, Kode, MCPJam, Mistral Vibe, Mux, OpenClaude IDE, OpenHands, Pi, Qoder, Qwen Code, Trae, Trae CN, Zencoder, Neovate, Pochi, AdaL, Kilo Code, Roo Code, Goose, Gemini CLI, GitHub Copilot, Clawdbot, Droid, Windsurf, MoltBot, and more.

---

## Download

Available for macOS (dmg), Windows (MSI/NSIS), and Linux (deb/AppImage) from the [Releases page](https://github.com/astarktc/skills-hub/releases/tag/v1.0.0).
