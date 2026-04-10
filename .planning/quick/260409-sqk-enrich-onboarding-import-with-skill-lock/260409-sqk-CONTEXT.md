# Quick Task 260409-sqk: Enrich onboarding import with .skill-lock.json git provenance and add ~/.agents/skills as a tool directory - Context

**Gathered:** 2026-04-10
**Status:** Ready for planning

<domain>
## Task Boundary

Enrich the onboarding import flow so that skills originally installed via `npx skills add` (which stores canonical copies in `~/.agents/skills/` and symlinks into tool directories) are imported with full git provenance instead of `source_type: "local"`.

The `npx skills` CLI maintains `~/.agents/.skill-lock.json` with per-skill metadata: source repo URL, subpath within repo, content hash, and timestamps.

</domain>

<decisions>
## Implementation Decisions

### Task Scope

- Lock file enrichment only — read `~/.agents/.skill-lock.json` during import, upgrade `source_type` from `"local"` to `"git"` with full repo URL/subpath
- Backend-only change, no frontend modifications needed
- Adding `~/.agents/skills` as a new `ToolId` adapter and project-level agent dir are deferred to separate tasks

### Lock File Matching Strategy

- Use **symlink target resolution**: when a detected skill in a tool dir is a symlink pointing into `~/.agents/skills/<name>`, resolve the link target, confirm it's under `~/.agents/skills/`, then look up `<name>` in the lock file
- This gives zero false positives — only skills that are genuinely managed by the `npx skills` CLI will be enriched
- Do NOT fall back to name-only matching

### Duplicate Handling

- No changes to existing dedup logic — the current onboarding already collapses multiple tool dirs with symlinks to the same skill into one group by name
- Enrichment just upgrades the `source_type` on whichever variant gets imported

### Claude's Discretion

- Lock file parsing: use serde for JSON deserialization, keep the parser minimal (only extract fields we need)
- Lock file absence: if `~/.agents/.skill-lock.json` doesn't exist, silently skip enrichment (no error, no warning)
- Lock file format changes: parse defensively — if the structure doesn't match expected format, skip enrichment

</decisions>

<specifics>
## Specific Ideas

- Lock file structure (version 3): `{ "version": N, "skills": { "<name>": { "source": "owner/repo", "sourceType": "github", "sourceUrl": "https://github.com/owner/repo.git", "skillPath": "path/to/SKILL.md", "skillFolderHash": "...", ... } } }`
- The `skillPath` field contains the path to SKILL.md within the repo — the parent directory of this path is the `source_subpath`
- The `sourceUrl` maps directly to Skills Hub's `source_ref` field
- The onboarding flow already has `is_link` and `link_target` on `DetectedSkill` via `detect_link()` in `tool_adapters/mod.rs`

</specifics>

<canonical_refs>

## Canonical References

- `~/.agents/.skill-lock.json` — the npx skills CLI manifest file
- `src-tauri/src/core/installer.rs` — `install_local_skill()` where `source_type` is set
- `src-tauri/src/core/onboarding.rs` — onboarding detection and plan generation
- `src-tauri/src/core/tool_adapters/mod.rs` — `detect_link()` for symlink resolution
- `src-tauri/src/commands/mod.rs` — `import_existing_skill` Tauri command

</canonical_refs>
