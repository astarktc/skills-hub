# Round 20 t18 — Deployment ownership means the current occupant: review

- **PR:** #22 (`t3/ownership-current-occupant`)
- **Commit reviewed:** `ac0626a` against `origin/main` `667239c`
- **Round 1 reviewers:** GPT-6 Astra (high) and GPT-6.1 Sol (high), working independently, 2026-11-06.
- **Verdicts:**
  - Astra: **fix-then-ship**.
  - Sol: **BLOCK** (finding 101).
- **CI:** all three jobs green, including `rust-windows`.

## Findings

### Sol 101. P1: a global Propagation Overwrite answer can replace a folder nobody asked about

- Propagation raises its collision at the row's recorded `target_path` (`propagation.rs:294–300`).
- The retry carries the skill, harness and answer, but not the path that was asked about
  (`useSyncOrchestration.ts:513–542`).
- The global batch rebuilds the destination from the current root and skill name, so the Overwrite applies there
  (`global_sync.rs:139`, `:722–736`, `:759–762`).

Reproduced: a copy row records `old-name`, and folders the app didn't create sit at both `old-name` and the current
destination. The ask names `old-name`. After Overwrite, the folder at `old-name` survives, the unasked folder at the
current destination is replaced, and the row moves to that path. This is the class of ticket 09's Sol 101 again: an
answer must apply to exactly the row and path it was asked about.

### Astra 1 = Sol 102. P2: Propagation treats a historical link mode as proof

Both Propagation arms return `Skipped { LinkFollowsSource }` from `owns_artifact()` and the recorded mode, before
checking what is at the path (`propagation.rs:280–287`, `:422–427`).

- **What happens:** when an operator replaces a deployed link with a folder of their own, Update raises no ask. The
  catalog reads Synced and Deployed, and the harness reads unrelated content.
- **Why the tests pass:** the test (`tests/ownership.rs:534–553`) only asserts "not Synced", which this outcome
  satisfies.
- **Fix:** take the shortcut only after verifying that the current entry is the expected link. Otherwise go through
  the normal occupant decision, so the result is `TARGET_EXISTS`. Require that at both scopes, and test Keep and
  Overwrite.

### Astra 2. P2: a recorded global path outside every managed skills folder can authorise a removal (pre-existing)

The executor builds `Fence::new(&library)` with no location claim; `Claim::Unclaimed` admits anything that passes the
Library protections (`artifact_removal.rs:749–757`, `removal_fence.rs:309–317`, `:380–381`).

- **Reproduced:** a Deep Agents copy whose stored path is `home/Documents/alpha` still has matching evidence, so
  Delete removes it.
- **Reach:** only a malformed stored path can trigger it; no normal flow found creates one. It still breaks
  ADR-0007's "only inside a skills folder it manages".
- **Fix:** carry the managed-directory claim into removal execution and admit physically against it:
  - global targets: the current and former registry roots;
  - project targets: their project skills root.

  An out-of-bounds row is refused and kept. Apply the same rule to stored-path replacement: global Propagation and the
  Library-move relink.

### Sol 103. P2: the bulk Delete report hides left-in-place folders of skills that were deleted

Left-in-place details render only for kept skills (`DeleteModal.tsx:176–195`, `:215–218`). A deleted skill
(`deleted: true`) whose `leftInPlace` list isn't empty appears only as "Removed". Render those details for deleted
rows too, keep Retry for kept skills only, and add a pure regression test for the mixed outcome.

### Sol 104. P3: the fixture can't model global copy evidence

`StoredTarget` has no deployed identity (`fixtures/model.ts:41–45`), so a claiming copy with no hash is removed in the
fixture (`fixtures/backend.ts:617–669`), while the real core leaves it in place. Store the deployed and current
identities in the fixture's internal model.

## Behaviour changes (Sol): all acceptable

- Pre-V12 global copies are left in place, and Propagation asks about them until an Overwrite records evidence.
  Acceptable, once 101 is fixed.
- An orphaned assignment is left in place on project removal.
- A folder that reappears at a disabled pair's path is visible to Onboarding.
- The V12 clearing is one-way.

## Verified sound (Astra)

- **Ownership:** Disable gives up ownership; a failed removal keeps both its claim and its hash.
- **Evidence:**
  - a stale copy of our own is still replaceable through its recorded identity, and an edited copy goes to the ask;
  - reconcile keeps the hash;
  - the global batch's `row_evidence` requires the row's own key and its exact destination.
- **Answers:** a collision answer reaches a claiming project row, and Keep clears its claim.
- **V12 migration:** conservative and transactional.
- **Settlement:** `LeftInPlace` deletes the row for Delete, unassign and project removal, and keeps a disabled row with
  no claim for Disable.
- **Dangling links:** the rule is sound.
- **Windows:** the junction stand-in is correct.
- **Other removal seams:** central-copy Delete, onboarding originals, Library-move Clean (which renames aside and
  re-verifies) and superseded-target cleanup all hold.

**Gate (both):** `npm run check` and `cargo test --all` passed (1,424 frontend tests; 1,111 Rust tests, 1 ignored).

## Disposition (orchestrator, 2026-11-06)

All six findings go to one fix pass:
- **101:** a Propagation collision gets a retry tied to the row and path it asked about, and the answer is checked
  against that recorded row and path. It never becomes permission to overwrite the batch destination. Test it with
  foreign folders at both paths.
- **102:** take the link shortcut only after the occupant check.
- **Astra 2:** apply the managed-directory claim at removal and at stored-path replacement.
- **103 and 104** as their reviewers suggest.
