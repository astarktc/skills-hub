# 10 — Docs, CHANGELOG, `version:set 1.2.18`, release

Status: ready-for-agent
Blocked by: 01, 02, 03, 04
Spec: `.scratch/round18/spec.md` — ticket 10.

## Work

- `CHANGELOG.md` 1.2.18: Augment global dir fix; GitHub token moved to the OS keychain (one-time migration);
  Chinese locale removed; dev-only fixture browser mode (`npm run dev:fixture`); `PRODUCT.md` added.
- `AGENTS.md`: Commands gains `dev:fixture`; Environment gotchas gains "fixture mode is the only way to screenshot
  the app without touching the operator's library; `preview_*` needs a served URL, never `file://`".
- `npm run version:set 1.2.18`; `npm run version:check && npm run check`; commit; `git fetch && git rebase
  origin/main` (take origin's `featured-skills.json` on conflict); push; confirm `auto-tag.yml` → `release.yml`
  green with five targets. Record the sha and tag here. Orchestrator commits and pushes — the child prepares.
