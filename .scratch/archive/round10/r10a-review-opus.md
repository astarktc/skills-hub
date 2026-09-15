# Round-10 wave-A review — Opus seat

HEAD `300f89b` (9 commits on `7a7c519`, linear). Gate verified at HEAD: `version:check` OK (1.2.9),
`eslint` clean, `vitest` 265/265, `npm run build` green, `cargo fmt --check` clean,
`cargo clippy --all-targets --all-features -D warnings` clean, `cargo test --all` 605 passed.

## Standards

**Depth (deletion test).**
- `content_identity` (118 lines, 3 doors + `Source`) — **deep enough**. Deleting it forces every caller to
  re-decide "row or re-hash", which is the whole point; `hash_dir`, the symlink rule, the backfill write and
  the warn-once policy are genuinely hidden. Verified: no `hash_dir` caller survives outside the module.
- `skill_update` (298 lines) — **passes**, but the tail partly *moved* rather than deepened. Q16 asked for
  "one entry"; the module ships **three** doors (`apply_unlocked`, `acquire_update`,
  `UpdateRequest::local`), and `acquire_update` is ~90 lines of git-specific procedure lifted verbatim from
  `installer::acquire_managed_skill_update_from`. The asymmetry is visible in the interface: git bytes are
  built by a module function, local bytes by an associated constructor on the request type. `installer.rs`
  is correctly reduced to Add/listing/preview/backfill (605 lines, `finalize_and_propagate_unlocked`,
  `AcquiredUpdate` and the pass-through wrapper all gone; no stale references anywhere in code or docs).
- `reportOutcome.ts` (356 lines, 7 pure folds) — **deep**. `useSkillLibrary.ts` 900→451 lines,
  `managedSkillsRef` gone (grep: zero hits), no report field read in any hook (`plan.groups` in
  `useAddSkillFlow` is selection input, not a report).

**Breaches / smells.**
- `AGENTS.md` "Frontend presentation logic is pure and lives once" still enumerates only
  `skillPresentation.ts` / `persistedPreference.ts` / `preferences.ts`. `src/lib/reportOutcome.ts` is now a
  peer of exactly that kind and is named nowhere in `AGENTS.md` or `CONTEXT.md` — the next agent has no
  documented reason not to hand-roll a second fold. (Backend docs *were* updated correctly.)
- `content_identity::read`'s warn-once dedupe is a process-global `OnceLock<Mutex<HashSet<String>>>` that
  never clears. Ambient mutable state in `core/`, which this codebase otherwise refuses (explicit roots
  everywhere); it also makes log behaviour test-order-dependent and grows once per distinct error string.
- Duplicated code: `skill_update.rs` re-inlines `is_skill_dir(..) → SkillInvalid{missing_skill_md}` instead
  of reusing `installer::ensure_installable_skill_dir` (still present, still used by Add).
- `apply_unlocked` re-destructures `UpdateBytes` in a nested match with an `unreachable!()` arm — a
  switch-on-a-tag smell inside the module that owns the tag.
- `refreshOutcome`'s toast is a 5-deep nested ternary (Fowler: complicated conditional); the rest of the
  file is clean.
- `commands/mod.rs` correctly extracted `to_propagation_target_dto` and reuses it for Edit — wiring only,
  no logic leak. Error contract respected: skips are report data + typed DTO, and the impossible-state bail
  in `set_invocation_override` ("edited skill changed under guard") is prose→`OTHER`, which AGENTS.md
  explicitly sanctions for a condition the UI cannot produce.

## Spec

**B1 — met.** `record` / `read` / `same_content` with `Source::{Managed,Directory}`; `hash_dir` private
(no external caller). `propagate_unlocked` dropped `content_hash` and asks `read` **once per propagation**
(`propagation.rs:116`), not per target — `propagate_one_assignment` reads `skill.content_hash`. I/O failure
→ `None` with status unchanged is pinned in `tests/content_identity.rs` across `Synced/Stale/Error`;
backfill-exactly-once is counted with a SQL trigger. Backlog 15 done: the 30 s `try_serialized` poll is
replaced by `project_sync::reconcile_listing_unlocked`. `.skills-hub-manifest-` was **folded** into
`skill_files::is_ignored` (the ticket's first option) — a side effect is that manifest temp files now also
vanish from `list_files`, which is right but undocumented.

**Onboarding migration is not scope creep**: `same_content` takes a `Source`, so `onboarding_import`'s two
call sites *had* to move; commit `3db3adc` isolates it.

**B2 — met.** Admission compares `source_ref` / `source_subpath` / `source_type` against the row re-read
under the guard, and `repoint == true` bypasses the stale check exactly as Q4 words it; `SkillGone` is
checked first and is not bypassed. `admission_discards_staging_for_deleted_or_changed_sources` proves all
four cases discard the staging dir, leave central bytes and write no target rows;
`admission_preserves_current_non_source_fields` proves unrelated current fields win. Acquire-first holds on
every adapter (no `upsert_skill` before finalize in `acquire_update` or `UpdateRequest::local`). Local
Re-point is atomic: `validated_local_repoint` only validates, the new `source_ref` rides in the record, and
`failed_local_repoint_preserves_source_and_old_bytes` asserts whole-record equality **and** old bytes for
both a staging fault and a settle fault. Local staging happens in the acquire closure, outside the guard
(`local_bytes_are_acquired_before_apply_and_do_not_follow_later_source_edits`). Edit returns
`InvocationEditOutcome { entry, propagation }` and no longer logs target failures
(`edit_returns_a_failed_copy_target_in_its_propagation_report`); the guard is held once — `serialized` →
`apply_unlocked` (unlocked seam), no entry point calls another (grep of all `serialized` sites confirms).
D2/ADR-0004 rollback tests are byte-for-byte present (8 rollback/failure tests in `install_finalize.rs`,
`skill_edits.rs` replay tests intact). `refresh.rs` keeps pool + guard + reassert + report assembly only.
Backlog 13 done: the replay IIFE is `apply_replay`; `roll_back_update` → `restore_central_bytes` (one
spelling).

**F1/F2 — met.** Six folds + `installOutcome`/`deleteOutcome`; actions carry `{skillId, skillName}` and the
hook resolves at click time against the current render's list —
`resolves a notification id against a replaced row at click time` proves it with a replaced row. Per-action
hook coverage is a 12-row `it.each` over Update / Restore / both Re-points / Refresh-all / unsync-all /
unsync / delete / sync-to-all / auto-sync / toggle-on / toggle-off, each asserting fold→reporter and
`completion` (reload called or not, modal closed or not) in both directions, plus a 4-row thrown-request
table. vitest 237→265. i18n: two new keys, EN + ZH, no hardcoded copy.

**Behaviour changes vs v1.2.9 — each judged.**
1. *Refresh-all toast when only targets failed*: was success `refreshCompleted`, now warning
   `partialFailure`. **Correct** — the old toast contradicted the error panel.
2. *Conflict now outranks failure in the toast* (old: only when `failed===0 && skipped===0`).
   **Correct** — literally the ruling's precedence, and counts remain in the panel.
3. *Single Update/Restore/Re-point failure*: was `action.fail(msg)` → one error toast; now an error entry
   (which `showActionBatch` still toasts) **plus** a warning summary toast "Updated 0, failed 1".
   **Not a regression** (content is richer) but **double-notifies** a batch-of-one. Follow-up.
4. *Local Re-point now reloads on failure and honours conflicts* — the documented fork closure. **Correct.**
5. *Git Re-point modal now also stays open when the batch skipped* (`closeModal` adds `!skipped`).
   **Correct** — a skipped Re-point repaired nothing.
6. *sync-to-all / auto-sync partial failure*: was a success toast + error toast, now error toast only.
   **Correct** (the old success toast was a lie); the fully-successful path keeps `status.syncCompleted`.
7. *Explicit toggle failure*: was `action.fail`, now an error entry — still toasted by `showActionBatch`
   with a better title. **Correct.**

**Integration commit.** `ACQUISITION_SKIP_KEY` sits beside `SKIPPED_REASON_KEY` /
`UNLOCATABLE_STATE_KEY` in `skillPresentation.ts` — **right home**. Consuming the variant in wave A
deviates from Q13 ("consumed in a wave-B follow-up") but is forced: without the branch the fold's `else`
would read `status.targets` off the widened union and fail `npm run build`. Minimal and separately
committed. The Edit `propagation` field is correctly *not* consumed yet (`useSkillLibrary.ts:88` reads only
`.entry`), per Q13.

**Worktree safety.** `git diff 7a7c519..HEAD -- src-tauri/src src/` shows no reverts: the 9 commits are
linear on the fixed point, v1.2.9's D1/D2 rollback tests, `unlocatable` states and the `artifact_removal`
/ `mutation_guard` / `tool_adapters` files are untouched. Working tree clean; bindings committed and
regenerate identically (`cargo test --all` dirtied nothing).

## Summary

**Blocking** — none. I could not produce a data-safety bug, a spec miss, or an operator-visible regression
at HEAD; every claim in the brief verified, and the full gate is green.

**Follow-up**
1. **`AGENTS.md` invariant gap (do this first):** add `src/lib/reportOutcome.ts` to the "Frontend
   presentation logic is pure and lives once" bullet, and record the unified precedence rule
   (conflict › failure › skipped › success; every returned report reloads; only a fully refreshed batch
   closes a repair modal). Right now that rule exists only as a comment in `reportOutcome.ts:44-49`.
2. **`same_content` now trusts the row for the source side.** A central copy edited outside the app makes
   `overwrite_if_same_content` believe a divergent target is identical and clobber it. This follows Q2
   ("readers trust the row") so it is not a breach, but it is a new way to lose an operator's hand-edited
   target copy; worth an explicit note in the ADR or a reconcile-driven invalidation in wave B.
3. **`content_identity::read`'s global warn-once set** — replace the `OnceLock<Mutex<HashSet<String>>>`
   with a caller-supplied sink or drop the dedupe; ambient process state in `core/` is against the grain of
   every other module here.
4. **Batch-of-one double toast** (change 3 above): suppress `refreshOutcome`'s count summary when
   `ctx.single` is set and `out.errors` is non-empty.
5. **`skill_update` interface asymmetry** — fold `UpdateRequest::local` into a module-level
   `acquire_local(…)` beside `acquire_update`, or make `acquire_update` dispatch both, so the module has
   one acquire door and one apply door.
6. **Small cleanups**: reuse `installer::ensure_installable_skill_dir` instead of the inline copy in
   `skill_update.rs`; flatten `apply_unlocked`'s nested `UpdateBytes` match to remove the `unreachable!()`;
   note in `is_ignored` that folding `.skills-hub-manifest-` in also hides those temps from
   `skill_files::list_files`.
