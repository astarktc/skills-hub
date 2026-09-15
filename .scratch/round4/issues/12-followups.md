# 12: Round-4 review follow-ups (non-blocking)

Status: superseded — round5/issues/08-panel-and-version

**Source:** round-4 panel (Fable 5.1 / Opus 5 / GPT-5.6 Sol) over `f057190..7fcec94`; each item verified at HEAD by the orchestrator. Items taken into ticket 11 are omitted.

## Needs an operator ruling
1. **`formatError` as a component prop vs AGENTS.md "no prop carries a formatting function"** (Sol Std 2). Round-3 spec Q10/#9 directed "pass `formatError` (or `notifyError`) alongside `notify`"; the AGENTS.md sentence is about `skillPresentation`'s pure formatters. Options: (a) amend AGENTS.md to sanction the reporter's `formatError` as a binder-passed seam (matches the "interfaces App passes into hooks" pattern); (b) refactor `ProjectsPage`/`SkillDetailView`/`SettingsPage` to receive pre-formatted strings or use `notifyError` only. Orchestrator recommends (a).

## Behaviour
2. **API path follows only a leaf symlink** (Sol Spec 2, Fable Spec a). A link on a *component* (`skills -> ../pkg/skills`, subpath `skills/foo`) gets a typed 404 from the Contents API and, by design, never falls back to the clone (which would follow it). Real-world case (tanstack) is the leaf. → Probe prefixes on the API path, or fall back to clone when a 404 has a link-shaped prefix.
3. **Import force-override vs live re-hash** (Sol Spec 1). Identity comes from the plan rebuilt inside the import call, then `BatchOverride { overwrite: true }` per identical Tool; a millisecond TOCTOU within one guarded operation. → Check whether the force override is still needed given `overwrite_if_same_content: true`; if not, drop it and the window vanishes.
4. **`open_log_folder` non-macOS behaviour change** (ticket 01 deviation; Opus/Fable). A plain (non-`.app`) log dir is now opened, not revealed in its parent. → Record in CHANGELOG (ticket 10).
5. **Re-point validates `require_skill_md` + `tool_holding_path`** beyond Q7's wording (Fable). Sensible; record as the rule in CONTEXT.md **Unlocatable skill** if not already.

## Error contract / wording
6. **Prose-encoded conditions → `OTHER`** (Opus Std 6, Fable): `unlocatable::require_local` ("only a local skill has a source folder…"), `LinkChain::follow` depth bound. Both are caller guards, but both are nameable. → Typed variants or keep as sanctioned safety valve — decide once.
7. **`errors.symlinkEscapesRepo` interpolates `{{target}}` into prose** (Fable) — same pattern as pre-existing `pathOutsideToolDirs`. → `withDetail` for both, or leave both.
8. **`unlocatable.source_missing` / `central_missing` i18n keys are snake_case** (Opus Std 5) so the card can `t(\`unlocatable.${state}\`)`. → A `Record<UnlocatableState, key>` map as `useSkillLibrary`'s `SKIPPED_REASON_KEY` already does.

## Code shape
9. **New `expect` on a command path** — `project_sync::sync_assignment_target` (`"a live skill name always locates the artifact"`), ticket 03 deviation (Opus Std 3, Fable). True by construction; mirrors an existing `expect`. → Return the typed `NotFound` instead, or leave.
10. ~~chosen-variant lookup written twice~~ — done in ticket 11 (`chosen_variant` helper).
11. **Naming**: `tool_owning_path` / `tool_shaping_path` / `tool_holding_path` — three near-synonyms for two questions (Fable). → rename the shape one (`tool_dir_shape_of`).
12. **Code comments cite `spec Q5` / `ticket r4/06`** — `.scratch/` is gitignored and "Q5" now means two rounds' things (Fable). → Cite CONTEXT.md/ADR terms instead; pre-existing pattern (9 hits at base).
13. **Global-rows Propagation path still resolves the adapter twice per row** (ticket 03 deviation) — outside #10's per-assignment scope.

## Verified clean by all three (for the record)
Error contract complete for every new code (EN + ZH, no path in prose except #7); import cannot overwrite a divergent sibling (`overwrite: false` + `overwrite_if_same_content` + fingerprint-only force); `refresh_eligibility`/DTO/Update agree and auto-sync reassert can never mint a target for an imported or unlocatable skill; cache entries only widen; V9 + `user_version` fix is sound for every upgrade path (atomicity aside → ticket 11).

## Comments

- 2026-09-15 — Status reconciliation: needs-triage → superseded — round5/issues/08-panel-and-version. Evidence: Round5/spec.md Q1–Q11 explicitly disposes this aggregate; terminal round5/issues/08-panel-and-version records the release of tickets 01–07 (historical 01–06 ticket files are absent, so no invented pointers). Verified implementation commits 66b87b5,7500898,6cb48a0,aa99191,e35e46e; CHANGELOG.md:63,68. Q2 says "API path stays leaf-link-only" and Q6 retains interpolation; ancestor-link residue is BACKLOG #25, not silently marked implemented.
