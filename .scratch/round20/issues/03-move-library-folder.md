# 03: Move Library folder

Status: done — 26ce2b6

**Blocked by:** None (can start immediately).

**Spec:** `.scratch/round20/spec.md` § Library folder move (#91); user stories 21–35; § Testing Decisions → Library move.

**What to build:** Settings → General → Browse… becomes one confirmed, guarded, all-or-nothing **Move Library folder**
action. The flow:

1. Admit the destination.
2. Copy every recorded, locatable skill.
3. Switch every record and the setting in one transaction.
4. Re-point every owned linked Deployment (global and project).
5. Remove the old copies through the removal fence.

It answers a truthful report. Today's code: `core/central_repo.rs::move_central_repo` (no guard, `?` mid-loop),
`core/settings.rs::set_central_repo_path`, `commands/mod.rs::update_setting`, `src/hooks/useSettingsState.ts` →
`handlePickStoragePath`.

## Acceptance criteria

- [ ] **The command.**
  - A new `move_library(dest)` command is registered in `specta_builder()`, with bindings regenerated.
  - It answers `{ report, skills }` and streams progress over a `Channel`.
  - `CentralRepoPath` is removed from the generic `update_setting`, and nothing else writes the setting.
  - The body runs in one `mutation_guard::serialized` section at its entry point (the AGENTS.md sync-target mutation
    rule).
- [ ] **Admission refusals,** each a typed `CommandError` with a Rust variant, binding, `describeCommandError` branch and
      i18n key, and a test each. The destination is refused when it:
  - is relative;
  - is inside or contains the current Library;
  - is inside any harness global or project skills dir, or inside the git cache;
  - is not empty;
  - is not writable.
- [ ] **Cloud-synced destinations** (`~/Library/Mobile Documents`, `~/Library/CloudStorage/*`, a OneDrive root) are
      classified by the backend and warned about in the confirmation, not refused. A read-only command or a field the
      confirmation reads works; no path logic in the frontend.
- [ ] **Copy.**
  - Every recorded, locatable skill is copied and verified by Content identity.
  - On any failure the staged copies are removed and records and setting are unchanged (test).
  - `.explore-cache` and `.skills-hub-staging-*` are not copied.
  - An Unlocatable skill is skipped, reported `skipped { missing }`, and its record keeps its old path (test).
- [ ] **Switch.** One DB transaction flips each moved skill's `central_path` and the setting.
- [ ] **Re-point.**
  - Every owned (`synced_at` set), non-disabled global target row and project assignment in link mode whose link
    resolves to the skill's old copy is re-made into the new copy via `sync_engine`'s overwrite path.
  - The other cases:
    - copy-mode Deployments are `unchanged { copy }`;
    - an owned link pointing elsewhere is `skipped { not_ours }` and untouched;
    - a missing project root is `skipped { folder_missing }`;
    - per-target failures are `failed { error }` report data.
  - A test covers each.
- [ ] **Clean.**
  - Each moved skill's old copy is removed through `artifact_removal` and the removal fence, under a claim on that
    recorded path.
  - The old root and anything unrecorded stay, and are listed in `left_behind`.
  - A test proves unrecorded content survives, and that nothing outside the old copies is removed.
- [ ] **The frontend.**
  - Browse… keeps the folder pick outside the gate, then opens a confirmation: "Move N skills to X? Every harness link
    is re-pointed", plus the cloud warning when it applies.
  - Confirm runs `move_library` under `runAction` with header progress, not cancellable. The skills world replaces
    its catalog from `{ report, skills }`.
  - `reportOutcome` folds the report: moved, re-linked, skipped (missing / not ours / folder missing), failed, and
    left behind.
  - Hook tests: cancel runs nothing; refused while another action runs; the fold's counts.
  - The settings world's displayed path updates from the response.
- [ ] The fixture backend implements `move_library` (success, plus one refusal), and the flow is driven once in
      `npm run dev:fixture`.
- [ ] CHANGELOG is the orchestrator's job: report the user-facing change in one plain sentence.
- [ ] `npm run version:check && npm run check` passes.

## Comments
- 2026-11-06 (orchestrator): five review rounds. Round 1 was blocked on three P1s; round 2 was blocked again; round 3
  was fix-then-ship from two reviewers; round 4 had two findings that the orchestrator fixed. Full record:
  `.scratch/reviews/round20-t03-move-library-review.md`. Merged as `26ce2b6` (PR #12).

