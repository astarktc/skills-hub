# Round 20 t10 — redeploy (global) and the registry corrections: review

- **PR:** #24 (`t3/redeploy-global-registry-corrections`)
- **Commit reviewed:** `7b80302f`, against `origin/main` `690d3e8e`
- **Round 1:** GPT-6 Astra (high) and GPT-6.1 Sol (high), independently, 2026-11-07
- **Scope stated in both briefs:** another process swapping entries while the app acts is out of scope (CONTEXT.md;
  BACKLOG #119).
- **Verdicts:** Astra **BLOCK** (findings 1–3); Sol **SHIP** (no blocking findings).
- **CI:** green on all three jobs. `git merge-tree` against `main` reports no textual conflict; a rebase and a gate run
  are still needed.

## Blocking (Astra)

### 1. P1: removing the old copy can delete the new Deployment through a nested folder alias

The retention check protects an old artifact only when another row records exactly the same entry
(`redeploy.rs:1076–1110`). Nothing protects a Deployment whose parent sits *inside* the old artifact.

Reproduced on the Kilo correction:

1. `~/.kilo/skills` is a symlink to the old copy `~/.kilocode/skills/alpha`.
2. The new Deployment lands inside the old copy.
3. Cleanup removes the old copy, and the new Deployment with it.
4. The report says `Moved`, and the journal is empty.

**Fix:** use physical ancestry, not entry equality.
- Refuse, or safely keep, any old/new pair whose placements overlap.
- At cleanup, protect the journaled destination and every live Deployment whose entry or parent lineage runs through
  the old folder.
- Apply this to normal cleanup, the startup finish and the answered edited-copy delete.

Add the nested-alias case beside F14.

### 2. P1: answers are bound to a path, but not to the row or journal entry that raised the question

`RedeployAnswer` carries only `(skill_id, folder_key, path)` for an occupied destination, and only
`(skill_id, tool, path)` for an edited copy (`redeploy.rs:311–331`, `:725–745`, `:1183–1197`).

Reproduced twice:
- **A recreated row:** an Overwrite answered for `row-kilo_code` overwrote the destination on behalf of a recreated
  `replacement-row`.
- **A reused journal entry:** an old Delete answer deleted a later journal entry's copy that sat at the same key and
  path.

**Fix:**
- Bind each answer to a stable question identity: the source row id (or row-set generation) plus the destination for
  an occupied destination, and the journal-entry id for an edited copy.
- Validate that identity, and refuse a stale answer without touching anything.
- Thread the identity through the bindings and the ask adapter.

### 3. P1: "adopting" an identical destination removes it first, so an interruption leaves the skill missing

Redeploy uses `Replace::if_ours(..).or_same_content(true)`. For a copy, the engine removes the occupant and then
re-creates it (`redeploy.rs:810–825`, `:932–937`; `sync_engine.rs:295–296`, `:365–376`).

Reproduced: the old recorded path is absent, and an identical copy sits at the destination. Interrupting just after
`replace_occupant` leaves neither copy, with no row switch and no journal entry.

**Fix:**
- Make adoption genuinely non-destructive: record the acceptable existing artifact as it is.
- Where a replacement really is needed, stage it so it can be recovered.

Add an interruption test inside the destination step.

## Non-blocking

Each item has a home and is carried into that ticket.

| Item | Ticket | What |
|---|---|---|
| Sol 101 | 12 | A normal sync folds `REDEPLOY_PENDING` into a bare "1 skipped" and offers a plain Retry. It needs an actionable line pointing to Settings → Harnesses. |
| Sol 102 | 16 | Readers of the copy left behind are reported as "shows twice", even when they don't read the new folder. Say "still visible in …" unless a duplicate is proven. |
| Sol 103 | 12 | Declining remembers counts and harness names (`useRedeploy.ts:47–53`), not the exact work. It needs an opaque work fingerprint from core. |
| Sol 104 = Astra 4 | 16 | A move with both a folder in the way and an edited old copy asks the edited-copy question only on a second run. |
| Sol 105 = Astra 5 | 12 | A move blocked by a failed Disable is reported as "re-recorded in its new folder". It needs truthful pending copy and a recovery action. |
| Sol 106 | 12 | The fixture registry still has pre-correction dirs, so after a move the Harnesses pane disagrees with where things went. The edited-copy readers are hardcoded too. |

## Verified sound

- **Ordering:** for disjoint paths, it deploys, then switches, then cleans up. The switch and the journal write are
  one transaction.
- **Journal:**
  - added without a version bump and without a foreign key, both on purpose;
  - each recovery checks the row, both entries and the link kind.
- **Cleanup:** it goes through `ownership::check` and the managed-folder and Library fence. Kimi/Amp's shared artifact
  is kept.
- **Q24, disabled and never-deployed rows:** behave as specified.
- **Path binding and the mode exception:** answers are bound to the path, and the copy-in-a-link-folder exception holds.
- **Additions beyond the ticket:** admission of first writes, the `REDEPLOY_PENDING` skip, the former-dir footprint
  and `tool_holding_path`.
- **Release split:** correct. `the_operators_machine_has_nothing_to_move` passes.
- **F14–F16:** pass.
- **Fixture walkthrough (Sol):** the offer, one ask, "58 moved · 1 kept existing · 1 old copy left", then "Move 1
  Deployment", and the retry clears it.

**Gate:** `npm run check` and `cargo test --all` passed (1,459 frontend tests; 1,154 Rust tests, 1 ignored).

## Disposition (orchestrator, 2026-11-07)

Fix pass 1 takes findings 1–3 and rebases onto `main`, which now has ticket 19. The non-blocking items are carried
into tickets 12 and 16.
