# 02: Propagation module and backend-owned Refresh batch

Status: resolved

Type: task
Source: `../spec.md` Q2, Q3, Q11, Q12; CONTEXT.md **Propagation**, **Refresh (all)**

**What to build:** When a Managed skill's central copy changes, one core module — Propagation — brings every Sync target into line: it reads the skill's global target rows and project assignment rows itself, honours each Tool's capability and each row's Sync mode through the same sync entry point the batch engines use, records outcomes through typed transitions on both tables, and returns per-target outcomes as report data (continue-and-report; one target's failure never fails the operation). The managed-skill update flow becomes "acquire → finalize → propagate" with no inline fan-out. Refresh (all) becomes one backend batch command over a skill set (or all) with `Channel` progress and a `reassert_auto_sync` policy: phase one acquires every skill's bytes, phase two finalizes and propagates each under the mutation guard; with the policy on, skills are also synced to installed Tools they are not yet on. The per-skill update command is deleted; the My Skills Update and Refresh actions each issue one invoke and render the report. Phase separation is deliberate: it is what makes parallel acquisition (ticket 10) a drop-in later.

**Blocked by:** 01

- [x] The update flow contains no target loop; the capability-aware sync entry point is the only way bytes reach a target; the `force_copy` predicate exists once
- [x] Propagation tests against a temp home/central dir/DB cover: symlink target untouched, copy target refreshed, uninstalled Tool skipped, shared-skills-dir group updates every member row, missing central source → typed failure, project copy refreshed, missing project skipped — all as report data
- [x] Global target rows reach `synced`/`error` through typed transitions (no hand-built record literal)
- [x] One `refresh_managed_skills` command; `update_managed_skill` removed from the command registry and bindings; per-skill errors and per-target outcomes arrive as one report; progress ticks come from the backend
- [x] The hook's Refresh no longer calls `syncSkillsToTools` itself; its tests assert report rendering, not call order
- [x] A skill that fails acquisition is reported and excluded from propagation and from the auto-sync re-assert
- [x] i18n keys (EN + ZH) for any new progress/report copy; `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.

Shipped on `arch/02-propagation-refresh` (commit `acd5ebb`, from `main` at `1005731`). Gate green:
`npm run version:check && npm run check` (exit 0) and `cd src-tauri && cargo test --all` (349 passed).

**What shipped**

- `core/propagation.rs` — the Propagation module, spanning both scopes. `propagate_unlocked(store,
  paths, skill_id, content_hash, now) -> PropagationReport` reads its own rows. One `force_copy`
  predicate (`needs_new_bytes(mode, adapter)`) lives here and the two old copies in `installer.rs`
  are gone. Bytes reach a target only through `sync_engine::sync_dir_for_tool_with_overwrite`.
  Global rows are handled one **shared skills dir group** at a time: written once, every member row
  settled and reported. Links are reported `Skipped { LinkFollowsSource }` and left untouched.
- `skill_store::TargetTransition { SyncCompleted { mode, target_path, synced_at } | SyncFailed }`
  + `transition_skill_target` — the global counterpart of `AssignmentTransition`. Propagation uses
  it; no hand-built `SkillTargetRecord` literal remains in the update path.
- `installer.rs` split: `acquire_managed_skill_update` (unlocked, slow I/O; `clone_to_cache` /
  `clone_to_cache_subpath` call expressions moved verbatim for ticket 08's merge) and
  `finalize_and_propagate_unlocked` = `finalize_update` + Propagation. `update_managed_skill_from_source`
  and `UpdateResult` deleted.
- `core/refresh.rs` — `refresh_managed_skills(paths, store, selection, policy, now, on_progress)`.
  Phase one acquires sequentially into a `Vec` of per-skill results (drop-in point for ticket 10);
  phase two takes `mutation_guard::serialized` **per skill** for finalize + propagate + optional
  auto-sync re-assert. Acquisition failures are report data and skip phase two entirely.
- Command `refresh_managed_skills(skillIds: string[] | null, policy, onProgress: Channel)` returning
  `RefreshReportDto` (per-skill status + per-target `PropagationTargetDto`s + counts).
  `update_managed_skill` / `UpdateResultDto` removed from `collect_commands![]` and the bindings.
- Frontend: `useSkillLibrary` Update(one) and Refresh(all) each issue one invoke with a `Channel` and
  render the report; the hook's own `syncSkillsToTools` re-assert is gone. Tests assert report
  rendering (which entries reach `showActionErrors`, which toast copy) rather than call order.
- i18n EN + ZH: `actions.refreshFetchStep`, `actions.refreshApplyStep` (replacing `actions.refreshStep`),
  `errors.propagationFailedTitle`, `status.refreshSummary`.

**Deviations (with reasons)**

1. *A drifting copy on a symlink-capable Tool comes back as a link.* The old inline loop forced
   `sync_dir_copy_with_overwrite`; the capability-aware entry point the criterion mandates prefers a
   link. The row records the mode actually used (typed transition), so it stays truthful, and the
   target can no longer drift. Documented in the module doc and pinned by
   `a_drifting_copy_on_a_symlink_capable_tool_becomes_a_link`. Genuine copy semantics are tested via
   a copy-only registry shadow.
2. *Symlink targets are reported `Skipped { LinkFollowsSource }`, not "synced-untouched".* Skipping
   is the honest word for "no work was needed"; the frontend already ignores skips.
3. *The auto-sync re-assert uses `overwrite_if_same_content`, not `overwrite: true`* (the frontend's
   old refresh policy). Propagation already refreshed every target that has a row, so the re-assert
   only creates targets that never existed; clobbering an unknown same-named directory there would
   be a data-loss risk. A directory in the way is reported (`TARGET_EXISTS`) instead.
4. *No `SignalError` / `CommandError` variant was added.* Skips carry a dedicated
   `PropagationSkipDto` tagged union (`link_follows_source`, `tool_not_installed`, `unknown_tool`,
   `project_unavailable`); only failures carry a `CommandError`, so `describeCommandError` is
   unchanged.
5. *`update_resyncs_project_copy_assignments` (installer test) deleted, and the target half of
   `installs_local_skill_and_updates_from_source` dropped.* Both are now covered at the Propagation
   seam against a temp home, which is where the behaviour lives; the old test drove a target row for
   a tool key the registry does not know (`unknown_tool`), which the capability-aware path
   deliberately skips.
6. *Unknown tool keys on a target row are skipped, not copied.* The capability-aware entry point
   needs an adapter; there is no capability to honour for a key the registry never heard of.
7. `global_sync::sync_skill_into_root` still builds a `SkillTargetRecord` literal — it *inserts*
   rows for new targets rather than transitioning existing ones, so migrating it was not trivial
   (the ticket allows leaving it).

**Follow-ups**

- **Ticket 06 (Onboarding import)**: the import flow's "finalize then propagate" half can now call
  `propagation::propagate_unlocked` instead of any bespoke fan-out, and its per-variant outcomes can
  reuse `PropagationOutcome` / `PropagationSkip` rather than a second report vocabulary.
- **Ticket 09 (git acquisition module)**: `installer::acquire_managed_skill_update` is the whole
  acquisition surface for updates now — one function, no guard, no DB writes except the legacy
  `source_subpath` backfill. The GitHub API fast-path belongs inside it (it still uses the clone path
  only; `fetch_skill_files` remains the fast-path-aware sibling used by explore preview).
- **Ticket 10 (parallel acquisition)**: phase one in `core/refresh.rs` is a plain sequential loop
  producing `Vec<(skill_id, skill_name, Result<AcquiredUpdate>)>`. A bounded pool replaces the loop
  body only; phase two, the guard scope and the report shape stay as they are. Note `StagingDir` is
  moved into phase two, so the pool must hand back the `AcquiredUpdate` values, not paths.
- Consider migrating `sync_skill_into_root`'s record upsert to `TargetTransition` once an
  insert-shaped transition exists (see deviation 7).
- `useSkillLibrary` cyclomatic complexity is now flagged by pi-lens (43); ticket 04 owns that world's
  frontend split.
