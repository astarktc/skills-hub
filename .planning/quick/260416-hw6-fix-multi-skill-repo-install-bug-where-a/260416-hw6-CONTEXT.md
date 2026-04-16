# Quick Task 260416-hw6: Fix multi-skill repo install bug - Context

**Gathered:** 2026-04-16
**Status:** Ready for planning

<domain>
## Task Boundary

Fix the bug where installing any individual skill from a multi-skill repo (e.g., better-auth/skills) via the Explore page always installs the wrong skill. All six skills from that repo resolve to the same name ("better-auth-security-best-practices") instead of their actual name from their own SKILL.md frontmatter.

</domain>

<decisions>
## Implementation Decisions

### Name detection strategy

- SKILL.md frontmatter is the canonical source of truth for skill names
- Folder name is only a fallback when SKILL.md is missing or has no valid frontmatter
- This matches how npx skills CLI works

### Install flow routing

- Always use the candidate-based flow (clone repo, scan for skills, match by name, install specific candidate)
- When user clicks "Install" on a specific skill from Explore page, the skills.sh name should be used to auto-select the correct candidate from the scan results
- Each candidate install reads its own SKILL.md from its own subdirectory
- The direct-install shortcut path (isFolderUrl) is the bug-prone path — route through candidate flow instead

### Collision handling

- Not applicable as a separate concern — once naming is fixed, each skill gets its correct unique name
- No batch install feature exists in the current UI; skills are installed one at a time from Explore

### Claude's Discretion

- Implementation details of how the candidate matching logic works (exact match vs containment)
- Whether to remove the isFolderUrl shortcut entirely or just ensure it doesn't apply to multi-skill repos

</decisions>

<specifics>
## Specific Ideas

- The repo `better-auth/skills` has structure: `security/` contains one skill, `better-auth/` contains five skills
- skills.sh API returns individual skill entries with `source_url` pointing to the repo
- The Explore page correctly shows all six skills because skills.sh provides them individually
- The bug is in the install path: after cloning the repo, the app fails to select the correct subdirectory matching the clicked skill

</specifics>

<canonical_refs>

## Canonical References

- Bug report: "Skill already exists" error when installing different skills from same multi-skill repo
- Reference implementation: `npx skills add` correctly handles this repo structure

</canonical_refs>
