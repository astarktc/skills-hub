# 21: The overwrite ask: defects found after ticket 16

Status: ready-for-agent

**Blocked by:** 16.

**Spec:** `.scratch/round20/spec.md` § Wave B spec → § Overwrite ask (#70); ticket 16 and its review record
(`.scratch/reviews/round20-t16-overwrite-ask-review.md`).

**What to build:** fixes for the defects that ticket 16's code still has. Ticket 16 was built four times in a workflow
trial (Quartermaster QM-365); every finding from all four builds' reviews and from one blinded audit of each was then
tested against the build that was ported here. Items 1 to 6 are present and judged real. Items 7 to 10 are present and
were judged not worth blocking on; decide each.

**Reproductions:** `21-overwrite-ask-audit-fixes.repro.patch` beside this file adds one test per item (names start
`qm365_`), each asserting the correct behaviour, so it fails until the item is fixed. Apply it with `git apply`, keep
each test with its fix and rename it to say what it pins. Rust: `cargo test --lib qm365`. Frontend:
`src/lib/qm365.repro.test.ts` and `src/hooks/qm365.repro.test.ts`. The patch also holds passing tests for defects this
build does not have; keep or drop those under the test rules.

## Acceptance criteria

- [ ] **1. A Delete answered ahead of a move deletes the folder the operator was shown, or nothing.** Today the retry
      journals whatever is at the old path at retry time (`redeploy.rs:981`) and matches the ahead answer by row id and
      path only (`ahead_answer`, `:2064`). If the old copy was swapped for another folder while the ask was open,
      Delete removes the new folder and the run reads "Moved". The code before ticket 16 checked the folder's identity
      when the answer arrived. Test: `qm365_c07_…`.
- [ ] **2. "Shows twice" is claimed only where the registry proves it.** `readers_of` (`redeploy.rs:2107-2124`) counts
      any harness that reads both folders as a duplicate reader and ignores `DuplicateBehaviour`. Xum shadows the old
      copy and Devin's behaviour is `Unknown`, yet the ask and the run say "shows twice" for both
      (`overwriteAsk.leaveShowsTwice`, `redeploy.report.showsTwice`). This is ticket 16's carried-over Sol 102.
      Tests: `qm365_c08a_…`, `qm365_c08b_…`.
- [ ] **3. An edited old copy that another Deployment still uses is not offered Delete or Leave.** `edited_ahead`
      (`redeploy.rs:2085`) does not check whether another live row records the folder (Kimi Code and Amp sharing
      `~/.config/agents/skills/<name>`). The backend rightly keeps the folder, and the report says nothing about it
      (`redeploy.rs:1039`). Tests: `qm365_c13a_…`, `qm365_c13b_…`.
- [ ] **4. A redeploy ask does not say kept skills "stay off" their harnesses.** The "Keep all" explanation
      (`resources.ts:711`, shown by `OverwriteModal.tsx:162-163` whatever the scope) is deploy wording. A kept move
      leaves the old Deployment in place and the move pending, as the row's own Keep line says. Test: check 15.
- [ ] **5. The content view never says "same" for unequal files.** `lineDiff` maps a missing file and an empty file
      to the same thing and drops a final newline (`skillFiles.ts:185`); `useSkillDocument.ts:130` turns a missing
      occupant side into `""`; `SkillFiles.tsx:177-178` then shows "This file is the same as the skill's" while the
      tree badge says "missing here" or "changed". Tests: check 16.
- [ ] **6. On Windows the compare view gets `/`-separated paths.** `skill_files.rs:59` returns the platform separator
      and `buildTree` (`skillFiles.ts:108`) splits on `/`, so a nested file shows as one flat entry `scripts\run.sh`.
      Decided by reading; the repository's own test accepts either separator, so add one that does not.
- [ ] **7 to 10 are each fixed, or closed with one line saying why not:**
  - **7.** An Enable that overwrites one target and leaves another collision unanswered loses the overwritten line
    from the recorded run (`reportOutcome.ts:1346-1347`, `useStatusReporter.ts:563`). No way to reach that state
    through the product was found, because one ask answers every row it shows. Test: check 4(b).
  - **8.** The record of reported occupants holds 512 paths (`occupants.rs:24`), so an action that meets more folders
    than that refuses Reveal and contents for the first rows of its own ask. Test: `qm365_c09_…`.
  - **9.** When the operator keeps the folder in the way and answers Delete for the edited copy, the ask says nothing
    happens to the copy now, and the run's report afterwards is silent about it. Test: `qm365_c01c_…`.
  - **10.** The spec's fixture line asks for an edited-copy ask row in the `registry` scenario
    (`.scratch/round20/spec.md:985`); the row exists only in the `redeploy` scenario. Test: check 17.
- [ ] `npm run version:check && npm run check` is green.

## Comments

- 2026-11-19 (port of ticket 16): opened with the port. Ticket 17 (release 1.4.0) lists this ticket under "Blocked
  by" because item 1 can delete a folder the operator never saw; Alex can lift that.
