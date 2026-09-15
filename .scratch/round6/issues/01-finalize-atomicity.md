# 01: finalize_update is failure-atomic (rename-aside)

Status: done — faf109a

**Lane:** R1  **Files:** `src-tauri/src/core/install_finalize.rs` (+ its tests), nothing else in core.

## Problem
`finalize_update` (`install_finalize.rs:259`): `remove_dir_all(central)` → `staged.move_into(central)` (rename, fallback copy)
→ `store.upsert_skill`. A move failure leaves the central copy missing (Unlocatable `central_missing`); an upsert failure
leaves new bytes under the old source row. Inherited from `fe60d94`; every Update, Restore and Re-point has it.

## Decision (spec D1/D2)
Rename-aside swap:
1. If `central` exists, rename it to a sibling `central.old` (choose a name that cannot collide with a skill dir — e.g.
   `<name>.old-<now_ms>`; document the choice).
2. `staged.move_into(central)`. On failure: rename `.old` back to `central`, return the move error.
3. `store.upsert_skill(updated)`. On failure: `remove_dir_all(central)`, rename `.old` back, return the upsert error.
4. On success: `remove_dir_all(.old)`; a failure here is logged, not returned (bytes + row are already consistent).
If a rollback rename itself fails, leave `.old` in place and return an error whose context names the `.old` path so the
operator can recover by hand. No journal, no new trait.
Check whether `finalize_install` (first install) has an analogous hole and, if so, whether the same swap applies — report
rather than silently widen scope if it needs a different shape.

## Tests
- Move failure: `#[cfg(unix)]` — make the central parent read-only so `move_into` fails; assert `central` still holds the
  old bytes and the row is unchanged; restore perms in a guard.
- Upsert failure: break the store before finalize (drop/rename the `skills` table via the connection, or whatever the store
  test helpers allow); assert old bytes and old row. A `#[cfg(test)]` fault hook is acceptable **only** if the store cannot
  be made to fail cleanly — say so in Comments.
- Success path: `.old` is gone afterwards.

## Gate
`npm run version:check && npm run check`. Commit on your branch with a conventional title. Paste `## Comments` in the final message.

## Comments

- 2026-09-15 — Status reconciliation: ready → done — faf109a. Evidence: git log v1.2.5..v1.2.6 identifies faf109a, the rename-aside implementation (fe60d94 in the ticket is the buggy ancestor, not completion). src-tauri/src/core/install_finalize.rs:289,304,353 preserves rollback and rename-aside.
