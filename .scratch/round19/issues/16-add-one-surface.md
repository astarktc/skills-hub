# 16 — Add as one stepped surface: sticky summary, picker search, one ask, retryable partial failure

Status: implemented
Blocked by: 04, 09, 15
Spec: `.scratch/round19/spec.md` — D12, D27 (grill Q10, Q21). Absorbs BACKLOG **#47** and D8's "sticky install
summary". Sources: `flows.md` Flow 1; `critique-astra` F06, F14; `critique-opus` P1-4; `audit.md` Add/picker rows.

## Work

- **One Dialog, three steps** — *Source* (Git URL / Local folder segmented control with real `aria-selected`; the
  optional name only when one candidate) → *Candidates* (the picker: search box filtering the list — only checked
  **and visible** candidates install, and the summary says "3 of 4 checked · 1 hidden by filter"; invalid local
  candidates disabled with the reason in words; each checkbox labelled by the candidate name) → *Settlement*
  (progress "Adding (i/n) name", then the report inline: installed rows ticked, failed rows with the typed reason and
  **Retry**, cancelled rows marked). A **sticky summary** across every step: source · N candidates · Tool scope (the
  effective target set with an "Edit Tools" popover using the catalog — visible, focusable Checkbox primitives; the
  audit's `display:none` inputs are gone).
- **Backend calls**: listing as today; install through ticket 04's **`install_selection` batch** with a progress
  Channel; collisions come back as rows and raise **one** ask (`useOverwriteConfirmation`'s list shape) with per-row
  Keep/Overwrite, showing path, modified time and "differs from the skill"; one retry batch with overrides; Keep
  records disabled (ticket 02). `install_git_selection` / `install_local_selection` are deleted from the frontend,
  then from `collect_commands!` and the fixture in the same change.
- **Cancel** (D20): between candidates; the settlement step shows what landed and what was cancelled; the dialog
  stays open on partial failure or cancel; closes only on full success or the operator's Close.
- **Add flow exposes intents** (arch 10): `useAddSkillFlow` becomes `open({ source?, preselect? })`, `install()`,
  `retry(ids)`; the form draft is internal; Explore's "install this" calls the intent (the render-cycle trigger
  effect goes). `useCandidatePick` keeps the pick logic; two instances remain internal.
- Vocabulary: "Add", "Add selected", "Adding (i/n)"; "Import" nowhere in this flow.
- The deploy-after-add choice ("Sync to selected tools after creation") stays, as part of the Tool scope summary.
- Migration: `AddSkillModal`, `GitPickModal`, `LocalPickModal` (or their replacement) join `migrated`; `pick-*`,
  `form` rules leave `App.css`.

## Gate (D31)

Astra audit: keyboard-only Add of a 4-candidate repo with two collisions — Tool scope changed by keyboard, search
filters, one ask, Keep on one / Overwrite on the other, settlement shows 4 rows, focus returns to the Add button;
Escape closes only the ask; 960×640; contrast; `npm run check`.

## Result

Implemented 2026-09-30 (child, Opus 5.5). Nothing committed. Evidence: `.scratch/round19/evidence/16/` (drivers
`gate.ts` · `flows.ts` · `a11y.ts` · `motion.ts`, trusted CDP keys + `Input.insertText` through `keys-cdp.ts`; JSON
transcripts, screenshots, `check.log`).

### Shipped

- **One stepped surface** — `components/skills/add/AddSkillDialog.tsx` (new, `lg`): one `addSkill` kind on the surface
  stack (the `gitPick` / `localPick` kinds are gone; the step is the flow hook's state). Header names the steps
  (Source › Skills › Result, `aria-current="step"`). **Sticky summary** under it: Source (`owner/repo` / folder) ·
  Skills ("3 of 4 checked · 1 hidden by filter"; "4 sent" on the Result) · Deploys to (the effective target set, first
  three named, "+N") with **Edit Tools** — a Popover over the catalog's installed Tools (plus a selected one detection
  no longer sees, "not detected") as visible Checkbox rows, and "N skills to M Tools"; the old `display:none`
  `tool-pill-toggle` inputs are gone. Add's progress (`HeaderProgress placement="dialog"`, its Cancel) sits at the
  summary's foot. **Source**: real Tabs (`aria-selected`) Git repository / Local folder, one Field each, refusals by
  the field (`FieldError`, never a toast); Continue lists. **Skills**: the one picker; a lone candidate offers
  "Name (optional)". **Result**: one row per sent candidate — Queued / Adding… / Waiting for the result → Added (what
  its first sync did: deployed to N Tools · kept the existing folder in X · N folders to review · not deployed to X:
  reason · skipped X), Failed (typed reason + **Retry**), Cancelled (+ **Add**); "Retry failed (N)" when several.
- **The one picker** — `components/skills/add/CandidatePicker.tsx`, `mode: "multiple" | "single"`: search (once there
  are 2+), "Select all [shown]" over visible selectable rows, a status line, rows whose control is named by the
  candidate (Field label) and described by description · path · "Can't be added: {reason}" (invalid local candidates
  disabled, out of the Tab order). **Single-choice mode replaces Re-point's radio step** (`RepointModal`, ticket 12's
  comment): native radios in a `radiogroup`, the same search, "N of M shown"; only a choice still shown is sent as
  `RepointTarget::Git { url, selection: { subpath, resolution } }`.
- **Intents** — `useAddSkillFlow` returns `open({ kind?, source?, preselect? })`, `install()`, `retry(ids)` and `add`
  (the view: step, kind, draft, refusals, picker values, settlement rows, summary, Tool choices, and the editing intents
  `setKind · editDraft · browse · list · back · setQuery · toggle · toggleAll · setTool · close`). The draft, the step,
  the listing and the settlement are internal. Explore's install calls `open({ source, preselect })` (the
  `exploreInstallTrigger` render-cycle effect is deleted); a given source is listed at once with the named skill
  checked (ambiguous → every match; missing → said, nothing checked); it never installs by itself. `useCandidatePick`
  is the pick logic only (listing + ctx, selection, search, `picked` = checked ∧ visible ∧ selectable), two instances
  internal; it no longer owns a dialog or an install.
- **Backend calls** — listing as before, as a `read: true` action cancellable at its end (`cancel: "installSelection"`;
  a Cancel discards the listing). Install: **one** `runAction({ cancel: "installSelection", activity: "Adding" })` per
  `install()` / `retry(ids)` → one `installSelection` batch with the progress Channel (`reportProgress` →
  "Adding (i/n) name", `done/total`), collisions → **one** ask through `useSyncOrchestration.resolveGlobalCollisions`
  (per-row Keep/Overwrite with path, modified time, "differs from the skill"; the retry under `uncancellable`), merged
  by `mergeInstallRetry`, folded by `installOutcome`. `retry(ids)` re-sends only the named not-installed entries.
- **Close rule** — `installOutcome.completion.closeModal` is now *full success* (installed ≥ 1 and no failure,
  cancellation, failed target or unanswered collision; a kept folder counts as success) and the flow also requires every
  sent entry installed; otherwise the dialog stays on its Result. Close is refused only while Add's own action runs.
- **Deleted** — `install_git_selection` / `install_local_selection`: frontend (`CANCELLABLE_COMMANDS`, fixture
  handlers + their tests rewritten onto `installSelection`), `collect_commands!` (`lib.rs`), the command fns +
  `InstallResultDto` + `to_install_dto` (`commands/mod.rs`), `installer::install_git_skill_from_listing` (dead);
  `install_git_selection_with` / `install_local_skill_from_selection` are now `#[cfg(test)]` doors. `cargo test`
  regenerated `src/bindings/index.ts`: **12 deletions, 0 additions** (the two commands + the DTO). Also deleted:
  `AddSkillModal.tsx`, `GitPickModal.tsx`, `LocalPickModal.tsx`; `useSyncOrchestration.targetAllInstalled` (its only
  caller was Explore's install).
- **Fixture** — `acme/collide-kit` gains a 4th candidate, `release-notes`, whose first acquisition is `RATE_LIMITED`
  (`FixtureRepoCandidate.failOnce`, consumed) so a Retry lands it; fixture test added.
- **Vocabulary** — the `add.*` family appended at the end of `en`; `outcome.done.installed` reads "N added"; Add's
  failure title is `add.failedTitle`; single install toast "{name} added."; 20 dead Add/picker keys removed
  (`addSkillTitle`, `gitPickTitle` "Select skills to import", `installSelected`, …). No `add.*` string says Import;
  the one Import mention reachable from Add is `localSkillInvalid.insideToolDir` ("…use Import instead"), which names
  Onboarding import on purpose.
- **Styling** — `AddSkillDialog`, `CandidatePicker` and `ToolConfigModal` (migrated so every `pick-*` rule could go)
  join `migrated`; 33 rule blocks left `App.css` (`pick-*`, `tool-pick-list`, `tool-filter-toggle`, `tool-matrix`,
  `tool-pill-toggle`, `status-badge`, `tabs`, `tab-item`): ceiling **1862 → 1676**. The `form` family
  (`.form-group/.label/.input/.input-row/.input-action/.helper-text`) stays — Project modals and InvocationModeModal
  still use it (handed to 17).
- **Docs** — DESIGN.md § Add (ticket 16) and the motion-table row; AGENTS.md's "remain registered until ticket 16"
  sentence now says `install_selection` is the only install command.

### Decisions

- **Install landing moves, once, small**: the Result row's settled glyph fades in (opacity 0 → 1, 120 ms `ease-out`,
  `@starting-style`), staggered 20 ms per row (≤ 8) — DESIGN.md's rare moment as a sequence of ≤ 120 ms steps.
  Nothing else in the dialog moves; reduced motion shows it settled. Measured below.
- **The optional name lives on the Skills step**, shown only when the listing holds one candidate (the count is unknown
  on the Source step).
- **A listing always lands on the Skills step**, even for one candidate (the stepped surface; the Tool scope and name
  are visible before anything installs).
- **Edit Tools is a `PopoverTrigger`, not `PopoverContent anchor`**: the button raises it on the operator's own press,
  which DESIGN.md reserves the Trigger for (`anchor` is for triggerless popovers such as the row switch's); the
  Trigger gives `aria-expanded` / `aria-haspopup` and the focus return for free.
- **`bulkCounts.syncCounts` does not fit** (it counts pairs of existing managed skills); the popover says "N skills to
  M Tools" from the summary.
- **Focus**: each step's first control takes focus in the same commit as the step change (`useLayoutEffect`); a
  focused control a render removes — notably the summary's Cancel, withdrawn when the ask's retry starts — hands focus
  to the step target in that commit, so the ask returns to the Result heading and no frame lands on BODY.
- An installed row's unwritable Tool (`TOOL_NOT_WRITABLE` skip) reads as "not deployed to X: reason", the same rule
  as `installOutcome`'s install fold.

### Verification (fresh, fixture 5175 `failures`; dark 1280×800 and light 960×640)

- **Gate transcript** (`gate-{dark-1280,light-960}.json`, 67 keyboard steps each, **0 BODY**, every stop
  `:focus-visible`): Tab → Add skill → Enter (focus: URL field) → type URL → Enter → Skills (focus: Filter) → "es" →
  **"3 of 4 checked · 1 hidden by filter"** → ⌘A ⌫ → "4 of 4 checked" → Space on commit-helper "3 of 4" / "4 of 4" →
  Shift+Tab → Edit Tools → Enter (focus: Cursor checkbox) → Tab to Gemini CLI → Space ("+5" → "+4") → Escape (focus:
  Edit Tools) → Add selected → Result heading focused, "Adding (2/4) commit-helper · Cancel" in the summary → **one
  ask, two rows** (lint-staged-rules, changelog-writer; focus Overwrite) → per-row Keep on one / Overwrite on the
  other → Apply choices → **4 rows**: 3 Added (changelog-writer "kept the existing folder in Claude Code"), release-notes
  Failed "GitHub API rate limit reached…" + Retry → Retry release-notes → summary "Adding (1/1) release-notes" (only
  the failed id) → "Added 4 of 4" → Close → **focus on Add skill**.
- **Flows** (`flows-*.json`, 0 BODY): Cancel mid-install from the summary → "Cancelling…" → the landed skill's
  collision still asks once → **Escape closes only the ask** (1 dialog left, focus the Result heading) → Result "Added
  1 of 4": 1 Added (1 folder to review) + 3 Cancelled with Add; dialog stayed open. Local folder `~/Projects/team-skills`
  via the Local folder tab (ArrowRight, `aria-selected=true`): draft-notes / scratch disabled, `tabindex=-1`,
  described "Can't be added: Missing name in frontmatter" / "Missing SKILL.md"; Tab skips them. Explore Install →
  Add opened on `some-org/ui-ux-pro-max`, listed, its skill checked, focus on it; Escape → focus back on Install.
  Re-point deprecated-helper → collide-kit → the single-choice picker (radiogroup, search focused), "notes" →
  "1 of 4 shown", Space chooses release-notes; Escape → "More actions for deprecated-helper".
- **axe 4.10.3, 8 runs** (candidates + Tools popover, the ask, the Result, Re-point's picker × dark 1280 / light 960):
  **0 violations**; incompletes only Base UI's `aria-hidden-focus`, the popover trigger's portal `aria-controls`, and
  two contrast nodes overlapped by the popover (measured below).
- **Contrast** (rendered, composited): Add dialog min **4.68 dark** (white on primary) / **5.32 light**; Tools popover
  7.78 / 8.36; the ask 4.68 / 5.86; Result 4.68 / 5.32; Re-point picker 5.73 / 6.24; **0 text pairs < 4.5**.
- **Motion** (`motion-dark-1280[-reduce].json`): landing glyphs `opacity` 120 ms `cubic-bezier(0.16,1,0.3,1)`, delays
  0 / 20 ms, settled 148 ms after the rows settled, 0 transitions running after; **reduce**: opacity 1 on the first
  frame, no transition.
- **Tests**: vitest 924 (`useAddSkillFlow.test.ts` rewritten: open / list / visible-and-checked / Explore preselect /
  install / partial failure / retry(ids) / whole-command failure / Close / no-targets / auto-sync off / one ask (Keep +
  Overwrite) / retry-throws / Cancel listing & batch over the real reporter / progress ticks; `useCandidatePick.test.ts`
  rewritten; `lib/addFlow.test.ts` new; fixture test for the 4th candidate). Gate: see `check.log`.

## Comments


- 2026-09-30 — from ticket 12: Re-point's multi-skill case lists the repository's candidates as a radio step
  inside `RepointModal` (single choice), because `GitPickModal` is multi-select and bound to `useAddSkillFlow`. When
  the one picker lands, give it a single-choice mode and use it there. A pick crosses the wire as
  `RepointTarget::Git { url, selection: { subpath, resolution } }` (new; the preview's `ref` is null for a
  default-branch repository, so no URL can name the folder) — the same shape Add's batch sends.

- 2026-09-30 — from ticket 13: `ConfirmDialog` gained `initialFocus: "cancel" | "confirm"` and `PopoverContent`
  gained `anchor` (a triggerless, controlled popover with `initialFocus` / `finalFocus`) — both documented in
  DESIGN.md. The Sync-to-Tools pick in `CountConfirmModal` (a two-column checkbox FieldSet over the installed Tools,
  counts from `lib/bulkCounts.syncCounts` following the pick) is a candidate for Add's Tool-scope step.

- 2026-09-30 — from ticket 14: the Add dialog, both pickers and Import render `<HeaderProgress placement="dialog">`
  at their foot (a `progress` prop App passes) — "Adding · Cancel" while `installSelection` / the listing runs; Cancel
  there is the header's (the page behind is inert). Callers set `activity` / `label` on `runAction`
  (`activity.adding`, `activity.labels.add`); a one-surface Add should keep the progress in its sticky summary instead.
  `installOutcome` folds an unanswered collision as "N folders to review" (never "targets failed"). After a cancelled
  listing `returnLostFocus` puts focus back on Install.

- 2026-09-30 — from ticket 15 (Emil pass): (1) **One visible progress** — `App.tsx`'s `progressInDialog` lists the dialogs
  whose foot shows the progress (Add, Import, the two pickers, Re-point, Notifications); the page copy yields while one
  is open (`HeaderProgress yielded`). When Add becomes one stepped surface, keep that list true (drop the pickers if
  they merge into it). (2) **Motion**: DESIGN.md § Motion now has the motion table; "an install landing" is one of the
  two permitted rare moments — decide whether it moves at all (≤ 120 ms steps, opacity/transform, reduce removes);
  the strip light (`useSyncLight`) already lights a new install's marks as they become synced. (3) `App.css`'s
  `transition: all` declarations are gone (hover/focus on the legacy `.input`, `.btn`, `.discovered-list` rules is now
  instant) — nothing to restore; migrate onto the primitives. (4) A Button/Switch that holds focus while disabled draws
  at 75 % while focused (ring ≥ 3:1) — use `focusableWhenDisabled` / `pending` rather than a local opacity.

- 2026-09-30 — child (Opus 5.5): `ready-for-agent` → `implemented`, uncommitted over `93c77dc`; evidence
  `.scratch/round19/evidence/16/` (`gate-{dark-1280,light-960}.json`, `flows-*.json`, `a11y-*.json`, `motion-*.json`,
  `check.log`); see § Result. Awaits the D31 Astra audit. Handoffs written to 17 (the `form` rules' last users), 20
  (ToolConfigModal migrated), 21 (picker + vocabulary), 22 (Explore's install is the Add intent).

- 2026-09-30 — independent D31 audit of `f5615fd`: **FIX-THEN-PASS**, three P1s: A1 shared-folder confirmation also dismisses Edit Tools and loses checkbox focus; A2 a clean candidate Retry auto-closes Result despite earlier unresolved collisions/target failures; A3 changing Source tabs during listing opens an empty Skills step with focus on BODY. Reproduced in dark 1280×800 and light 960×640; exact fixes/re-check scope in `.scratch/round19/review/ticket-16-audit.md`, fresh evidence in `.scratch/round19/evidence/16-audit/`. Ordinary keyboard Add/one ask/filtered actual submit/failed-id Retry/Cancel/local/Explore/Re-point wire pass; 12 axe runs have zero violations, measured contrast and landing/reduce pass. Full version + check gate passes (924 Vitest / 807 Rust; 1676 CSS lines / 29 migrated), export bindings and `cargo test --all` pass with no source/bindings drift. Repeated Result permission prose is P2 only; minimum-size scrolling remains usable. ToolConfigModal migration, syncAfterCreate copy and AGENTS sentence are accepted. A1–A3 require fixes and bounded re-audit before acceptance; A4 may be deferred. No application source edited or commit made.
