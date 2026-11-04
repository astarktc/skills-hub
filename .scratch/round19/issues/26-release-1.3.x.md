# 26 — CHANGELOG, `version:set 1.3.x`, release; operator smoke

Status: ready-for-agent
Blocked by: 25
Spec: `.scratch/round19/spec.md` — D31, D32. Phase 19b release.

## Work

- CHANGELOG block for the 19b surfaces (Settings panes with Configure Tools moved, Import review, Explore rows, ⌘K,
  grid view, `App.css` gone); `npm run version:set 1.3.<n>`; `version:check && check`; rebase on origin; push =
  release; record the run ids.
- Operator smoke list (D31): VoiceOver over Settings, Import, Explore and the palette; reduced motion; 960×640.
- Then the orchestrator closes the effort per `docs/agents/issue-tracker.md` § Lifecycle (`## Closure` block in the
  spec, extract-then-archive).

## Result

(child fills in; orchestrator adds the smoke outcome and closure)

## Comments

- 2026-11-02 — Vocabulary: read Tool/Sync/Refresh as Harness/Deploy/Update — ticket 32.
- 2026-11-04 — operator grill (autonomous run through 31 → 20 ∥ 21 → 34 → 24 → 25 → 26; results reviewed at the end): **version 1.3.1; the release push is pre-authorised** (the orchestrator bumps, rebases, pushes; the operator smoke-tests the built release afterwards). #70 stays out of this release (opens 19c). Audits run without the operator's screenshot round; every ticket's captures stay under `evidence/<ticket>/` for one end-of-wave review deck.
