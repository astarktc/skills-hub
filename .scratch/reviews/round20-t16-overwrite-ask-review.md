# Round 20 t16 — an overwrite ask you can answer knowingly: review

- **Where it was built:** in a throwaway clone of this repository, as the control lane of a four-lane workflow
  trial (Quartermaster QM-365), and ported here unchanged by cherry-pick. "PR #1" and the commit ids in this record
  are that clone's. Here they are `bda747f` → `42838742`, `87d17ae` → `0aa016bf`, `35a2a02` → `0f83a5c7`,
  `be08e2f` → `62b37e58`.
- **PR in the clone:** #1 (`t3/overwrite-ask-you-can-answer`)
- **Commit reviewed:** `bda747f`, against that clone's `main` `ae25107`
- **Round 1:** GPT-6 Astra (high), 2026-11-18. Scope stated in the brief: another process changing entries while the app
  acts is out of scope (CONTEXT.md; BACKLOG #119). Merge bar: P0/P1 or a violation of the principle blocks.
- **Verdict:** **SHIP**, no blocking findings. The gate was not re-run.
- **CI on `bda747f`:** `rust-windows` failed to compile. A `cfg(windows)` junction test still called `Occupant::observe`
  with two arguments. The orchestrator fixed it in `87d17ae`.

## Non-blocking (P2), all fixed in this PR (fix round 1)

1. **A truncated occupant listing reports existing files as "missing here"** (`skill_files.rs:155,186`). The listing
   stops at 500 files, and the skill-side pass infers absence from that partial list.
2. **The listing reads whole files to compare them** (`skill_files.rs:165–168`). Nothing bounds the bytes read; only
   the 500-file count is capped.
3. **Ahead-of-move edited-copy questions promise consequences that may not happen** (`OverwriteModal.tsx:131`,
   `overwriteAsk.ts:592–607`). After a Keep on the blocking destination, `redeploy::execute` never processes the
   ahead answer.

## Verified sound (Astra)

- Occupant reads require an exact match in the record of reported paths. Reveal admits a reported occupant without
  its children, parents or siblings.
- Provenance requires a live recorded claim and that row's ownership evidence. A matching path alone is not enough.
- Ahead answers match the moved row and the old path, and pass through journal settlement, protection checks and the
  removal fence.
- Dismissed global Propagation collisions keep their recorded path and are answered through
  `resolveTargetCollisions`.
- Overwritten outcomes reach the report folds and run notes. The redeploy retry merge matches entries by `entry_id`.

## Fix round 1 — `35a2a02` (1, 2), `be08e2f` (3)

- **1:** the listing reports when the 500-file cap cut it short. On the wire it is
  `OccupantFileListing { entries, truncated }`, and the compared file browser says the listing is cut. Any skill file
  missing from the first 500 is probed directly under `read_file`'s containment rule before it may be marked
  "missing here".
- **2:** `same_bytes` compares the two lengths first, then reads both files in fixed 64 KiB buffers. The 1 MB display
  limit stays separate.
- **3:** ahead rows are "visibly void", not disabled: `Segmented` has no `focusableWhenDisabled`.
  - Their consequence lines read "If the move goes ahead: …".
  - While the folder row that blocks the move is answered Keep, the row says nothing happens to the copy now.
  - The hint is split: copies already moved get one, copies asked ahead get another.
  - The backend agrees: Keep returns before any ahead answer is read (`redeploy.rs:843–848`, `:1033`).
- **Gate on `be08e2f`:** 1,503 frontend tests; 1,226 Rust tests, 1 ignored.

## Round 2 — GPT-6 Astra (high), 2026-11-18, fix diff `87d17ae..be08e2f` only

- **Verdict:** **SHIP**. All three findings are closed, and there are no new findings.

## After the merge in the clone: one blinded audit and the pooled checks (2026-11-18)

- **Audit:** GPT-6 Astra (high) audited the finished change with no history and no review record, reading only. It
  reported two P2 and two P3 findings.
- **Pooled checks:** every finding from the trial's four lanes was then tested against this code. Six defects are
  present and judged real; four more are present and judged not worth blocking on.
- **Disposition:** none is fixed in this PR. All ten are ticket 21 (`.scratch/round20/issues/21-overwrite-ask-audit-fixes.md`),
  with the failing reproduction tests beside it.
