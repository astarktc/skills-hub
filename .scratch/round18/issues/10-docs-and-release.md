# 10 — Docs, CHANGELOG, `version:set 1.2.18`, release

Status: implemented
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

## Result

Status: implemented (release building)

- CHANGELOG 1.2.18 block and AGENTS.md gotcha: `3d31fbc`. Version bump: `5218b12` (`npm run version:set 1.2.18`, gate green:
  version:check OK, vitest 17/449, cargo test --all, clippy, fmt, build). Pushed to origin/main as `ae7b8a7` (head) on 2026-09-27;
  `auto-tag.yml` created **`v1.2.18`** and dispatched `release.yml` (run 36358980829). CI run 36358972230.
- Reviews closed before the push: ticket 02 — Astra ×3 (fix-then-ship → fix-then-ship → **ship** at `4f6e312`); ticket 04 — Astra
  (fix-then-ship, no production-safety blocker) → fixes in `5fbc3c5`; 01/03/05/08 orchestrator read.
- Operator smoke (closure checklist): install the v1.2.18 build; expect the Keychain prompt once (Always Allow); Settings shows
  "A token is saved"; `github_token` row gone from `skills_hub.db` (a `\*.cleanup_pending` marker must not remain); `~/.augment/skills`
  holds the relocated targets and `~/.augment/rules` is empty of Skills Hub artifacts; no ZH option.
