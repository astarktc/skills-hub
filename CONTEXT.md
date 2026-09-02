# Skills Hub

Desktop app that installs AI Agent Skills once and syncs them to many AI coding tools. This glossary is the canonical vocabulary; terms crystallise here as design decisions land (see `docs/adr/`).

## Language

**Tool**:
An AI coding tool that Skills Hub syncs skills into (Claude Code, Cursor, …), identified by a stable key. Every fact about a tool (its dirs, group membership, capabilities) lives in its single registry record (`ToolAdapter` in `core/tool_adapters/`).
_Avoid_: adapter (that's the code object serving a tool), client, IDE

**Tool capability**:
A per-tool fact that changes how sync behaves for that tool, recorded on its registry record rather than tested by name in sync code (today: `supports_symlink` — Cursor's own `~/.cursor/skills` dir did not follow symlinks, so syncs to that entry are always copied). A capability describes one registry entry: a virtual group carries its own capability and does not inherit its constituents'.
_Avoid_: special case, tool quirk, Cursor mode

**Virtual group**:
A tool entry that stands in for several tools sharing one project-scope skills convention (today: the AGENTS standard's `.agents/skills`). It appears in tool lists in place of its constituent tools and syncs by its own capability (symlink-capable), whatever its constituents' individual capabilities are.
_Avoid_: meta-tool, umbrella tool

**Constituent tools**:
The tools absorbed into a virtual group entry. The group counts as installed when any constituent is; group membership is owned by the backend and only presented by the frontend.
_Avoid_: member tools, sub-tools

**Shared skills dir group**:
Global tools whose skills directories resolve to the same location, so syncing to one member syncs to all. Owned by the backend; the frontend only presents it.
_Avoid_: dir alias, linked tools

**Tool catalog**:
The presentation-ready tool list for one scope (global or project) with installedness, shared-dir groups and virtual-group constituents already resolved against the operator's home. Assembled by the backend (`tool_adapters::global_tool_entries` / `project_tool_entries`); commands only map it to DTOs.
_Avoid_: tool status (that's the DTO carrying a catalog), tool list

**Managed skill**:
A skill installed through Skills Hub and tracked in its database, eligible for sync fan-out to tools.
_Avoid_: installed skill (ambiguous with a tool being installed)

**Sync target**:
One (skill, tool) pair a sync batch attempts; each target resolves to synced, skipped, or failed as report data, never as a command error.
_Avoid_: sync pair, destination

**Staging dir**:
A scratch directory inside the central repo that an install or update flow fills with a skill's bytes before the finalize step moves it into place; discarded automatically if the flow fails first.
_Avoid_: temp dir, download dir

**Finalize (install)**:
The single last mile that turns a staging dir into a managed skill: final-name choice (SKILL.md `name` beats a derived name; an operator-provided name always wins), typed collision check, move, description + content hash, record upsert. Flows acquire bytes; only finalize records them.
_Avoid_: register, materialise

**Skill discovery**:
The single scan ladder (`core/skill_discovery.rs::discover_skills`) that turns a directory tree into skill candidates: root `SKILL.md`, known scan bases, root-level skills/containers, `marketplace.json` plugins, then a depth-5 walk; one dedup by subpath, one sort by name. Every candidate carries a validity (valid / invalid `SKILL.md` with reason / missing `SKILL.md`); listings and the update backfill are adapters that decide what to admit.
_Avoid_: scan, collect skill dirs

**Skill candidate**:
One discovered directory with its subpath, name, description and validity. "Installable" means it has skill bytes (any `SKILL.md`, or a `.claude/skills/` child) — the git side's admission rule; "valid" means the manifest parsed — the local picker's rule.
_Avoid_: skill entry, hit

**Sync status**:
The lifecycle of one synced artifact (a project assignment row or a global skill target row): `pending` → `synced` / `stale` / `missing` / `error`. A typed enum (`SyncStatus`, `core/sync_status.rs`) whose stored and wire spelling are those strings; the store parses it at its seam (legacy `ok` reads as `synced`; an unrecognised value surfaces as `error` with the raw value as diagnostic — the store never coerces it to healthy). Status changes are typed transitions (`AssignmentTransition`), and the "what should it be" decision (`next_status`) is pure so a reconcile pass can plan before it writes. The reconcile pass (run by the project listing) may re-derive any row's status from what it observes on disk — source/target presence and, for copies, the content hash — and write the canonical string; that is how a legacy or `error` row recovers, and it is grounded in observation, not in the stored value.
_Avoid_: state, health, "ok"

**Sync mode**:
How an artifact was materialised — `symlink`, `junction` (Windows fallback) or `copy` — recorded on the row because only copies can drift from their source (`SyncMode`, same module).
_Avoid_: link type, strategy

**Project sync status**:
The precedence fold of a project's assignment statuses shown on the project list: error/missing > stale > pending > synced, `none` when the project has no assignments (`ProjectSyncStatus`, `aggregate`).
_Avoid_: project health, overall status
