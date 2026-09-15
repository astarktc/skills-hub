## Standards

1. **Hard — business logic lives in a command.** `AGENTS.md` says “`commands/` is wiring only … business logic goes in `core/`.” `src-tauri/src/commands/mod.rs:905-916` implements the Re-point workflow itself: `repoint_local_source(...)?;` followed by `refresh_managed_skills_core(... RefreshSelection::Ids(vec![skillId]) ...)`. Move that sequencing to one core entry point; the command should resolve roots/DTOs and delegate.

2. **Hard — formatting functions are threaded as props.** `AGENTS.md` says frontend presentation logic “lives once” and “no prop carries a formatting function.” This diff adds `formatError: FormatErrorFn` to `ProjectsPage`, `AssignmentMatrix`, and `SkillDetailView`, with `App.tsx` passing `formatError={formatError}`. Keep command-error presentation at the reporter/world-hook seam and pass results or `notifyError`, not a formatter through component props.

3. **Judgement — Duplicated Code.** `SkillCard.tsx` and `SkillDetailView.tsx` both independently translate `imported_from_tool` and compose `provenance.managedHere` + `provenance.importedFrom` (for example, each repeats `t(\`tools.${skill.imported_from_tool}\`, { defaultValue: ... })`). Extend the existing `skillPresentation.ts` seam with one imported-source presentation helper; each component can still choose its markup.

## Spec

1. **Data-safety bug — Import can overwrite a now-divergent sibling.** Q3 requires “every byte-identical original” be taken over and “divergent siblings … left in place.” `onboarding_import.rs:300-358` decides identity from the pre-apply plan’s cached `variant.fingerprint`, then creates unconditional `overwrite: true` overrides. Unlike auto-sync-off’s `settle_original`, it never re-compares each path with the finalized central copy. If either chosen bytes or a sibling changes after planning, a divergent directory is deleted. Re-hash at the overwrite boundary and force only a live match.

2. **Partial symlink support.** Q8 says “when the requested subpath (or a component of it) is a symlink.” Clone acquisition checks every prefix in `git_fetcher::symlink_on_path`; `github_download.rs:157-173` only follows when the response for the *complete requested path* is itself `ContentsResponse::Entry(... type == "symlink")`. It never probes prefixes, so the API adapter does not implement or validate a symlink component. Add the API-prefix case and tests.

3. **Legacy reclassification misses ancestor links.** Q5 requires classification when a source “resolves through a symlink into `central_dir`.” `legacy_reclassification.rs:87-93` additionally requires `symlink_metadata(path).file_type().is_symlink()`, so `/outside/link-to-central/skill` is missed when an ancestor, rather than the leaf, is the symlink. Canonical containment should be paired with evidence that any path component is linked, not only the leaf.

4. **V9 is not crash-idempotent.** Q5’s amendment says the `user_version` fix prevents an `ALTER TABLE` migration failing “on its second launch.” In `skill_store.rs:361-369`, `ALTER TABLE ... imported_from_tool` and `PRAGMA user_version = 9` are separate autocommit operations. A crash between them leaves version 8 with the column present; the next launch repeats `ALTER TABLE` and fails. Wrap migration + version update in one transaction (or make column detection idempotent).

## Summary

**Blocking:** command-layer Re-point orchestration; formatter props; stale-fingerprint Import overwrite; missing API component-symlink handling; ancestor-symlink reclassification gap; non-atomic V9 migration.  
**Follow-up:** deduplicate imported-source presentation in `skillPresentation.ts`.
