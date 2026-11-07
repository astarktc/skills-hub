# 18: Deployment ownership means the current occupant

Status: done — fcf08cbd

**Blocked by:** 09.

**Blocks:** 10. The redeploy's removal step reuses the occupant check built here.

**Source:** `.scratch/reviews/round20-t09-placement-review.md`, round 3: Astra's findings 10 and 11, and Sol's 102.
This is released behaviour, not a regression. Spec: `.scratch/round20/spec.md` § Wave B (the removal principle in the
grill log and Further Notes).

**What to build:** the app only removes or replaces a Deployment's folder when it can show, right when it acts, that
the folder at that path is its own. Today a row counts as the owner of its path forever once the app wrote it there
(`synced_at` is set). So:

- Delete, after a successful Disable, deletes a different folder that later appeared at the same path.
- Resync and Propagation replace a folder the operator put where the app's link used to be, and report Synced.

After this ticket, a folder that isn't verifiably ours stays where it is and goes to the one overwrite ask (D12).

**Decision record:** `docs/adr/0007-ownership-is-evidence-and-legacy-rows-convert-explicitly.md`. Implement rule 1 and
its Consequences.

## Acceptance criteria

- [x] **Each row records what it deployed.** Global Deployment rows gain the deployed Content identity
      (`content_hash`), which project assignments already carry. It is written at every successful write and
      re-materialisation (sync, Propagation, Enable, collision Overwrite). Schema migration through
      `migrate_legacy_db_if_needed`. An existing row with no recorded evidence owns only a link resolving to its
      Library copy; an unrecorded copy is not provably ours.

- [x] **Disable gives up ownership when it succeeds.** When Disable removes the artifact, or confirms it is already
      absent, the row stops owning its path (at both scopes). A failed removal keeps ownership, as ADR-0002 requires.
      Existing disabled rows that still carry ownership are handled conservatively on upgrade: none of them may give
      Delete the right to remove a later occupant.
- [x] **One occupant check, used at every destructive seam.**
  - A path counts as ours only when one of these holds:
    - it is a link resolving, by physical identity, to its skill's Library copy;
    - it is a copy whose Content identity matches the row's recorded deployed identity.
  - Anything else, or anything that can't be verified, is not ours.
  - The check runs at the moment of removal or replacement, not as an earlier observation that permission is carried
    over from.
  - It lives beside the removal fence (`removal_fence` / `artifact_removal`), so the redeploy (10) and every other
    caller use the one check.
- [x] **Delete, unassign, project removal and Disable** only remove an occupant that passes the check. Any other
      occupant is kept and reported as left in place; it is never deleted silently.
- [x] **Resync, Propagation, Enable's first sync and the global batch** silently replace only an occupant that passes
      the check, or identical content (the existing rule). A different occupant is a `TARGET_EXISTS` row for the
      overwrite ask, never a silent replacement and never a Synced report.
- [x] **Tests** (core integration, both scopes):
  - Disable, then a foreign folder appears, then Delete: the folder survives.
  - An owned link replaced by a foreign folder, then resync: an ask, not a replacement.
  - The same through Propagation.
  - A copy edited in place (content no longer what was deployed) is not ours.
  - A link still pointing at the Library copy is removed or replaced as before.
  - Disable with a failed removal keeps ownership.
  - The upgrade path for already-disabled owners.
  - The test `an_owned_disabled_target_is_still_removed_by_delete` is replaced, not deleted silently: its replacement
    pins the new rule.
- [x] **CONTEXT.md:** the Artifact removal and Disabled entries state the rule. Ownership is "the app wrote this row's
      path, and what is there now is still that".
- [x] `npm run version:check && npm run check` is green.

## Notes

- Review it with Astra and Sol in parallel (Q28), each briefed with the principle only: the app removes or replaces a
  folder only when it can show that folder is its own duplicate, at the moment it acts, and only inside a skills folder
  it manages.
- Prior art: `library_move.rs` (physical identity, verification over compared facets), the `artifact_removal` fence tests.

## Comments

- 2026-11-06 (orchestrator): **done — merged as `fcf08cbd` (PR #22)**, after three review rounds with Astra and Sol
  (`.scratch/reviews/round20-t18-ownership-review.md`).
  - **What shipped:**
    - `core/ownership.rs`, whose `check` returns `Absent`, `Ours` or `Foreign` against a row's `Deployed` evidence;
    - schema V12 `skill_targets.content_hash`, and the V12 clearing of disabled claims;
    - a successful Disable or a Keep answer gives up the claim;
    - `LeftInPlace` removal settlement;
    - `Replace` evaluated at the occupant, with a managed-dir claim on every replacement and removal;
    - one name rule (`placement::deployment_path`);
    - overwrite answers bound to the asked path on every route (`resolve_target_collisions` and the path-carrying
      `AssignmentDecision` / `BatchSyncOverrideDto`).
  - **Gate on the merged main:** `npm run version:check && npm run check` exit 0 (1,432 frontend tests; 1,126 Rust
    tests, 1 ignored).
  - **Follow-ups:**
    - ticket 10: first-write admission, and recovery from an out-of-bounds refusal;
    - ticket 16: Notifications repair provenance, and the stale-answer copy;
    - ticket 19: install-time names.
  - The children's per-pass Comments entries stayed uncommitted in their worktrees; this entry stands in for them.
