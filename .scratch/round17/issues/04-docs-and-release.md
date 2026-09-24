# 04 — Docs, version, release 1.2.17

Status: ready-for-agent
Blocked by: 01, 02, 03
Spec: `.scratch/round17/spec.md`. Orchestrator ticket.

## Work

- **CHANGELOG** `[1.2.17]`: Fixed — "Only scan for existing skills within selected tools" now scopes Review &
  Import; a tool directory holding nothing but a skills folder (a skill deployer's footprint) no longer counts
  as an installed tool. Added — syncing onto an existing folder with different content asks before overwriting;
  identical content is still replaced silently.
- **CONTEXT.md**: the installedness definition (footprint rule + group exemption); the overwrite ask under
  the sync vocabulary if the language section names the same-content rule.
- **AGENTS.md**: the sync fan-out bullet — the seam owns the overwrite ask; installedness pointer under the
  adapter invariant if it states "detect dir exists".
- `npm run version:set 1.2.17`; `npm run version:check && npm run check`; `npx eslint src` tail for hook
  warnings; `git fetch && git rebase origin/main` (take origin's `featured-skills.json`); push main = release.
  Watch `release.yml`: five assets, `.sig`s, `updater.json`.

## Comments
