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

Round 2 dispatched to an implementer on top of `t3/move-library-folder` (orchestrator, 2026-11-06).
