# Round 20 t19 — skill names are one folder name: review

- **PR:** #23 (`t3/skill-names-folder`)
- **Commit reviewed:** `cd3722c8` against `origin/main` `6d6472fe`
- **Round 1:** GPT-6 Astra (high), 2026-11-06. The first attempt was stopped by OpenAI's content filter; the retry
  ran with plain wording.
- **Verdict:** **fix-then-ship**.
- **CI:** green on all three jobs.

## Findings

### 1. P1, blocking: publishing a new install can overwrite or follow a destination that appeared after admission (predates this PR)

`admit_library_entry` checks that the destination is absent and that its parent is physically the Library, but it
reserves nothing. Publishing then goes through `StagingDir::move_into` (`install_finalize.rs:161–174`, `:234–254`,
`:267–284`), which calls `rename` and, on any rename error, falls back to `copy_dir_recursive` into the destination.

Reproduced at helper level, by occupying the destination between admission and `move_into`:

- **An existing non-empty folder:** its `SKILL.md` is overwritten, and the call returns `Ok`.
- **A symlink to an outside folder:** the staged files are written outside the Library, and the call returns `Ok`.

**Fix:** publish a new install exclusively, never replacing anything. A destination that is occupied, or of the wrong
type, is a refusal, never a reason to copy into it. If publishing across devices is needed, copy into a private
staging folder on the destination filesystem, then publish exclusively. Clean up only what the operation created. Add
both interleavings as regression tests.

### 2. P2: direct Edit bypasses the quarantine on invalid records

Edit writes the recorded manifest through `UpdateBytes::EditInPlace` without checking `record.name`
(`skill_update.rs:183–190`; `skill_edits.rs:88–123`, `:137–159`). Reproduced: a legacy record named `../escaped`
accepts `set_invocation_override`, and the outside `SKILL.md` changes.

**Fix:** reject an invalid record name before persisting the Edit row, and enforce the same rule at the shared Update
apply boundary.

## Results

- **Every path that creates or renames a Library folder** goes through the one rule:
  - git Add, local Add, Explore and onboarding import (both renames and group names) all converge on it;
  - Re-point and Edit never rename;
  - the Library move reserves its own leaf with an exclusive `create_dir`, and no bypass was found there.
- **`SKILL_NAME_INVALID` works end to end** and is raised before any download, cache entry or staging folder.
- **A derived name that fails the rule** is itself refused, never trusted.
- **Physical admission** is correct on a stable filesystem: a dangling link counts as taken, and an aliased Library
  root is accepted.
- **Quarantining existing records by name** is sound, given split Libraries. Delete removes the recorded copy through
  the existing admission, and the probe passed, so Remove works as the repair.
- **Both behaviour changes are acceptable.**
- **Gate:** passed (1,435 frontend tests; 1,136 Rust tests, 1 ignored).

## Disposition (orchestrator, 2026-11-06)

Fix pass 1 takes both findings: finding 2 is small and part of the same quarantine.
