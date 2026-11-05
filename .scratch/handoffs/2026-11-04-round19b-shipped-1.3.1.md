# Handoff — 2026-11-04 · 19b shipped as **1.3.1** (tag `v1.3.1`, `ecf3752`); 34 held → BACKLOG #88/#89; closure walk next

The queue is `.scratch/BACKLOG.md` (next free **#91**). Read it, then this note. The 11-02 handoff is superseded by
this one (its whole plan ran); delete it in the first commit of the next session. Round 19's record:
`round19/spec.md`, `round19/issues/01`–`35`, `round19/review/ticket-*-audit*.md`.

## What shipped (all on `main`, each with an Astra audit; fix passes where the audit failed)

30 colour + identity tokens (`9ca0bc8`) · 35 dark two-tone ground (`03065db`) · 31 My Skills anatomy + projects on the
wire (`a2eed57`) · 21 Import review (`e6be057`) · 20 Settings dialog (`1c2cc4b`) · 24 card grid (`6b2e343`) · 25 App.css
deleted (`20d3fb4`) · 26 release (`ecf3752`). Wave 1 (33 32 23 22) was already in. CHANGELOG 1.3.1 is the user-facing
record; the "Known limitations" entry names 34 and the #89 caveat.

Operator decisions this session (grill 2026-11-04, recorded on the tickets): release push pre-authorised; 1.3.1 not 1.4.0;
#70 out (opens 19c); audits ran without the operator's screenshot round; #83 default on (fresh installs); Clawdbot /
MoltBot never installed; Theme menu stays; Layout/Dashboard deleted; grid stands. "Leave fork network" on GitHub: done.

## What did not ship — and why

**Ticket 34 (registry truth + relocation)**: three full Astra audits, three fix passes (897 cargo tests on the branch),
and every audit found a new P0 in moving a live deployment between folders — last: directory aliases let relocation
delete the destination *including the central skill* while reporting success. Parked under rule 11 as
`superseded — BACKLOG #88` (redesign: one alias-aware artifact identity in every seam — relocation, cleanup, rollback,
Keep, `sync_engine` overwrite — and resumable moves with truthful status) and **#89 (pre-existing on `main`)**: the
launch-time Augment `.augment/rules` relocation has the same lexical hazard today. Branch `t3/registry-truth-pass`
(pushed) holds all three passes; audit 3's F14–F16 are the acceptance tests. **#89 is the first thing to decide for
1.3.2**: guard the launch pass (refuse a move whose source and destination resolve to one artifact or into the central
repo) or disable it until #88.

## Operator smoke — pending on ticket 26

Install the 1.3.1 build; VoiceOver over Settings, Import, Explore, the palette; reduced motion; 960×640; the work-Mac
`impeccable` exec-bit check (ticket 19 item 8). Record the outcome on ticket 26.

## Next session, in order

1. Delete this note's predecessor; read the smoke outcome from the operator.
2. **Close round 19** per `docs/agents/issue-tracker.md` § When an effort is finished — the extract walk over 35 tickets'
   Comments and every review "should" (the audits under `round19/review/` carry many P2s that were noted, not applied:
   20's zoom/cache controls not held, 21's fixture parity for shared dirs, 24's Dense name allowance, 25's regex guard
   limits, 31's reverse-read scan) → BACKLOG items or dropped by name; prune `evidence/`; `git mv` to `archive/round19`;
   README row; rewrite inbound paths.
3. Decide #89 (small, 1.3.2) and open 19c: #70 (overwrite ask), #88, #85/#86 (matrix keyboard + selection), #84, #87, #90.

## Things learned this session worth keeping

- **Every surface ticket needed a fix pass after its audit** (31 stale project data; 21 a destructive-path P0 + accounting;
  20 lifecycle discipline; 24 focus; 34 three rounds). Implementers' own gates were green every time. Budget the audit
  + fix + re-check as the ticket's second half, not an afterthought.
- A fixer's "mechanism you didn't specify" is a review item, not a gift: 20's `noteHeld` (a refused click treated as
  intent) contradicted D19 and was removed; the re-check then found the same idea in the coordinator's tests.
- Three-way merges of `App.css` between two pure-deletion branches: take the **intersection** of surviving base lines
  (script in this session's log), then set the ceiling to the counted result; `resources.ts` tails: keep both, check the
  closing brace.
- `.scratch/**/evidence/` is gitignored and lives in the worktree — **copy it out before `git worktree remove`** (30's
  contrast script was lost once and rebuilt). This session's captures are in the main checkout's `evidence/`.
- The shell's bash timeout kills a long `npm run check` when three worktrees compile at once; run the gate alone, or
  background it with a log and poll.
- Vitest teardown "window is not defined" from `sonner.test.tsx` was real on CI's Node 22 and intermittent locally:
  unmount every rendered tree in `afterEach` (`cleanup()`), not just `body.innerHTML = ""`.
