# Round 20 ticket 03 (Move Library folder) — adversarial review

## Round 1 — GPT-6 Astra (high), 2026-11-06, `be610918` vs `main` `771d0fd`: **BLOCK**

The reviewer proved each finding with a temporary core probe test. The probes were discarded afterwards.

1. **P1, case-colliding destination names.** `library_move.rs:563–589` (`unique_name`, `copy_verified`) checks
   uniqueness in an `OsString` set, not on the destination filesystem. On case-insensitive APFS, `alpha` and `ALPHA`
   name one directory. When two recorded skills have those names, the second copy overwrites the first *after* the
   first was verified. Both records are switched and both originals removed, so one skill's bytes are lost. Probe
   `review_probe_case_colliding_copy_names_preserve_both_skills` failed (beta's new `SKILL.md` held alpha's bytes).
2. **P1, the move drops `.git` and internal symlinks, then deletes the original.** The move reuses the deploy copy
   routine, which excludes `.git` and internal links. Content identity excludes them too, so verification passes. Clean
   then removes the whole original. Probes `review_probe_preserves_git_history_and_internal_links` ("move deleted the
   only git history") and `review_probe_preserves_internal_link` both failed.
3. **P1, Clean can delete another skill's retained, still-linked copy.** `clean_phase` / `still_reached`
   (`:922–953`, `:1005–1030`) decide retention per skill id, but deletion acts on a physical directory. Suppose alpha
   records `central/alpha` and beta records `alias-to-central/alpha`. Beta is retained as `still_linked`, and then
   alpha's clean deletes the same physical copy. Probe `review_probe_other_skill_alias_retains_still_linked_old_copy`
   failed: the report said `StillLinked` while the link dangled.
4. **P2, a crash after Switch loses the recovery information.** `:283–293` and `:630–644`: the old central paths are
   overwritten and nothing durable records the pending re-points. Retrying to the same destination is refused. A move to
   a third folder classifies the original links as `not_ours`. Propagation skips owned link rows (`LinkFollowsSource`,
   `propagation.rs:280–295`), so harnesses read stale bytes from then on. Probe `review_probe_crash_after_switch_is_recoverable`.
5. **P2, case-insensitive admission fails for protected dirs that don't exist yet.** `physical()` (`:375–385`,
   `:445–464`) appends the nonexistent components literally and compares with a case-sensitive `starts_with`. With no
   `~/.claude/skills` on disk yet, the destination `~/.CLAUDE/SKILLS` is admitted, and the harness's own skills dir
   becomes the Library. Probe `review_probe_missing_case_alias_harness_destination_is_refused`.

**Deviations accepted:**

- one `LIBRARY_DESTINATION_REFUSED` with a reason enum;
- the read-only `preview_library_move`, since the move re-admits under the guard;
- `.DS_Store` ignored for emptiness (it doesn't license dropping other content);
- retaining `still_linked` copies (policy accepted; the implementation is broken by #3);
- the "cloud storage folder" wording.

**Sound:**

- one guard section, with no nested entry point;
- Switch is one SQLite transaction (a rollback probe passed);
- existing symlinked destinations are refused;
- a case alias of an existing old root is refused;
- ordinary re-point, copy-mode, not-ours, disabled, missing-project and Unlocatable paths;
- copy-failure rollback;
- unrecorded root content survives;
- cleanup goes through `artifact_removal` and the fence;
- typed errors, bindings, the fold and the lifecycle wiring.

Verification: move tests 22, switch tests 2, removal-fence tests 21 (+1 ignored), frontend 1,331, build ok.

## Disposition

Round 2 (Opus 5.5, high) fixed all five on PR #12, head `5e8dd507`. Each fix has a regression test that failed on
`be610918` (#4's resume tests could not compile there). CI is green.

1. Exclusive `create_dir` reservation, with `-2`, `-3` suffixes for a taken name.
2. A relocation-fidelity copy that keeps every entry, `.git` and links as text, verified over the full tree. A skill
   that can't be copied faithfully is `kept { unrelocatable }`.
3. Clean decides by physical identity (`removal_fence::Place`): `still_recorded`, `still_linked`, or
   `kept { shared_copy }`.
4. A move journal (settings row) is written in the Switch transaction. It is resumed at startup and at every
   `move_library`.
5. Admission runs after creation, by identity. A refusal removes only the empty dirs the move created.

Known gap: the preview still counts kept skills as moving, so its "Move N skills" can overstate. Round-2 review
dispatched.

## Round 2 — GPT-6 Astra (high), 2026-11-06, `5e8dd507`: **BLOCK**

Round-1 status:

- #1 and #5 are closed.
- #2 and #3 are closed for their reproducers.
- #4 is not fully closed: the automatic resume introduced new deletion paths.

Nine probes failed.

1. **P1, Clean deletes unverified bytes.** Verification happens before Switch and is not repeated at deletion:
   - Resume after the destination was deleted removes the only copy.
   - An operator edit made after verification is deleted.

   Probes `review_probe_resume_missing_destination_preserves_last_copy`, `review_probe_edit_after_verify_survives_clean`.
2. **P1, the journal's path strings authorise removal of whatever occupies them now.** Two cases:
   - A replacement directory at the old path is deleted.
   - An edited journal `old` value outside the Library is deleted.

   `journal.to` is not checked against the configured root. Probes `review_probe_stale_journal_does_not_remove_replacement_directory`,
   `review_probe_journal_cannot_authorize_unrelated_removal`.
3. **P2, a kept, unrelocatable skill's absolute link into another old copy dangles.** The other skill moves and its old
   copy is removed. Probe `review_probe_kept_skill_link_dependency_survives`.
4. **P1, Clean modifies a copy-mode deployment that contains an old copy.** Only `at.within(old)` is checked, not the
   reverse. Probe `review_probe_copy_deployment_containing_old_copy_is_untouched`.
5. **P1, the fidelity copy drops directory permissions (`0700` → `0755`) and directory xattrs.** File xattrs and resource
   forks survived.
6. **P2, hardlinks become independent files.**
7. **P3, the preview overcounts** kept and shared copies.

## Round 3 direction (orchestrator)

One principle: **delete only what is provably duplicated, at the moment of deletion, and only inside the old Library
folder.** The parts:

1. **Scope.** Clean removes only direct children of the old configured Library root. Old copies recorded elsewhere (a
   split Library) are copied and switched, then left in place and reported.
2. **Identity.** The journal records the old and new copy identities (device+inode, or the Windows file id).
3. **Authority.** Resume requires all of these:
   - the configured root equals `journal.to`;
   - the record equals `entry.new`;
   - both identities still match;
   - the old copy is a direct child of `journal.from`.

   Otherwise nothing is removed, the copy is reported left behind, and the entry is cleared.
4. **Removal.** Rename the old copy aside within the old root, then re-verify it against the new copy over every
   compared facet. Delete it if identical; otherwise rename it back and keep it.
5. **Retention.** Retention checks deployments in both directions, and also links inside every retained skill.
6. **Fidelity.** Extended to directory modes, directory and file xattrs, and hardlink groups (recreated inside the
   skill; a link outside the skill is unrelocatable).
7. **Preview.** Classifies kept skills read-only.

## Round 3 (Opus 5.5, high): PR #12 head `011f13d`

All seven round-2 findings were fixed as the direction above set out. All 13 regressions failed on `5e8dd507`.

Compared facets:

- the tree, with links never followed;
- bytes and link text;
- mode bits on files and directories;
- xattrs on files, directories and links (macOS excludes `com.apple.provenance` and `com.apple.rootless`);
- hardlink structure.

Residuals: ACLs, Windows ADS, sparse holes, BSD flags, ownership and creation times. CI is green, and rust-windows ran
908 tests. Round-3 review: two independent reviewers (Astra and Sol) in parallel.

## Round 3 reviews, 2026-11-06, `011f13d`: **fix-then-ship** from both

Astra and Sol (GPT-6.1) reviewed in parallel and independently. Both say all seven round-2 findings are closed, and
neither found a P1.

- **A1 / S4 (P2, both found it): a set-aside old copy loses its skill identity on recovery.** Resume checks the new
  copy's identity before looking in `.skills-hub-clean-*` (`library_move.rs:542–568`). The cases:
  - new copy missing: the last copy is left hidden, reported only as an anonymous `unrecorded` entry, and the journal
    is cleared;
  - old path replaced: the same loss of identity.

  Probes `review_aside_missing_destination_restores_last_copy` and `review_aside_replaced_old_is_reported_with_skill_identity`.
- **S3 (P2): startup drops the resume report** (`lib.rs:162` matches `Ok(Some(_))`). `changed_since_copy`,
  `still_linked` and failed re-links never reach the operator.
- **S1 (P2): an error after Switch leaves the UI stale.** `useSettingsState.ts:271–275` adopts path and catalog only
  on resolve. A failure after Switch (journal clear, catalog read) leaves Settings showing the old root, and the
  inspector shows removed paths.
- **S2 (P2): project views are never refreshed after a move.** A failed project re-link (`error` in the DB) still shows
  as Deployed in the matrix, and `ProjectsPage.tsx:128` does not re-read an already-selected project.
- **A2 (P3): the preservation limits need a user-facing contract.**
  - Windows alternate data streams can hold unique bytes and are not verified.
  - Linux non-`user.*` xattrs (ACLs, capabilities) are outside the compared set.

  These should be named in the CHANGELOG, and a report warning raised when such content is actually detected.

Accepted limit, from both: rename-aside is not isolation from writers. An already-open file descriptor can still write
into the set-aside copy. This is a concurrent-writer case, not an ordinary workflow. Say "re-verified before removal",
not "atomic".

## Round 4 direction (orchestrator)

1. **A1 / S4:** before deciding, resume locates the journaled old identity at both its original path and in any
   `.skills-hub-clean-*` dir.
   - If the old path is free, restore the copy there.
   - Otherwise leave it where it is and report it with the skill's id and its actual path (`kept { set_aside }`).
   - Never reduce an app-created recovery copy to anonymous `unrecorded`.
2. **S3:** persist the resume report at startup, and add a command that hands it to the frontend once. The frontend
   folds it through `reportOutcome` and records one Notifications run. It shows a toast only when the report has
   warnings or failures.
3. **S1:** on a thrown `move_library`, reconcile through the failure path: re-read settings (the storage path) and the
   catalog (`loadManagedSkills`).
4. **S2:** after the move settles, success or failure, App's project world re-reads the selected project's view and
   the placements. The settings world gets this as an interface App passes in, never by import.
5. **A2:** any source entry carrying content the comparison can't verify keeps its skill in place as
   `kept { unrelocatable: unverifiable_metadata }`:
   - Windows: a non-default ADS;
   - Linux: `system.*` / `security.*` xattrs, e.g. a POSIX ACL or file capabilities;
   - macOS: an extended ACL (if cheap to detect).

   The CHANGELOG names the residuals.

## Round 4 (Opus 5.5, high): PR #12 head `1577061`

Every finding below was fixed as the round-4 direction set out, with a regression test that failed on `011f13d`.

- **A1/S4:** resume finds set-aside copies before it decides anything. A copy goes back to its old path when that
  path is free; otherwise it is reported `set_aside` with the skill id.
- **S3:** the resume report is persisted in the transaction that drops the journal, and `take_resumed_library_move`
  hands it out once. One Notifications run is recorded; `applyOutcome { background }` shows a toast only on a warning
  or error.
- **S1:** after a throw, settings and the catalog are re-read.
- **S2:** `useProjectState.reloadViews` reaches Settings as a `Pick<>`, and runs after every move.
- **A2:** content the move cannot verify keeps the skill in place as `unverifiable_metadata`:
  - Windows named alternate data streams;
  - Linux non-`user.*` xattrs (SELinux and SMACK labels excepted);
  - macOS extended ACLs.

Gate green (Rust 1,000, vitest 1,367); CI green (rust-windows: 926 tests). Round-4 re-check dispatched (Astra).

## Round 4 re-check — GPT-6 Astra (high), `1577061`: **BLOCK** (2)

Closed: A1/S4, S3 and S1. Not closed:

1. **P1:** resume still deleted an old copy carrying metadata added while the app was stopped (a macOS extended ACL, a
   Windows named stream). The A2 check ran only at classification; `same_tree` ignores that content. Probe
   `review_r4_metadata_added_while_app_is_stopped_is_not_deleted`.
2. **P2:** `reloadViews` captured the selected project, so a project picked during the move kept a stale matrix.

## Disposition

Both fixed by the orchestrator (PR #12, `3e84def`), each regression test written first and red before the fix:

1. The clean runs `carries_unverifiable_metadata` over a set-aside copy right before removal. Content added since the
   copy keeps the old copy as `changed_since_copy`. Test: `metadata_added_after_the_copy_is_never_deleted`.
2. `reloadViews` reads the selection through a ref when it runs. Test: `re-reads the project selected when it runs,
   not when it was handed out`.

Gate green (Rust 1,018, vitest 1,376); CI green. **Merged `26ce2b6`.** The final release review (`v1.3.3..main`)
re-verifies both fixes.
