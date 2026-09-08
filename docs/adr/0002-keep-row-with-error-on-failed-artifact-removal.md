# A Sync target whose artifact could not be removed keeps its row, with status `error`

Artifact removal (see CONTEXT.md **Artifact removal**) takes a Sync target off disk and then
settles the row that describes it. When the filesystem removal fails — a read-only tool
directory, a locked file, a permission-denied parent — the row is the only record of where
that artifact is. We decided: **rows are deleted only on successful removal; a row whose
artifact could not be removed is kept with Sync status `error`, carrying the failure chain
in `last_error`.** This holds for both tables (`skill_targets` and
`project_skill_assignments`), and for the whole-skill scope it extends to the skill itself:
if any target failed, the central copy and the `skills` row are kept too and the operation
raises the typed `DELETE_CLEANUP_FAILED`, so a retry can still find every artifact. Only a
store failure fails the whole operation; per-target failures are report data.

## Considered options

- **Delete the row blind (the shipped `unsync_skill` / `unsync_all_skills` behaviour).** They
  swallowed the `remove_path_any` error and dropped rows regardless, so a skill kept working
  in a tool while Skills Hub showed it as unsynced — the artifact became invisible garbage no
  later operation could plan against. Rejected: the failure disappears exactly where it
  matters.
- **Delete the row and log.** Same invisibility to the operator (the log is not the UI), with
  the extra cost of pretending the state is clean.
- **Keep the row, don't change its status.** The row survives, but the list still shows
  `synced`, so nothing tells the operator to retry, and the next reconcile pass may confirm
  the (still present) artifact as healthy. Rejected: `error` is the status this exact
  situation is for.
- **Abort the whole operation on the first failure.** One stuck directory would block removal
  from every other tool. Rejected: per-target isolation is the established rule for target
  fan-out (see Propagation, global sync batch); removal reports the same way.
- **Delete the skill row anyway when only some targets failed (the previous delete
  behaviour).** The `skills` row cascades to `skill_targets` and
  `project_skill_assignments`, so the failed artifacts' rows would be erased by the cascade
  and the leftover directories would be unreachable — precisely the state the rest of this
  decision avoids. Rejected: the retry needs the plan, and the plan needs the rows.

## Consequences

- A failed removal is visible in the library and project lists as an `error` row, and
  re-running the same operation re-plans exactly the artifacts that are still there.
- Deleting a Managed skill can now leave the skill in place. The frontend copy for
  `DELETE_CLEANUP_FAILED` says the skill was kept and the operator can retry (it previously
  said the record was already deleted).
- The unsync commands return a removal report (`removed` / `failed` counts plus per-target
  outcomes) instead of a bare count, so the frontend can name every path it could not remove.

## Note: a target row whose Tool is no longer detected

The rules above are unchanged; only *how the path is found* differs. When the registry can no
longer locate a Tool (it is uninstalled, so no member of its shared-skills-dir group is
detected), the skill × Tool scope plans **the row itself, by its own recorded `target_path`** —
no group fan-out, because there is no group left to fan out to. Deriving a path from the
registry for an absent Tool stays forbidden; removing the artifact we ourselves created and
recorded is cleaning up after ourselves, and without it an orphaned row has no supported
removal path at all. Because that path is a stored string rather than a derived one, planning
fences it with `tool_adapters::ensure_path_within_tool_dirs`: a row pointing outside every Tool
skills dir raises the typed `PATH_OUTSIDE_TOOL_DIRS` and nothing is planned or settled, so the
row survives. The presence rule and the settlement rule then apply unchanged — an absent
artifact is a successful removal that deletes the row, a failed removal keeps the row with Sync
status `error`.
