# Round-9 architecture panel — Opus seat

Explored at `96cd33c` (v1.2.8). Read `AGENTS.md`, `CONTEXT.md`, ADRs 0001–0003, the codebase-design
vocabulary, `git log --oneline -150` (hot files by touch count in the last 60 commits:
`useSkillLibrary.ts` 12, `installer.rs` 8, `App.tsx` 8, `install_finalize.rs` 7, `git_acquisition.rs` 6,
`commands/mod.rs` 6, `refresh.rs` 5). Checked `.scratch/round7/backlog.md` and `.scratch/arch-deepening/`
so nothing below repeats a scheduled or resolved item.

---

## 1. A Managed skill's content identity has no owner — Strong

**Files** `core/install_finalize.rs:469-484`, `core/skill_edits.rs:73-77`, `core/project_sync.rs:203-215`
and `:497-512`, `core/global_sync.rs:80-92`, `core/propagation.rs:104-108`, `:350-357`,
`core/sync_status.rs:161-184`, `core/content_hash.rs`.

**Problem** `skills.content_hash` is written by four modules under four different rules.
`install_finalize::compute_content_hash` computes it **only in debug builds or when
`SKILLS_HUB_COMPUTE_HASH=1`** (`:477-484`); `skill_edits::record_hash` computes it unconditionally
(`:75`); `project_sync::observe_assignment` lazily backfills it and writes it to the skill row as a
side effect of a *read* pass (`:503-507`); `global_sync::target_has_same_content` ignores the stored
value and re-hashes both sides. Nothing states which of these is the rule.

The consequence is live, not theoretical. In a release build finalize records `None`; propagation passes
that `None` down to every copy target (`propagation.rs:353-357`, doc: "absent when finalize did not
compute one"), so the assignment row is recorded `synced` with a NULL hash; the very next reconcile
pass hashes the source itself (`project_sync.rs:503`) and `next_status` sees `source_hash = Some`,
`recorded_hash = None` → **`Stale`** (`sync_status.rs:172-177`). Every copy-mode assignment goes stale
after every Update, in shipped builds only. Every test and every `tauri:dev` run is `debug_assertions`,
so the suite is structurally blind to it. `project_sync.rs:203-215` documents the opposite behaviour
("a hashing failure … the next reconcile pass then reports `Stale`" — but `next_status` returns
`obs.current` for a `None` hash), which is what a scattered rule looks like after two years.

**Solution** One module owns content identity: what a skill's hash *is*, when it is computed, and who
may write it. Finalize, Edits replay, propagation and the reconcile pass ask it rather than each
deciding; the perf switch becomes that module's business (and can be a real decision — cache by
mtime, hash on demand — instead of a `cfg!` fork of behaviour).

**Benefits** Locality: the copy-drift rule stops being an emergent property of four call sites.
Leverage: propagation and the reconcile pass stop threading `Option<&str>` hashes through their
signatures. Testability: the release policy becomes a value the test picks, so the shipped path is
exercisable at all.

**Evidence** Deletion test: delete the module and the complexity reappears in four places — it is
currently *already* deleted, and it has. Backlog item 9 (`IGNORE_NAMES` duplicated in `content_hash.rs`
and `skill_files.rs`) is a different, smaller observation about the same unowned concept.

---

## 2. Report → Notification folding is copied into every world hook — Strong

**Files** `src/hooks/useSkillLibrary.ts:168-262`, `:280-300`, `:330-360`; `useAddSkillFlow.ts:331`,
`:388`; `useCandidatePick.ts:196-222`; `useSyncOrchestration.ts` (`syncFailureEntries`).

**Problem** Turning a backend report into notifications is a pure fold — `(report, t, toolLabelById)
→ { toast, errors[], warnings[] }` — yet there are at least nine hand-rolled instances of it, each
buried in a `useCallback` inside a stateful hook: `skillFailureEntries`, `targetFailureEntries`,
`skippedEntries`, `editConflictEntries`, `removalFailureEntries`, `removalToast`, the `handleRefresh`
success-toast fold, `settleSingleReport`, and the add-flow's twins. They are pure, so nothing forces
them to live there; being there costs a dependency array each and makes them testable only through
1174 lines of `useSkillLibrary.test.ts`.

Worse, the impurity that *did* creep in caused shipped bugs. `skillFailureEntries` closes over
`managedSkills` for the repoint-eligibility check but must reach for `managedSkillsRef.current` inside
the notification's `onClick` because the list has moved on by then (`:76-79`, `:171-205`) — that ref
exists solely because of commit `70651c8` ("guard stale skill repair notification actions"). This is the
brief's third smell exactly: purity was *not* extracted, so the bug hid in how the fold was called.

**Solution** A pure presentation module beside `skillPresentation.ts` that takes a report plus the
labels it needs and answers with the notification set. The hook keeps only what is genuinely stateful:
which skill an action's button should reopen (an id, resolved at click time by the hook, not captured
by the fold).

**Benefits** Locality: "what does a batch report look like to the operator" lives once, next to
`describeCommandError`, the established home for that judgement. Leverage: four report shapes × N call
sites collapse. Testability: plain vitest on values, replacing slices of three large hook tests.

**Evidence** Deletion test passes loudly — the module does not exist and the complexity is spread over
four hooks. Commits paying the cost: `70651c8` (stale action guard), `4fe97ab` (batch error ordering),
`d4c1f42` (record every action error once), `12f5d91` (failed batch is a warning), `9dd4e20`.

---

## 3. Every fan-out report is written twice: core type and wire mirror — Worth exploring

**Files** `commands/mod.rs:527-545` (`to_sync_target_result_dto`), `:586-618` (`to_removal_report_dto`),
`:799-885` (`to_refresh_report_dto`), `:1160+` (`to_import_report_dto`); 45 `*Dto` types across
`commands/`.

**Problem** `PropagationScope/Skip/Status`, `RemovalTargetStatus`, `SkillRefreshStatus`,
`BatchTargetStatus` and the import statuses each exist a second time as a `…Dto` enum with identical
variants, plus a hand-written total-match transcription. `to_refresh_report_dto` alone is ~85 lines of
`Match A::X {..} => B::X {..}`. Adding one propagation skip reason means touching four places before
the frontend sees it. And the mappers are not pure wiring: they *compute report semantics* —
`refreshed` / `failed` / `skipped` / `target_failures` are counted in `commands/mod.rs:799-885`, while
the removal report's equivalents come from core (`report.removed_rows()`). Same concept, two homes,
one of them the tier AGENTS.md says is "wiring only".

**Solution** Let the reports cross the wire themselves. The single thing the mirror buys is
`anyhow::Error → CommandError`; if a report's per-target failure were a typed value rather than an
`anyhow::Error` (classification at the point of failure, where the chain is richest), the core report
types could derive `specta::Type` directly — AGENTS.md already sanctions DTOs living in `core/` — and
the counters become the report's own answer.

**Benefits** Leverage: one type per report instead of two plus a mapper. Locality: fan-out counting
happens where fan-out happens. Test surface: `commands/tests/commands.rs:35`
(`removal_report_dto_classifies_a_typed_target_failure_at_the_seam`) exists only to test the mirror.

**ADR conflict** None, but it touches ADR-0001's grain: today classification-by-downcast happens at the
command seam. This proposal moves classification to where the failure is raised and keeps
`from_anyhow` only for errors that fail a whole command. That is an amendment to ADR-0001's *timing*,
not its contract, and worth reopening only if candidate 3 is picked up — the friction (four mappers,
one drifting counter rule) is real but it is transcription pain, not a bug source.

---

## 4. The skills world refetches where the projects world applies — Worth exploring

**Files** `src/hooks/useSkillLibrary.ts` (30-member return; `await loadManagedSkills()` at 11 sites),
`useAddSkillFlow.ts:168`, `:469`, `useSettingsState.ts:209`.

**Problem** Every skills mutation ends in "invoke, then refetch the whole library and re-derive
everything from it", and three other hooks reach into the library hook for `loadManagedSkills` to do
the same. The projects world settled this differently and better: mutations answer with a
`ProjectViewDto` the hook applies, with a refetch only on the failure path (AGENTS.md, ticket 07). One
skills command already follows the project rule — `setSkillInvocationOverride` returns the updated
skill and the hook splices it (`:88-94`) — so the asymmetry is now *within* the module.

The cost shows up as ordering hazards rather than crashes: `runSingleRefresh` needs a `try/finally`
to refetch before settling the report (`:580-592`); `handleRefresh` refetches *between* reading the
report and rendering it; the ref-vs-state hazard in candidate 2 is a direct consequence of the list
being replaced wholesale under in-flight notifications.

**Solution** Give the skills-world commands the same shape as project mutations: the affected skill
(or the batch's affected skills) comes back with the report, and the hook applies it. Refetch stays for
the honest exceptions — delete and Refresh (all).

**Benefits** Locality: one "apply what the mutation returned" rule per world instead of two idioms.
Leverage: the hook's 30-member interface shrinks by the refetch tail it exports to three other hooks.
Testability: assertions on applied state, not on invoke-call ordering.

**Evidence** Deletion test on `loadManagedSkills` as an *exported* member: remove it and the complexity
reappears in `useAddSkillFlow` and `useSettingsState` — which is why it is exported; that is the tell
that the seam is "refetch the world" rather than "here is what changed".

---

## 5. Turning a stored skill record into an acquisition is caller knowledge — Worth exploring

**Files** `core/installer.rs:296-352` (update/re-point), `:560-600` (install from selection),
`:640-670` (explore preview), `:450-470` (listing).

**Problem** `git_acquisition` is genuinely deep, but the translation *into* it is not, and it is
duplicated. Each call site independently knows: which `SkillIntent` to pick from
(override? stored subpath? name?) (`:313-322`), that `stored_subpath` must be suppressed when a source
override is present (`:326-331`), that `resolved_subpath` must be written back filtered against `"."`
(`:347`, `:592`), that a selection must be re-validated with `ensure_installable_skill_dir` afterwards
(`:344`, `:588`), and that `git_cache_ttl_ms` + `github_token_or_none` must be read from settings first
(five sites). Round-8's S3 fix had to reason about all of these at once, and backlog item 2 (double
`matching-refs` call) is the same seam leaking.

**Solution** One module that answers "acquire this skill record (optionally re-pointed) into this
directory, and tell me what to record" — owning intent choice, the hint/override rule, the `"."`
filtering, the post-acquire skill-shape check and the settings reads. Install-from-selection, Update,
Re-point and preview become what their doc comments already claim to be: destination choosers.

**Benefits** Locality: the branch/subpath resolution story stops being half in `git_acquisition` and
half in `installer`. Leverage: `acquire_managed_skill_update_from`'s seven parameters (already
`#[allow(clippy::too_many_arguments)]`) collapse. Testability: intent selection becomes assertable
without staging bytes.

**Evidence** Recent commits paying it: `0779a41`, `f48c001`, `b5e398b`, `6b0ec7d`, `93ee36e`, `364f1bc`
— six subpath/branch-boundary fixes in ~40 commits, split across the two modules.

---

## 6. Modal and route state has no owner — Speculative

**Files** `src/App.tsx:57-85`, `:150-170`; `useSkillLibrary.ts:78-98`, `:339-345`, `:565-600`.

**Problem** "What is on screen" is split: `App` owns `activeView`, `exploreDetailSkill` and the derived
`effectiveView`; `useSkillLibrary` owns `detailSkillId`, `invocationEditSkillId`, `pendingDeleteId` and
`pendingGitRepointSkill` plus their open/close/guard-while-loading callbacks; `useSyncOrchestration`
owns three more modal flags. Each modal costs a state, two callbacks, a `?? null` lookup against a list
that can change underneath it, and ~12 lines of JSX in the binder. Every recent bug here was at the
split: `6bd05b7`, `3190623` (detail skill gone), `cf47020` (keep the repoint modal open until success),
`7b760ec`/`2379c50` (move detail selection into the library hook).

**Solution** One module owning "which surface is showing, over which entity id", with the
resolve-id-against-current-data and close-unless-loading rules in one place. Worlds keep actions; they
stop keeping screens.

**Benefits** Locality for a class of bug that has recurred four times in 40 commits. **Weakest of the
six** — `effectiveView` (round-8 item 5) already removed the sharpest edge, so this is a shape
complaint, not a live defect. Do it only alongside candidate 2 or 4, which touch the same hook.

---

## Top recommendation

**Candidate 1.** It is the only one where the missing module is currently causing wrong behaviour in
shipped builds that the test suite cannot see — a debug/release behaviour fork inside a policy that
three other modules quietly compensate for. Fix the ownership and the compensations (`project_sync`'s
write-during-read backfill, `skill_edits`' unconditional recompute) can go with it. Candidate 2 is the
best value per unit of risk on the frontend and is independent of it, so the two can run in parallel
worktrees without touching a shared file.

## What I would not deepen

`core/mutation_guard` is three functions and a private mutex and looks shallow by line count, but it
earns its keep: the "entry point wraps its own body, internals are `pub(crate)` unlocked seams"
discipline is exactly what keeps eight operations from re-entering each other, and `try_serialized`
gives the reconcile pass a door nothing else needs. `core/tool_adapters`' registry is a wide interface
by nature — one literal per tool — and every attempt to narrow it would reintroduce name-based special
casing the project has deliberately eliminated. `SkillStore`'s 41 methods look like a shallow row-CRUD
surface, but the deletion test says otherwise: delete it and SQL reappears in a dozen core modules; its
mutation half is already deepened into typed transitions, and its read half is genuinely irreducible
row access. And `git_acquisition` itself — 874 lines behind `acquire(request, api)` with a real
two-adapter seam — is the best-shaped module in the repo; candidate 5 proposes deepening its *callers*,
not it.
