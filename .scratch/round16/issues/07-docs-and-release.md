# 07 — Docs, version, release 1.2.16

Status: ready-for-agent
Blocked by: 01, 02, 03, 04, 05, 06 (and the review fixes)
Spec: `.scratch/round16/spec.md`. Orchestrator ticket.

## Work

- **CHANGELOG** `[1.2.16]`: Added — Change source for every managed skill (git ↔ local, imported → either);
  bulk unassign; Add flow flags Tool-dir folders at listing. Changed — toggle-on / bulk assign / resync report
  per-assignment outcomes with toasts. Internal — `ProjectSyncReport` replaces `ResyncSummary` and the bulk
  assign DTO; `RemovalScope::ProjectSkill`; `acquire` intent narrowed; old `bulk_assign_*` tests retired;
  `GIT_REPOINT_REQUIRES_GIT` retired.
- **CONTEXT.md**: **Re-point** (one operation, two targets, every provenance, D4 rules; Unlocatable repair is
  the same door), **Unlocatable skill** (repair wording), **Provenance** (imported is leavable), **Project sync
  report** (new term if the language section needs it — check the round-15 wording for fan-out reports).
- **ADR-0003** amendment block (D4): imported is a provenance a skill can leave by Re-point; re-entered only by
  Detach; the "no external source" statement is about the imported state, not a permanent property of the skill.
- **AGENTS.md**: fan-out bullet list — add project sync (`toggle`-on / bulk assign / resync →
  `ProjectSyncReport`) and bulk unassign (`RemovalReport`); the "three project mutations cannot answer with a
  view" paragraph — `resync_all_projects` now returns `{ report, projects }`; Re-point sentence under skills
  world ("either Re-point" → "Re-point").
- `npm run version:set 1.2.16`; `npm run version:check && npm run check`; `git fetch && git rebase origin/main`
  (take origin's `featured-skills.json`); push main = release. Watch `release.yml`: five assets, `.sig`s,
  `updater.json`.
