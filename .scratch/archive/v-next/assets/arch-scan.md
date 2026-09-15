# Skills Hub — Architectural Friction Scan (read-only)

Scanned: `/Users/alexstark/Projects/skills-hub` (Tauri 2 + React 19). No files edited.
All terms per the deep-module vocabulary (module / interface / depth / seam / adapter / leverage / locality / deletion test).

---

## Map of the territory

### Tier flow (general)

```
UI component (props from App.tsx or useProjectState)
  → invoke("command_name", args)                      [src/App.tsx invokeTauri wrapper, or direct invoke in projects/]
  → #[tauri::command] fn in src-tauri/src/commands/{mod.rs,projects.rs}   [registered in lib.rs generate_handler!]
  → tauri::async_runtime::spawn_blocking(move || ...)
  → core/ module (installer.rs, sync_engine.rs, project_sync.rs, skill_store.rs, tool_adapters/)
  → filesystem (~/.skillshub central repo; ~/.claude/skills etc. via symlink→junction→copy) + SQLite (skill_store)
  ← anyhow::Error stringified by format_anyhow_error() (commands/mod.rs:40-113)
  ← frontend string-prefix parsing (App.tsx / useProjectState.ts formatProjectError)
```

Two frontend state worlds, no shared store (by ADR: useState + props only):
- **Global skills world**: ALL state in `App.tsx` (60 `useState` calls, 85 `invokeTauri` calls, 2,716 lines), props-drilled to `SkillsList`, `SkillDetailView`, `FilterBar`, `ExplorePage`, `SettingsPage`, 8 modals.
- **Projects world**: `useProjectState.ts` hook (442 lines) consumed only by `ProjectsPage.tsx`, drilled to `ProjectList`, `AssignmentMatrix`, 4 modals. Uses raw `invoke`, not App's `invokeTauri` wrapper.

### Fact chain 1 — ToolConfigModal duplication (for diagrams)

**Global variant** (`src/components/skills/modals/ToolConfigModal.tsx`, 158 lines, added in commit `698968b`; `git log --follow` shows it was literally copied from the projects modal — its history resolves to the projects file's commits):

```
FilterBar onConfigureTools={handleOpenToolConfig}            (App.tsx:2460)
App.tsx: handleOpenToolConfig → setShowToolConfigModal(true) (App.tsx:1110-1111)
<ToolConfigModal open={showToolConfigModal} loading toolStatus
   selectedTools={globalSelectedTools} scanSelectedOnly={scanSelectedToolsOnly}
   onConfirm={handleToolConfigConfirm} onRequestClose={handleCloseToolConfig} t/>   (App.tsx:2583-2591)
handleToolConfigConfirm(selected, scanOnly)                   (App.tsx:1126-1150)
  → invoke("set_global_tool_config", {selectedTools, scanSelectedOnly})
  → setGlobalSelectedTools / setScanSelectedToolsOnly / setSyncTargets (recomputed from toolInfos)
```

**Per-project variant** (`src/components/projects/ToolConfigModal.tsx`, 145 lines):

```
AssignmentMatrix onConfigureTools={handleConfigureToolsFromToolbar}   (ProjectsPage.tsx:204,272)
  → state.loadToolStatus() → invoke("get_project_tool_status")
  → state.setShowToolConfigModal(true)                                 (useProjectState.ts:105)
<ToolConfigModal open={state.showToolConfigModal} toolStatus={state.toolStatus}
   currentTools={state.tools} onConfirm={handleToolConfigConfirm} .../> (ProjectsPage.tsx:311-318)
handleToolConfigConfirm(selectedTools)                                 (ProjectsPage.tsx:63-100)
  → diff vs state.tools → state.addTools(toAdd) / state.removeTools(toRemove)
     → invoke("add_project_tool"/"remove_project_tool") per tool
  → then D-13: invoke("update_project_gitignore", pendingGitignoreRef)
```

Interface diff between the two modals (everything else is byte-for-byte copy-paste — `buildInitialSelection`, `handleToggle`, `detectedOnly` filter, the full JSX skeleton, the `open`-gate wrapper + `memo`):

| | skills/modals variant | projects variant |
|---|---|---|
| baseline prop | `selectedTools: string[] \| null` | `currentTools: ProjectToolDto[]` |
| extra state | `scanSelectedOnly` checkbox (2nd toggle) | `agents_skills` subtitle listing 9 tool names as a hardcoded string |
| onConfirm | `(selectedTools, scanSelectedOnly) => Promise<void>` | `(selectedTools) => Promise<void>` |
| i18n keys | `globalToolConfig*` | `projects.toolConfig*` |

### Fact chain 2 — installer (for diagrams)

```
App.tsx handleCreateGit → invoke("install_skill_from_git")
  → commands/mod.rs → core/installer.rs install_git_skill (115-210)
      → parse_github_url (219-287)
      → fetch_skill_files (1628-1752)  ← "shared download engine" (commit 0756e05)
          Path A: parse_github_api_params → github_download.rs download_github_directory
                  on failure (404/rate-limit handling inline) → fall back to clone_to_cache
          Path B: clone_to_cache (1438-1517) → git_fetcher.rs clone_or_pull
                  (git2 first, clone_or_pull_via_git_cli fallback, 387-528)
          multi-skill repo → collect_skill_dirs → find_skill_by_name (1756-1792, emits MULTI_SKILLS|)
      → copy into central repo, DB register (skill_store)

clone_for_explore_preview (1797-1850) → also fetch_skill_files  ✅ unified
install_git_skill_from_selection (1292-1396) → clone_to_cache directly (subpath known)
update_managed_skill_from_source (733-971, 238 lines) → does NOT use fetch_skill_files:
   own clone_to_cache/clone_to_cache_subpath calls + its OWN multi-skill matcher
   (fuzzy bidirectional-containment, lines 795-820) ≠ find_skill_by_name's matcher
list_git_skills (989-1099) → clone_to_cache + its own candidate-scan branching
```

After install, App.tsx (not the backend) fans out sync: filter `syncTargets`+`isInstalled` → `uniqueToolIdsBySkillsDir` → loop `invoke("sync_skill_to_tool")` per (skill, tool) — this block is duplicated ~8× (see Finding 2).

---

## Findings (ranked by real pain)

### 1. Two ToolConfigModals — one module copied, not deepened

- **Files**: `src/components/skills/modals/ToolConfigModal.tsx` (158), `src/components/projects/ToolConfigModal.tsx` (145); wiring at `App.tsx:1110-1150, 2583-2591` and `ProjectsPage.tsx:63-100, 311-318`.
- **Problem**: A single shallow module cloned across a fake seam. ~120 of ~150 lines are identical (state shape, `buildInitialSelection` structure, toggle handler, entire JSX). The interfaces differ only in the baseline-selection type and one extra checkbox. Every visual or behavioural fix (e.g. commit `f7d83bb`'s height-limit/scroll/detected-only fix landed on the projects one) must now be re-applied by hand to the other — no locality.
- **Evidence**: `git log --follow` on the skills variant resolves to the projects variant's history (literal copy in `698968b`). Both define an identical `buildInitialSelection(toolStatus, X)` differing only in how the "already saved" branch reads X. The projects variant additionally hardcodes the 9 AGENTS-standard tool names as prose (`"Cursor, Codex, Amp, Kimi Code CLI, Antigravity, Cline, Gemini CLI, GitHub Copilot, OpenCode"`), duplicating Rust's `AGENTS_STANDARD_KEYS` (see Finding 6).
- **Deletion test**: delete either file and the complexity reappears verbatim at the other call site → the duplication is pure pass-through weight; a single module earns its keep.
- **Deepened shape**: one `ToolConfigModal` whose interface is `initialSelection: Set<string>` (caller computes baseline — that's the only genuinely different logic), `extraControls?: ReactNode` or a typed `variant` prop for the scan-only checkbox, and `onConfirm(selected: string[])` with the global caller closing over `scanSelectedOnly` itself. Callers keep their own i18n key prefix via a `labels` prop or key-prefix prop.
- **Strength**: Strong. **ADR conflict?**: none — pure component extraction, no state library.

### 2. Sync fan-out orchestration lives in untested App.tsx, ~8 copies

- **Files**: `src/App.tsx` — `handleSyncSkillToAllTools` (828-880), refresh-all loop (~1300-1350), `handleSyncAllManagedToTools` (2176-2250), `runToggleToolForSkill` (2272-2330), plus install-then-sync blocks at 1476, 1597, 1724, 1793, 1973, 2085. Backend: `sync_skill_to_tool` in `commands/mod.rs:562-650`.
- **Problem**: `sync_skill_to_tool` is a shallow interface — one (skill, tool) pair per call — so every "sync N skills to M tools" flow re-implements the same double loop in the UI: filter selected+installed → `uniqueToolIdsBySkillsDir` dedupe → loop → catch → skip `TOOL_NOT_INSTALLED|`/`TOOL_NOT_WRITABLE|` → collect errors → `showActionErrors`. Zero frontend tests exist (no vitest/jest in `package.json`, no `*.test.*` under `src/`), so the app's most business-critical behaviour (what actually gets synced where, which errors are swallowed) has no locality and no coverage, while `core/` has 15 well-fed test files.
- **Evidence**: the `raw.startsWith("TOOL_NOT_INSTALLED|") || raw.startsWith("TOOL_NOT_WRITABLE|") → continue` idiom appears at App.tsx:841, 1332, 2215 (and variants at 218-227, 2313-2320); `uniqueToolIdsBySkillsDir(selectedInstalledIds)` at 6 install sites; each copy varies subtly (`overwrite` vs `overwriteIfSameContent`, `collectedErrors` vs `toast.error` vs `setError`) — divergence that is indistinguishable from bugs.
- **Deletion test**: deleting any one copy forces its logic to reappear at that call site → the copies individually pass; but deleting a hypothetical `sync_skill_to_tools(skillId, tools[], policy)` backend command would scatter exactly this complexity — which is the current state.
- **Deepened shape**: push the fan-out one seam down. Either (a) a batch Tauri command in `core/` (`sync_skill_to_tools` / `sync_all_skills`) returning a structured per-target result list — testable in Rust, with the skip-not-installed policy as data, or (b) if progress-message streaming keeps it client-side, a single frontend function `syncSkillsToTools(skills, toolIds, opts)` extracted into a custom hook (the `useProjectState` pattern), so the loop + error taxonomy exists once. (a) is deeper: the backend already knows installedness and shared-dir groups.
- **Strength**: Strong. **ADR conflict?**: none — hook extraction stays within useState+props; a batch command honors "business logic in core/".

### 3. The string-prefix error contract is a shallow interface leaking across the Rust/TS seam — and it has already drifted

- **Files**: `src-tauri/src/commands/mod.rs:40-113` (`format_anyhow_error`), `src/App.tsx` (parsing at 200-232, 841, 1332, 1501, 2215, 2310-2320), `src/components/projects/useProjectState.ts:62-78` (`formatProjectError`).
- **Problem**: The seam's interface is "a string that may start with one of N pipe-delimited prefixes" — the caller must know the prefix list, per-prefix payload arity (`TOOL_NOT_WRITABLE|tool|path` = 2 fields, `TARGET_EXISTS|path` = 1), and that anything else is prose (sometimes Chinese prose composed in `format_anyhow_error` itself, e.g. GitHub-failure hints at mod.rs:80-110 — user-facing copy generated on the wrong side of the i18n seam). Interface complexity ≈ implementation complexity ⇒ shallow. And the contract already drifted: AGENTS.md documents **five** prefixes, but `format_anyhow_error` whitelists **eight** (`DUPLICATE_PROJECT|`, `ASSIGNMENT_EXISTS|`, `NOT_FOUND|` added for projects), parsed in a *different* frontend module. Nothing type-checks any of this; `parts[1] ?? ""` is the failure mode.
- **Evidence**: prefix parsing exists in ≥7 App.tsx locations with three different downstream behaviours (skip / setError / collectedErrors); `formatErrorMessage` (App.tsx:200-232) additionally sniffs raw substrings (`"skill already exists in central repo"`, a Chinese literal `"未在该仓库中发现可导入的 Skills"`) — the seam leaks implementation prose. `CANCELLED|` and `RATE_LIMITED|` are further ad-hoc prefixes matched with `.contains()` inside `installer.rs:1670-1695`, a third dialect of the same convention.
- **Deletion test**: delete the prefix convention and the need for typed error discrimination reappears at every catch block → the *concept* earns its keep; the *string encoding* is the shallow part.
- **Deepened shape**: one serde-tagged error enum (`{ code: "TOOL_NOT_WRITABLE", tool, path }`) serialized as the Tauri command error (Tauri supports structured `Err` payloads), mirrored once in `types.ts`, with a single frontend `describeCommandError(e, t): string` module. The five/eight-prefix strings can be kept as a compatibility `Display` while callers migrate. Locality: adding an error variant becomes a two-file change checked by both compilers.
- **Strength**: Strong. **ADR conflict?**: AGENTS.md documents the five-prefix contract as a fact, not a preference ("Adding a prefix means handling it in App.tsx too"). Deepening replaces the contract — flag to the operator, but the contract's own drift (8 ≠ 5) is the argument.

### 4. `commands/` is not "wiring only": business logic has leaked, and one leak looks like a live bug

- **Files**: `src-tauri/src/commands/mod.rs` — `sync_skill_to_tool` (562-650: writability probing, content-hash compare via `target_has_same_content`, error re-mapping, shared-dir DB fan-out loop); `src-tauri/src/commands/projects.rs` — `update_project_gitignore` (394-558: **164 lines** of gitignore block parsing/rewriting incl. the `remove_block` closure at 441-486).
- **Problem**: The documented seam (commands = DTO + error formatting; core = independently testable logic) leaks. Logic in `commands/` is only reachable through Tauri `State`, so `core/tests/` (15 files, incl. 1,246-line installer tests) can't touch it; `commands/tests/commands.rs` is 190 lines that mostly test `format_anyhow_error`. No locality: sync semantics live half in `sync_engine.rs`, half in the command.
- **Evidence** (suspected latent bug from the leak): `update_project_gitignore` builds ignore patterns from `adapter.relative_skills_dir` (the **global** home-dir path, projects.rs:419) — but project sync writes to `project_relative_skills_dir(adapter)` (`project_sync.rs:13-19`). These differ for non-consolidated tools: Windsurf `.codeium/windsurf/skills` vs `.windsurf/skills`; Pi `.pi/agent/skills` vs `.pi/skills`; Goose `.config/goose/skills` vs `.goose/skills`; Augment `.augment/rules` vs `.augment/skills`. A project configured with those tools gets gitignore entries that don't match what's synced into the repo. (Unverified at runtime — read-only scan — but the two functions demonstrably use different mappings.) Also the `SyncMode → "symlink"/"junction"/"copy"` match is hand-duplicated 3× inside commands/mod.rs alone (540-546, 625-632, 645-651) where a `SyncMode::as_str()` in core would do.
- **Deletion test**: delete `sync_skill_to_tool`'s body and its ~90 lines reappear — because there is no `core::sync_skill_to_tool_with_records()` to call. The command is doing a core module's job.
- **Deepened shape**: move each leaked body into `core/` (`core/global_sync.rs` for the sync+record fan-out; `core/gitignore.rs` for block rewrite, taking `&[String] patterns` so the dir-mapping decision is made once next to `project_relative_skills_dir`), leaving the command as the documented 5-line wrapper. This makes the gitignore mapping question a one-line, unit-tested choice.
- **Strength**: Strong (the gitignore mismatch alone justifies it). **ADR conflict?**: none — it *restores* the stated ADR.

### 5. installer.rs: `fetch_skill_files` deepening is half-finished — the update path kept its own fork

- **Files**: `src-tauri/src/core/installer.rs` (1,944): `fetch_skill_files` 1628-1752, `update_managed_skill_from_source` 733-971, `find_skill_by_name` 1756-1792, `list_git_skills` 989-1099; `git_fetcher.rs` (543), `github_download.rs` (423).
- **Problem**: Commit `0756e05` created a genuinely deep module — `fetch_skill_files` hides GitHub-API vs git-clone vs sparse-clone vs multi-skill resolution behind `(parsed, skill_name, dest_dir) → SkillFetchResult`, and `clone_for_explore_preview` shrank to a cache check (its doc comment says so). But `update_managed_skill_from_source` (238 lines) still calls `clone_to_cache`/`clone_to_cache_subpath` directly, never hits the GitHub-API fast path, and carries a **second multi-skill matcher** (fuzzy bidirectional containment + subpath backfill, lines 795-820) with different semantics than `find_skill_by_name` (exact→contains, MULTI_SKILLS| on ambiguity). Two adapters for the same "which dir in this repo is my skill?" question — bugs fixed in one (e.g. `f140140` fixed explore-preview resolution via the shared path) won't fix the other. The self-deadlock fix (`82c36d5`, comment at 1820-1824) shows the GIT_CACHE_LOCK discipline is part of `clone_to_cache`'s *interface* (callers must know not to hold the lock) — an invariant currently documented only by one comment.
- **Deletion test**: deleting `update_managed_skill_from_source`'s bespoke download half would force ~80 lines into `fetch_skill_files` callers only if the update-specific bits (revision compare, staging-swap, subpath backfill) can't be layered on top — they can: they're post-download concerns, exactly what `fetch_skill_files`' doc comment assigns to callers.
- **Deepened shape**: finish the unification: `update_managed_skill_from_source` computes its `ParsedGitSource` (+ stored subpath preference) and delegates to `fetch_skill_files` into the staging dir; fold the fuzzy-name backfill into `find_skill_by_name` as an explicit matching policy so there is one matcher with one test suite (`tests/installer.rs` already covers this area). Consider making the lock non-reentrancy invariant structural (private lock acquired only inside the two cache functions — mostly already true; the explore-preview cache probe is the one external acquisition).
- **Strength**: Worth exploring (real, but the tests here are strong and the pain is slower-burn than 1-4). **ADR conflict?**: none.

### 6. Tool identity is a 6-place parallel-update ritual (ToolId enum ceremony around a data table)

- **Files**: `src-tauri/src/core/tool_adapters/mod.rs` (611): `ToolId` enum (19-65), `as_key()` match (67-116, 46 arms), `default_tool_adapters()` (139-448, 46 entries), `project_relative_skills_dir()` match (505-556, 46 arms); plus README supported-tools table (manual, see commit `77803a7` "add 4 missing adapters to README table"); plus frontend hardcodes: projects/ToolConfigModal.tsx's 9-tool prose subtitle and `".agents/skills (9 tools)"` display_name.
- **Problem**: Is this a real seam with adapters? Half. `ToolAdapter` the struct *is* a real adapter over a real seam (46 adapters satisfying one interface; `sync_dir_for_tool_with_overwrite`, detection, scanning all program against it — good depth). But `ToolId` adds no behaviour: every variant exists to be immediately mapped back to a string key and a table row. Adding one tool = enum variant + `as_key` arm + adapters entry + `project_relative_skills_dir` arm + README row (+ possibly `AGENTS_STANDARD_KEYS`, the `(9 tools)` label, and the modal prose). The compiler enforces 4 of those; the README/UI copies rot silently (77803a7 is the receipt).
- **Deletion test**: delete `ToolId` and replace with the string key already stored in DB/DTOs: `as_key()` and the match-arm ceremony vanish; `project_relative_skills_dir` becomes a field (or small override map) on the one table. Complexity does not reappear — most call sites (`adapter_by_key`, DB records, DTOs) already speak strings. The few typed matches (`ToolId::Codex && name == ".system"`, `supports_project_scope`) become fields. Verdict: `ToolId` is largely pass-through.
- **Deepened shape**: collapse to a single declarative table — each entry: `key, display_name, global_dir, detect_dir, project_dir (defaulting to global or .agents/skills membership flag), quirk flags` — and derive everything (AGENTS_STANDARD membership, "(N tools)" label, README table via a tiny generator or a test that diffs README against the table) from it. One place to add a tool; the count string and README stop drifting.
- **Strength**: Worth exploring. **ADR conflict?**: AGENTS.md treats the current invariant list as law ("Rust arms are compiler-enforced; the README table is not — check it") — this proposal *replaces* that invariant with a smaller one, so raise it explicitly rather than just doing it.

### 7. The "tools share one skills dir" invariant is re-derived on both sides of the IPC seam

- **Files**: Rust: `adapters_sharing_skills_dir` (tool_adapters/mod.rs:451-457), used in `sync_skill_to_tool`/`unsync_skill_from_tool` DB fan-out (commands/mod.rs:614-636, 660-690). TS: `sharedToolIdsByToolId` (App.tsx:615-630), `uniqueToolIdsBySkillsDir` (App.tsx:632-647), SharedDirModal confirm flow (App.tsx:1185-1205, 2380-2401).
- **Problem**: One domain invariant (Amp + Kimi share `~/.config/agents/skills`; the 9 AGENTS-standard tools share `.agents/skills` at project scope), two implementations on opposite sides of the seam, coupled only by the `skills_dir` string in `ToolInfoDto`. The frontend dedupes before calling; the backend *also* fans records out after syncing. If either side's grouping drifts (e.g. a new shared dir added in Rust while the UI caches stale `toolInfos`), you get double-sync or orphaned DB records, and no test would notice (frontend untested).
- **Deletion test**: delete the frontend grouping and let the backend own it (backend already must, for DB consistency) — frontend complexity vanishes except the "this dir is shared, confirm?" UX which needs only a `shared_with: string[]` field on `ToolInfoDto`. Complexity does not reappear ⇒ the frontend copy is pass-through.
- **Deepened shape**: backend exposes the grouping as data (`ToolInfoDto.shared_with` or a `dir_group` id); frontend consumes it for the SharedDirModal label and stops re-deriving dedupe (which a batch sync command from Finding 2 would subsume entirely).
- **Strength**: Worth exploring (rides on Finding 2). **ADR conflict?**: none.

### 8. App.tsx global-skills world vs useProjectState — the good pattern exists but only one page uses it

- **Files**: `src/App.tsx` (2,716; 60 useState; ~35 useCallback handlers; the render tree at 2404-2716 drills 10-18 props into each child — e.g. `SkillsList` gets `plan, visibleSkills, groupByRepo, viewMode, installedTools, loading, getGithubInfo, getSkillSourceLabel, formatRelative, onReviewImport, onUpdateSkill, onDeleteSkill, onToggleTool, onUnsyncSkill, onSyncSkillToAllTools, onOpenDetail, t`); `src/components/projects/useProjectState.ts` (442) as the sanctioned counter-example.
- **Problem**: Not the props drilling per se (ADR-protected) but the **absence of module boundaries inside App()**: settings state (theme/zoom/cache/token, ~10 states + 6 handlers + 5 mount effects at 431-473), update-checker state (5 states + the updater promise chain at 520-541), explore state (6 states), add/import flow state (~14 states), sync orchestration (Finding 2) all interleave in one closure. Understanding "how does the git-add flow work" means bouncing across lines 83-97, 157, 1476-1560, 1724-1850, 2536-2560 of one file. `useProjectState` proves the sanctioned fix: one hook = one world's state + actions + error normalization, page stays a thin binder. Note `useProjectState` itself repeats a "re-fetch assignments on error, silent fallback" block 4× (toggleAssignment, bulkAssign, resyncProject, resyncAll) — the pattern works but wants a small `refreshAssignments()` helper.
- **Deletion test**: n/a (nothing to delete; missing seams).
- **Deepened shape**: carve App.tsx along its existing worlds into hooks with the useProjectState shape: `useSettingsState`, `useUpdateChecker`, `useExploreState`, `useSkillLibrary` (managedSkills + load/refresh), `useSyncOrchestration` (Finding 2's single loop). Each returns `{data, actions}` and App keeps composing via props — pure useState+props, no Context.
- **Strength**: Strong (it is where every future feature lands). **ADR conflict?**: none if done as hooks; explicitly do NOT introduce Context/Zustand.

### 9. Frontend test vacuum + hand-mirrored DTO seam

- **Files**: `src/components/skills/types.ts` (110), `src/components/projects/types.ts`; no test runner in `package.json` (no `"test"` script, no vitest/jest/@testing-library anywhere); contrast `src-tauri/src/core/tests/` (15 files; skill_store 1,262, installer 1,246, project_sync 965 lines).
- **Problem**: The IPC seam's type contract is maintained by hand in both languages (snake_case field mirroring, `?`-optionality guessed to match `Option<T>`), verified by nothing. Combined with zero frontend tests, both halves of every cross-seam behaviour (error prefixes, DTO shapes, sync orchestration) are unverified where they live. This is why Findings 2, 3, 7 have no safety net: the *called* pure Rust functions are tested; the *calling* choreography is not — testability was achieved in core/ while the bugs' habitat (App.tsx flows) stayed untestable through its current interface (2,700-line component, everything closed over).
- **Deepened shape**: two cheap moves: (a) generate the TS types from Rust (`ts-rs` or `specta`/`tauri-specta` derive on the existing DTO structs) so the seam is checked at build time — no runtime change; (b) once Finding 8's hooks exist, they are unit-testable with a mocked `invoke` — add vitest for exactly those hooks + the error-describing module, not for JSX. This turns "extractable but never extracted" logic into tested modules with locality.
- **Strength**: Worth exploring ((a) alone is near-free). **ADR conflict?**: adds a dev-dependency + build step; AGENTS.md's DTO invariant ("update both sides") would be replaced by generation — same caveat as Finding 6: it replaces a documented invariant with a smaller one.

### 10. (Minor) Global CSS traceability

- **Files**: `src/App.css` (3,329, ~293 distinct class selectors), `src/index.css`.
- **Problem**: Not proposing CSS Modules (ADR). Friction is only in shared-class blast radius: e.g. `.pick-item*` (15 CSS rules) is used by 4 tsx files (both ToolConfigModals, LocalPickModal, GitPickModal), so styling "the tool config list" safely requires grep-auditing all consumers; `.modal*` skeleton is shared by ~9 modals. Deduplicating the modal shell (one `<Modal>` layout component, still globally-styled) would shrink both this and Finding 1's copy-paste surface without touching the CSS architecture.
- **Strength**: Speculative. **ADR conflict?**: none as scoped (component extraction, not CSS Modules).

---

## Summary

1. Worst pain is frontend: two copy-pasted ToolConfigModals, ~8 copies of the sync fan-out loop in a 2,716-line untested App.tsx, and a string-prefix error contract that has already drifted (8 prefixes shipped vs 5 documented) — all fixable within the useState+props ADR via useProjectState-style hooks and/or a batch sync command in core/.
2. The "commands/ = wiring only" ADR is violated twice materially: `sync_skill_to_tool` (~90 lines of logic) and `update_project_gitignore` (164 lines), the latter using the global skills dir where project sync uses the project dir — a suspected live gitignore mismatch for Windsurf/Pi/Goose/Augment-style tools.
3. installer.rs's `fetch_skill_files` unification is real but half-done: `update_managed_skill_from_source` keeps a private download path and a second, semantically different multi-skill matcher.
4. `ToolId` is mostly pass-through ceremony around a genuinely deep 46-adapter table; tool identity currently costs 6 parallel updates (4 compiler-checked, README + UI prose unchecked), and the shared-skills-dir invariant is re-derived independently on both sides of the IPC seam.
5. Core Rust is well-tested (15 test files); the frontend has zero tests and hand-mirrored DTOs — generating TS types from the Rust DTOs and unit-testing the extracted hooks are the cheapest levers to give the seam a safety net.
