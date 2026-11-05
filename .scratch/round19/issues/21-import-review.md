# 21 — Import review: unchecked conflicts and managed names, Replace/Leave, remove-originals checkbox, counts, variant details, Tool labels, first-run empty state

Status: done — e6be057
Blocked by: 05, 19
Spec: `.scratch/round19/spec.md` — D13, D14 (grill Q11, Q12); absorbs BACKLOG **#46**'s UI half. Sources: `flows.md`
Flow 3; `critique-astra` F08; `critique-opus` P1-5; `audit.md` Import rows. Phase 19b.

## Work

- Review dialog on the new primitives: header counts "8 skills · 11 copies in 8 tools"; groups as rows; **Match**
  quiet (not amber), **Conflict** the exceptional row; conflict groups and **managed-name** groups start unchecked;
  managed-name groups offer "Replace *Cursor*'s copy with the managed skill" / "Leave"; conflict variants show
  description + modified time + Tool label, radios named by Tool + path; invalid manifests listed disabled with the
  reason in words.
- **Policy line + checkbox** before the button: auto-sync on — "Your identical originals in *Cursor, Claude Code* are
  replaced by links to the managed copy"; auto-sync off — "Remove my 9 identical originals from *Cursor, Claude Code*"
  checkbox (default = current policy); the button names the action ("Import and sync" / "Import and remove 9
  originals" / "Import"). Report lists removed / left-in-place originals per Tool; divergent siblings are one row and
  route into the consolidated ask (ticket 04).
- After a partial import: failed groups deselected and annotated; imported groups gone; the dialog stays.
- **First-run**: the empty state *is* the invitation (ticket 11's design); a large import completing gets D6's one
  deliberate moment (brief, does not gate interaction, removed under reduce).
- Migration: `ImportModal` joins `migrated`; its rules leave `App.css`.

## Gate (D31)

Astra audit on `first-run`: keyboard-only review with a conflict chosen by arrows and a managed-name group set to
Replace; auto-sync-off path shows the checkbox and the button text changes; counts match the plan; contrast;
`npm run check`.

## Result

Implemented 2026-11-04 (delegated child, worktree `t3/import-review` from `main` `8b42ce5`; nothing committed).

### Wire changes (each needed for a binding thing the review says)

- **`OnboardingVariant`** gains `description: string | null` (the copy's `SKILL.md` description — a Conflict's copies
  are told apart before choosing), `modified_at: number | null` (newest file mtime in the copy, ms) and
  `same_as_managed: boolean | null` (managed-name groups only: the copy's plan fingerprint against the Managed skill's
  **stored** identity, `content_identity::read(Source::Managed)` — no second hash; `onboarding::compare_with_managed`).
  This is what "differs from / same as your managed copy" reads (think-aloud: Match was misread as "identical to my
  managed copy").
- **`OnboardingSelectionDto.remove_divergent?: boolean`** (`serde(default)`) → `ImportSelection.remove_divergent`: the
  keep / remove choice the operator asked for. When set, `onboarding_import::remove_divergent_original` removes every copy
  of the group whose content differs from the finalized central copy — **before** the sync/settle step, within the
  harness dirs only (`ensure_path_within_tool_dirs` + `remove_path_any`, the sanctioned original-removal site), reported
  per harness as `removed` / `failed`; a target harness then gets the link at that path (no `TARGET_EXISTS`, no ask).
  Ignored for a managed-name group (Replace's differing copies stay askable rows).
- **`ImportGroupStatus.imported` / `.replaced` gain `central_path`**: the one ask's retry is a sync batch for the skill,
  and `BatchSyncSkillDto.source_path` needs it (ticket 04's install batch carries the same through `InstallResult`).
- `OriginalStatus.removed`'s doc now covers both removals. Bindings regenerated (`cargo test`); no new types, so no
  shim change.

### The review (fates and the keep / remove model)

- Files: `src/lib/importReview.ts` (pure folds: `defaultChoices`, `importReviewModel`, `importSelections`,
  `importPolicy`, `importResultRows`, `mergeImportReports`, `failedGroups`, `importedSkillIds`),
  `src/hooks/useImportReview.ts` (a building block of the Add world, composed by `useAddSkillFlow`; replaces the
  import half there), `src/components/skills/import/ImportReviewDialog.tsx` (replaces the deleted
  `modals/ImportModal.tsx`; always mounted, `xl`, Review → Result like Add).
- Rows in three sections: **New to your Library** (checkbox rows; Match = `soft neutral` badge, only with ≥ 2 copies;
  Conflict = `soft stale`; one legend line says Match/Conflict compare the copies **with each other**), **Names you
  already manage** (copy list compared **with your managed copy**, then a Leave / Replace radiogroup — Leave default),
  **Can't be imported** (disabled checkbox rows, reason in words). Conflict and managed-name groups start unchecked
  (Leave); a group whose last import failed starts unchecked, annotated "Last import failed: {reason}".
- **Fates** (stated per copy while the group goes ahead): the chosen copy "Becomes the Library skill; this folder is
  replaced by a link to it / is then removed / is left in place" (per auto-deploy and the originals checkbox); a copy
  that differs: "Removed" (keep/remove = remove), "Left in place", or — kept, in a harness auto-deploy reaches — "Left
  in place until you answer: you're asked before a link replaces it". A checked Match row states its one group fate.
  Replace: an identical copy "Replaced by a link…", a differing one asked first.
- **Replace decision (05's open question)**: Replace does **not** pre-confirm an overwrite; it sends `replace: true` and
  every differing copy goes through the **one overwrite ask** — the import's `TARGET_EXISTS` rows (Replace's and a kept
  divergent sibling's under auto-deploy) become `sync` parts of `useSyncOrchestration.settleCollisionParts` (one ask per
  import, retried by exact pair with the import's same-content rule; answers folded back into the report).
- Radios are native, named by **harness — path** (`aria-labelledby`), described by description · modified time ·
  fate; choosing a copy checks its group. The keep / remove choice is one Checkbox per Conflict ("Remove the other
  copy of handoff", default unchecked = keep).

### Policy line (D13 / D49), before the button

- Auto-deploy **on**: "Your identical originals in *Claude Code, Codex, …* are replaced by links to the managed copy;
  the imported skills are deployed to your selected harnesses (*…*)." Button **"Import and deploy"**.
- Auto-deploy **off**: Checkbox **"Remove my 11 identical originals from *Claude Code, Codex, Amp, Cursor, Pi*"**
  (default checked = today's policy) + "Auto-deploy is off: the imported skills are added to your Library only."
  Button **"Import and remove 11 originals"** / **"Import"** when unchecked. Only Replace chosen → "Replace N skills'
  copies"; nothing chosen → disabled (focusable) with the reason.

### Result step

Heading = the D11 composite line (`importOutcome(...).toast.message`, i.e. `compositeHeadline`, over every run of the
opening — a Retry merges in place). One row per sent group: **Imported** ("Deployed to …" / "Added to your Library
only", "Copies removed from …", "Copies left in place in …", "Your different copy kept in …", "Kept your existing folder
in …" (declined), "Folder left to review in …", "Not deployed to X: {reason}", "Couldn't remove the copy in X:
{reason}"), **Replaced** ("Linked in …"), **Kept** (a Replace whose every copy was kept in the ask), **Skipped** (why),
**Failed** (typed error in words + **Retry**, "Retry failed (N)" when several — re-sends exactly the sent selection).
The dialog stays on its Result until Close; reopening reviews a fresh plan (imported groups gone, failed ones
deselected and annotated). The banner stays while the plan has any group (verified: "Review 3/4 discovered skills"
after partial imports) — never dismissed for good.

### First-run cascade

`StripMarkIcon` gains `landing?: { token, index }` (plumbed `SkillsList → SkillRow → SyncedStrip`): closing the Result
of an import that added ≥ 10 skills with auto-deploy re-keys each **deployed** mark of those rows to the run's token with
`transition-delay` 20 ms × mark index (`starting:opacity-0`, 120 ms, `ease-out`, `motion-reduce:transition-none`);
`useImportReview` withdraws `landing` after 1.2 s. Measured on the fixture: 67 marks over 10 rows lit with delays
0–120 ms, 0 after 1.5 s; computed `transition-duration` 0s under emulated `prefers-reduced-motion: reduce`.

### i18n

Added `importReview.*` at the end of `en`. **Deleted** (orphaned by `ImportModal`): `importTitle`, `importSummary`,
`importAndSync`, `selectAll`, `conflict`, `consistent`, `directory`, `linkLabel`, `toolsScanned`, `skillsFound`,
`foundIn`.

### Migration

`ImportReviewDialog.tsx` appended to `migrated`; `App.css` lost every rule only Import used (`.import-summary`,
`.import-metrics`, `.groups`, `.discovered-list *`, `.group-card`, `.group-title`, `.group-select`, `.badge`,
`.badge.danger`, `.group-variants`, `.variant-*`, `.found-pill`, `.modal-discovered *`, `.sync-row`,
`.inline-checkbox`). **`appCssCeiling` 812 → 606.**

### Other files

`App.tsx` (dialog always mounted, `progressInDialog`, `landing`), `fixtures/{model,backend,scenarios}.ts` (variant
description / modified time / `same_as_managed`, `central_path`, `remove_divergent`; first-run and rich copies get
descriptions and times), `lib/reportOutcome.ts` (stale "Import does not ask yet" comment), `DESIGN.md` (new § Import
review; motion table row; § Empty and first-run; ³ footnote superseding the round-18 Match/Conflict figures),
`CONTEXT.md` § Onboarding import, `CHANGELOG.md` Unreleased.

### Tests

Rust: `each_copy_carries_its_description_modified_time_and_managed_comparison`;
`remove_divergent_removes_only_the_copies_that_differ`, `without_remove_divergent_the_other_copies_stay`,
`remove_divergent_frees_a_target_path_for_the_link`, `remove_divergent_never_touches_a_managed_name_group`.
Vitest: `lib/importReview.test.ts` (14: defaults, counts, fates on/off/keep/remove/ask, managed compare, group fate,
policy count, button labels + refusal, selections, policy DTO, result rows, Kept, merge/failures/ids);
`useAddSkillFlow.test.ts` import block rewritten (11: one call + Result, link vs dir, D13 checkbox, unchecked defaults +
Replace + remove sent, collisions → one ask (`settleCollisionParts` parts with `central_path`) folded into the Result,
Retry + annotation, cascade on close, fresh plan, scope reload, reload failure, fold spy); `fixtures/backend.test.ts`
(+2: plan facts, `remove_divergent`).

### Fixture proof (`npx vite --mode fixture --port 5182`)

`?scenario=first-run` header "12 skills · 22 copies in 6 harnesses · 1 can't be imported" — matches the plan (12 groups,
22 variants, harnesses cursor/claude_code/codex/pi/amp/gemini_cli, invalid `broken-skill`). Keyboard-only (Tab /
arrows / Space / Enter): banner Review → focus on the counts heading → Tab to commit-message's checked radio → ArrowDown
chose Gemini CLI (group checked, Codex's fate "Removed" after Space on "Remove the other copy") → Tab to tdd's Leave →
ArrowDown = Replace (3 copies "Differs from your managed copy") → Import and deploy → one ask with the 3 tdd rows →
Keep existing → Result "9 imported · 3 kept existing" (tdd "Kept"); the Overwrite path read "tdd Replaced · Linked in
Claude Code, Codex, Pi". Auto-deploy off (Settings switch): checkbox "Remove my 11 identical originals from Claude Code,
Codex, Amp, Cursor, Pi", button "Import and remove 11 originals" → unchecked "Import" → back. Off-path Result: "8
imported · 1 original to review" with "Copies removed from …" and "Your different copy kept in Pi".
`?scenario=rich`: banner → review "5 skills · 7 copies in 5 harnesses".
Contrast (rendered, `contrast.json`), dark / light: Match 7.65 / 6.75, Conflict 8.94 / 4.94, managed badge 7.65 / 6.75,
"Differs" 10.13 / 5.68, "Removed" 6.96 / 5.64, modified time 5.52 / 6.24, invalid reason 10.13 / 5.68.
Screenshots (gitignored): `.scratch/round19/evidence/21/` — `review-{initial,conflict-chosen,managed-replace,invalid}-
{dark,light}-{1280,960}.png`, `ask-{dark,light}-{1280,960}.png`, `result-{dark,light}-{1280,960}.png`,
`result-autodeploy-off-dark-1280.png`, `policy-autodeploy-on-{dark,light}-{1280,960}.png`,
`policy-autodeploy-off-checked-{dark,light}-{1280,960}.png`, `policy-autodeploy-off-unchecked-dark-1280.png`,
`rich-{banner,review}-{dark,light}-1280.png`, `cascade-after-close-dark-1280.png`.

### Gate

`npm run version:check` → Version OK (1.3.0). `npm run check` → exit 0 (lint clean; styles:check ok — 606 lines, 40
migrated; vitest 54 files / 1167 tests; `tsc -b` + vite build; rustfmt; clippy `-D warnings`; `cargo test` 844 passed).
`cargo test --all` 844 passed; bindings regenerated and stable.

### Not proven / notes for the audit

- The success toast ("9 imported") overlaps the Result's footer for its lifetime at 1280 (the Toaster's corner; Add
  has the same) — Close stays reachable by keyboard.
- "1 original to review" still counts a copy the operator chose to keep (fold semantics unchanged: the copy remains
  unmanaged and is listed again in the next review).
- The cascade was verified through the DOM (marks, delays, withdrawal, reduced motion), not a frame capture.
- `.scratch/round19/evidence/21/` is gitignored; screenshots live only in this worktree.

### Fix pass (audit)

2026-11-04, after `.scratch/round19/review/ticket-21-audit.md` (FAIL); dispositions by the orchestrator. Nothing committed.

- **F1 (P0) — deletion needs two known, unequal identities.** `content_identity::compare` → `Comparison::{Same,
  Different, Unknown}` (a missing/dangling target or an unreadable side is `Unknown`); `same_content` is now
  `compare(..) == Same` — the same conservative answer for its other callers. `onboarding_import::remove_divergent_copy`
  removes only on `Different`; `Unknown`, and a non-NotFound `symlink_metadata` error, retain the path and report
  `OriginalStatus::Failed { IDENTITY_UNKNOWN }` (new `SignalError::IdentityUnknown { path, detail }` →
  `CommandError::IdentityUnknown`; `describeCommandError` branch + `errors.identityUnknown`; the Result says "Left the
  copy in Codex in place: it could not be compared…"). The chosen source is an explicit guard (`path == chosen` →
  never a candidate); NotFound stays `None`; the registry fence still runs immediately before `remove_path_any`.
  Tests (`core/tests/onboarding_import.rs`): `remove_divergent_retains_a_copy_whose_identity_is_unknown` (the audit's
  A/B/C: B removed, C byte-identical + one unreadable file retained `IDENTITY_UNKNOWN`, A `LeftInPlace`, disk bytes
  asserted), `remove_divergent_retains_a_copy_when_the_managed_identity_is_unknown`,
  `remove_divergent_retains_and_reports_a_copy_whose_metadata_is_unreadable`,
  `remove_divergent_never_touches_the_chosen_path`, `remove_divergent_removes_a_link_not_the_other_tools_dir_it_points_at`,
  `remove_divergent_never_follows_a_link_into_the_central_repo`; `core/tests/content_identity.rs`
  `compare_says_unknown_when_either_identity_cannot_be_read`. The five behavioural tests were red against the old
  `same_content` seam first (the two link tests are characterisation tests that already held, as the audit traced).
  No permission test took its skip branch (`--nocapture` checked).
- **F6 (P1) — a shared dir is settled once, reported per harness.** `remove_divergent_copies` settles each physical
  path once and fans the status out to every variant naming it (`OriginalStatus` is now `Clone`). Test:
  `remove_divergent_settles_a_shared_dir_once_and_reports_every_harness` (Amp + Kimi via `mark_installed_in`, chosen
  Claude copy → one deletion, `removed` rows for `amp` and `kimi_cli`). The fixture's plan still dedupes a shared path
  (`seenPaths`) — not brought into parity in this pass.
- **F2 (P1) — focus after Close/Escape.** Both Review invitations (`SkillsList` banner and empty state) carry
  `data-import-review`, `focusableWhenDisabled` and the refusal as `aria-description`; `ImportReviewDialog` passes
  `fallbackFocus` = `resolveFirst(importReviewFallback(), null, usable)` (`lib/actionLifecycle.ts`: a new
  `{ kind: "importReview" }` target, then the My Skills heading, then the main landmark). Tests:
  `actionLifecycle.test.ts` › `importReviewFallback (audit F2)` (4). Browser (fixture 5188, `first-run`, after two
  frames + 300 ms): Escape after a partial import → `BUTTON "Review 5 discovered skills"`; pointer Close after a full
  import (banner gone) → `H1 "My Skills"` (`data-surface-heading="skills"`); `failures` at 960 and 1280 →
  `BUTTON "Review 6 discovered skills"`.
- **F3 + F4 (P1) — one settlement fold.** `reportOutcome.importGroupKind` derives each group from its settled
  target rows: imported · replaced (≥ 1 synced) · kept (every collision answered Keep, nothing deployed) · skipped
  (every target skipped, no target, or an unanswered collision) · failed. `importOutcome`'s headline uses verbs
  `imported` / `replaced` / `kept` (new `outcome.done.replaced|kept`); target-level declines read "N existing folders
  kept" (`OutcomeTally.declined: "folders"` → `outcome.declinedFolders`) and are counted only under a group that went
  ahead; target failures under a Replace that replaced nothing are not also "deployments failed".
  `importResultRows` takes its status from the same rule and renders a non-failure skip as a neutral "Skipped Cursor:
  {reason}". Tests: `reportOutcome.test.ts` › `importGroupKind / import headline by settled targets` (17: the 8 kind
  cases; "8 imported · 1 kept"; "10 imported · 2 replaced"; all-skipped `TARGET_DISABLED` → skipped; mixed
  synced/skipped; mixed declined/unanswered → skipped + 1 folder to review; folders kept; English strings);
  `importReview.test.ts` › `importResultRows by settled targets` (6). Two old expectations that encoded the defect were
  corrected ("replaced over an identical copy" → "1 replaced"; "replace answered TARGET_EXISTS" → "1 skipped · 1 folder
  to review"). Browser: `first-run`, the 7 defaults + tdd Replace, ask answered Keep existing → **"7 imported · 1
  kept"**, tdd row `kept`; full import with both Replaces overwritten → **"10 imported · 2 replaced"**.
- **F5 (P1) — the toast waits for the Result.** `OutcomeHandlers.resultShownIn?: DialogKind`: while that dialog is
  open the reporter holds the run's one toast (a later run's supersedes it) and shows it when the dialog closes (an
  effect checking the surface door after each render); reload, the history block and the completion are unchanged;
  the live host and pointer behaviour of toasts are untouched. Applied to Import (`"import"`) **and Add**
  (`"addSkill"` — its settlement step has the same shape). Tests: `useStatusReporter.test.ts` › `a dialog that shows
  its own Result defers the run's toast until it closes…` and `the deferral applies only while that dialog is open…`;
  `useAddSkillFlow.test.ts` pins both call sites. Browser: `failures` ("4 imported · 4 deployments failed") at 960 ×
  640 and 1280 × 800 — no toast while the Result is open, `elementFromPoint` at Close's centre is Close (footer 121–839
  × 550–615 / 281–999 × 657–722), pointer Close works, the toast appears after it.
- **F7 (P2) — an intentional keep is a quiet note.** `importReview.keptByChoice(plan, selections)` (a Conflict sent
  without `remove_divergent`) is kept on the hook per run and passed to `importOutcome` / `importResultRows`: its
  `kept_divergent` originals fold as the note "handoff: 1 different copy kept in Pi" (`outcome.keptDifferentLine`) and
  the row needs no attention; a kept copy without that decision stays "to review". Tests: `reportOutcome.test.ts` (F7
  case), `importReview.test.ts` › `keptByChoice`, `useAddSkillFlow.test.ts` › `a Conflict sent without remove-other…`.
- **F8 (P2) — `git diff --check` green.** The trailing spaces are specta's `field; ` before an inline doc comment (not
  blank `///` lines), so the `export_bindings` test now trims trailing whitespace per line after `export` (pinned by
  `bindings_normalisation_trims_line_ends_and_keeps_the_final_newline`); regeneration is byte-stable (SHA-1
  `96c86973…` before and after a re-export). This also trims the pre-existing trailing spaces in older bindings.
- **F9 (P2)** — `list_all_skill_target_paths` → `list_owned_skill_target_paths` (store, `onboarding.rs`,
  `tests/skill_health.rs`); behaviour unchanged.
- Docs: DESIGN.md § Import review (focus chain, settlement rule, deferred toast) and § Empty and first-run's count
  line; CONTEXT.md § Onboarding import (known-different only, once per path); CHANGELOG Unreleased.
- Evidence (gitignored): `.scratch/round19/evidence/21-fix/` — `result-kept-1280.png`,
  `failures-result-{960,1280}.png`.
- Not proven: the fixture's shared-path plan parity (F6, above); F4's skipped rows were proven in folds/tests, not
  driven in the browser; the unknown-identity path was exercised in Rust only (the fixture cannot make a file
  unreadable).

## Comments

- 2026-09-29 — from tickets 02/05: rename `list_all_skill_target_paths` to say "owned" (it now returns only rows the app materialised); the `replaced` group's `TARGET_EXISTS` rows route into ticket 04's one ask — decide whether the review's "Replace" button is the confirmation or raises the ask (05 chose not to force-overwrite from the plan because a copy can change between review and import).

- 2026-09-30 — from ticket 15: the strip light exists — `StripMarkIcon` + `useSyncLight` fade a mark that *becomes* synced
  on screen from opacity 0 over 120 ms (`@starting-style`, never on mount; reduced motion shows it settled). The rare
  delight (DESIGN.md § Empty and first-run) only has to add the stagger: a per-mark `transition-delay` (20 ms × index)
  for the imported rows, once. It is one of the two permitted rare moments in DESIGN.md § Motion.

- 2026-09-30 — from ticket 16: Add no longer says "Import" anywhere (the `outcome.done.installed` count reads "N added";
  an Add candidate's failure title is `add.failedTitle`); `errors.importFailedTitle` is Import's alone now. The one
  candidate picker (`skills/add/CandidatePicker.tsx`) — search, "Select all shown", rows named by the candidate,
  unavailable rows disabled with "Can't be added: {reason}" in words — fits the review's invalid-manifest rows and
  group checkboxes (D14) if a `mode` for variants is added. `ImportModal` still uses `.inline-checkbox` (its last user
  in `App.css`). Import keeps `HeaderProgress placement="dialog"` at its foot; `progressInDialog` in App lists Add,
  Import, Re-point and Notifications only (the pickers are gone).

- 2026-10-27 — **from ticket 18's think-aloud (binding).** The legacy review was used by the operator, and three
  misreadings are exactly this ticket's scope (`18-operator-think-aloud.md` § Findings, P3·Q2–Q4, debrief 1):
  - **Managed names are invisible.** zod-schema-first and tdd are names he already manages, but the review shows them
    as ordinary rows. He expected "import + remove the original"; the run skips them. Say the name is taken before
    offering Replace / Leave.
  - **"Match" was read as "identical to my managed copy"**, which would be dangerous under Replace (tdd's Pi copy is a
    fork). The badge (`consistent`) means the discovered copies agree *with each other*. Name what each badge compares
    against, and show "differs from your managed copy" where it does.
  - **A Conflict's unchosen copy:** he assumed it is deleted; it is left in place and reported. State each variant's
    fate, and offer the keep/remove choice he asked for: *"nor is there a choice being given as to whether you want to
    migrate the one you choose and then keep versus delete the other."* At the debrief this was the most confusing
    thing in the session.
  - **After Import & Sync the dialog stays open with no result.** Six of nine rows remained, with no word on which
    were skipped, kept or failed. Add a Result step with the D11 count line.
  - Stated need: unmanaged skills stay visible and revisitable (*"I do want to always be able to know when I have
    unmanaged skills deployed in my environment"*).

- 2026-11-02 — grill: vocabulary D45 applies (Harness / Deploy / Update — ticket 32 renames first; this ticket writes
  the new words). Auto-deploy's state is shown in the review (D49): the Import & Deploy button's label and the review's
  summary line say whether the imported skills will be deployed to the selected harnesses or only added to the Library.
  Under D34 (arrival-only) an import's first deploy is the one time auto-deploy acts on these skills.
- 2026-11-02 — Vocabulary: read Tool/Sync/Refresh as Harness/Deploy/Update — ticket 32.
- 2026-11-04 — implemented by delegated child; awaiting Astra audit.
- 2026-11-04 — fix pass after audit: F1–F9 as the orchestrator disposed; re-check pending.
