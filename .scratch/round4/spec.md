# Round 4 — Refresh failures you can act on (post v1.2.3)

Source: `source-12-followups.md` (19 verified, non-blocking items from the round-3 review panel and the
operator's smoke test of an 86-skill library) plus two findings from the operator's smoke test of the
built 1.2.3 on 2026-09-05: (a) Onboarding import left the Pi original of `plane-fallback` untouched when
the group's chosen variant was Claude's symlink to it; (b) tracing that led to the root cause of most
"stale rows" — an imported skill is recorded with a Tool path as its *source*, which the import itself
then replaces with a link or deletes. Decisions below were settled with the operator on 2026-09-05 at HEAD
`f057190` (main, v1.2.3).

Vocabulary is in `CONTEXT.md`. This round adds **Provenance** (git / local / imported) and
**Unlocatable skill**; ticket 06 and ticket 09 add them to the glossary, and ticket 06 records an ADR.

**Status:** ready-for-agent

## Problem Statement

Refresh (all) over the operator's real library fails the same 12 skills every time and always will:

- 5 skills whose upstream publishes them as **in-repo symlinks** (an aggregation bundle such as
  `tanstack-all/skills/<name> → ../../<name>/skills/<name>`). A sparse fetch of the alias directory
  yields a dangling link; the Contents API answers `type: symlink`. Install worked in the days of full
  clones (the central copies are real); Refresh never has, and since round 3 made Add sparse too, a
  fresh Add of such a skill fails as well.
- 7 skills whose recorded source no longer exists: skills **imported from a Tool directory** whose
  original was then removed (auto-sync off) or replaced with a link back to the central copy (auto-sync
  on — a self-referential "refresh"), plus rows migrated from a Windows/WSL database whose Tool paths do
  not exist on this machine.

The failures surface as raw `OTHER` strings that leak absolute cache paths, and the card offers nothing
to do about any of it. Separately, an import whose chosen variant was a *symlink* into another Tool's
copy left that other Tool's real copy behind as an untracked duplicate, and the round-3 review left a
list of small same-rule-twice / dead-seam / test-flake items.

## Solution

An imported skill has **no external source**: the central copy is its source of truth, it is never
in a Refresh batch, and legacy rows that were really imports are reclassified once on upgrade — most of
the seven fix themselves with zero clicks. Acquisition follows upstream symlinks at fetch time, so the
five refresh. What remains unlocatable (a genuine external folder that vanished; a central copy that
vanished) is shown on the card with the actions that fix it, and Refresh skips it with a count instead
of failing it. Missing-path failures become typed errors with their own copy. Import takes over every
byte-identical original in a group, never just the chosen one, and Add refuses a folder that already
lives inside a Tool's skills dir, pointing at Import instead. The review residue is cleared in
file-partitioned lanes.

## User Stories

1. As an operator, I want a skill published upstream as an in-repo symlink to refresh, so that my
   TanStack bundle skills stop failing every Refresh.
2. As an operator, I want to Add such a skill fresh from Explore and get its real content, so that
   the sparse-clone optimisation never hands me an empty directory.
3. As an operator, I want the skill's recorded subpath to stay the alias I chose (not the resolved
   target), so that if the maintainer reorganises the repo and re-points the alias, I follow it.
4. As an operator, I want a symlink whose target escapes the repository refused with a clear error,
   so that a hostile or broken repo can never make the app read outside the checkout.
5. As an operator, I want a skill I imported from a Tool directory to be shown as "managed here",
   with no Update button, so that I am not invited to refresh something that has nothing to refresh
   from.
6. As an operator, I want the Tool the skill was found in kept as display-only history on the card,
   so that I remember where it came from without the app treating that path as a source.
7. As an operator, I want Refresh (all) to leave imported skills out of the batch entirely, so that
   my summary counts only skills that can actually be refreshed.
8. As an operator upgrading from 1.2.3, I want my old imported skills recognised and reclassified
   automatically, so that the rows that failed every Refresh stop failing without my doing anything.
9. As an operator, I want a `local` skill whose folder is genuinely my own (outside every Tool dir)
   left exactly as it is by that reclassification, so that upgrading never changes a real source.
10. As an operator, I want Add → local folder to refuse a folder inside a Tool's skills dir and tell
    me to use Import, so that I never again create a skill whose "source" the app will overwrite.
11. As an operator, I want a skill whose local source folder is gone marked on its card, so that I
    see the problem when I open the app rather than after a failed batch.
12. As an operator, I want to re-point such a skill at the folder's new location, so that a move on
    disk is a one-click fix.
13. As an operator, I want to detach such a skill from its vanished source (central becomes truth),
    so that a skill I no longer maintain externally keeps working everywhere.
14. As an operator, I want a skill whose central copy is gone marked on its card, so that I know
    every Tool's link for it is dangling.
15. As an operator, I want to restore a git or local skill whose central copy is gone from its
    source, so that the links come back to life without a delete-and-re-add.
16. As an operator, I want Refresh (all) to skip unlocatable skills and tell me how many it skipped
    and why, so that real failures are visible again and no dangling links are minted by auto-sync.
17. As an operator, I want "source path not found" / "central path not found" / "path not found in
    repo" failures shown as their own messages with the path as a detail, so that I read what went
    wrong instead of an absolute cache path.
18. As an operator, I want an "Open log folder" failure shown as its own message, so that a rare
    platform quirk is legible.
19. As an operator importing a skill that exists in two Tools (one a real directory, one a symlink to
    it), I want the real directory chosen as the source, so that a link is never what the app copies.
20. As an operator importing with auto-sync on, I want every byte-identical original in the group
    replaced by a link to the central copy — not just the chosen one — so that no Tool keeps an
    untracked duplicate.
21. As an operator importing with auto-sync on, I want a divergent sibling left in place and reported,
    so that sharing a name never destroys different content.
22. As an operator, I want batch error entries to read top-down in the notification panel in the
    order they happened, so that the first failure of a batch is at the top of its block.
23. As an operator, I want copying a skill's path not to fill the notification history with "Copied",
    so that the panel stays a record of outcomes that matter.
24. As an operator, I want a fourth open error toast visible without hovering, so that stacked
    failures are not hidden.
25. As a maintainer, I want the `refresh` overlap test to assert concurrency by observed ordering, so
    that the gate stops flaking under parallel cargo load.
26. As a maintainer, I want the adapter and skill resolved once per assignment during Propagation and
    project sync, so that one lookup rule lives in one place.
27. As a maintainer, I want a row's own Tool always part of its shared-dir group, so that a test
    override of the registry cannot drop it.
28. As a maintainer, I want one subpath normaliser, so that the cache key and the fetcher agree by
    construction.
29. As a maintainer, I want the notification ring in its own hook and one clipboard helper, so that
    the reporter has one reason to change.
30. As a maintainer, I want project components to hand errors to the reporter's own formatter, so that
    the `if (msg)` dance is written once.
31. As a maintainer, I want the log-reveal target rule to be a pure, tested core function, so that the
    `.app`-suffix quirk is regression-guarded.
32. As a maintainer, I want the three provenances and "central is truth for an imported skill" written
    down in the glossary and an ADR, so that the next import-shaped feature does not reinvent the
    accident.

## Settled decisions (do not re-litigate; reopen via a comment on the ticket)

| # | Decision |
|---|---|
| Q0 | Round lives in `.scratch/round4/`; round-3 dir is closed. Source ticket copied here as `source-12-followups.md` (read-only reference). Version for the round: **1.2.4** (one feature = patch; minor bumps are reserved for whole-app overhauls). |
| Q1 | **Three provenances**: `git` (external repo; refreshable), `local` (an independent folder the operator maintains — outside every Tool skills dir; refreshable; the original is **never** touched), `imported` (taken over from a Tool's skills dir; the **central copy is the source of truth**; not refreshable; `source_ref` is `None` — where it was found is kept as display-only history in its own field). `source_type` stays a string column (`"imported"` is a new value); the found-in Tool lives in a new nullable `imported_from_tool` column — schema **V9** (amended at ticket 06; spec originally said 8). |
| Q2 | **Import records `imported`.** The `.skill-lock.json` upgrade to `git` (a Tool copy installed by `npx skills add`) stays — a discovered upstream is a real source. Refresh (all) and single Update never include an imported skill; the card says "Managed here" and offers no Update. |
| Q3 | **Import takes over every byte-identical original in the group** with auto-sync on: every Tool holding a variant whose fingerprint equals the chosen variant's is force-included in the target set and overwritten in place (the round-3 rule generalised from "the chosen variant's Tool" to "every identical variant's Tool"); divergent siblings are left in place and reported, exactly as the auto-sync-off path already does. The report names every Tool included beyond the policy. |
| Q4 | **Default variant** for a consistent (no-conflict) group prefers a real directory over a link; ties fall back to registry order as today. Frontend-only rule; the backend still admits whatever path the selection names. |
| Q5 | **Legacy reclassification** runs once at startup as a core function taking `(store, home, central_dir)` — not a schema-version migration (`ensure_schema` is SQL-only by design). **Amended at ticket 06:** the found-in Tool needs its own column (`imported_from_tool`), so the schema is **V9** — the column is the schema change; the reclassification pass is still a separate startup core function. Ticket 06 also fixed a pre-existing bug: `ensure_schema`'s incremental branch never wrote `user_version` (the operator's live DB read 7 while the app was at 8), so every `ALTER TABLE` migration would have failed on its second launch. A `local` row is reclassified to `imported` when its `source_ref` (i) is inside any Tool's skills dir under `home`, or (ii) resolves through a symlink into `central_dir`, or (iii) does not exist **and** has the shape of a Tool skills-dir path (matches an adapter's relative skills dir, any home prefix — covers migrated WSL/Windows rows). Anything else is left alone. The pass is idempotent and logged (count reclassified). |
| Q6 | **Add → local folder refuses Tool-dir paths and steers to Import**: a typed `SignalError` → new `CommandError` code with EN/ZH copy that names Import as the way to take over a skill already in a Tool. The predicate is the registry's (the inverse of `ensure_path_within_tool_dirs`), never a hand-rolled path check. |
| Q7 | **Unlocatable skill** — two states, computed at listing time from the stored paths (no roots needed, nothing persisted): `source_missing` (provenance `local`, `source_ref` does not exist) and `central_missing` (`central_path` does not exist). Card badge + actions: source missing → **Re-point** (folder picker → new `source_ref` → the single-skill Update pipeline) / **Detach** (→ `imported`) / **Remove**; central missing → **Restore** (git/local: the single-skill Update pipeline, which must rebuild a missing central dir) / **Remove**. Refresh (all) does not dispatch unlocatable skills; the batch summary reports the skipped count as a **warning**-kind line with one entry per skipped skill. Editing a git skill's URL/ref is out of scope. |
| Q8 | **Acquisition follows upstream in-repo symlinks at acquire time, every time**; the recorded `source_subpath` stays the alias the operator chose. Clone path: when the requested subpath (or a component of it) is a symlink, add the target to the sparse set and re-checkout, following chains to a bounded depth; copy the resolved directory. Contents API path: a `type: symlink` entry is followed via its `target`, relative to the entry's directory. A target that is absolute or escapes the repository root is refused with a typed `SignalError` (never read). The resolved path is logged as diagnostics, never written to the record. |
| Q9 | **Typed missing-path errors**: `SourcePathMissing { path }`, `CentralPathMissing { path }`, `SubpathMissing { subpath }` (the "path not found in repo" case — reports the requested subpath, never the cache-internal absolute path) and `RevealLogFailed { detail }`, per ADR-0001 (Rust variant + regenerated binding + `describeCommandError` branch + EN/ZH keys). Paths travel in structured fields as `detail`, never in prose. `log_reveal_target` becomes a pure core function with a test for the `.app`-suffix rule. |
| Q10 | **Review residue lanes** (source items): 03 = #4 #10 #11 #12 #16 #19; 04 = #5 #7 #8 #9 #14 #15 #18; 02 also takes #13. Item #11: include `row.tool` unconditionally in its group. Item #4: assert overlap by observed ordering (barrier / observed concurrency ≥ 2), never elapsed time. Item #14: copy-success is **not** a Notification (toast only, not recorded). Item #5: `visibleToasts` 5. |
| Q11 | **Execution**: every ticket delegated to a Fable 5.1 child (`providerInstanceId: "pi"`, `model: "anthropic/claude-fable-5-1"`, `runtimeMode: "full-access"`, `mode: "async"`), one ticket per child, one worktree each (`r4/<NN>-<slug>`), `npm ci` serially. Wave 1 = 01–05 in parallel (file-partitioned); 06 after 04 and 05; 07 and 08 after 06 (08 also after 01); 09 after 07; 10 last. Merge ritual, gate and model routing as in round 3 (`npm run version:check && npm run check`; rebase → diff → gate → `--ff-only` → deletion check). The operator owns the `tauri:dev` smoke test on the real library and the Release confirmation. |
| Q12 | End-of-round 3-model review panel (Standards + Spec axes) before the version bump, same brief shape as round 3 (`review-brief-round3.md`). |

## Implementation Decisions

- **Provenance** is a stored fact on the skill record: `source_type ∈ {git, local, imported}`. Imported
  rows carry no `source_ref`; the Tool they were found in is kept in a new display-only field. The
  managed-skill listing DTO exposes it; the frontend's single `sourceKind` rule becomes three-way and
  every card/detail decision (icon, repo label, Update availability) derives from that one rule.
- **Refresh membership** is one predicate — "is this skill refreshable?" — owned by core and consulted by
  Refresh (all), single Update and the UI's Update affordance. `imported` and unlocatable skills answer
  no; the summary distinguishes "not in the batch" (imported; not reported) from "skipped" (unlocatable;
  warning count).
- **Reclassification** is a core function with explicit roots, called once by the app's startup after
  `ensure_schema`, idempotent, returning a count for the log. Its three rules are the spec's Q5 verbatim.
- **Unlocatable state** is derived by the listing from the stored `central_path` / `source_ref`
  (`Path::exists`, following links — a dangling link is missing for the acquire step too; amended at ticket 09 from `symlink_metadata`), exposed as a nullable enum on the DTO, never stored. Both paths gone reports `source_missing` (Re-point's Update rebuilds central). Restore = an explicit-Id Update of a `git`/`local` skill whose central copy is gone, so Update no longer raises `CentralPathMissing` (only `move_central_repo` does). Re-point is one
  new command (skill id + new path) that rewrites `source_ref` then runs the existing single-skill
  Update. Detach is a provenance change on the record (to `imported`, `source_ref` cleared). Restore is
  the existing Update; the finalize step must tolerate a missing central directory.
- **Acquisition** resolves symlinks in both adapters of `git_acquisition::acquire` (clone via the sparse
  set; Contents API via the entry's `target`), sharing one bounded "resolve link chain within repo root"
  rule and one typed refusal. The cache entry's coverage widens by the target, never narrows (round-3
  invariant). One subpath normaliser serves the cache key and the fetcher.
- **Import** widens the round-3 force-include from the chosen variant's Tool to every Tool holding a
  variant with the chosen variant's fingerprint; the per-group report lists every Tool included beyond
  the policy (a list, replacing the single optional key), and divergent siblings are reported as they are
  on the auto-sync-off path. The frontend composes one toast line per forced Tool.
- **Add refusal** raises a typed condition from the local-install entry point when the folder is inside a
  Tool's skills dir; the copy steers to Import.
- **Errors** follow ADR-0001 end to end; the wire codes are new, nothing existing renames.
- **Reporter** gains a `useNotificationHistory` building block (ring, unread, clear) composed by
  `useStatusReporter`; one `copyToClipboard` helper; `showActionErrors` records oldest-first; `formatError`
  (or a `notifyError`) travels alongside `notify` into project components; `visibleToasts` is set once
  on the Toaster.

## Testing Decisions

A good test exercises an entry point the operator or the frontend actually calls, on a temp home with
explicit `InstallerPaths`, and asserts observable outcomes: what is on disk, what the store holds, what
the report says, what the DTO carries. No test touches the real library. Frontend tests are hook-level
(`renderHook`, `invokeTauri` mocked at `src/lib/tauri.ts`, `sonner` mocked) — no JSX tests.

| Behaviour | Seam | Prior art |
|---|---|---|
| Typed missing-path / log-reveal errors | `CommandError::from_anyhow` over the `SignalError` raised by the install/update/central-repo entry points; pure `log_reveal_target` | `core/tests/installer.rs`, `commands/error.rs` tests |
| Upstream symlink resolution | `git_acquisition::acquire` over a fixture repo containing `120000` entries (clone path) and the `GithubApi` test impl answering `type: symlink` (API path) | `core/tests/git_acquisition.rs`, `core/tests/git_cache.rs` |
| Propagation / project-sync dedupe; overlap test | `refresh_managed_skills`, `propagate_unlocked`, `sync_single_assignment` | `core/tests/refresh.rs`, `propagation.rs`, `project_sync.rs` |
| Reporter world | `renderHook(useStatusReporter)`; project hooks with mocked `invokeTauri` | `src/hooks/useStatusReporter.test.ts` |
| Import takes over identical originals; default variant | `import_onboarding_selection` on a temp home with two Tool dirs (one real, one link to it); `useAddSkillFlow` selection payload | `core/tests/onboarding_import.rs`, `useAddSkillFlow.test.ts` |
| `imported` provenance; Refresh membership; card copy | `import_onboarding_selection` → stored record; `refresh_managed_skills` batch membership; `sourceKind` three-way | same + `skillPresentation.test.ts` |
| Legacy reclassification | the core function on a seeded store + temp home | `core/tests/skill_store.rs` |
| Add refuses Tool-dir paths | `install_local_skill` raising the `SignalError`; `describeCommandError` branch | `core/tests/installer.rs`, `commandError.test.ts` |
| Unlocatable badge, actions, Refresh skip | `managed_skill_catalog`; `refresh_managed_skills` skip count; `useSkillLibrary` actions | `core/tests/refresh.rs`, `core/tests/skill_catalog.rs`, `useSkillLibrary.test.ts` |

## Out of Scope

- Editing a git skill's URL / ref / subpath ("edit source") — delete and re-add is the workaround.
- A bulk "re-point base directory" for many stale local rows at once (possible follow-on if Re-point per
  skill proves tedious).
- WSL ↔ Windows path translation.
- Persisting notification history; any change to the import selection UI beyond the default variant.
- Anything on the round-3 list already fixed in ticket 11.

## Further Notes — facts verified at HEAD (so tickets don't re-derive them)

- `installer.rs::install_local_skill` (~42–90) copies the chosen path into staging and records
  `SkillProvenance::local(source_path)` (`install_finalize.rs:76`) unless `skill_lock.rs:75
  try_enrich_from_skill_lock_with_home` finds a lock entry (→ `git`). `onboarding_import.rs::apply_one_unlocked`
  (~222) calls it, then `sync_imported_unlocked` (~280) syncs to `policy.tools ∪ {chosen variant's Tool}`
  or `settle_original` (~336) removes byte-identical originals. `ImportGroupStatus::Imported.forced_source_tool:
  Option<String>` is the round-3 report field.
- The single-skill update path is `installer.rs` ~200–265: `source_type == "git"` → `acquire`; `"local"` →
  `source_path.exists()` else `bail!("source path not found")` then copy; anything else bails
  "unsupported source_type". `finalize_and_propagate_unlocked` follows. `refresh.rs::reassert_auto_sync_unlocked`
  (~400) creates targets for every installed Tool missing one — it must never run for an unlocatable skill.
- Raw path-leaking bails: `installer.rs:49,194,257,410,507,516`, `central_repo.rs:24`,
  `git_acquisition.rs:381` ("path not found in repo: {abs cache path}"). `open_log_folder` is
  `commands/mod.rs:~202–229`.
- Registry: `tool_adapters::ensure_path_within_tool_dirs(home, path)` (`mod.rs:702`), `skills_dir_in`
  (`:693`), `global_tool_entries`. Registry order puts `claude_code` before `pi` — why the smoke-test
  import picked Claude's symlink: `useAddSkillFlow.ts:429` falls back to `group.variants[0]` when no
  conflict. `OnboardingVariant { tool, path, fingerprint, is_link, link_target }` (`onboarding.rs:15`).
- Symlink handling today: `git_fetcher.rs::clone_or_pull_sparse` (`:102`; `sparse-checkout set --no-cone`
  at `:260`); `github_download.rs:158` skips entries that are not `file`/`dir` ("Skip symlinks, submodules").
  Upstream fact: `tanstack-skills/tanstack-skills` has published `plugins/tanstack-all/skills/*` as
  `120000` symlinks since its first commit; each targets `../../<name>/skills/<name>`. The operator's five
  rows record the alias subpath and SHA `6f5521e` (= upstream HEAD); their central copies are real.
- Frontend rules: `skillPresentation.ts:40 sourceKind` → `"git" | "local"` by substring; `SkillCard.tsx:52`
  icon by substring; `SkillDetailView.tsx:503 isGitSource`. Listing is `skill_catalog.rs:36
  managed_skill_catalog(store)` → `commands/mod.rs:1119 get_managed_skills`.
- Store: `skill_store.rs` `SCHEMA_VERSION = 8`; `ensure_schema` is SQL-only; startup order in `lib.rs:94–96`
  is `migrate_legacy_db_if_needed` → `ensure_schema`.
- Auto-sync fan-out paths (for the operator's smoke tests): the toggle writes a setting only; import
  syncs only the ticked groups; Refresh/Update with auto-sync on reasserts every installed Tool.

## Ticket map

```
01 typed missing-path errors + log-reveal core fn (#2 #6 #17)          ┐
02 acquisition follows upstream symlinks + one normaliser (#1 #13)     │ wave 1 — parallel,
03 propagation/project-sync dedupe + overlap test (#4 #10 #11 #12 #16 #19) │ file-partitioned
04 reporter world (#5 #7 #8 #9 #14 #15 #18)                            │
05 import takes over every identical original; default variant        ┘
06 `imported` provenance; Refresh membership; card; glossary + ADR   ◄── 04, 05
07 legacy reclassification at startup                                ◄── 06
08 Add refuses Tool-dir paths, steers to Import                       ◄── 01, 06
09 Unlocatable skill: badge, Re-point/Detach/Restore/Remove, Refresh skip ◄── 07 (and 06)
10 review panel + version:set 1.2.4 + CHANGELOG + Release             ◄── everything
```

Work the frontier: lowest-numbered open ticket whose blockers are resolved.

## Closure — 2026-09-15

Shipped: v1.2.4 / ed630c0. Tickets: 12 terminal (11 done, 1 superseded), 0 still open (none).
Residue → BACKLOG: #22, #23, #24, #25. Dropped by name: none.
