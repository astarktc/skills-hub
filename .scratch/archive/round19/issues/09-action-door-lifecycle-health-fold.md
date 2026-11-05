# 09 — Skill action door, action lifecycle (Outcome executor, per-subject pending, focus rules), health/source fold

Status: done — 302bbb1
Blocked by: 02, 03, 08
Spec: `.scratch/round19/spec.md` — D7 (arch 2, 3, 6), D19, D20, D21. Source: `architecture.md` candidates 2, 3, 6.

## Work

- **Action door — `src/hooks/useSkillActions.ts`** (candidate 2): one `run(verb, skills: ManagedSkill[] | ids, opts)`
  with verbs `syncToTargets · disable · enable · update · restore · repoint · detach · delete · editInvocation ·
  assignToProject · unassignFromProject · deployGroup`. The door decides batch vs one-at-a-time, where confirmation
  happens (through the Dialog-backed ask hooks), which fold settles it, and the reload rule ("a thrown sync also
  reloads" — `syncOrReload` — becomes door knowledge). Card, inspector, bulk rail and palette are callers. The 11
  per-skill callbacks threaded App → `SkillsList` → `SkillCard` collapse to one handle. Backend reached only through
  `invokeTauri`.
- **Health/source fold — `src/lib/skillPresentation.ts`** (candidate 3): `skillHealth(skill, toolCatalog) →
  { rows: HealthRow[], severity, source: { label, href, copyValue, icon } }` with skill-level rows (Unlocatable source
  / central copy, stored `health` from ticket 02: fetch failed, Edit conflict, delete incomplete, …) and target-level
  rows (N failed, N stale, disabled, not deployed), one precedence rule. Card, ladder, banner, chip and strip read it.
  Corpus-style tests as in the file's existing pattern. `SkillDetailView`'s second source-line rule is deleted.
- **Lifecycle — `useStatusReporter`** (candidate 6): the reporter **executes an `Outcome`** (`applyOutcome(outcome)`
  — toast with the composite headline, history entry with `runId`, completion), so the five hand-rolled unpackers
  (`useSkillLibrary`, `useAddSkillFlow` ×2, `useCandidatePick`, `ProjectsPage`) go. One failure dialect (`notifyError`);
  `setError(formatError())` sites migrate. **Per-subject pending**: `pending: { kind: 'skill' | 'cell' | 'batch', id? }`
  replaces the single `loading` boolean and its 18 prop-threading sites; a streamed batch exposes `{ label, done,
  total, cancellable }` for the header progress (rendered in ticket 14; the `LoadingOverlay` keeps rendering from the
  new state until then). The project world's `pendingCells` folds into it. **One mutation at a time** (D19) is a
  reporter rule: while a mutation is pending, other mutations are refused with a typed reason the UI can show.
- **Cancel (D20)**: `cancellable` is true only for the token-taking commands (list them from `commands/mod.rs` +
  ticket 04's batch); `cancelLoading` no longer hides anything it did not stop.
- **Focus rules (D21)** live here as `focusAfter(outcome, context)` returning the target per the spec's table, consumed
  by the Dialog primitive's `finalFocus` and by list/inspector after mutations. Unit-tested as a pure function.
- Tests: door (parametrised over verbs: fold called, reload rule, confirmation path), reporter (Outcome execution,
  pending kinds, refusal while pending, cancellable set), fold (corpus).
- AGENTS.md § Ambiguity: the "Frontend presentation logic" and hook bullets gain the door, the fold and the reporter's
  Outcome execution; the "props carry state, never pre-bound functions" rule is restated for the door handle.

## Gate

`npm run check` green; fixture walk: every existing action still works from the card through the door; a Refresh
shows the progress state; Cancel appears only on Refresh/Update/Re-point/Add.

## Review

Astra @ high: the door's confirmation and reload paths, the reporter's one-at-a-time rule under a thrown request,
focus-rule table coverage.

## Result

Implemented 2026-09-29 (child, Opus 5.5), sharing the tree with ticket 04's child, which was running at the same time. The
two coordinated over intercom: 04 owns the install batch, `useSyncOrchestration`, the Add hooks' bodies and their
test assertions; 09 made one-line switches in those hooks and edited only the reporter literals in their tests.

### Files (mine)

- **New**
  - `src/hooks/useSkillActions.ts` (+ `.test.ts`, 53 tests): the door.
  - `src/lib/actionLifecycle.ts` (+ `.test.ts`, 23 tests): pending subjects, `CANCELLABLE_COMMANDS`, refusal,
    `focusAfter`, `resolveFocusTarget`.
- **Rewritten**
  - `src/hooks/useStatusReporter.ts` (+ test): Outcome executor, pending, one-at-a-time, cancel, reads.
  - `src/hooks/useSkillLibrary.ts` (+ test): now only the catalog and the dialog subjects.
  - `skillPresentation.ts`: the health/source fold and the `deployAction` fix.
- **Surfaces**
  - `SkillCard.tsx`, `SkillsList.tsx`: one `actions` handle, `pending` + `blockedReason`, fold-driven source and
    health rows.
  - `SkillDetailView.tsx`: its second source-line rule is deleted in favour of `skillSourceLine`; it takes
    `actions`/`blockedReason`.
  - `FilterBar.tsx`: `actions`, `blockedReason`.
  - `Header.tsx`: the unused `loading` prop is gone.
  - `LoadingOverlay.tsx`: renders from `pending`; Cancel only when `cancellable`; "Cancelling…" disabled.
  - `DeleteModal.tsx`: `fallbackFocus` prop.
  - `App.tsx`: the action wiring.
- **Project world**
  - `ProjectsPage.tsx`: every matrix and project mutation runs under `runAction` with a cell or batch subject, and
    settles through `applyOutcome`.
  - `useProjectState.ts` (+ test): `pendingCells` state deleted.
- **Failure dialect**: `useSettingsState.ts` (+ test) and `useExploreState.ts` migrated to `notifyError`/`notify`.
  The Explore preview is `read: true, cancel: "cloneExploreSkill"`.
- **One-liners in 04's files**
  - `useAddSkillFlow.ts`: `publishOutcome` → `applyOutcome`; `cancel: "installSelection"` on create local/git;
    `loadPlan` is `read: true`.
  - `useCandidatePick.ts`: `applyOutcome`; `cancel: "installSelection"`.
  - Their tests: reporter literal members only.
- **Rust**
  - `core/skill_health.rs`: `OutcomeFamily::Import`, `dismissable_family`, `dismiss_outcome`.
  - `commands/mod.rs`: `clear_skill_health(skillId, kind)`, with `#[specta::specta]`.
  - `lib.rs`: `collect_commands!`.
  - Test `only_an_incomplete_import_is_dismissable_and_a_newer_outcome_survives_the_dismiss`.
  - Bindings regenerated by `cargo test`.
- **Fixture**: `clearSkillHealth` handler.
- **i18n** (appended at the end of `en`): `lifecycle.*`, `health.rows.*`, `health.repairs.*`.
- **AGENTS.md § Ambiguity**: the door, the reporter's lifecycle, the fold, `actionLifecycle.ts`, and the
  door-handle rule.

### The door's verb table (`run(verb, skills | ids | "all", opts)`)

| verb | grain | confirmation | backend / seam | fold | reload on success | reload on throw |
|---|---|---|---|---|---|---|
| `syncToTargets` | `tool` → pair (pill); else effective targets / `tools` | pair: shared-dir ask; empty target set → warning, nothing sent | `sync.syncSkillsToTools` | `syncOutcome` toggle / bulk | completion | yes |
| `retry` | each error/missing row's Tool (or `tool`) | — | `syncSkillsToTools` per Tool | `syncOutcome` | completion | yes |
| `disable` | `tool` → `skill_tool`; one → `skill`; set/`"all"` → `skills` | pair: shared-dir ask | `disableSkillTargets` | `removalOutcome` toggle/skill/all | completion | no |
| `enable` | same scopes | pair: shared-dir ask | `sync.enableSkillTargets` (04's one ask) | `enableOutcome` | yes | yes |
| `update` | one → `updateManagedSkill` (catalog replaced); set → `refreshManagedSkills(ids)`; `"all"` → `(null)` | — | cancel: update/refresh | `refreshOutcome` (single / batch) | batch only | yes |
| `restore` | batch of one per skill | — | `updateManagedSkill` | `refreshOutcome` single | no (replaced) | yes |
| `repoint` | one | no `target` → opens Change source (`preselect` or `repointKind`) | `repointSkillSource` (trimmed), cancel | `refreshOutcome` single; closeModal → dismiss | no | yes |
| `detach` | one | (confirm: ticket 12) | `detachSkillFromSource` | success toast | yes | no |
| `delete` | one | no `confirmed` → opens Delete dialog | `deleteManagedSkill` | `deleteOutcome`; closeModal → dismiss | completion | no |
| `editInvocation` | one | no `mode` → opens Invocation dialog | `setSkillInvocationOverride` | `invocationEditOutcome`; closeModal | no (replaced) | no |
| `dismissHealth` | skills | — | `clearSkillHealth(id, kind)` | — | yes | no |
| `assignToProject` / `unassignFromProject` | skills, one at a time | — | `projects.assignToProject` / `unassignFromProject` | `projectSyncOutcome` bulkAssign / `removalOutcome` bulkUnassign (merged reports) | no | no |
| `deployGroup` | set | empty targets → warning | `enableSkillTargets({skills})` + `syncSkillsToTools` | `enableOutcome` + `syncOutcome` | yes | yes |

- `repair(skill, row, key)` maps each fold repair onto its verb. `source_missing` → Re-point on `local`.
  `delete_incomplete` → Delete (Retry = finish the delete). `import_failed` → `dismissHealth`.
- The handle is stable across renders (a latest-ref), and ids resolve at click time.
- The card chooses verbs from `deployAction` and the pill state: disabled → enable; target → disable;
  none → sync.

### The fold (`skillHealth(skill, { t })` → `{ rows, severity, source }`)

- **Row kinds, in the precedence `HEALTH_ROW_ORDER`, which is also severity order:**
  - Failed: `central_missing`, `restore_failed`, `delete_incomplete`, `targets_failed`, `targets_missing`.
  - Stale: `source_missing`, `repoint_failed`, `fetch_failed`, `edit_conflict`, `import_failed`,
    `targets_stale`.
  - Muted: `disabled`, `targets_disabled`, `not_deployed`.
- `severity` is the loudest row, or `healthy` when there are none.
- Each row carries `scope`, `count`/`tools` (target rows), `error`/`at` (the stored kind) and its `repairs`.
- A live `invocation_override.conflict` and a stored `edit_conflict` fold into one row.
- `targets_disabled` and `not_deployed` are suppressed under skill-grain Disable. `not_deployed` is also
  suppressed while Unlocatable.
- `source` = `skillSourceLine`:
  - GitHub → `owner/repo` + href + copy.
  - Other git / local → the recorded ref, copyable (D15).
  - Imported → "Managed here" + "Imported from {Tool}", with no copy value.
- The Explore preview uses the same function.
- **`deployAction` fix:** `isLiveTarget` excludes `disabled` rows and never-synced `error` rows. A skill whose only
  target is a failed first sync now reads `sync`, so Deploy/Retry is a first sync.

### The reporter after

- **Interface:** `pending`, `loading` (derived), `refusal`, `actionMessage`, `runAction`, `setActionMessage`,
  `reportProgress`, `applyOutcome`, `notify`, `notifications`, `unreadCount`, `markAllRead`,
  `clearNotifications`, `setError` (legacy), `setSuccessToastMessage`, `formatError`, `notifyError`,
  `showActionErrors`/`showActionWarnings` (legacy), `copyToClipboard`, `cancelPending`.
- **Removed:** `loadingStartAt` (now `pending.startedAt`) and `cancelLoading`.
- **`runAction` options:** `subject` (`skill` / `cell` ids / `batch`, default batch), `label`,
  `cancel: CancellableCommand`, `read`.
- **`applyOutcome(outcome, { reload, closeModal, onAction })`:**
  - reloads first;
  - then shows the error batch toast, the warning batch toast and the headline toast (tone kind);
  - records one `recordRun` block (headline over its entries; quiet notes are rows only when no headline carries
    them; silenced entries are dropped);
  - closes the modal;
  - resolves an entry's declarative action through `onAction`.
- **One mutation at a time:**
  - A second `runAction` is refused (ref-checked, same-tick safe): an info toast with `describeRefusal` ("Refresh
    is running — wait for it to finish"), no history entry, and the body never runs.
  - `read: true` actions wait for the slot. This was found in the fixture: the startup plan load and the
    scan-scope reload collided.
  - The slot is freed after a thrown request.
- **Cancel:** `cancellable` only when `cancel:` names one of `CANCELLABLE_COMMANDS`:
  - `refreshManagedSkills`, `updateManagedSkill`, `repointSkillSource`, `previewRepointSource`,
    `installGitSelection`, `cloneExploreSkill` (the token-taking commands in `commands/mod.rs`);
  - plus 04's `installSelection`.
  - `cancelPending` invokes `cancelCurrentOperation` and marks `cancelling`, but keeps `pending` until the command
    returns.
- **Presentation by scale:** the overlay renders only for a batch, a streamed action or a cancellable one
  (`showsProgressSurface`). Skill actions pend inline on the card (`aria-busy`); cell actions pend in the matrix
  (`pendingCellKeys`). Every verb control is disabled with `blockedReason` as its title.

### Focus rules (D21, `focusAfter` + `resolveFocusTarget`)

- **Rules:**
  - Delete that removed the skill → the next visible row, the previous one if it was last, else the heading. It
    falls back to the main landmark while no list heading exists; ticket 11 adds `data-surface-heading`.
  - Restore → `data-skill-health` (the card's health row), then the row.
  - Everything else → the same control (the trigger captured at `run` start).
- **How it is applied:**
  - After settle, the door refocuses only if focus was lost (on body or on the trigger). It retries for up to 5
    ticks until the control is re-enabled. It leaves dialog-confirmed verbs to the Dialog primitive.
  - The Delete dialog's `fallbackFocus` is the door's `dialogFallbackFocus`, decided before the dialog closes.
  - `resolveFocusTarget` does no `inert` check: the closing modal still marks `#root` inert when Base UI asks for
    the target. The fixture proved this.

### Tests

- **Door:** `the door, verb by verb` (16-row `it.each`: fold called, backend reached, reload rule, pending cleared);
  ids resolved / unknown dropped; batch vs one; Re-point trim + close; dismiss; deployGroup partition;
  `where confirmation happens` (3 open-dialog cases, 3 shared-dir-declined cases, empty targets, Delete →
  next-row fallback); `the reload rule on a thrown request` (16 rows against `RELOAD_ON_THROW`);
  `one mutation at a time`; the preselect table + health-row repoint repair; report action by id + gone-skill
  warning; `canRepoint`.
- **Reporter:** cancel keeps pending until return; no cancel for a non-cancellable action; pending subjects ×3;
  refusal in the same tick; slot freed after a throw; `reportProgress`; the 5 `applyOutcome` tests; the read waits.
- **Fold:** `deployAction` (8 rows, incl. failed first sync); `skillSourceLine` (5); `skillHealth` (15-row corpus +
  dedupe, counts, precedence, disabled suppression, stored error).
- **Lifecycle:** the `focusAfter` table (10), dialog fallback, the resolver (3), refusal, pending readers, the
  cancellable set.
- **Rust:** the dismiss test.

### Gate

- `npm run version:check` → Version OK (1.2.18).
- `npm run check` → exit 0: lint clean, styles:check ok, vitest 32 files / 690 passed, build ok, rustfmt/clippy ok,
  cargo test 789 passed. This was the final run with 04's work in the tree.

### Fixture walk (5175, `failures`, t3 preview, driven in `preview_evaluate`)

- **Health and dismissal:** every health kind renders with its repairs. Dismiss clears `import_failed`.
- **Refresh:**
  - The overlay shows the streamed line "Fetched (1/19) tdd ..." with Cancel.
  - Every card verb and toolbar button is disabled, titled "Refresh is running — wait for it to finish".
  - Cancel → "Cancelling…" (disabled), and the overlay stays until "Refresh cancelled — nothing was changed".
- **Single-skill verbs:**
  - Update: the overlay has Cancel.
  - Re-point from the card: opens Change source; its submit shows the overlay with Cancel; a failure keeps the
    dialog.
  - Pill toggle off/on: inline `aria-busy`, no overlay; focus stays on the pill.
  - Deploy button: disable → focus stays on it; enable raised 04's overwrite ask.
  - Detach → the imported source line.
  - Restore → overlay with Cancel; focus lands on the row.
  - Review invocation → dialog → saved.
  - Finish removing (kept) → the dialog stays with its report toast.
  - Delete `store-hiccup` → "Skill removed.", and focus lands on the next row `tdd`.
- **Other surfaces:**
  - Detail: source line `obra/superpowers`; Change source goes through the door.
  - Add (git URL) → overlay with Cancel → the picker opens.
  - "Uninstall from tool directories" → batch overlay without Cancel.
  - Projects: toggling a second cell while one pends → "Project sync is running — wait for it to finish". The
    label was later changed to "Project assignment is running…".

### Deferrals

- **Ticket 11:** a `data-surface-heading="skills"` on the list heading, so the empty-list focus target exists
  instead of the main-landmark fallback. The strip and ladder should render `skillHealth` rows; the card shows
  only its skill-level rows today. Group "Deploy all" is a caller of `run("deployGroup", group.skills)`.
- **Ticket 12:** a Detach confirmation (a confirming option on `detach`). The inspector ladder reads `fold.rows` /
  `repair`, and inline Retry is `run("retry", [skill], { tool })`.
- **Ticket 13:**
  - The bulk rail calls `run(verb, surface.selection.skills ids)`.
  - Count-bearing confirmations need an ask kind; `useSurfaceState` ask kinds are 04's this round.
  - `run("disable","all")` replaces the toolbar button.
- **Ticket 14:**
  - Render the header progress from `pending.{label, progress, cancellable, cancelling}` and delete
    `LoadingOverlay`.
  - "View report" should read `runId`.
  - Known edge: when a mid-action ask dialog (overwrite) closes, the Dialog primitive's return can land on another
    control, so the door does not re-take focus. Give asks a `fallbackFocus` = the action's trigger.
- **Legacy reporter members (04's files):**
  - `setError` remains on the reporter only for 04's files: `useSyncOrchestration` ×2 and `useAddSkillFlow` ×5 /
    `useCandidatePick` ×1 (validation copy). Migrate those to `notifyError` / `notify("error", …)`, then delete
    `setError`.
  - `showActionErrors` / `showActionWarnings` have no production caller left. Only the Add tests' reporter
    literal forwards to them. Delete them with 04's test rewrite.

## Comments

- 2026-09-29 — from ticket 02: `ManagedSkill.health: SkillHealth | null` (`{ kind, error, at }`, six kinds) is on the wire; `skillPresentation.skillHealth` is a stub returning it — build the full fold here. Decide how `delete_incomplete` / `import_failed` are dismissed (today only a newer outcome or Delete clears them) — recommend: Retry (delete → finish; import → nothing to retry, so an explicit "Dismiss" that clears the family). `deployAction` counts an `error` row as deployed, so a skill whose only target is a failed first sync shows no deploy — the door's `retry` verb must cover it.
- 2026-09-29 — adversarial review: **FIX BEFORE ACCEPTANCE**, `.scratch/round19/review/ticket-09-review.md`. Blocking: B1 project registration/Edit/path mutations bypass the reporter and other project controls remain enabled; B2 Refresh/re-assert and Update/Restore/Re-point/Edit propagation collisions never reach the one overwrite ask (04/09 integration); B3 compound Deploy/Retry/project-assignment actions can ask repeatedly and Deploy publishes separate runs. Should-fix S4–S9 cover mixed-group/no-target reload, impossible stale-target repairs for imported skills, grouped Delete focus (fixture: store-hiccup → tdd instead of json-canvas), post-Cancel progress copy, queued reads after unmount, and Add/picker completion still interpreted outside applyOutcome. Requested gates pass: vitest 22 files/541 tests, Rust skill_health 19 tests, export_bindings 1 test with unchanged SHA-256, tsc -b --noEmit exit 0. Review only; source untouched; status not advanced.
- 2026-09-29 — review fixes (B1–B3, S4–S9 + the defensive confirm test), child Opus. Status stays `implemented`.
  - **B1** — every project mutation moved into `src/components/projects/useProjectActions.ts` (new) under
    `runAction`: registration (register + select; the tool-config dialog opens after), Edit save (`updateGitignore`,
    `successToast`), path update (the picker stays outside and is not even opened while refused; the
    `updateProjectPath` it leads to is gated), configure / remove / resync / resync-all / cell / row. The hook derives
    `blockedReason` from the reporter's `pending`; `ProjectList` (update-path), `AssignmentMatrix` (every cell, row
    verb, Sync project / Sync all; memo comparators include it), `EditProjectModal` (Save) disable with it as the title,
    and Add / Remove / tool-config dialogs take `loading={pending !== null}` (were `false`). `pendingCells` is only the
    localized mark. Tests: `useProjectActions.test.ts` — "while a cell toggle runs, '%s' is refused and sends no
    command" × 10 mutations (zero `invokeTauri` calls, picker not opened, `blockedReason` names the running action);
    "registration, the Edit save and a path update run under the lifecycle when nothing else does".
  - **B2** — the sync world gained the collect-then-ask-once seam: `collectSync` (the batch without its ask),
    `collectEnable`, and `settleCollisionParts(parts)` over `sync | enable | project | refresh | propagation` report
    parts (pure shapes in `overwriteAsk.ts`: `propagationCollisionRows`, `refreshCollisionRows`,
    `mergePropagationRetry`, `mergeRefreshRetry`, `CollisionRetry`, `DeclinedPair`). Update / Restore / Re-point
    (`refreshOne`), Refresh-all and Edit route their report through it inside `action.uncancellable` (no Cancel during
    the ask/retry); after an answered retry the door re-reads the catalog and `refreshView`s every touched project
    (`ProjectState.refreshView` now public); the fold runs once over the merged report. A kept Propagation row
    becomes `declined` (the report shape has no declined status) → `refreshOutcome` / `invocationEditOutcome` take
    `ctx.declined` (count + quiet note/detail). `syncSkillsToTools` / `enableSkillTargets` / `settleProjectCollisions`
    / `resolveGlobalCollisions` are now thin wrappers over the same seam. Tests (`useSkillActions.test.ts`, "one
    operator action, one overwrite ask (the real sync world)" — real surface + sync + catalog + reporter):
    update / restore / repoint re-assert + project Propagation collisions → one ask with both rows, exact retry,
    catalog + project view re-read, no error entry, one run; dismissed → no retry, failure stays; Refresh-all Keep →
    `keep` decision, folded once with `declined`; Edit Propagation → ask + retry. Pure: `overwriteAsk.test.ts`
    "Propagation / re-assert rows (ticket 09 B2)" ×3; `reportOutcome.test.ts` "an answered overwrite ask inside
    Propagation-shaped reports" ×3.
  - **B3** — `deployGroup` collects Enable + first sync, then one `settleCollisionParts`, one `combineOutcomes`
    (new, `reportOutcome.ts`: first toast heads — the sync count line when it ran — re-toned on another part's
    failure, other parts' notes as detail), one `applyOutcome` → one `recordRun`. `retry` collects one batch per
    distinct skill Tool-set (exact pairs), then one ask. Multi-skill `assignToProject` / `unassignFromProject` now take
    `skillIds` in the project world: every per-skill report first, then one `settleCollisions`, one view re-read.
    `settleClaims` retries with **pair identity** (one sync batch per skill Tool-set; 04's Cartesian retry replaced —
    its test updated to pin two exact batches, plus "skills asking about the same Tools share one retry batch").
    Tests: "deploy a group: a disabled skill's Enable and a new skill's sync collide — one ask with both pairs, one
    run"; "retry over two occupied Tools: one batch, one ask with both pairs, one retry of exactly them";
    `useProjectState.test.ts` "assigns several skills, then asks about all their collisions once".
  - **S4** — runnable work decided first; with no targets the Enable part runs, the undeployed part is a warning
    entry in the same run (`lifecycle.deployNoTargets`), one reload. Test: "a mixed group with no sync targets
    enables, reports the undeployed part, and reloads once (S4)".
  - **S5** — `skillHealth`: failed / missing / stale target rows repair with `retryTargets` for every provenance
    (central-to-target re-sync; never `update`); under skill-grain Disable they offer the new repair `disable`
    ("Finish disabling", `health.repairs.finishDisable`). `retry` selects `error | missing | stale` and takes
    `tools`; `repair` passes `row.tools`. Tests: `skillPresentation.test.ts` target repairs × {git, local, imported,
    imported (detached)} × {error, missing, stale}; "under skill-grain Disable a %s target offers finishing the
    Disable"; door test "a stale target's repair re-syncs that Tool from the central copy, never an upstream Update".
  - **S6** — `displayOrder(visible, groups)` (`skillPresentation.ts`); `useListLens` exposes `displayed` (and
    `visibleSelection` follows it); App's door `order`, and `SkillsList`'s flat render, read it. Test:
    `useListLens.test.ts` "walks a grouped list in its display order … (Delete's next row)" (interleaved repos:
    alpha → gamma, not beta).
  - **S7** — the reporter's `actionMessage` (and `LoadingOverlay`) derive "Cancelling…" from `pending.cancelling`;
    `cancelPending` no longer writes the line. Test: "a progress tick after Cancel keeps the Cancelling line until
    the command returns" (tick + another channel's `setActionMessage`; count still moves).
  - **S8** — reporter: queued reads are woken without running on unmount (`disposedRef`), and `read` actions take
    `stillWanted` (checked when their turn comes). Explore preview: generation + `isActive` (App: area is still
    `explore`), checked before running and before opening; import review: generation + `isReviewWanted` (area is
    still `skills`). Tests: `useStatusReporter.test.ts` "queued reads (lifetime and intent)" ×4 (mutation → two reads
    in order; unmount; stale intent dropped; rejection after a rejected mutation); `useExploreState.test.ts` (new) ×3.
  - **S9** — `useCandidatePick` takes `outcomeHandlers: { reload, closeModal }` instead of `afterBatch`; the picker's
    close handler resets itself, the form, then the parent. `useAddSkillFlow` (after the 04 child's recorded finish):
    `finishInstall` / `publishOutcome` deleted; single Add, the picker and Import each make one awaited
    `applyOutcome(outcome, handlers)` (nonfatal reload kept via `refreshWithoutFailingAction`). Tests: the two
    reporter doubles now execute handlers in the real order; picker "installs … resets and finishes" asserts
    `reload → entries → close`; "picker hands completion=%s to the one applyOutcome, never interpreting it"; Add
    "%s install applies fold completion" asserts one `applyOutcome` and reload before the toast.
  - **Defensive** — `confirmPair` catches a rejected shared-dir confirmation → `notifyError`, acts as "no". Test: "a
    rejected shared-dir confirmation reaches notifyError and writes nothing" (run resolves, zero commands).
  - **Command-API compromises** (src-tauri is the 04 child's lane this round): (1) `bulk_assign_skill` /
    `bulk_unassign_skill` are per skill, so a multi-skill assignment still sends one command per skill — but one ask,
    one report, one run; a batch command is a follow-up. (2) Deploy all is two commands (Enable scope + first-sync
    batch) with one ask/fold; no combined backend surface. (3) `resolve_project_collisions` answers no sync mode, so an
    answered Propagation row that synced *leaves* the report (the fold counts only failures) rather than being
    rewritten as `synced`. (4) Enable-route retries re-run the whole scope with overrides (04's shape).
  - AGENTS.md: the D12 bullet gains the collect-then-ask-once rule; the project-world bullet names `useProjectActions`
    and the every-mutation-gated rule.
  - Gate: `npm run version:check && npm run check` → exit 0 — Version OK (1.2.18); lint clean (0 warnings — the
    `idleWaitersRef` cleanup warning is fixed); styles:check ok; vitest 34 files / 744 tests; build ok; fmt; clippy;
    `cargo test` 792 passed.
  - Fixture (5175 `failures`, t3 preview, driven in `preview_evaluate`): Update on `prisma-review` → its re-assert's
    Cursor + Codex collisions raise **one** ask (2 rows), no Cancel during it; Overwrite → Cursor and Codex deployed,
    the unwritable-Amp failure reported. Deploy all over `[retired-playbook (disabled), prisma-review (undeployed)]`
    through the door handle → **one** ask with both pairs; Keep Cursor / Overwrite Codex → one run block "4 synced · 2
    skipped · 1 kept existing" + "prisma-review: kept your existing Cursor folder", retired-playbook enabled. The fixture
    has no disabled skill with an occupied target, so enable+sync *both colliding* is proven by the hook test only.
    One early scripted attempt saw the ask close ~180 ms after opening before any input; two instrumented reruns
    (immediately after load and 4 s after) kept it open without input for 2 s+ — treated as an automation artifact,
    not reproduced.

### 2026-09-29 — bounded committed re-review (Astra): fix-then-ship

Reviewed `df7cfc8` vs `6d7b6f4`, in a detached scratch worktree, not ticket 11's concurrent edits. Full evidence and fixes:
`.scratch/round19/review/ticket-09-review.md` § Re-review — 2026-09-29. **B2 and S4–S9 closed.** One ask/one run
mechanics of B3 are now correct; two narrower acceptance blockers remain:

- **B1 remainder:** every tested project mutation is admitted/refused correctly (ten held-toggle cases send zero
  commands); Update path refuses its picker. But **Add Project Browse** still calls `pickFolder` while blocked
  (`useProjectActions.ts:226`, `AddProjectModal.tsx:89`), and Add/Tool-config/Remove submit controls receive only
  `loading`, not the running reason. A scratch hook regression held a cell mutation, called `handlePickProjectFolder`,
  and failed the no-picker assertion: one call with `{ title: "projects.pathLabel" }`. Gate/disable Browse and thread
  the reason through those dialogs. This is not a claim that a second disk mutation escapes the repaired reporter.
- **B3-R1 (new aggregation regression):** `combineOutcomes` retains the first (sync) toast, dropping Enable's success
  summary and its declined/skipped counts/tone. Real-fold repro: one fresh target synced + one Enable target kept →
  **success-tone “1 synced”**, with no declined count (only a detail note); both successful → Enable disappears;
  Enable failure keeps its error row but contributes no failure count to the headline. Scratch regression expected
  declined tone and received success. Fold typed settled reports/counts once for the whole Deploy action, preserve
  every contribution, and test the headline/tone as well as the single run-id. Do not parse toast strings or add a
  second toast.

B2 verification included temporary assertions proving retries → catalog read → touched project view → fold for
Update/Restore/Re-point (3 pass); committed tests also cover Refresh-all Keep, Edit and dismissal. Pair-set batching
legitimately replaces 04's unsafe Cartesian retry and still groups identical Tool sets. F1 ownership and F2 Add-cancel
sensitivity checks pass/fail as intended; no pending leak or read-queue/uncancellable deadlock found in the reviewed paths.

**API-debt judgment for the parent to queue in BACKLOG:** accept for 1.3.0 (1) per-skill assign/unassign commands under
one outer action, followed later by a multi-skill command preserving partial reports; (2) Enable+sync as two disjoint
commands under one action, later a combined surface; (3) eliding successful answered project Propagation rows while
folds count only failures and views are re-read, later a lossless typed settlement; (4) re-running Enable's scope,
which currently reselects disabled rows and leaves successful/project-error rows out, later a pair-exact retry API.
These explicit exceptions to the one-command ideal **do not defer B3-R1's lossy operator report**.

Fresh final restored-snapshot gates: Rust requested filters **10 / 33 / 36 / 8 / 1**; Vitest **25 files / 664 tests**;
extra stored-identity/relocation Rust **1 / 14**. Bindings unchanged. Two residual scratch probes fail on the reviewed
implementation as described; both were removed. Scratch worktree restored clean and removed. Only review/ticket
appendices edited in main; no commit, archive, move or status advancement.

### 2026-09-29 — re-review residuals fixed (B1 remainder, B3-R1), child Opus. Status stays `implemented`.

- **B1 remainder** — `useProjectActions.handlePickProjectFolder` (Add Project's Browse) is refused before
  `pickFolder` opens while any action runs (same shape as `handleUpdatePath`: a refused `runAction`, answers null).
  The project dialogs now take the running reason instead of a bare `loading`: `AddProjectModal` (Browse + Register
  disabled, `title` = reason), `RemoveProjectModal` (Cancel + Remove disabled, Remove titled), and the shared
  `ToolConfigModal` gained an optional `blockedReason` (Confirm disabled + titled; App's global use is unchanged).
  `ProjectsPage` passes `blockedReason` to all three (Edit already had it). Tests (`useProjectActions.test.ts`): the
  held-toggle table gains "while a cell toggle runs, 'browse for a project to add' is refused and sends no command"
  (zero `invokeTauri`, zero `pickFolder`, `blockedReason` names the running action); the unblocked case asserts Browse
  opens the picker (`{ title: "projects.pathLabel" }`), returns the path and sends no command. Dialog prop wiring by
  inspection (no product-component JSX tests by design).
- **B3-R1** — `Outcome` carries an optional structured `tally` (`OutcomeTally`: done count(s) per verb + the shared
  failed/targetsFailed/skipped/declined/cancelled/review counts); the bulk `syncOutcome` and `enableOutcome` set it
  (Enable in targets: global synced + project synced/already assigned as `enabled`, failures/skips/declines summed
  from global and project rows). `combineOutcomes(parts, { t })` (signature changed) merges the tallies into **one**
  composite line (`"1 synced · 1 enabled · 1 kept existing"`, via the shared `tallyHeadline` behind
  `compositeHeadline`), picks the tone as the strongest over the merged counts, every part's toast tone and any
  error entry (D10 rank), keeps every error/warning/note, and carries all notes (plus any untallied part's sentence)
  as the detail. No toast string is parsed. The door's `deployGroup` passes both parts. Tests
  (`reportOutcome.test.ts` "combineOutcomes …"): table "fresh sync success + Enable with {one synced target → success
  `1 synced · 1 enabled`; one declined target (Keep) → declined `1 synced · 1 kept existing`; one Disabled skip →
  skipped; one failed target → failure `1 synced · 1 failed`}" over the real `syncOutcome` / `enableOutcome` /
  `combineOutcomes`; "sums a project row of Enable with the global parts, in targets"; the two no-tally cases kept.
  The real-sync-world door test "deploy a group: … one ask with both pairs, one run" now also asserts the one success
  headline `2 synced · 1 enabled`. The exact-shape bulk-declined `syncOutcome` test pins the new `tally`.
- **BACKLOG #64** records the four accepted command-API compromises (source: `round19/review/ticket-09-review.md`
  § Re-review); next free number bumped to #65.
- Gate: `npm run version:check && npm run check` → exit 0 — Version OK (1.2.18); eslint clean; styles:check ok;
  vitest 34 files / 768 tests; build ok; fmt; clippy; `cargo test` 792 passed.
- 2026-10-29 — **done — 302bbb1 (+ 17d3276 residuals)** (post-rebase SHAs on `origin/main`; the pre-rebase SHAs cited above exist only in the orchestrator's local history). Shipped in 1.3.0 (`v1.3.0`, published 2026-10-28; operator smoke on the main Mac passed 2026-10-29, ticket 19 § Smoke outcome). 19a closure: `round19/spec.md` § Closure — 2026-10-29.
