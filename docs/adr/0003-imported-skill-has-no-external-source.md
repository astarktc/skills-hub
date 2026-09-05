# An imported skill has no external source; the central copy is its truth

Onboarding import (see CONTEXT.md **Onboarding import**, **Provenance**) copies a skill out of a
Tool's skills directory into the central repo and then, in the same operation, either overwrites
that directory with a link to the central copy (auto-sync on) or removes it (auto-sync off). Until
round 4 the import reused the "Add from a local folder" flow and so recorded the Tool path as the
skill's `local` source — a path the import itself had just replaced or deleted. We decided: **a
skill taken over from a Tool's skills directory is recorded with provenance `imported`: it has no
`source_ref`, the central copy is its source of truth, it is never a member of a Refresh batch and
offers no Update, and the Tool it was found in is kept in its own display-only field, never read
as a path.** The `.skill-lock.json` upgrade stays: a Tool copy that `npx skills add` installed has
a real upstream and is recorded `git`.

## Considered options

- **Keep recording the Tool path as a `local` source (the shipped behaviour).** With auto-sync
  on, the "source" is a link back to the central copy, so Refresh copied central onto itself — a
  self-referential no-op that reported success. With auto-sync off, the source was gone, so every
  Refresh failed the skill forever with a path-leaking error. The operator's real library held
  seven such rows, and nobody had ever intended either behaviour: the upstream fork had "Add from
  a local folder" and import borrowed it. Rejected: the record described a source that did not
  exist.
- **Record the Tool path but mark the skill non-refreshable with a flag.** Keeps a `source_ref`
  that no operation may read, inviting the next feature to read it (Re-point, Restore, the
  legacy reclassification all inspect `source_ref`). Rejected: a path that is not a source must
  not sit in the source column.
- **Record the Tool path in `source_subpath` or another existing column.** Rejected: overloading
  a field with a second meaning is what made the original mistake invisible.
- **Drop the found-in Tool entirely.** The operator asked to remember where a skill came from;
  the information is cheap to keep and answers "why is this here?" on the card. Kept, in a
  field whose only reader is presentation (`imported_from_tool`).
- **Make imported skills refreshable from the Tool directory.** The Tool directory is a Sync
  target of the central copy after import; treating it as a source inverts the flow of bytes
  and would let a stale copy overwrite the operator's edits. Rejected: one direction only —
  central to Tools.

## Consequences

- Refresh (all) selects its members through one backend predicate (`provenance::is_refreshable`);
  an imported skill is not "skipped" — it is not in the batch, so the summary counts only skills
  that can actually be refreshed. A single Update of an imported skill is a typed refusal
  (`NOT_REFRESHABLE`), and the card does not offer Update for it in the first place.
- Because an imported skill's truth is its central copy, the operator's edits to that copy are the
  skill; there is nothing upstream to reconcile against. Detaching a `local` skill from a vanished
  folder (round 4, ticket 09) is the same state reached by another road.
- The skills table gains a nullable `imported_from_tool` column (schema version 9). Existing
  `local` rows that were really imports are reclassified once on upgrade (ticket 07); a genuine
  `local` folder outside every Tool directory is never touched by that pass.
- Adding a folder that lives inside a Tool's skills directory through "Add from a local folder"
  would recreate the accident; that entry point refuses such a path and steers to Import
  (ticket 08).
