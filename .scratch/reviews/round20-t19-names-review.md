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

---

# Round 2 (Astra): fix pass 1 (`e737e928`, delta from `cd3722c8`)

- **Verdict as written:** **FIX-THEN-SHIP**, with three blocking findings under the bar the orchestrator wrote.
- **Round 1 status:** finding 1 is fixed for every collision the app itself can cause, and finding 2 (Edit
  quarantine) is resolved.
- **Gate:** passed, 1,435 frontend tests and 1,150 Rust tests. CI is green on Linux, Windows and web. There is no macOS
  CI job; macOS was verified locally.

## Findings

All three need another process to swap an entry inside the Library while the app is acting.

- **3. P2: the fallback can replace an empty folder swapped in** for the reservation between `mkdirat` and `renameat`
  (`no_replace.rs:139–143`). This is the limit the module already documents.
- **4. P1: a private staging folder replaced by a link** while it is being populated sends the copy through the link,
  and the link itself gets published (`install_finalize.rs:204–221`). Private-dir creation, copy and `drop` go by
  path, not through the held directory.
- **5. P1: rollback with `Published::Yes`** deletes a foreign folder substituted after publication, then restores the
  backup and reports success (`install_finalize.rs:623–629`).
- **6. P3: `move_old_central_aside`** checks a name, then renames onto it. Its home is BACKLOG #50.

Two lesser points: the module doc listed only `ENOTSUP` for macOS (the code also accepts `EOPNOTSUPP` and `EINVAL`), and
on Windows the held handle doesn't pin a path reached through a replaceable reparse point.

## Disposition (orchestrator, 2026-11-06): ship

- **These findings are outside the project's stated scope.** `CONTEXT.md`, in the Artifact removal entry (since 1.3.3,
  #107), says: "Admission is a check before the removal, not atomic with it: a process swapping a folder for a link,
  or mounting one inside the entry, in between is out of scope."
- **The bar didn't restate that scope, and that is the orchestrator's error.** Findings 3–5 apply a stricter scope:
  another same-user process racing the app inside its Library. A process like that can already rewrite every file the
  user owns.
- **What this PR has to guarantee is met.** Nothing the app itself does, and no input it receives (a SKILL.md name, a
  typed name, a rename), places bytes outside the Library or replaces an existing entry.
- **Applied at merge:** the macOS error list in the module doc, and a scope note pointing to BACKLOG #119
  (`eec57f08`).
- **The hardening itself is BACKLOG #119:**
  - owned-directory staging through held handles;
  - rollback that carries the published identity;
  - a fallback that fails closed;
  - the Windows reparse-point binding;
  - finding 6 together with #50.
- **From now on, every review brief states this scope.** Ticket 10 is the first.
