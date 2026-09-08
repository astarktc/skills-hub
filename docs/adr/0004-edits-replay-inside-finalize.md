# Edits replay inside finalize's failure-atomic window; the Edit row is the source of truth; the Edit wins on conflict

An **Edit** (see CONTEXT.md) is an operator change layered on a Managed skill's central copy —
V1: the invocation mode in `SKILL.md`'s frontmatter. Every path that replaces the central copy
(Update, Refresh, Restore, Re-point) brings fresh upstream bytes that do not carry the Edit, so
the Edit must be re-applied. We decided three things about that replay:

1. **The `skill_edits` row is the source of truth; bytes follow it.** Replay upserts the row
   (new base value, applied-at, conflict flag) *before* writing the manifest. A row without
   bytes self-heals on the next replay; bytes without a row are just upstream.
2. **Replay runs inside finalize's failure-atomic window.** `finalize_update` lands the staged
   bytes, upserts the skill row, then runs a settle step — Edit replay — *before* it releases
   the previous bytes (`.skills-hub-old-*` backup). A failed replay takes the same rollback as
   a failed upsert: old bytes restored, the pre-finalize skill row re-upserted (snapshotted
   from the store, not from the possibly re-pointed input record), and the Edit row restored
   to the snapshot replay read at entry. A failed Update therefore leaves nothing
   half-settled; a double fault names the retained path/row in the error.
3. **The Edit wins on conflict.** A conflict is upstream changing the edited value to something
   *other than* the Edit's value (base ≠ upstream ≠ override). The override is applied
   regardless, the row is flagged, and the flag survives unchanged Updates until the operator
   re-chooses or clears — or until upstream converges to the override's value, which clears it
   (there is nothing left to disagree about). A conflict is reported only when newly detected.

## Considered options

- **Replay after finalize returns (shipped in v1.2.7).** Finalize's rollback had already
  deleted the backup when replay ran, so a replay failure reported a failed Update whose
  central bytes were already replaced with no recovery material. Rejected in v1.2.9: it broke
  the promise finalize makes.
- **Make replay failure non-fatal (log, keep upstream bytes, retry next Update).** Consistent
  with rule 1 (the row would still be true), but the operator would see a *successful* Update
  whose skill silently lost its override until the next Update. Rejected: an Edit is an
  operator decision; silently shipping upstream's value is the one outcome they did not choose.
- **Write bytes first, then the row.** Simpler rollback, but a crash between the two leaves an
  edited manifest nothing remembers, and the next Update would treat the operator's value as
  upstream's. Rejected: the row must be able to explain every byte it governs.
- **Upstream wins on conflict, Edit dropped with a warning.** Rejected: same reasoning as
  non-fatal replay — the operator chose; upstream's later choice is information, not authority.
- **Any upstream change to the edited key flags a conflict (v1.2.7).** Flagged even when
  upstream adopted the operator's value. Rejected in v1.2.9 (backlog 11): a flag with nothing
  to resolve trains operators to ignore flags.

## Consequences

- `finalize_update` takes a settle closure and returns the settled record plus the step's
  result; Edit replay is the only settle step today. A future Edit kind adds a replay to that
  same step, not a new tail.
- The rollback machinery has two owners: finalize restores bytes and the skill row; replay
  restores its own row. Neither reaches into the other's table.
- Restore of an Unlocatable skill still passes: the skill row exists (only the central copy is
  gone), which is the precondition finalize snapshots against.
- Tests inject the fault at the SQLite seam (triggers on `skill_edits` / `skills`) and at the
  manifest write, and assert byte-for-byte equality of bytes, skill row and Edit row after a
  failed Update, then a clean retry.
