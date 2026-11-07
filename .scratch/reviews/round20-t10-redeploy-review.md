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

---

# Round 2: fix pass 1 (`ca7fc957`, rebased onto `ee373883`)

- **Verdicts:** Sol **SHIP**; Astra **BLOCK** (findings 7 and 8).
- **Round 1:** findings 1–3 are closed. Ticket 19's safeguards survived the rebase. CI is green.

## Blocking (Astra)

### 7. P1: the two-rename fallback can interrupt the only working Deployment (`redeploy.rs:1413–1420`)

Without an atomic exchange (always the case on Windows), replacement moves the occupant to `.old`, then `.new` into
place. Nothing checks that a working old Deployment still exists.

Reproduced: the old Kilo place is absent, and the destination holds a different, usable copy. After an Overwrite, an
interruption between the two renames leaves the skill at neither place. Startup recovers this case, but it still
breaks "never a missing skill".

**Fix:** use the two-rename path only while an independently verified, distinct old Deployment stays intact. When it's
the only working placement and no exchange is available, leave the occupant untouched and keep the move pending.

### 8. P1: an interruption inside the reservation fallback defeats recovery and drops the journal

- **Where:** `no_replace.rs:165`, `:221–232`; `redeploy.rs:1770–1794`, `:1861`, `:1892–1893`.
- **Cause:** on a filesystem without an exclusive rename, `rename_no_replace` does `mkdirat` and then `renameat`.
  Recovery doesn't recognise the empty reservation: it reports `LeftBehind / Stale` and deletes the replacement journal
  entry.
- **Result:** the old place is absent, an empty folder sits at the new place, and the staged skill sits beside it with
  no journal.

**Fix:** give redeploy a strict no-replace primitive that returns "unsupported" instead of falling back, and handle
that refusal without disturbing the working placement. Alternatively, journal the reservation and recover it
explicitly. Never treat an arbitrary empty folder as evidence.

## Verified (Astra)

- **Nested aliases:** closed, including the inverse case and the protected cleanup on every path. F15's
  `holds_deployment` is correct.
- **Answer identity:** closed. The identity is preserved through `report → ask → answers → command`.
- **Adoption:** non-destructive.
- **The atomic exchange:** sound. It uses Linux `RENAME_EXCHANGE` and macOS `RENAME_SWAP`.
- **The ordinary two-rename recovery tests** pass.

## Non-blocking

Each item has a home and is carried into that ticket.

| Item | Ticket | What |
|---|---|---|
| Sol 111 | 12 | `NOT_FOUND` of kind `redeploy_question` on a failed move reads "Item not found: redeploy_question:…". Specialise it in `describeCommandError` with the "question changed, run Move again" copy. |
| Sol 112 | 16 | Two edited-copy questions at one path look identical in the ask and have identical accessible labels. Show enough provenance to tell them apart. |
| Sol 113 | 12 | An interruption before the swap, undone at startup, reads "1 old copy removed". Distinguish "discarded an unfinished replacement; still pending". |
| Astra | 16 | The retry-report merge (`overwriteAsk.ts:468–480`) associates settlement by `(skill_id, tool, path)`. Use `entry_id`. |
| Astra | 12 | The fixture rebuilds row ids as `${skillId}:${tool}`. Use stable row-instance ids. |

**Gate (both reviewers):** 1,467 frontend tests; 1,195 Rust tests, 1 ignored.

## Disposition (orchestrator, 2026-11-07)

Fix pass 2 takes findings 7 and 8 as narrowly as possible. A destination is replaced only:
- by an atomic exchange; or
- by two strict no-replace renames, while a verified distinct old Deployment stays intact.

Otherwise the move is refused and stays pending, with an actionable reason: move the folder at the new place aside,
then Move again. The reservation fallback is never used inside redeploy.

---

# Round 3 (Astra): fix pass 2 (`d65f9962`, delta from `ca7fc957`)

- **Verdict:** **BLOCK** (finding 9).
- **Finding 8: fixed.** All four renames in redeploy use `strict_rename`, which never reserves a name. Install still
  uses the unchanged `rename_no_replace`.
- **Finding 7:** fixed except for finding 9. These all hold:
  - the atomic exchange is tried first;
  - a missing, changed or unproven old copy can't authorise the fallback;
  - the crash point after staging is safe;
  - the Deep Agents refusal on Windows is acceptable, and its message is clear.

## 9. P1, blocking: an "intact" old Deployment can depend on the destination for its resolution

`IntactOld::find` / `holds` checks entry identity, ownership and ancestry (`redeploy.rs:1535–1590`, used at
`:1434–1437`), but not whether resolving the old Deployment passes through the destination.

Reproduced on Mux → Xum:
1. The old link `~/.mux/skills/alpha` points to `~/.xum/skills/alpha/../../../../central/alpha`. It resolves to the
   Library, so `ownership::check` accepts it.
2. The first fallback rename moves the destination aside, and the old link dangles: neither place serves the skill.
3. If startup's strict rename is also unsupported, the journal entry is dropped as `LeftBehind`.

**Fix:** establish that the old copy's resolution path is independent of the destination, or refuse before the first
rename.

## 10. P2 → ticket 12: a recovery refusal at startup asks the operator to move a destination that is absent

`redeploy.rs:1911–1917` and `:1497–1504` give recovery the initial refusal's copy, so it names the destination even
though the occupant is already at `.old` and the replacement is staged. Recovery should get its own copy.

**Gate:** passed, 1,468 frontend tests and 1,200 Rust tests. CI is green. Two probes fail; both demonstrate finding 9.

## Disposition (orchestrator, 2026-11-07)

Fix pass 3 makes the fallback's witness match what the app writes. An old Deployment counts as independent only when
it is one of these:

- **a copy**: a real directory, not a link, that isn't nested with the destination either way;
- **a link** whose `read_link` text is exactly the absolute recorded Library copy path, which is what the app writes.

Anything else (a relative link, any `..` component, an intermediate link) is not a witness, and the move is refused.
Finding 10 goes to ticket 12.
