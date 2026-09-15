# 10: Round-4 review panel, CHANGELOG, version 1.2.4

Status: done — ed630c0

**What to build:** The round's diff since `f057190` is reviewed by the three-model panel (Fable 5.1 / Opus 5 / GPT-5.6 Sol) on both axes using the round-3 brief shape (`../review-brief-round3.md`, fixed point and ticket list updated); every reported blocker is verified at HEAD before action, true blockers are fixed by a follow-up ticket in this directory, follow-ups are filed as a `needs-triage` ticket. `CHANGELOG.md` `[Unreleased]` gains the round's operator-visible entries (symlinked upstream skills refresh; imported skills are "managed here" and leave Refresh; legacy rows reclassified; unlocatable-skill badge and actions; typed missing-path errors; import takes over every identical original; Add refuses Tool-dir paths; notification panel ordering and copy behaviour). Then `npm run version:set 1.2.4`, the gate, one release commit, push, and the Release run is confirmed green with all five targets.

Source: `../spec.md` Q0, Q12.

**Blocked by:** 01, 02, 03, 04, 05, 06, 07, 08, 09

- [ ] Three panel reports saved under `$TMPDIR/r4-review-*.md`; every "Blocking" claim has a verify-at-HEAD note (upheld / downgraded with evidence)
- [ ] `CHANGELOG.md` renamed `[Unreleased]` → `[1.2.4]` in the bump commit
- [ ] `npm run version:check && npm run check` green at the bump; `cargo test` after `version:set` leaves the tree clean
- [ ] Release run green on all five targets; GitHub release has 12 assets incl. `updater.json`

## Comments

- 2026-09-15 — Status reconciliation: done → done — ed630c0. Evidence: Cited ed630c0 and local v1.2.4 tag match the recorded release; CHANGELOG.md:70. Existing comments record run 33989157337 green; operator smoke still owed in that historical note, not rerun here.

### 2026-09-05 — orchestrator
- Panel: `../r4-review-{fable,opus,sol}.md`; every Blocking claim verified at HEAD `7fcec94` — 8 upheld (all fixed in ticket 11), 3 downgraded with evidence (Sol: import TOCTOU — plan rebuilt inside the guarded call; `formatError` prop — spec Q10 vs AGENTS.md wording, needs operator ruling → 12 #1; API component symlink — typed 404, not wrong bytes → 12 #2).
- Ticket 11 merged `2fac087`; gate vitest 214 / cargo 513; operator DB copy (v7, 85 rows) upgraded 7→9 in one transaction, second run no-op.
- `CHANGELOG.md` `[1.2.4]` written; `npm run version:set 1.2.4`; gate green; `cargo test` left the tree clean. Release commit `ed630c0`, pushed; auto-tag `v1.2.4`; Release run `33989157337` green on all five targets + updater.json.
- Operator smoke test on the built 1.2.4 is still owed (ticket 02's last box, reclassification on first launch, Refresh (all) summary).
