# Milestones

## v1.1.8 — Polish, Features & Stability (2026-04-09 to 2026-05-09)

**Theme:** Post-ship refinement — UI polish, install reliability, Explore page features, performance optimizations, and tool adapter expansion.

**Scope:** 23 quick tasks across releases v1.1.0–v1.1.7, plus the v1.1.8 bugfix release. No formal phases — all work was ad-hoc quick tasks and one bugfix PR with atomic commits.

**Key deliverables:**

- Rebranding (com.skillshub.app, astarktc URLs)
- Group-by-repo UI with persistence
- Multi-skill repo install fixes
- Performance caching (source hashes, assignment lookup)
- Explore page: detail viewer, auto grid, hide skills, preview
- View mode toggle (list/auto grid/dense grid)
- UI scaling/zoom with Tauri native support
- AgentsStandard virtual ToolId for .agents/skills
- v1.1.8 bugfixes: unify installer download into `fetch_skill_files`; resolve multi-skill repos in explore preview by name; fix self-deadlock in `clone_for_explore_preview`

**Artifacts:** [milestones/v1.1.7-quick-tasks.md](milestones/v1.1.7-quick-tasks.md) (quick-task index)

**Git tag:** v1.1.8

---

## v1.0 — Per-Project Skill Distribution (2026-04-07 to 2026-04-09)

**Theme:** Core milestone — data layer, sync engine, IPC, frontend, and edge case handling for per-project skill distribution.

**Scope:** 6 phases, 12 plans, 2-day execution.

**Key deliverables:**

- Schema V4-V6 migrations (projects, project_tools, project_skill_assignments)
- Project-aware sync with symlink/junction/copy fallback
- 13 new Tauri IPC commands
- Full Projects tab UI with assignment matrix
- .gitignore prompt, stale path detection, orphan cleanup

**Artifacts:** [milestones/v1.0-ROADMAP.md](milestones/v1.0-ROADMAP.md) | [milestones/v1.0-REQUIREMENTS.md](milestones/v1.0-REQUIREMENTS.md) | [milestones/v1.0-MILESTONE-AUDIT.md](milestones/v1.0-MILESTONE-AUDIT.md) | [milestones/v1.0-phases/](milestones/v1.0-phases/) (archived phase artifacts)

**Git tag:** v1.0
