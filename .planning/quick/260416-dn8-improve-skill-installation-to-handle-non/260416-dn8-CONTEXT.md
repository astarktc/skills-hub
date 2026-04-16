# Quick Task 260416-dn8: Improve skill installation to handle non-standard repo structures — Context

**Gathered:** 2026-04-16
**Status:** Ready for planning

<domain>
## Task Boundary

Improve skill installation to handle non-standard repo structures (e.g., wshobson/agents with dozens of skills but no root SKILL.md), matching npx skills CLI capabilities. Forward-looking architecture to support future skill update checking.

</domain>

<decisions>
## Implementation Decisions

### Discovery Strategy

- **Registry-first lookup** via skills.sh API to get repo structure/skill manifest
- **Deep SKILL.md scan as fallback** when repo is not in registry or for direct URL installs
- Research phase will investigate exactly how skills.sh and npx skills CLI handle discovery to align our approach

### Skill Identity Model

- Primary: SKILL.md presence in directory
- Fallback: skills.sh registry authority — if the registry lists a directory as a skill for that repo, treat it as valid even without SKILL.md
- Registry becomes the authority for repos that don't follow SKILL.md convention

### Update Architecture

- Store **repo commit SHA + per-skill content hash** at install time
- Update check flow: compare repo SHA first (cheap), then re-hash specific skill subdirectory only if repo changed
- Provides precise per-skill update detection even in mono-repos with 50+ skills
- Schema must include: source_url, source_subpath, installed_commit, content_hash

</decisions>

<specifics>
## Specific Ideas

- The `npx skills add wshobson/agents` command works perfectly with this repo — research should reverse-engineer how it discovers skills
- skills.sh and npx skills are from the same ecosystem (Vercel/agent skills) — registry and CLI are designed to work together
- Current Skills Hub already has content hashing in `core/content_hash.rs` — reuse for per-skill hash tracking
- Current Skills Hub already reads `~/.agents/.skill-lock.json` for provenance — could inform registry lookups

</specifics>

<canonical_refs>

## Canonical References

- skills.sh registry API — needs investigation during research phase
- npx skills CLI — source discovery mechanism to be researched
- Current installer: `src-tauri/src/core/installer.rs` (list_git_skills, is_skill_dir)
- Current search: `src-tauri/src/core/github_search.rs`, `src-tauri/src/core/skills_search.rs`

</canonical_refs>
