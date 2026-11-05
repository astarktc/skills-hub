# 26 — CHANGELOG, `version:set 1.3.x`, release; operator smoke

Status: done — ecf3752 (tag v1.3.1; operator smoke pending)
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

- **Release 1.3.1** — commit `ecf3752`, tag `v1.3.1` by `auto-tag.yml` (run 41972643481), `release.yml` run
  41972658019 **success**: `Skills-Hub-v1.3.1-{macOS-aarch64,macOS-x86_64}.dmg` + `.tar.gz(.sig)`,
  `Windows-{x64,arm64}.exe`, `Linux-x86_64.{AppImage(.sig),deb}`, `updater.json`. CI on the release commit was
  superseded by a follow-up push; CI **success** at `718509e` (run 41973009427) — the only change in between is the
  `sonner.test.tsx` teardown fix (`75aabc3`), which had failed CI intermittently at `771358b` ("window is not defined"
  after the run: a Toaster left mounted past jsdom teardown).
- Gate at the release commit: vitest 1250 · cargo 854 · clippy · fmt · build · styles:check (App.css absent, 44 retired
  classes unused) · `version:check` 1.3.1 (five locations).
- **Operator smoke (D31) — pending**: install the built release; VoiceOver over Settings, Import, Explore and the
  palette; reduced motion; 960×640; the work-Mac `impeccable` exec-bit check from ticket 19.
- **Closure of round 19** — not done this session: ticket 34 is `superseded — BACKLOG #88/#89`, every other ticket
  terminal; the extract-then-archive walk (35 tickets' comments and review "should"s) is the next session's first task
  with the operator present (handoff 2026-11-04).

## Comments

- 2026-11-02 — Vocabulary: read Tool/Sync/Refresh as Harness/Deploy/Update — ticket 32.
- 2026-11-04 — operator grill (autonomous run through 31 → 20 ∥ 21 → 34 → 24 → 25 → 26; results reviewed at the end): **version 1.3.1; the release push is pre-authorised** (the orchestrator bumps, rebases, pushes; the operator smoke-tests the built release afterwards). #70 stays out of this release (opens 19c). Audits run without the operator's screenshot round; every ticket's captures stay under `evidence/<ticket>/` for one end-of-wave review deck.
- 2026-11-04 — orchestrator: CHANGELOG 1.3.1 block written (30 35 31 20 21 22 23 24 25 32 33; 34 under Known limitations
  with the #89 caveat); `npm run version:set 1.3.1`; `version:check && check` green (vitest 1250, cargo 854, App.css
  absent). Operator confirmed the GitHub "Leave fork network" detach finished; the release push follows (pre-authorised).
