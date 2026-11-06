# Ownership of a Deployment's folder is evidence checked when acting; legacy rows convert explicitly

A Deployment row records three things: intent ("this skill belongs in this folder"), a record of what the app wrote
and where, and, so far, ownership: because `synced_at` is set, whatever sits at the row's path is the app's to
replace or remove. The third claim does not follow from the second. Something else can occupy the path after a
Disable, or after the operator replaces a link by hand. Round 20 ticket 09 also showed a second trap: reading legacy
rows as if they already followed a new model forces every write to guess whether two rows mean the same thing.

We decided two rules.

1. **Ownership is evidence, checked at the moment of acting.** A row owns the occupant of its path only when that
   occupant is what the app wrote through that row:
   - a link resolving, by physical identity, to its skill's Library copy; or
   - a copy whose Content identity equals the content the row recorded as deployed.

   The check runs where the app removes or replaces the folder, never as an earlier observation that permission is
   carried over from. `synced_at` stays as history ("the app wrote here"), but it grants nothing. A successful
   removal ends ownership.

   Anything unverifiable is not ours: it is kept and goes to the overwrite ask (D12), or is reported left in place.
   Physical identity and folder grouping may make a decision more cautious (refuse, or de-duplicate what is
   displayed), but never grant ownership, overwrite permission or settlement on another row's behalf.
2. **Legacy data keeps its old meaning until it is converted explicitly.** A model change (per-harness rows → one row
   per Deployment folder, retired harness keys, corrected folders) never reinterprets existing rows in place. Until the
   change is applied, old rows keep their old semantics in every mutation. The conversion runs once, as an operator
   action with a report: the redeploy (`core/redeploy.rs`, round 20 tickets 10 and 11).

## Considered options

- **Keep `synced_at` as ownership (the behaviour through 1.3.4).** It is simple, but Delete after a Disable can
  remove a folder the operator put there later, and resync silently replaces a hand-made replacement while reporting
  Synced (round 20 t09 review round 3: Astra 10–11, Sol 102). Rejected.
- **Prove that two legacy rows share an artifact (same inode, aliases, absent slots) and let them share ownership.**
  Tried in ticket 09's fix passes 1–2. Each proof had a counterexample: hard-linked names, a folder appearing between
  check and write, an answer applied to a different row. Rejected: grouping is presentation, never authority.
- **Collapse legacy rows silently at startup.** No report, no ask, and a crash midway leaves a half-converted
  database. Rejected: the redeploy is journaled, offered and reported.
- **Rewrite the deployment modules.** Unnecessary. The seams held (the removal fence, the single removal module, the
  Mutation guard, pure placement). The flaw was one concept in the data model.

## Consequences

- **Every row records the evidence of what it deployed.**
  - Project assignments already store the deployed `content_hash`.
  - Global Deployment rows gain the same column (round 20 ticket 18), migrated conservatively: an existing row with no
    recorded evidence owns only a link that resolves to its Library copy.
- **One occupant check, beside the removal fence.** Every destructive seam calls it: Delete, unassign, Disable,
  project removal, resync, Propagation, the global batch, Enable and the redeploy.
- **Disable gives up ownership when its removal succeeds.** It keeps ownership when the removal fails (ADR-0002 still
  holds).
- **More asks, fewer surprises.** Folders the operator replaced by hand now raise the overwrite ask instead of being
  replaced silently.
- **Folder grouping (round 20 Wave B) stays a display and counting concept.** Ticket 09 adds new rows per folder, and
  old per-harness rows act as they always did until the redeploy converts them.
