# 09 — Skill action door, action lifecycle (Outcome executor, per-subject pending, focus rules), health/source fold

Status: implemented
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
