# 05 — Onboarding plan carries `name_is_managed`; `remove_originals` is a per-run input; `SKILL.md`-gated scan

Status: implemented
Blocked by: —
Spec: `.scratch/round19/spec.md` — D13, D14 (grill Q11, Q12). Absorbs BACKLOG **#46**. Source: `flows.md` Flow 3.

## Work

- **Plan.** `OnboardingPlan` groups gain `name_is_managed: bool` (and the managed skill's id when true) so the review
  can start them unchecked and offer Replace/Leave (`core/onboarding.rs`). Counts: the plan exposes both name-group
  count and copy count; the frontend stops deriving one from the other.
- **`SKILL.md`-gated scan (#46).** `tool_adapters::scan_tool_dir` lists every subdirectory today
  (`tool_adapters/mod.rs` ~888–926); it now admits only directories whose `SKILL.md` parses to a valid manifest,
  and reports the invalid ones as a separate list with a typed reason (`missing_skill_md`, `missing_name`, …) so the
  review can show them disabled with the reason in words. Re-verify the line numbers against the current file.
- **`remove_originals: bool` on `import_onboarding_selection`** (`#[serde(default)]` is wrong here — make it required
  so the frontend must decide). Auto-sync on: byte-identical originals are taken over by the sync batch as today;
  auto-sync off: originals removed **only when `remove_originals` is true**, otherwise left and reported as
  `left_in_place` rows per Tool. The report lists removed originals per Tool in either mode.
- **Managed-name groups**: importing one with `replace: true` routes the Tool's copy through the same `TARGET_EXISTS`
  path ticket 04 makes askable (the managed skill is synced over the copy with an Overwrite override); with no
  decision it is skipped with a typed reason, never the current `already exists — go to My Skills` failure.
- Bindings regenerated; shims re-export; the existing `ImportModal` gets the minimum to compile (pass
  `remove_originals` = current policy; hide nothing) — the review UI is ticket 21.
- **Fixture**: `first-run` seeds two managed-name groups, one invalid manifest, `remove_originals` honoured.
- **Tests**: gated scan admits/reports; managed-name flag; `remove_originals` both values × both modes; replace path
  produces an askable row.
- Docs: CONTEXT.md § Onboarding import (the per-run choice; managed-name groups); CHANGELOG.

## Review

Orchestrator diff read; Astra only if the removal step changes shape (it should not — it still pairs with
`ensure_path_within_tool_dirs`).

## Result

Implemented 2026-09-27 (child of the round-19 orchestrator; shared tree with tickets 02 and 08).

### Shapes (wire, generated into `src/bindings/index.ts`)

- **`OnboardingPlan`** `{ total_tools_scanned, group_count, copy_count, groups (sorted by name), invalid: OnboardingInvalidEntry[] }`
  — `total_skills_found` is gone: the frontend reads names and copies separately.
- **`OnboardingGroup`** gains `name_is_managed: boolean` + `managed_skill_id: string | null` (`Some` exactly when
  managed). Managed names = every record's `name` and its central directory's name (`onboarding::managed_names`, the
  name finalize's collision check reads). A store read failure fails the plan (no silent "not managed").
- **`OnboardingInvalidEntry`** `{ tool, name, path, reason: InvalidManifestReason }`;
  **`InvalidManifestReason`** = `missing_skill_md | read_failed | invalid_frontmatter | missing_name`
  (`skill_discovery.rs`, mapped from `Validity`; the gate is `skill_discovery::manifest_validity` = the strict
  `find_skill_md` + `parse_skill_md_with_reason` read install admission uses).
- **`ImportPolicyDto.remove_originals: boolean`** — required (no `serde(default)`). Core `ImportPolicy` lost its
  `Default` derive so core callers state it too.
- **`OnboardingSelectionDto.replace?: boolean`** (`serde(default)` = "no decision").
- **`ImportGroupStatus`** gains `replaced { skill_id, skill_name, targets: BatchTargetOutcome[] }` and
  `skipped { reason: ImportSkipReason }`; **`ImportSkipReason`** = `{ reason: "name_managed", skill_id }`.
- **`OriginalStatus`** gains `left_in_place`.

### Behaviour

- **Gated scan (#46)** — `tool_adapters::scan_tool_dir` returns `ToolDirScan { skills, invalid: Vec<InvalidSkillDir> }`;
  a subdirectory is a skill only when its `SKILL.md` parses with a `name`; every other visible subdirectory is
  reported with its reason; a hidden non-skill dir (`.git`, …) is not a candidate and is not reported. The plan applies
  the same central-repo / managed-target exclusions to the invalid list (`onboarding::is_unmanaged`, extracted from
  `filter_detected`). An invalid entry is never in a group, so naming it fails admission `NOT_FOUND onboarding_group`.
- **`remove_originals`** — auto-sync off + `true`: `settle_original` as before (still `ensure_path_within_tool_dirs`
  → `remove_path_any`, unchanged shape). Auto-sync off + `false`: new `leave_original` touches nothing and reports each
  original still present per Tool — `left_in_place` (identical) / `kept_divergent`. Auto-sync on: ignored; the sync
  batch takes identical originals over as today (`originals` stays divergent-only; the identical ones are `targets` +
  `forced_tools`).
- **Managed-name groups** — decided in `admit` (never by finalize's `SKILL_EXISTS`): the name is kept (no rename) and
  managed → `replace: false` = `Skipped { NameManaged }`, no applying tick, nothing touched; `replace: true` =
  `replace_with_managed_unlocked` under the guard: one `sync_skills_to_tools_unlocked` batch of the Managed skill over
  the group's Tools with the **same-content policy and no override**. An identical copy is replaced silently; a
  divergent copy answers a `TARGET_EXISTS` row carrying `(skill_id, tool_key)` — the askable row. **Choice recorded:**
  the confirmed ask's retry is `sync_skills_to_tools` with a per-pair Overwrite override (test proves it); the plan
  never authorizes a force-overwrite (a copy can change between review and import). Ticket 21 decides whether the
  review's "Replace" pre-confirms that retry or raises D12's ask. A renamed selection of a managed group is an ordinary
  import (finalize still refuses a taken name).

### Files

Rust: `core/skill_discovery.rs` (reason enum + `manifest_validity`), `core/tool_adapters/mod.rs` (`ToolDirScan`,
`InvalidSkillDir`, gated `scan_tool_dir`), `core/onboarding.rs`, `core/onboarding_import.rs` (module doc, selection
`replace`, policy `remove_originals`, `Admission`, `replace_with_managed_unlocked`, `leave_original`, new statuses),
`commands/mod.rs` (DTO fields + mapping only). Tests: `core/tests/{onboarding,onboarding_import,tool_adapters,skill_disable}.rs`.
Frontend: `src/bindings/index.ts` (regenerated), `components/skills/types.ts` (shim: `OnboardingInvalidEntry`,
`InvalidManifestReason`, `ImportSkipReason`), `ImportModal.tsx` + `SkillsList.tsx` (count = `group_count`),
`hooks/useAddSkillFlow.ts` (`remove_originals: true` = today's policy), `lib/reportOutcome.ts` (`importOutcome`:
`skipped` → one warning, `replaced` targets folded like import targets, `left_in_place` silent; counts cover the new
statuses), `i18n/resources.ts` (`importGroups.nameManaged*`, appended to `en`), fixtures (`model.ts`
`ForeignDir.invalid`, `backend.ts` plan/admit/replace/leave, `scenarios.ts`), tests (`fixtures/backend.test.ts`,
`hooks/useAddSkillFlow.test.ts`, `lib/reportOutcome.test.ts`). Docs: `CONTEXT.md` § Onboarding import, `CHANGELOG.md`
Unreleased.

### Fixture

`first-run` now seeds two managed skills — `tdd` (its three Tool copies diverge → Replace answers `TARGET_EXISTS`
rows) and `frontend-design` (copies byte-identical → Replace links silently) — and one invalid manifest
(`cursor/broken-skill`, `invalid_frontmatter`). **Note for 11/21:** first-run therefore no longer has an empty
library; 21's gate needs the managed-name groups there. `failures`' `broken-skill` is now an invalid entry and
`BROKEN_IMPORT_NAMES` is deleted. `remove_originals` honoured.

### Tests (new)

Rust: `scan_tool_dir_admits_only_valid_manifests_and_reports_the_rest`; `invalid_manifests_are_listed_apart_from_the_groups`;
`plan_counts_groups_and_copies_separately`; `a_group_whose_name_is_managed_carries_the_flag_and_the_skill_id`;
`the_store_backed_plan_marks_managed_names_from_the_library`;
`auto_sync_off_without_remove_originals_leaves_and_reports_every_original` (off × false);
`auto_sync_on_takes_originals_over_whatever_remove_originals_says` (on × false/true; off × true is the existing
`auto_sync_off_removes_identical_originals`); `a_managed_name_group_without_a_decision_is_skipped_untouched`;
`replace_syncs_the_managed_skill_over_an_identical_copy`;
`replace_over_a_divergent_copy_produces_an_askable_target_exists_row` (incl. the Overwrite-override retry);
`an_invalid_manifest_is_never_admitted`. Existing scan/plan/import tests updated for the gate (fixtures now write a
`SKILL.md`); `onboarding_import_never_rematerialises_a_disabled_pair` runs with `replace` both ways.
Vitest: first-run plan shape (2 managed, 1 invalid, counts) + import-all (managed skipped); "Replace syncs … and asks
for divergent ones"; "remove_originals … false/true"; failures admission/skip ticks; `importOutcome` cases
(left_in_place, skipped, replaced ×2).

### Gate

Shared tree is mid-flight for 02 and 08, so the gate was also run on an isolated copy (`git archive HEAD` + this
ticket's hunks only, `/tmp/r19t05`): `version:check` OK; `lint` clean; `styles:check` ok; vitest **526/526**;
`tsc -b` + `vite build` ok; `cargo fmt --check` clean; `clippy --all-targets` 0 warnings; `cargo test --all` **751
passed**.
Shared tree (2026-09-27): `cargo test --all` 769 passed, clippy clean; remaining failures are all in other children's
in-flight hunks — `cargo fmt --check`: `onboarding_import.rs:434` (02's health writer), `skill_disable.rs`,
`skill_health.rs`, `tests/skill_health.rs` (02); `tsc`: `fixtures/backend.test.ts:167` (`BatchSyncOverrideDto`
`overwrite` → 02's `decision`), `useSkillLibrary.test.ts:90` (`health`, 02); vitest: 2 in `fixtures/backend.test.ts`
("raises TARGET_EXISTS … under an override" — 02's override API; "F1-R1" — project sync now typed `TARGET_EXISTS`).
Earlier, `useAddSkillFlow.test.ts` failed wholesale on 08's `surface` dep (in flight). 02 migrated this ticket's
`replace_over_a_divergent_copy…` test to `OverrideDecision::Overwrite` — keep it.

### Deferred to 21

The review UI: invalid entries disabled with the reason in words (keys per `InvalidManifestReason`), managed-name
groups unchecked with Replace/Leave (sending `replace: true`), the remove-originals checkbox (today the hook always
sends `remove_originals: true`), per-Tool removed/left report, "N skills · M copies" header, routing `replaced`
`TARGET_EXISTS` rows into ticket 04's one ask. Managed-name groups are still preselected (Import skips them with a
warning, never fails).
