# Quick Task 260416-nwg: Cherry-pick upstream commits and manually port improvements - Context

**Gathered:** 2026-04-16
**Status:** Ready for planning

<domain>
## Task Boundary

Cherry-pick clean upstream commits from qufei1993/skills-hub (Copaw adapter, git install/frontmatter fixes, container path fixes, featured-skills update, docs) and manually port isolated improvements (Hermes adapter, window size bump, overwriteIfSameContent, project_relative_skills_dir mapping) onto our branch.

</domain>

<decisions>
## Implementation Decisions

### Conflict Resolution Strategy

- Use theirs-first, ours-on-top approach for installer.rs conflicts
- Cherry-pick upstream's commits (6e8e733, 1826e2a) which add sparse git checkout and container path fixes
- Then re-apply our skill-lock enrichment and multi-skill detection code on top

### Manual Port Scope

- YES: Hermes Agent adapter (tool_adapters/mod.rs only, ~15 lines)
- YES: Window size 960x680 (tauri.conf.json, 1-line)
- YES: overwriteIfSameContent param on sync_skill_to_tool (~20 lines in commands/mod.rs)
- YES: project_relative_skills_dir() mapping (~60 lines in tool_adapters/mod.rs) — fixes bug where our project_sync.rs uses global relative_skills_dir for project paths, wrong for ~12 tools
- NO: macOS close-to-hide behavior (user preference)

### Doc/Changelog Handling

- Include upstream's doc commits (Copaw release notes, v0.4.3 bugfix notes, README updates)

### Claude's Discretion

- featured-skills.json: take latest version only (effe079), not all 9 intermediate bot commits

</decisions>

<specifics>
## Specific Ideas

Cherry-pick order (chronological, clean commits first):

1. e58cb56 - feat: support copaw (tool_adapters only)
2. 99c9b9e - docs: add Copaw to README
3. c882fa0 - docs: add Copaw to README.zh.md
4. 6da08dc - docs: v0.4.3 release notes for Copaw
5. 97489f7 - docs: contributor PR link
6. 6e8e733 - fix: git skill install and frontmatter rendering (CONFLICTS EXPECTED in installer.rs)
7. a5b4ffe - docs: v0.4.3 bugfix notes
8. 1826e2a - fix: git skill discovery for container paths (CONFLICTS EXPECTED in installer.rs)
9. effe079 - chore: update featured-skills.json (latest)

Then manual ports (after conflicts resolved):

- Hermes Agent adapter
- Window size 960x680
- overwriteIfSameContent
- project_relative_skills_dir() + update project_sync.rs to use it

Excluded commits: 00c41cc, 160b7c1, 8df1106 (project scope sync), 4a79ceb, 50a66e7 (version/changelog for their numbering), 827b878 (Hermes entangled with scope code in App.tsx), eba7809 (depends on macOS hide), 61998f6 (project sync README), fabf493 (CLAUDE.md rewrite), 2ff00e0, 1857f22, 55d1de8 (merge commits), 7604df0 (their version bump)

</specifics>

<canonical_refs>

## Canonical References

- Upstream comparison: https://github.com/astarktc/skills-hub/compare/main...qufei1993:skills-hub:main
- Upstream v0.5.0 release: https://github.com/qufei1993/skills-hub/releases/tag/v0.5.0
- Upstream remote: `upstream` pointing to https://github.com/qufei1993/skills-hub.git

</canonical_refs>
