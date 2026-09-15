# 06: Onboarding import as one core operation with a report

Status: resolved

Type: task
Source: `../spec.md` Q7, Q18; CONTEXT.md **Onboarding import**

**What to build:** Importing pre-existing Tool skills is one backend operation: given the operator's selections (one chosen variant per name-group plus, with auto-sync on, the Tools to sync to) and the auto-sync policy, core admits each chosen variant, finalizes it as a Managed skill, then either propagates it (auto-sync on — the chosen variant's own Tool is overwritten in place) or removes the originals (auto-sync off). Only originals byte-identical to the chosen variant are removed; a divergent sibling is left in place and reported. Progress streams per group; the result is a per-group report with per-target outcomes. The two per-step commands are deleted; the add-skill hook keeps selection state and renders the report.

**Blocked by:** 02, 03

- [x] One `import_onboarding_selection` command; `import_existing_skill` and `remove_skill_source` are gone from the registry and bindings
- [x] Core tests against a temp home/central/DB cover: auto-sync on with the source Tool in a shared-skills-dir group (source is one of the targets), auto-sync off with identical originals removed, auto-sync off with a divergent sibling kept and reported, a group whose chosen variant fails admission (others still import), partial failure mid-batch
- [x] The hook's import tests assert report rendering, not the sequence of three commands per group
- [x] Import copy for the divergent-sibling outcome exists in EN + ZH
- [x] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.

Implemented on `arch/06-onboarding-import`.

- New core module `src-tauri/src/core/onboarding_import.rs`: `ImportSelection`,
  `ImportPolicy { auto_sync, tools }`, `import_onboarding_selection(...) -> ImportReport`.
  Core re-reads a fresh `build_onboarding_plan` and resolves each group's paths itself, so a
  stale UI cannot name a path outside the group (the selection carries only
  `(group_name, chosen_path, name)`). Admission (`require_skill_md`) runs outside the guard;
  finalize + sync/original-removal run under `mutation_guard::serialized` **per group**, using
  only the unlocked seams (`sync_skills_to_tools_unlocked`, `remove_path_any`).
  Auto-sync ON: `BatchPolicy { overwrite: false, overwrite_if_same_content: true }` plus a
  `BatchOverride` force-overwrite for the chosen variant's own tool (that path IS the source;
  its bytes are already in the central repo). Auto-sync OFF: every variant path goes through
  `ensure_path_within_tool_dirs` then `target_has_same_content` — identical → removed,
  divergent → `KeptDivergent`, refusal/IO failure → `Failed` (report data).
- Command `import_onboarding_selection(selections, policy, on_progress: Channel<ImportProgressDto>)`;
  `import_existing_skill` and `remove_skill_source` deleted from `commands/mod.rs`,
  `collect_commands![]` and the generated bindings. Target outcomes reuse `SyncTargetResultDto`
  (the sync command's mapping was factored into `to_sync_target_result_dto`), so there is no
  third target-outcome union.
- Frontend: `useAddSkillFlow` issues ONE invoke with a `Channel` and renders the report
  (`importReportEntries`); its import tests assert report rendering, not a command sequence.
  New i18n keys EN + ZH: `errors.importKeptDivergentTitle/Message`,
  `errors.importCleanupFailedTitle`, `actions.importApplyStep`; the now-dead
  `actions.importExisting` was removed.
- 7 new core tests (temp home/central/DB) cover: auto-sync on with the source tool in the
  amp/kimi_cli shared-skills-dir group, auto-sync off identical originals removed, divergent
  sibling kept + reported, admission failure isolated to its group, finalize collision mid-batch,
  a path the plan does not own, and the progress phases.
- Gate: `npm run version:check && npm run check` green; `cargo test --all` 407 passed.
