# Skills Hub — architectural friction scan (independent review, panel member 2)

Repo: `~/Projects/skills-hub` @ HEAD `943f85c` ("chore: translate remaining Chinese comments and test assertion messages to English").
Recent history is dominated by a refactoring epic (v-next tickets 01–16: `ee4f826` commands/core seam, `ac3395c` tagged CommandError, `8484538` backend batch sync, `fc89d1f`/`bb77d83` App.tsx carve into world hooks, `be9a74c` hook tests, `de649b6` downcast-not-prose fixes). The scan weights what that epic touched — and probes what it left behind, which turns out to be mostly `core/installer.rs` (1,951 lines, the largest module in the repo) and the two tool-status commands.

## Map of the territory

Three tiers, four load-bearing seams:

- **Frontend** — `src/App.tsx` (627 lines) is the binder: it owns navigation state and composes per-world hooks (`useStatusReporter` → `useSyncOrchestration` → `useSkillLibrary` → `useSettingsState`/`useExploreState`/`useAddSkillFlow`, plus `useProjectState` for the projects world). Hooks depend on each other only through interfaces App passes in, narrowed with `Pick<>` (`useAddSkillFlow.ts:22-34`). The frontend test surface is the `invokeTauri` module seam (`src/lib/tauri.ts`, mocked in every hook test, e.g. `useSkillLibrary.test.ts:17-20`).
- **IPC seam** — generated DTO bindings (`src/bindings/`, ts-rs) plus the tagged `CommandError` union with its single frontend consumer `src/commandError.ts` (ADR-0001). Both compilers check every variant.
- **Command tier** — `commands/mod.rs` (1,294 lines) + `commands/projects.rs` (493): per AGENTS.md, wiring only (spawn_blocking + DTO conversion + `CommandError::from_anyhow`).
- **Core** — `core/installer.rs` (install/update/discovery), `core/global_sync.rs` + `core/project_sync.rs` (fan-out choreography), `core/sync_engine.rs` (symlink→junction→copy fallback + the typed `TargetExistsError` seam), `core/tool_adapters/mod.rs` (the 40+-tool adapter table, virtual group constant), `core/skill_store.rs` (SQLite repo). Core tests live in `core/tests/` and run against real tempdirs.

The recently refactored parts are in good shape: `sync_engine.rs` is a genuinely deep module (one function call hides the platform fallback ladder), the batch sync command is a single seam, and the ADR-0001 error contract holds at the seams that were migrated. The friction below concentrates where the epic didn't reach.

## Findings

### 1. Installer: four copies of the "finalize a skill into the central repo" tail

**Files**: `src-tauri/src/core/installer.rs`

**Problem**: The install flows are shallow siblings that each re-implement the same implementation instead of sharing a deep module. The "finalize" choreography — collision check, SKILL.md-preferred rename dance, content hash, `SkillRecord` construction, `upsert_skill` — is copied, not called. There is no locality: the #28 rename fix had to be applied in two places, and any future change to record shape or naming policy fans out to four call sites.

**Evidence** (verified):
- `SkillRecord { ... }` literal construction at installer.rs:90, 187, 869, 1379 — four sites building the same record by hand.
- The SKILL.md-preferred-name rename dance (parse → compare → rename → re-read description) is verbatim-duplicated at installer.rs:159-183 (`install_git_skill`) and installer.rs:1348-1371 (`install_git_skill_from_selection`), both carrying the same "fixes #28" comment.
- The "already exists in central repo" collision bail is copied at installer.rs:61, 147, 1324.
- `install_git_skill` (117-212) and `install_git_skill_from_selection` (1299-1403) share ~80% of their bodies (name derivation from subpath, central-dir resolution, fetch/copy, validate, finalize) and differ only in how content arrives.

**Deletion test**: deleting any one of these functions would make its complexity reappear almost line-for-line in a sibling — the classic "was never a module" signal.

**Proposed deepening**: extract one deep module — roughly `finalize_install(central_dir, staged_content, provenance, name_hint) -> InstallResult` — that owns collision checking, SKILL.md name preference, hashing, and record persistence. The four flows become thin adapters that only differ in how they stage content (local copy, git fetch, git selection, update staging swap).

**Benefits**: naming/collision/record policy gets one home (locality); one test suite exercises the whole finalize behaviour through a small interface instead of re-testing it per flow (the interface is the test surface); each install flow shrinks to its genuinely distinct part, which is what an AI navigating "how does install work" actually needs to read.

**Strength**: Strong.

### 2. `tauri::AppHandle` threaded through 10 installer interfaces to reach two path lookups

**Files**: `src-tauri/src/core/installer.rs`, `src-tauri/src/core/central_repo.rs`, `src-tauri/src/core/tests/installer.rs`

**Problem**: The interface of nearly every installer function includes `app: &tauri::AppHandle<R>` plus a generic `R: tauri::Runtime` — a large interface cost every caller and test pays — while the implementation only ever uses it for two things: `app_cache_dir()` in `clone_to_cache`/`clone_to_cache_subpath` (installer.rs:1461, 1543) and the rarely-taken `app_data_dir()` fallback inside `resolve_central_repo_path` (central_repo.rs:23-27, only reached when `home_dir()` is `None`). That is a pass-through dependency widening 10 interfaces (installer.rs:40, 118, 736, 997, 1300, 1406, 1452, 1533, 1642, 1805).

**Evidence** (verified): `rg AppHandle src-tauri/src/core` — 10 hits in installer.rs; the only `app.path()` uses in all of core are central_repo.rs:25, skill_store.rs:1079, cache_cleanup.rs:60-62, temp_cleanup.rs:25-27, installer.rs:1459-1462/1541-1544. Consequence for the test surface: `core/tests/installer.rs` constructs `tauri::test::mock_app()` 16 times just to satisfy the parameter. Meanwhile `cache_cleanup.rs` and `temp_cleanup.rs` already demonstrate the right shape — a thin `AppHandle` adapter delegating to a path-parameterized inner function (`cleanup_git_cache_dirs_in(&cache_dir, ...)`, cache_cleanup.rs:56-65), as does `onboarding.rs:40-54` (`build_onboarding_plan` → `build_onboarding_plan_in_home`).

**Proposed deepening**: resolve paths once at the command seam (or pass a two-field value like `central_dir: &Path, cache_dir: &Path`), and make every installer function take plain paths. The `AppHandle` adapter shrinks to one place per command. Accept dependencies, don't create them — but also don't forward them ten layers deep.

**Benefits**: core's install/update logic becomes tauri-free and testable with tempdirs alone (16 `mock_app()` constructions deleted); the generic `<R: tauri::Runtime>` noise disappears from every signature; the seam between "Tauri runtime environment" and "skill installation" becomes explicit and small.

**Strength**: Strong.

**ADR/invariant note**: aligns with AGENTS.md's "business logic goes in `core/`, which is independently testable" — currently that independence is compromised by the runtime type in the interfaces.

### 3. Tool status + virtual group logic lives in the command tier, untested

**Files**: `src-tauri/src/commands/mod.rs`, `src-tauri/src/core/tool_adapters/mod.rs`

**Problem**: `get_tool_status` (commands/mod.rs:68-130) and `get_project_tool_status` (commands/mod.rs:133-207) are not wiring — they contain real policy inside `spawn_blocking` closures: the newly-installed-tools diff against the persisted `installed_tools_v1` setting (103-120), the shared-dir presentation filtering (82-87), and the entire virtual group derivation ("installed if ANY constituent detect dir exists", constituent display-name list, group key substitution; 143-193). CONTEXT.md defines the **virtual group** as "owned by the backend and only presented by the frontend" — but it is owned by the wrong backend tier, behind an interface (`State<'_, SkillStore>` + Tauri command) that no unit test can reach.

**Evidence** (verified): `rg get_tool_status|get_project_tool_status` across `commands/tests/commands.rs` and `core/tests/*` returns zero matches — this logic has no tests at all. AGENTS.md invariant: "**`commands/` is wiring only** (DTO conversion, error formatting); business logic goes in `core/`". Ticket 01 (`ee4f826` "restore the commands/core seam") evidently did not migrate these two.

**Proposed deepening**: move the logic into a core module (e.g. `core/tool_status.rs`) with an interface like `build_tool_status(store, adapters) -> ToolStatus` and `build_project_tool_status(adapters) -> ToolStatus`, leaving the commands as the thin adapters they are documented to be.

**Benefits**: the virtual-group and newly-installed policies become testable through a plain-Rust interface (locality for the domain concept CONTEXT.md names); the command file loses ~140 lines of its densest logic; a future change to `AGENTS_STANDARD_KEYS` membership gets verified by cargo tests rather than by running the app — which matters in a repo where dev runs mutate the operator's real skill library.

**Strength**: Strong.

### 4. Skill discovery scan: one concept, two hand-rolled implementations

**Files**: `src-tauri/src/core/installer.rs`

**Problem**: "Discover skill candidates in a directory tree" is one concept implemented twice. `list_git_skills` (996-1106) and `list_local_skills` (1108-1297) each hand-roll the same multi-strategy scan — root SKILL.md, known scan bases, `marketplace.json`, recursive depth-5 — followed by the same sort + dedup-by-subpath, but with drifted details: the local side re-parses SKILL.md for marketplace hits that `scan_marketplace_skills` already parsed (1225-1250), tracks `valid`/`reason` per candidate, while the git side does neither. Two adapters exist over this hypothetical seam — which by the panel's own rule makes it a *real* seam that simply hasn't been built.

**Evidence** (verified): both functions call the same helper set (`find_skill_md`, `collect_skill_dirs`, `parse_marketplace_json`/`scan_marketplace_skills`, `find_skill_dirs_recursive(_, 0, 5)`) and end in identical `out.sort_by(...); out.dedup_by(|a,b| a.subpath == b.subpath)` (1102-1104, 1293-1295). The ~20 private scan helpers (installer.rs:393-723) form a de-facto discovery submodule with no interface of its own.

**Proposed deepening**: extract a discovery module with one entry point — `discover_skills(root) -> Vec<DiscoveredSkill { subpath, name, description, validity }>` — and make the git and local candidate listings two thin adapters mapping `DiscoveredSkill` to their DTOs. `update_managed_skill_from_source`'s `scan_skill_candidates_in_dir` (688-700) becomes a third caller of the same interface.

**Benefits**: scan-strategy changes (like the "always run marketplace + recursive" fix, currently commented in both copies at 1068-1070 and 1220) land once; discovery becomes testable as pure fixture-tree → candidate-list, independent of git and of the install flows; the git/local behaviour drift (validity info) becomes an explicit adapter decision instead of an accident.

**Strength**: Strong.

### 5. `StatusReporter` is a shallow interface: raw setters plus an unwritten ordering constraint at 14 call sites

**Files**: `src/hooks/useStatusReporter.ts`, `src/hooks/useSkillLibrary.ts`, `src/hooks/useAddSkillFlow.ts`, `src/hooks/useExploreState.ts`

**Problem**: The reporter's interface (useStatusReporter.ts:19-38) exposes five raw setters (`setLoading`, `setLoadingStartAt`, `setActionMessage`, `setError`, `setSuccessToastMessage`). The interface, in the full sense, therefore includes an ordering constraint every caller must know and repeat: begin with `setLoading(true); setLoadingStartAt(Date.now()); setError(null); setActionMessage(...)`, end in `finally { setLoading(false); setLoadingStartAt(null) }`, toast on success, `formatError` on failure. The behaviour per unit of interface learned is near zero — the definition of shallow — and the real invariant (loading state always resets, errors always cleared at start) lives in N copies instead of one implementation.

**Evidence** (verified): `setLoadingStartAt(Date.now())` appears 13 times across hooks (7× useSkillLibrary.ts, 5× useAddSkillFlow.ts, 1× useExploreState.ts); `setLoading(true)` 14 times; representative full choreography at useSkillLibrary.ts:170-198 (`syncAllManagedToTools`) and useAddSkillFlow.ts:503-628 (`handleCreateGit`, which must also remember to manually unwind loading state on the early-return-to-picker paths at 598-603 and 610-615 — exactly the kind of path where a missed reset hides).

**Proposed deepening**: give the reporter one deep action-lifecycle method — e.g. `runAction(opts: { message?, successToast? }, fn) -> Promise<T | undefined>` — that owns begin/success/error/finally, with the raw setters demoted to internal seams (kept only where a flow genuinely diverges, like the hand-off-to-picker paths). `formatError` stays the single error waist it already is.

**Benefits**: the loading invariant is implemented once and tested once through the reporter's interface (currently `useStatusReporter.test.ts` cannot test the choreography at all because the reporter doesn't own it — each hook test re-asserts fragments of it); every handler in the world hooks loses 8-10 lines of ceremony, which meaningfully shrinks the 841-line `useAddSkillFlow`; new handlers cannot forget the reset.

**Strength**: Strong.

### 6. Skill-name matching policy scattered across four sites and two tiers

**Files**: `src-tauri/src/core/installer.rs`, `src/hooks/useAddSkillFlow.ts`

**Problem**: The policy "does candidate X match target skill name Y" (case-insensitive exact match, falling back to bidirectional containment) is implemented four times with three different dialects and no locality: the frontend requires exact-or-*unique*-containment, `find_skill_by_name` takes the *first* containment hit, and the update backfill requires exact-or-unique-containment again but on a different candidate source. An Explore install (frontend picks) and a later update (backend re-picks) can resolve the same skill differently.

**Evidence** (verified):
- useAddSkillFlow.ts:528-535 — single-candidate verification (`candidateName !== target && !candidateName.includes(target) && !target.includes(candidateName)`).
- useAddSkillFlow.ts:564-571 — auto-select: exact match else unique bidirectional containment.
- installer.rs:1776-1793 (`find_skill_by_name`) — first match where `n == target || n.contains(&target) || target.contains(&n)`, tried on SKILL.md names then dir names.
- installer.rs:802-816 — update backfill: exact else unique bidirectional containment, with a comment re-explaining the same heuristic.

**Proposed deepening**: make the backend the single owner of matching. Either `list_git_skills` grows an optional `target_name` parameter that returns a resolved candidate (or the full list when ambiguous), or a small pure core function `match_skill_candidate(target, candidates)` is used by all three backend sites and exposed through the existing candidate-listing command so the frontend stops re-implementing it. The frontend's job reduces to "backend resolved it / backend says pick".

**Benefits**: one definition of matching (locality), unit-testable as a pure function against tricky pairs like `react` vs `json-render-react` (the exact case the comments cite); frontend/backend divergence becomes impossible; two blocks of `handleCreateGit`'s hardest-to-test logic move behind a cargo-testable interface.

**Strength**: Strong.

### 7. `update_managed_skill_from_source` re-implements propagation and re-derives the Cursor copy rule by raw string

**Files**: `src-tauri/src/core/installer.rs`, `src-tauri/src/core/sync_engine.rs`, `src-tauri/src/core/tool_adapters/mod.rs`

**Problem**: The 242-line update function (installer.rs:735-976) doesn't stop at refreshing the central copy — it then re-implements target propagation that the sync modules own: it loops global targets and project assignments itself, deciding copy-mode re-sync inline. In doing so it re-derives the "Cursor doesn't support symlinks" policy with raw string comparisons (`t.tool == "cursor"`), even though `sync_engine::sync_dir_for_tool_with_overwrite` (sync_engine.rs:154-162) exists precisely to own that decision. The policy has leaked across the seam: a second symlink-incapable tool means three edits, two of them string-typed and invisible to the compiler.

**Evidence** (verified): installer.rs:898 and installer.rs:921 (`let force_copy = t.mode == "copy" || t.tool == "cursor";` / `pa.mode == "copy" || pa.tool == "cursor"`), both calling `sync_dir_copy_with_overwrite` directly and bypassing the tool-aware entry point; the canonical rule at sync_engine.rs:158-161 keys on `tool_key.eq_ignore_ascii_case("cursor")` — itself a string, not a `ToolId` or adapter capability.

**Proposed deepening**: (a) put the capability on the adapter — a `supports_symlink: bool` (or similar) field on `ToolAdapter` (tool_adapters/mod.rs:119-127), consulted by `sync_dir_for_tool_with_overwrite`, deleting all string checks; (b) move the "re-propagate a refreshed skill to its targets and assignments" tail out of the installer into `global_sync`/`project_sync`, where the equivalent choreography already lives, so the update function ends at "central copy refreshed, record updated".

**Benefits**: the AGENTS.md do-not ("never 'fix' the Cursor adapter to use symlinks") becomes enforced by one compiler-checked table entry instead of three prose comments; update-propagation gains the same test coverage the sync modules already have; `update_managed_skill_from_source` drops ~90 lines and one responsibility.

**Strength**: Strong (part a is small and safe; part b Worth exploring in the same pass).

**ADR/invariant conflicts**: none — this *strengthens* the documented Cursor invariant.

### 8. The "skill already exists" condition still crosses the wire as prose, regex-parsed on the frontend

**Files**: `src/commandError.ts`, `src-tauri/src/core/installer.rs`, `src-tauri/src/core/errors.rs`

**Problem**: One error condition escaped the ADR-0001 migration. The frontend's `describeOther` sniffs `CommandError::Other`'s message for the literal English string "skill already exists in central repo" and regex-extracts the skill name from a Debug-formatted path — exactly the string-dialect adapter the ADR was written to eliminate. The seam contract ("never encode error conditions in message strings") is violated for the single most user-visible install failure.

**Evidence** (verified): commandError.ts:63-77 (`message.includes("skill already exists in central repo")` + `message.match(/central repo:\s*"?([^"]+)"?/)`); the prose is raised at installer.rs:61, 147, 1324; `SignalError` (errors.rs:16-48) has no variant for it; the ADR itself declares "raw prose reaching users through it is a smell that a typed variant is due".

**Proposed deepening**: add `SignalError::SkillExists { name }` (raised once, from the finalize module of finding 1), the `CommandError` variant, regenerated binding, `describeCommandError` branch, i18n keys — the exact recipe the ADR documents. Delete `describeOther`'s sniffing (leaving it a pure passthrough, or deleting it).

**Benefits**: closes the last known message-string dialect; the frontend's whitelist mechanism (`commandError.ts:14-29`) makes the compiler enforce handling; the name extraction stops depending on Rust's `{:?}` Debug formatting of paths, which is not a stable interface.

**Strength**: Strong (small, mechanical, high signal).

**ADR/invariant conflicts**: none — this is unfinished ADR-0001 work, not a re-litigation. (The 404/403 string-sniffing at installer.rs:1691-1707 is a *documented* boundary heuristic at the closest reachable point to the HTTP origin, converted immediately into typed signals — that one honours the ADR and needs no change.)

### 9. `useAddSkillFlow`: a 49-member interface over an 841-line implementation

**Files**: `src/hooks/useAddSkillFlow.ts`, `src/App.tsx`, `src/components/skills/modals/*`

**Problem**: The add/import world hook returns ~49 members (useAddSkillFlow.ts:795-839) — every modal's open flag, every field's value+setter, every candidate list and selection map, plus 20 handlers — and its lint metrics agree (complexity 95, "high fanout"). Its interface is nearly a transcript of its implementation: shallow by definition. Most of the width comes from the two structurally identical candidate-pick sub-flows (git: `gitCandidates`/`gitCandidateSelected`/`showGitPickModal`/toggle/close/cancel/install-selected; local: the same seven, ~lines 83-97).

**Evidence** (verified): return object at 795-839; the parallel git/local pick state at 83-97; `handleCreateGit` alone spans 499-628 mixing discovery, matching (finding 6), modal transitions and deploy.

**Proposed deepening**: this hook shrinks substantially as a *consequence* of findings 5 and 6 (lifecycle ceremony and matching policy leave). Beyond that, extract one candidate-pick module used twice — `useCandidatePick<C>(installSelected)` returning `{ candidates, selected, open, toggle, toggleAll, close, cancel, confirm }` — collapsing the twin sub-flows into two instances of one tested implementation. Two real adapters (git, local) already exist for that seam.

**Benefits**: the pick-flow invariants (select-all default, close-vs-cancel, batch install + error collection) get one home and one test suite; `useAddSkillFlow`'s interface drops by roughly a third; App.tsx's wiring shrinks correspondingly.

**Strength**: Worth exploring — the hook-per-world pattern is deliberate and freshly established; do this only after findings 5/6 land and only if the residual width still hurts.

## Top recommendation

**Finding 1 (the installer finalize module), sequenced to absorb findings 2 and 8 in the same campaign.** `core/installer.rs` is the largest, least-refactored module in the repo and the only tier the v-next epic left untouched; it concentrates four of the nine findings. Extracting the finalize-install module is the highest-leverage single move: it deletes the worst duplication in the codebase (four `SkillRecord` constructions, two verbatim rename dances, three collision bails), it creates the natural single home for the typed `SkillExists` signal (finding 8, closing the last ADR-0001 gap), and doing it forces the path-parameter decision that evicts `tauri::AppHandle` from core interfaces (finding 2), which alone deletes 16 `mock_app()` constructions from the test suite. The result is the thing every future change to installation will be tested through — a small interface over the deepest behaviour in the app.

Findings 3 (tool status into core) and 5 (deep StatusReporter action lifecycle) are the best independent follow-ups: each is self-contained, each converts currently-untestable choreography into a tested interface, and neither touches the installer campaign's files.
