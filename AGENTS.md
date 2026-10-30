# Skills Hub — agent context

Tauri 2 + React 19 desktop app that installs AI Agent Skills once and syncs them to many AI coding tools.
Canonical agent context for every harness. `CLAUDE.md` imports this file.

## Commands

```bash
npm run dev              # Vite dev server (port 5173, strict; VITE_DEV_PORT=5174 overrides — tauri:dev follows it)
npm run dev:fixture      # the app in a plain browser on an in-memory library (src/fixtures/, dev-only, tree-shaken from
                         #   production) — http://localhost:5175/?scenario=rich|empty|first-run|failures; touches nothing on disk
npm run tauri:dev        # Tauri dev window (frontend + backend) — see live-data warning below
npm run build            # tsc -b (TypeScript 7) && vite build
npm run lint             # ESLint
npm run test             # vitest unit tests (hooks + pure folds + commandError; jsdom, mocked seams)
npm run styles:check     # styling guard (ADR-0006 §6, scripts/check-styles.mjs): App.css ratchet, migrated files, no new
                         #   stylesheets, prose.css scope, no raw colours in JSX
npm run check            # lint + styles:check + test + build + rust:fmt:check + rust:clippy + rust:test
npm run version:check    # verify the 5 version locations (3 manifests + 2 lockfiles) agree
npm run version:set X.Y.Z   # bump all 5 version locations (never hand-edit)

cd src-tauri && cargo test <filter>              # single test / module by name substring
cd src-tauri && cargo test <filter> -- --nocapture   # show println!/dbg! output
```

**Two TypeScript compilers are installed, in the shape Microsoft's TS 7 release notes prescribe**: `typescript`
is aliased to the `@typescript/typescript6` shim (real TS 6 nested as `@typescript/old`; it owns the `tsc6` bin and
the `require('typescript')` API that `typescript-eslint` needs), and `@typescript/native` is TS 7 (`npm:typescript@^7.0.2`),
which owns the bare `tsc` bin. So `npx tsc` = TS 7 = the build's type-check; `npx tsc6` = TS 6, only for confirming
a failure is not a TS 7 regression. Don't collapse the pair: TS 7 ships no JS API, and `typescript-eslint` peers
`typescript <6.1.0`. Retire the TS 6 alias when `typescript-eslint` publishes a release whose peer range admits
TS 7 (gated on the TS 7.1 API, stable 2026-11-24; tracking: typescript-eslint/typescript-eslint#10940, QM ledger).

`npm run rust:test` runs `cargo test`; CI (`.github/workflows/ci.yml`) runs `cargo test --all` (includes all
workspace targets). Prefer `--all` locally when touching Rust to match the gate. CI runs the web job on Node 22
(jsdom 30 needs it); `release.yml` still builds on Node 20 and never runs vitest.

## Definition of done

```bash
npm run version:check && npm run check
```

`npm run check` **omits the version gate** that CI enforces (`.github/workflows/ci.yml`), so run both.
A version desync has shipped before (commit `f98bf9b`, "sync Cargo.toml version to 1.1.7").

## Release

Releases are built on GitHub, never locally. `npm run version:set X.Y.Z` + CHANGELOG entry, commit,
`git fetch && git rebase origin/main`, push `main`: `auto-tag.yml` sees the `package.json` change, creates `vX.Y.Z`
and dispatches `release.yml` (macOS ×2, Windows ×2, Linux). **A push that bumps the version is the release** — there is
no separate approval step. `release.yml` only compiles and packages; tests, clippy and the bindings-drift guard run in
`ci.yml` and in the local gate. The operator smoke-tests by installing the built release, not via `tauri:dev`.

## Environment gotchas

- **Dev runs mutate the operator's REAL skill library — there is no sandbox.** Dev and release builds share
  both the central repo (`~/.skillshub`; default named in `core/settings.rs`, materialised by `core/central_repo.rs`)
  and the app database (`~/Library/Application Support/com.skillshub.app/skills_hub.db`, from `identifier` in
  `tauri.conf.json`). Startup alone already writes: it wipes the Explore session cache and reclassifies legacy imports.
  Any sync/install/delete action in `npm run tauri:dev` writes the operator's live global skills and
  creates/removes real symlinks under `~/.claude/skills`, `~/.pi/agent/skills`, etc.
  Do destructive testing only with explicit permission; the `central_repo_path` setting can repoint the
  central repo, but the database path is not overridable. **Fixture mode (`npm run dev:fixture`) is the only way to
  run, drive or screenshot the app without touching the operator's library** — it also migrates nothing and asks the
  keychain for nothing. The t3 `preview_*` tools need a served URL (`http://localhost:5175/?scenario=rich`), never
  `file://`; in a hidden tab the page only runs while a `preview_evaluate` is in flight, so drive each flow inside one
  evaluate, one call at a time, and force a repaint before a capture. Saving any tracked file under the repo reloads an
  open fixture page and resets its in-memory library (Tailwind's Vite plugin scans tracked files). During a stateful
  fixture session, keep notes under a gitignored `.scratch/**/evidence/` path.
- After an Xcode update every `cargo` build fails at link with "You have not agreed to the Xcode license
  agreements" — the operator runs `sudo xcodebuild -license accept`; there is no repo-side fix.
- `.claude/skills/` and `.agents/skills/` are **hardlinked to each other** (same inodes) and gitignored —
  editing a skill file in one silently edits the other.
- The Rust crate is `app_lib`, not the package name — import as `app_lib::...`.

## Invariants — touch X, then also update Y

- **Version**: `package.json` + `src-tauri/tauri.conf.json` + `src-tauri/Cargo.toml` + the root `version`
  entries of `package-lock.json` + the `app` package entry in `src-tauri/Cargo.lock` must match. Use
  `npm run version:set X.Y.Z` (`scripts/version.mjs`) — it rewrites all five so the next `cargo test` or
  `npm ci` dirties nothing; never edit a version field by hand.
- **New Tauri command**: define it in `src-tauri/src/commands/` (`mod.rs` or `projects.rs`) with
  `#[tauri::command]` **and** `#[specta::specta]`, **and** list it in `specta_builder()`'s
  `collect_commands![...]` in `src-tauri/src/lib.rs` — that one list feeds both the invoke handler and
  the generated `src/bindings/index.ts`, so an unlisted command has no typed binding and its frontend
  call fails `npm run build`. Frontend arg names are the Rust parameter names in `camelCase`.
- **IPC types are generated by tauri-specta, never hand-mirrored**: any struct/enum crossing `invoke`
  derives `specta::Type` (including DTOs living in `core/`); `cargo test` regenerates
  `src/bindings/index.ts` (the `export_bindings` test does it — a filtered `cargo test <filter>` that skips that test
  leaves bindings stale) — one file with every DTO plus a typed `commands.<name>(args…)` function per
  command (committed; CI fails on drift *or* untracked bindings). Components import DTO types from the
  per-world shims (`src/components/skills/types.ts`, `src/components/projects/types.ts`), never from
  `src/bindings` directly. Adding a DTO = derive + `cargo test` + re-export from the shim.
  Fidelity rules: `Option<T>` is wire-accurate `T | null` (serde always emits the key; never `?`);
  `#[serde(default)]` input fields become `field?:`; `i64`/`u64`/`usize` export as `number` via
  `dangerously_cast_bigints_to_number()` (timestamps/counters only — keep it that way); `f64` exports
  as `number | null` (serde_json writes NaN/∞ as null) — a field the backend guarantees finite carries
  `#[specta(type = specta_typescript::Number)]` (see `core/settings.rs`).
  **Pins**: `tauri-specta`/`specta`/`specta-typescript` are exact `=` pins in `src-tauri/Cargo.toml` (an RC line
  with no semver guarantee). Bump deliberately: change the pins, run `cargo test`,
  review the `src/bindings/index.ts` diff for shape changes, then `npm run build`.
- **New AI tool adapter**: add the `ToolId` variant (+ its `as_key` arm) and one `ToolAdapter` literal in
  the `TOOL_ADAPTERS` registry (`core/tool_adapters/mod.rs`) — every per-tool fact lives in that literal
  (global/detect/project dirs, `former_relative_skills_dirs` — usually `&[]`; a corrected global dir goes there so
  Target relocation moves existing targets — `group` for virtual-group membership, `supports_symlink`), **and** add a row
  to the README supported-tools table, **and** extend the `project_relative_skills_dir_for_every_tool`
  table test (`core/tests/tool_adapters.rs`). Only `as_key`'s exhaustive match is compiler-enforced; registry
  completeness is test-enforced and the README table is not enforced at all — check it matches the `ToolId` variant
  count whenever adapters change. Tool lists shown to the UI come
  from `tool_adapters::global_tool_entries` / `project_tool_entries` — commands only map them to DTOs.
  Installedness is one rule, `tool_adapters::is_installed_in`: some `relative_detect_dirs` entry exists and is
  not a skills-only footprint (virtual groups exempt; a shared skills convention dir such as `.config/agents` is
  never a detect dir — `no_adapter_is_detected_by_a_shared_skills_convention_dir` enforces it). Test fixtures
  install a tool through `tool_adapters::mark_installed_in`, never a bare `create_dir_all(detect_dir)` — that is
  the footprint the rule rejects.
- **Sync-target mutation**: every operation that materialises or removes a Sync target (global sync
  batch, Disable / Enable (`skill_disable::disable_targets` / `enable_targets`), delete,
  assign/unassign/toggle, resync (of one assignment via `resync_project_assignment`, one project, or every project),
  `resolve_project_collisions`, configure project tools, remove project,
  gitignore update, Refresh's apply phase, Onboarding import's apply phase) wraps its **own** body in
  `core/mutation_guard::serialized` at its entry point — commands carry no lock state and make no
  per-call-site decision. The mutex is private to that module; per-target internals are unlocked
  `pub(crate)` `*_unlocked` seams. The guard is non-reentrant, so **an entry point never calls another
  entry point** — composites call the unlocked seam. Readers that must not queue use `try_serialized`:
  the project listing's reconcile pass takes that door and reports `reconciled: false`. `reveal_path` (Reveal in
  Finder) mutates nothing but is fenced like a removal: `core/log_reveal.rs::admit_reveal_path` admits only a recorded
  target path or a path physically inside the central repo (no NUL, no as-written `.`/`..`), and the command selects
  the item — or its nearest existing ancestor — in its folder, never opens it.
- **UI strings**: add keys to `en` in `src/i18n/resources.ts`; EN only (BACKLOG #61). No hardcoded UI text.
  `src/i18n/resources.test.ts` fails on any literal `t("…")` key missing from `en`.
- **New `core/` module**: declare it in `src-tauri/src/core/mod.rs`.
- **DB schema change**: consider `migrate_legacy_db_if_needed` in `core/skill_store.rs` — a migration path exists.
- **New or changed UI primitive** (`src/components/ui/`, ADR-0006): its `variant` / `tone` / `size` values are its
  whole styling interface — document each in `DESIGN.md` in the same change; a styling need props cannot express
  becomes a new documented variant, never a call-site `className` override (`className` is layout only). New
  tokens go in `src/styles/theme.css` alone, with a dark and a light value.

## Ambiguity resolution

- **No state-management library** (no Zustand/Redux/Context). State lives in plain `useState` inside
  per-world hooks (`src/hooks/`, `src/components/projects/useProjectState.ts`), each returning that world's
  data + actions; `src/App.tsx` is the binder that composes the hooks and passes props down. Cross-world
  needs flow through interfaces App passes into hooks (e.g. the shared `useStatusReporter` surface, the
  sync seam) — **world** hooks never import each other at runtime (type-only imports of another hook's
  types, narrowed with `Pick<>`, are the established pattern for declaring deps). A world may be built
  from smaller building-block hooks it owns (e.g. `useAddSkillFlow` instantiates `useCandidatePick`
  twice); those are internal to that world, not a cross-world seam. Every world — the project world
  (`useProjectState`) included — is composed in App and lives as long as the app, so selection and loaded
  data survive navigation; a view receives its world's value as a prop. **The Tool catalog is one module,
  `src/hooks/useToolCatalog.ts`** (tool status, labels, installedness, `shared_with` map, the project tool
  entries), read by sync, library, add flow and projects; `useSyncOrchestration` keeps only the saved
  selection, the two target sets and the sync seam. **The app update is one module,
  `src/hooks/useAppUpdate.ts`** (status, version, notes, `check`/`install`/`dismiss`/`ignore`), read by
  Settings, the passive sidebar item and the `UpdateDialog` it opens (no dialog on startup, D8). A world reads another world's catalog only as a value
  App passes in — the project world's skill rows are the library's `ManagedSkill[]` and its Tool entries
  the Tool catalog's, handed to `ProjectsPage` by App, never fetched or imported. **Every verb on a skill goes
  through one door, `src/hooks/useSkillActions.ts`**: `run(verb, skills | ids | "all", opts)` over `syncToTargets ·
  retry · disable · enable · update · restore · repoint · detach · delete · editInvocation · dismissHealth ·
  assignToProject · unassignFromProject · deployGroup`. The door decides one batch vs one skill, where confirmation
  happens (a verb without its confirming option — `target`, `mode`, `confirmed` — opens its dialog; a pair asks the
  shared-dir confirmation), which fold settles the report, the reload rule (`RELOAD_ON_THROW`) and the
  post-mutation focus; `repair(skill, row, key)` maps a health row's repair onto its verb. `useSkillLibrary` keeps
  only the catalog (`replaceCatalog`, `loadManagedSkills`) and which skill each dialog is about. **The action
  lifecycle is the reporter's, `src/hooks/useStatusReporter.ts`**: `runAction` holds the one `pending` action
  (subject `skill` / `cell` / `skills` / `batch`, label, `activity` words, streamed `progress`, `cancellable`) —
  **one mutation at a time**
  (D19): a second is refused with a typed reason (`refusal`, never queued), a `read: true` action waits its turn;
  `cancellable` only when the action declares a token-taking command (`cancel:`, D20), withdrawn for an
  `action.uncancellable(run)` phase, and a body checks `action.cancelRequested()` before sending a token-taking
  command that follows a token-less step (the command resets the token); `cancelPending` hides
  nothing until the command returns; `applyOutcome(outcome, { reload, closeModal, onAction })` executes every
  fold's Outcome (reload first, toasts, one `recordRun` history block, close) — no caller unpacks an Outcome;
  `notifyError(err)` is the one failure dialect (`setError` remains only in the sync and Add worlds until they
  migrate). The project matrix's pending cells are the reporter's `{ kind: "cell" }` subject. **Progress is
  `src/components/shell/HeaderProgress.tsx`, read from `pending` alone** (`showsHeaderProgress`): My Skills' header
  slot, a strip above any other area, or the foot of the dialog whose action runs (App's `progressInDialog`; the
  header copy yields while it shows) — there is no overlay; a bulk Remove shows in its Delete dialog. A refused
  control stays focusable while disabled (`focusableWhenDisabled`) and carries the reason; `runAction` ends every
  action with `actionLifecycle.returnLostFocus`, so focus never settles on `<body>`. **What is on screen is one module,
  `src/hooks/useSurfaceState.ts`** (created first in App, passed to worlds as a narrowed door): the area,
  the focused subject, per-surface selection and the dialog stack (one union across worlds). World hooks
  keep data and actions and *ask* it to open/close (`openDialog`, `pushDialog` for a nested step, `ask`
  for a confirmation, `offerDialog` for a startup announcement, `focus`) — no world keeps a visibility
  flag or a "which skill is this modal about" cell. Only area + focused subject reach `location.hash`
  (`#/skills/<id>`, `#/projects/<id>`, `#/explore`, `#/settings/<pane>`); dialogs, selection and drafts
  never do. A catalog owner calls `useFocusReconciliation` so a focused/selected id that leaves it
  clears itself. **Query, sort, grouping and view mode are the list lens, `src/hooks/useListLens.ts`**,
  one instance per list surface (My Skills in App, the matrix in `AssignmentMatrix`), persisting through
  `preferences.ts`; its folds stay pure in `skillPresentation.ts`. After a mutation, the **project**
  world applies what the mutation returned — a `ProjectViewDto` fed to `useProjectState::applyView`, no
  success-path refetch tail, only a failure-path `refreshView`. The **skills** world replaces its entire
  catalog from `{ report, skills }` after Update, Restore, Re-point and Edit — even when the returned
  report contains failures/skips. Commands read that catalog after settlement/reassert, outside the guard;
  a catalog-read failure is a command error, never an empty success. Refresh-all, import and delete retain
  `getManagedSkills` (`loadManagedSkills`); thrown Update/Restore/Re-point requests also reload, thrown Edit
  does not. Three project mutations cannot
  answer with a view: `remove_project` returns `{ projects, report }` (the project is normally gone; a
  project kept under ADR-0002 is still listed and the hook takes the failure-path `refreshView` for its matrix),
  `update_project_gitignore` returns `()` (it changes no view row), and `resync_all_projects` returns
  `{ report, projects }` — one `ProjectSyncReport` spanning every project — so the hook refetches
  `getProjectView` for the selected project alone. `configure_project_tools` returns `{ view, report }` — the
  dropped tools' removal reports merged into one — and so does `resync_project_assignment` (Sync this).
  `useMatrixFocusReconciliation` (`src/components/projects/`) moves focus to the next surviving row's cell when a
  repair takes a row out of the matrix filter. Every project report is folded in the view's action hook
  (`src/components/projects/useProjectActions.ts`), never in the world hook: removal reports by
  `projectRemovalOutcome` / `removalOutcome` (toggle-off, bulk unassign, configure, remove), `ProjectSyncReport`s by
  `projectSyncOutcome` (toggle-on, bulk assign, resync); `toggle_project_skill_assignment` answers a `kind`-tagged
  union so the page picks the fold without a boolean. **Every** project mutation — registration, the Edit dialog's
  ignore write, a path update included (a folder pick stays outside; the mutation it leads to is gated) — runs under
  `runAction`, and the list, matrix and dialogs disable their mutation controls with its `blockedReason` while
  anything runs; per-cell pending is only the localized mark.
- **The Add world exposes intents, not state cells**: `useAddSkillFlow` returns `open({ kind?, source?, preselect? })`,
  `install()`, `retry(ids)` and the `add` view (step, draft, picker values, settlement rows, summary) for the one
  stepped `AddSkillDialog`; Explore calls `open`, never an effect trigger. The file browser is one module:
  `src/hooks/useSkillDocument.ts` (fed `{ root, version }`) over `src/lib/skillFiles.ts`, read by the inspector's
  Files section and Explore's preview.
- **Frontend tests are hook- and pure-function-level; no component rendering tests** (vitest + `renderHook`,
  jsdom; colocated `src/**/*.test.ts`,
  type-checked by `npm run build`): mock at module seams — `src/lib/tauri.ts` for backend calls,
  `src/lib/platform.ts` for the native platform, `sonner` for toasts. **Everything native that is not a
  command crosses `src/lib/platform.ts`** — `pickFolder({ title, multiple? })` (the one picker rule:
  `string[]`, `[]` on cancel, a failure rejects and every caller catches), `createProgressChannel(onMessage)`,
  `checkForUpdate()`, `setZoom()`, `appVersion()` — with the `isTauri` guard inside; no hook or component
  imports a Tauri plugin or `@tauri-apps/api/*`, and the fixture installs its own adapter through
  `installPlatformAdapter`. Every hook **and component** calls the
  backend only through the typed seam `invokeTauri(name, ...args)` (`src/lib/tauri.ts`, generic over
  the generated `commands` table: a wrong name, missing/extra arg, or wrong arg type fails
  `npm run build`); raw `@tauri-apps/api/core` `invoke` appears nowhere outside `src/bindings/index.ts`.
  Test stubs cast the mock to `(command: CommandName, ...args: unknown[]) => Promise<unknown>` and
  switch on the camelCase command name. Components get no JSX tests by design. **One exception (round-19 spec D2, ADR-0006 §7):**
  the primitives' `src/components/ui/*.test.tsx` render them to pin their role/ARIA/focus contract (names, states,
  focus in/out, layer stack, label association) and `focus-ring.test.tsx` resolves their classes against compiled
  Tailwind — never product surfaces, never pixels or copy.
- **`commands/` is wiring only** (DTO conversion, error formatting); business logic goes in `core/`, which is
  independently testable. Async commands wrap sync work in `tauri::async_runtime::spawn_blocking`.
- **Error wire contract** — commands return `Result<T, CommandError>` (`core/errors.rs`, beside `SignalError`),
  a serde internally-tagged enum (`{ code: "TOOL_NOT_WRITABLE", tool, path }`). Core raises typed conditions
  as `SignalError` through `anyhow` chains; `CommandError::from_anyhow` recovers them by downcast — never
  encode error conditions in message strings. Classification has **two timings**: a fan-out report row is
  classified in core **where it settles** (after its `{:#}` chain is written to the row's `last_error`), so
  report types carry `CommandError` values; the command seam's `from_anyhow` is for errors that fail a whole
  command — core never classifies an error it then `?`-propagates (ADR-0001, round-15 amendment). The TS
  mirror is generated by tauri-specta into `src/bindings/index.ts` on `cargo test` (commit it; CI
  diff-guards it). Errors are *thrown* (`ErrorHandlingMode::Throw`), not returned as a result
  envelope — `runAction`/`toCommandError` own the catch.
  A guard against a condition the UI cannot produce may bail with prose (it surfaces as `OTHER`);
  a condition the operator or upstream can cause is a typed variant.
  All user-facing error copy lives in the frontend's `describeCommandError` (`src/commandError.ts`).
  **English-primary policy**: the backend composes no user-facing prose in any language — all locales
  live in the frontend catalog; new backend messages and comments are written in English. Structured
  `detail`-style fields carry diagnostics (stderr, env-var hints, paths), never localized copy.
  Adding a variant: Rust variant + regenerated binding + a `describeCommandError` branch + i18n keys
  (worked example: `PATH_OUTSIDE_TOOL_DIRS` / `SignalError::PathOutsideToolDirs`, the refusal to delete
  a path outside every Tool skills dir — owned by the registry's
  `tool_adapters::ensure_path_within_tool_dirs`). See `docs/adr/0001-tagged-command-error-contract.md`.
- **`src/components/Layout.tsx` and `src/pages/Dashboard.tsx` are dormant by design** — imported by nothing under `src/`.
  Leave them alone unless explicitly wiring them up.
- **Styling is Tailwind v4 per `docs/adr/0006-styling-architecture.md`**; `DESIGN.md` (repo root) is the reference
  for token values with measured contrast, every primitive's `variant` / `tone` / `size`, focus, motion and target
  size. Tokens only in `src/styles/theme.css` (dark `:root`, light `[data-theme='light']`, `@theme inline`; the
  Tailwind palette is removed), utilities in JSX via `cn()` (`@/lib/utils`), recurring shapes through the primitives
  in `src/components/ui/` (shadcn on Base UI, restyled to our tokens; `className` is layout only), no raw colours.
  `src/App.css` is legacy and only shrinks: its opening alias block maps the legacy custom properties onto the tokens
  and was its last addition — never add a rule to it; a surface counts as migrated once its files reference no class
  `App.css` defines. The only other stylesheets are `src/index.css` (entry + base layer) and `src/styles/prose.css`
  (rendered Markdown / code under `.prose`, token-only). No CSS Modules, no new `.css` files. `npm run styles:check`
  (`scripts/check-styles.mjs` + `scripts/styles-baseline.json`) enforces all of it: migrating a surface appends its
  files to `migrated` and lowers `appCssCeiling` in the same change (the baseline file is the live count — 1108 lines
  and 36 files at 1.3.0). Read the resolved theme from the settings world
  (`resolvedTheme`), never from `data-theme` on the DOM; the pre-paint script in `index.html` is the one sanctioned
  storage read outside `preferences.ts` (key pinned by `src/lib/preferences.test.ts`).
- **Every layer is on one stack, `src/components/ui/layer-stack.ts`** — modal layers (Dialog, ConfirmDialog, the
  overwrite and shared-dir asks) and floating ones (Popover). Only the top layer dismisses, and one event dismisses one
  layer (`claimDismissal`); a modal layer makes everything beneath it `inert` — except the Toaster's `body`-child host
  (`LIVE_HOST_ATTRIBUTE`; not a layer, so a toast under a modal stays announced); each layer carries its return chain
  (opener + caller fallback) and hands it up when it closes beneath another (`handOverReturn` / `returnChains`); a
  dialog opened from a menu returns focus to the menu's trigger. Dialog props cover the rest — `interceptEscape` (the
  top layer spends an Escape first, e.g. collapsing Files), `placement="sheet"` (the inspector below the 1200 px
  shell breakpoint, `useMediaQuery(SHELL_WIDE_QUERY)`), a fallback focus target. Keep a dialog's shell mounted and
  toggle `open`: a conditionally mounted dialog cannot return focus. Surfaces never wire Escape or focus return
  themselves; DESIGN.md documents the contract per primitive.
- Path handling must support `~` expansion (`core/environment.rs::expand_home_path_in(home, input)`).
- **Core never resolves filesystem roots from the environment.** `core/environment.rs` is the only core caller
  of `dirs::home_dir()`; every other core function takes explicit roots (`home: &Path`, `InstallerPaths { home,
  central_dir, cache_dir }`) so tests substitute a temp dir. Commands resolve roots once at the seam
  (`installer_paths`, `resolve_central_repo_path_for_app` in `commands/mod.rs`) — never thread
  `tauri::AppHandle` into `core/`. Sanctioned exceptions: `cache_cleanup.rs`/`temp_cleanup.rs`/
  `skill_store::default_db_path` take an `AppHandle` solely to call `app.path().app_cache_dir()`/`app_data_dir()`;
  and `SKILLS_HUB_*` **tuning env vars** (`git_fetcher.rs`: timeouts, libgit2 fallback, git executable;
  `sync_engine.rs`: IO profiling) are read where they apply — they tune behaviour, never locate skill data.
  **The same rule covers secrets**: core never constructs the OS keychain adapter — every function that needs the
  GitHub token takes `credentials: &dyn CredentialStore` (`core/credentials.rs`; `MemoryStore` in tests) and the
  command seam builds the one `KeyringStore` (`commands::credential_store`). The token is the one setting that
  never lives in the settings table or crosses the wire (`AppSettings.github_token_set` is a presence flag);
  a store failure is the typed `CREDENTIAL_STORE_UNAVAILABLE`, never a plaintext fallback.
- Sync uses a triple fallback: symlink → junction (Windows) → copy.
- **Manifest** reads and byte-preserving invocation writes live in `core/manifest.rs`; discovery,
  finalize and Edit consume it. Persisted `InvocationLines` fields are a compatibility contract. Fences
  are complete column-zero `---` lines (trailing whitespace/CRLF allowed); only a persisted Edit base
  authorizes the former indented opening for clear/re-choose. Fresh reads and upstream replay stay strict.
  The pure frontend adapter `src/lib/manifestPresentation.ts` uses the same input corpus, with independent
  header/name-validity and display-metadata expectations. Optional lock-file text reads use
  the shared read adapter, but `skill_lock` retains JSON/provenance policy and permissive failure handling.
- **Content identity** lives in `core/content_identity.rs`: finalize/Edit record it; Propagation,
  reconcile and same-content checks read the stored identity, backfilled by the module when absent.
  Unmanaged onboarding candidates use its directory read; hashing stays private to the module.
- **Target fan-out is backend-owned — one batch command per operator action, never a frontend loop.**
  Each answers with a report whose per-target failures/skips are **report data**, not command errors; sync,
  Refresh/Update and import also stream progress over a Tauri `Channel` (removal does not). **Core report types
  cross the wire as themselves**: they derive `Serialize + specta::Type` with their serde tags, there is no
  `*ReportDto` mirror or mapper in `commands/`, per-item failures are `CommandError` classified at settlement,
  and counters (`refreshed/failed/skipped`, `removed/failed`, …) are derived once by the frontend fold
  (`src/lib/reportOutcome.ts`), never carried on the wire. Delete returns its `RemovalReport` too:
  - sync → `sync_skills_to_tools` (`core/global_sync.rs`): installedness filtering, shared-dir dedupe,
    overwrite policy (batch default + per-(skill,tool) overrides), DB record fan-out. The frontend seam
    (`useSyncOrchestration.syncSkillsToTools`) owns the **overwrite ask**: every call site passes
    `overwriteIfSameContent: true` (identical content is replaced silently); `TARGET_EXISTS` rows raise one
    confirmation and one retry batch with per-pair `overrides` (`decision: "overwrite"` confirmed, `"keep"` declined —
    the backend records a kept pair `disabled`) whose rows replace the asked ones. An unanswered `TARGET_EXISTS` records
    no row; any other failed first sync records an `error` row that owns nothing (D9).
  - Skill health (`core/skill_health.rs`): the skill row's stored last outcome, written by core **where the outcome
    settles** (refresh / skill_update / artifact_removal / onboarding_import) with the report row's `CommandError`,
    never for a `?`-propagated error or a cancellation; a success clears only its own family.
  - Update / Refresh (all) → `refresh_managed_skills` (`core/refresh.rs`): acquire every skill (bounded
    pool of 4, std threads), then `skill_update::apply_unlocked` admits and settles each under the
    guard (finalize + Edit replay + Propagation; ADR-0004), plus the `reassert_auto_sync` policy. A single
    Update is a batch of one, reached through `update_managed_skill` for Update/Restore's catalog-bearing
    response. `core/skill_update.rs` owns the git/local/Edit/Restore byte adapters; stale acquisitions are
    discarded as skipped report data, never retried in-batch. Direct Edit settles through the same Update
    module; its core result `{ entry, propagation }` becomes a `{ report, skills }` command response.
    The Edit fold consumes target failures as report data, never unconditional saved success.
  - Re-point → `repoint_skill_source` (`core/repoint.rs`): one operation over `RepointTarget::{Git, Local}` for
    every provenance (imported included — it stops being imported, ADR-0003 amendment); validates the target,
    then the same batch-of-one Update settles source and bytes together.
  - Onboarding scan → `get_onboarding_plan`: `commands::onboarding_scan_scope` resolves
    `OnboardingScanScope::{Installed, Selected}` from the settings once; the plan and the import re-derive
    under the same scope.
  - Add → `install_selection` (`core/install_batch.rs`): one call per Add action over a source
    (`InstallSource::{Git { url, resolution }, Local { root }}`) and its picked candidates; per candidate, acquire
    outside the guard (`installer::stage_*`), then finalize + first sync through `sync_skills_to_tools_unlocked`
    in one `serialized` section. Answers one `InstallBatchReport`: per candidate `installed { skill, targets }` /
    `failed { error }` / `cancelled` (the `CancelToken`, honoured between candidates, after an acquisition and once the
    guard is held — never mid-finalize). Collisions are **collected** as `TARGET_EXISTS` target rows, never asked in core.
    It is the only install command (the per-candidate `install_git_selection` / `install_local_selection` are gone).
  - **The one overwrite ask (D12)**: every `TARGET_EXISTS` producer — the global batch, the install batch, the
    re-assert, import (a divergent sibling in a target's way is reported once, as that row), Enable, project
    assign / resync / Propagation — answers a row carrying pair identity plus `path`, `modifiedAt`, `sameContent`
    (observed where `sync_engine` raises the collision). The frontend raises **one** ask per operator action
    (`src/lib/overwriteAsk.ts` shapes the rows; `useSyncOrchestration` owns the ask) answered per row, and retries:
    global pairs through `sync_skills_to_tools` with per-pair overrides (the install batch's skills exist by then),
    Enable's global pairs through `enable_skill_targets(scope, overrides)`, project assignments through
    `resolve_project_collisions` (`Keep` → the row `disabled`, owning nothing, reported `declined`). A dismissed
    ask sends nothing. An operator action that settles several reports (a group's Deploy all, Retry over several
    Tools, a multi-skill project assignment) or a report of another shape (Refresh / Update / Restore / Re-point —
    Propagation and the re-assert — and Edit's Propagation) **collects every report first** (`collectSync`,
    `collectEnable`, the project world's `assignToProject(projectId, skillIds)`) and asks once through
    `useSyncOrchestration.settleCollisionParts`, which retries with exact pair identity (one sync batch per skill
    Tool-set, never a skills × Tools product), hands each part back with its answered rows settled (a kept
    Propagation row as `declined`, folded as a quiet note) and says whether a retry ran — the door then re-reads the
    catalog and the touched projects' views, and folds once.
  - Onboarding import → `import_onboarding_selection` (`core/onboarding_import.rs`): admit, finalize,
    then sync through the global sync batch (auto-sync on — a first sync, not Propagation) or remove
    byte-identical originals (auto-sync off), per group. An imported skill has no external source (ADR-0003).
  - project assignment on / bulk assign / resync (one assignment, one project, all) → `core/project_sync.rs`,
    answering a `ProjectSyncReport` (per-assignment `synced` / `already_assigned` / `skipped { disabled }` /
    `declined` / `failed { error }`, the failed row's id carried when the row exists, plus `skipped_projects` —
    `folder_missing` — for a project whose root is gone: every project sync, Enable's and the collision retry
    included, checks `project_ops::project_folder_exists` before touching a row); the store failure that fails a
    listing is a whole-command error, not an item. A project target write never creates the project root:
    `sync_engine` creates its parents through `create_dirs_within(root, dir)` (`TargetParents::WithinRoot`, one
    non-recursive `mkdir` per level, a vanished root → typed `InvalidPath missing`); global sync keeps creating a
    Tool's missing skills dir.
  - Disable / delete / project removal / bulk unassign → `core/artifact_removal.rs` (below).
  - Disable / Enable: `core/skill_disable.rs` (`disable_skill_targets` / `enable_skill_targets` over one
    `DisableScope::{SkillTool, Skill, Skills}`). This is the durable "not on this Tool" at two grains: a `disabled`
    target row (pair), or `skills.disabled_at` plus every row `disabled` (skill). There is no unrecorded unsync.
    **Every fan-out skips a disabled target as report data, never `Failed`**: Propagation and the re-assert
    (`PropagationSkip::Disabled`); the global batch, hence Onboarding import's first sync (`Skipped` with
    `TARGET_DISABLED`, also for a shared-dir sibling of a disabled row); project sync (`ProjectSyncSkip::Disabled`).
    A fan-out gates on `skills.disabled_at` as well as row status: a failed skill Disable keeps rows `error`
    (ADR-0002), and those are skipped too (resync, bulk assign, relocation).
    Only Enable re-materialises, as a first sync (`EnableReport { global, projects }`).
  Bringing every target of one changed skill into line is **Propagation** (`core/propagation.rs`) — the
  only writer of target rows on an update path, spanning global target rows and project assignment
  rows, honouring each Tool's capability through `sync_engine::sync_dir_for_tool_with_overwrite`.
  The shared-skills-dir grouping reaches the UI only as `ToolInfoDto.shared_with`; do not re-derive it
  from `skills_dir`.
- **Artifact removal is one module.** `core/artifact_removal.rs` plans by scope (`Skill`, `SkillGlobal`,
  `SkillTool`, `Project`, `ProjectTool`, `ProjectSkill`, `ProjectSkillTool`, `EveryGlobalTarget`; a scope carries the
  roots its planning needs), executes with one presence rule (`symlink_metadata`) and one settlement
  rule — a row whose artifact could not be removed is **kept** with sync status `error`, rows are
  deleted only on success (`docs/adr/0002-keep-row-with-error-on-failed-artifact-removal.md`) — and
  reports per target. Disable (`skill_disable`) is the one caller that settles success differently
  (`execute_settling_unlocked(.., Settlement::Disable)`: rows kept `disabled`, never deleted; failure is
  still `error`). **Ownership** is independent of status: only a row the app materialised
  (`synced_at` set — `owns_artifact`) owns its path. A never-synced row (the pair marker, an assignment recorded for a
  disabled skill, the same rows after a failed Enable turned them `error`) owns nothing: planning sets it aside, no path
  is touched, and once the scope succeeds Delete clears the row alone (`RemovalReport.cleared_markers`) and Disable
  flips it `disabled`; re-sync and Propagation never overwrite its path. Only a successful sync makes a row an owner;
  Enable's admission (`AssignmentTransition::EnableAdmitted`) clears a `disabled` assignment's ownership (its Disable
  removed the artifact), never an `error` row's. Callers (`project_ops`, `project_sync`, `skill_disable`, the delete command) build a scope
  and read the report; they never delete a path themselves (project reconcile may probe one with `symlink_metadata`).
- **Git bytes have two single entry points.** `git_cache::fetch_through_cache` is the only way into the
  clone cache (per-key locks, TTL as an `i64` value from `settings::git_cache_ttl_ms`, no DB handle in
  its interface). The key is the normalised URL + ref only — never the subpath: the checkout shape
  (full / sparse union) is entry metadata, a hit needs freshness *and* coverage, and an entry is only
  ever widened, never narrowed (so the Add flow's listing clone serves its install, and two skills of
  one repo share an entry safely under parallel Refresh). `git_acquisition::acquire` is the only way
  bytes land from a git source: `SkillIntent::{StoredRecord, Selection, ByName}` carries caller intent
  (a `Selection` always names a subpath — listing builds its own private resolution, so "list candidates"
  is unrepresentable as an acquisition);
  `git_acquisition/resolution.rs` privately owns branch splitting, stored-hint repair and assumed-branch
  policy. Listing uses acquisition's candidate admission and returns `GitSourceResolution` for the selected
  install to reuse (including deliberate null/default branch). The Contents API fast path records a real
  commit SHA; clone is the fallback. Content 404/403 and explicit-branch SHA 404 are typed refusals, never
  clone retries; a stored-hint SHA 404 permits one refs repair, an assumed-main SHA 404 permits default-
  branch clone fallback. Install, Refresh acquisition and Explore choose destinations. Explore acquires
  privately and publishes by rename under its short cache lock; a concurrent loser reuses the completed
  winner and cleans only its own scratch.
- **Frontend presentation logic is pure and lives once.** `src/lib/skillPresentation.ts` owns source
  kind, repo label/href, repo grouping, the My Skills search/sort fold and the relative-time formatter
  (`relative.*` is the only i18n family for it); `src/lib/persistedPreference.ts` +
  `src/lib/preferences.ts` own persisted view preferences (the literal storage keys are a compat
  contract with existing users); `src/lib/reportOutcome.ts` owns turning every backend report (Refresh /
  Update batch-of-one, Edit, removal, global sync, import, install, delete) into an `Outcome` — toast, error and
  warning entries, quiet `notes` (a declined pair), and a `completion` (reload / closeModal / conflict) the
  reporter's `applyOutcome` executes; a batch's headline is the composite count line (`compositeHeadline`: done · failed · targets failed ·
  skipped · declined · cancelled · to review, zeros omitted) and precedence only picks the toast `tone` (failure ›
  conflict › skipped › declined › success; a cancel alone is neutral) — `declined` and `cancelled` are outcome kinds,
  never failures (D10); the conflict completion fires independently; actions carry a skill *id* the hook resolves at click time, and
  hooks unpack response envelopes but never interpret nested report fields outside the fold.
  **`skillHealth(skill, { t })` is the one health/source fold**: skill-level rows (Unlocatable, the stored
  `ManagedSkill.health` kinds, a live Edit conflict, skill-grain Disable) and target-level rows (failed · missing ·
  stale · pair-disabled · not deployed), one precedence (`HEALTH_ROW_ORDER`, which is severity order failed ›
  stale › muted), each row's `repairs`, and the source line (`skillSourceLine`: label, href, copy value, icon).
  Row, inspector ladder, exception summary/Issues chip and strip read it; nothing composes health or a source line
  by hand. The other pure folds, one per surface: `src/lib/bulkCounts.ts` (what a wide verb's count confirmation
  states — exactly what its settlement touches), `src/lib/notificationRuns.ts` (the panel's runs and row actions;
  the reporter's `recordRun` is the one writer), `src/lib/projectPresentation.ts` (matrix cell, legend, chips),
  `src/lib/addFlow.ts` (checked ∧ visible installs, settlement rows), `src/lib/overwriteAsk.ts` (the ask's rows).
  `deployAction` counts only a live target (`isLiveTarget`: not `disabled`, not a never-synced `error` row) as
  deployed. `src/lib/actionLifecycle.ts` owns the lifecycle's pure vocabulary: pending subjects, the cancellable
  set (`CANCELLABLE_COMMANDS`), the refusal copy and `focusAfter` (D21: Delete → next row / previous / heading;
  Restore → health row; otherwise the same control) with its one DOM resolver. Components import pure functions
  (presentation, `describeCommandError`, the lifecycle rules); the props App passes carry state — `notify`,
  `runAction`, `pending`, data, actions — never a function that is only an import with an argument pre-bound.
  The door's handle (`actions`, stable across renders) is such a value: list, row, inspector, rail and toolbar receive it
  and call `actions.run(verb, …)` themselves — no per-verb callback props. The one binding App does is a
  confirmation dialog's `onConfirm`, which joins the dialog's answer to the verb it confirms.

## Do not

- Never hand-edit version numbers (use `version:set`).
- Never special-case a tool by name in sync code. Copy-vs-symlink is decided only by the registry capability
  `supports_symlink` (`sync_dir_for_tool_with_overwrite`, `core/sync_engine.rs`); every current entry is
  `true` (Cursor was copy-only until Cursor 2.5 fixed `~/.cursor/skills` symlink discovery — flipped in v-next
  ticket 38; the flag is the revert lever if a tool regresses). A capability describes **one entry**: a virtual
  group's capability is its own registry fact, not the AND of its constituents (decision: v-next ticket 36).
- Never commit `.claude/`, `.agents/`, `.pi/`, `.mcp.json`, or `docs/conversation-logs/` (all gitignored). `.scratch/` **is**
  tracked; its `**/evidence/`, `**/worktrees/`, `*.log` and `*.diff` are not.
- Removal goes through `artifact_removal`: `sync_engine::remove_path_any` is called only from that module, from
  `onboarding_import`'s original-removal step (paired with `ensure_path_within_tool_dirs`), and inside
  `sync_engine`'s own overwrite-replacement path — never from `project_ops`, `project_sync` or a command.
- Confirmations go through the Dialog primitive (`src/components/ui/dialog.tsx` / `confirm-dialog.tsx`) —
  `useSharedDirConfirmation` owns the shared-skills-dir one;
  the app uses no native browser dialog.
- One implementation each of relative time, repo grouping and storage access: extend
  `skillPresentation.ts` / `preferences.ts` instead of adding a second formatter or touching
  `localStorage` by hand.
- Don't refactor, reformat, or "improve" code unrelated to the requested change.
- Git uses vendored-openssl and HTTP uses rustls-tls on purpose — don't switch to system SSL.

## Worktree safety

Parallel worktrees branched from a stale base can silently revert changes already merged to main.
This cost 18 features in v1.1.4 (`5e1f42e` "restore 18 features lost during parallel worktree merges").

- `main` is routinely behind `origin/main`: a nightly workflow (`update-featured-skills.yml`) commits
  `featured-skills.json` to origin. `git fetch && git rebase origin/main` before any push; if that file conflicts,
  take origin's.

- Before merging a worktree branch: `git rebase main`, then review `git diff main...HEAD` for unexpected
  deletions in files the worktree never meant to touch.
- Highest-risk shared files: `core/tool_adapters/mod.rs`, `core/installer.rs`, `core/project_sync.rs`,
  `core/artifact_removal.rs`, `core/refresh.rs`, `commands/mod.rs`, `commands/projects.rs`,
  `src/App.tsx`.
- After merging: `git diff <pre-merge-sha>..HEAD -- src-tauri/src src/` and confirm no unintended reverts,
  then run the full gate. A green build proves structural integrity, **not** feature completeness — spot-check
  functions other worktrees recently added.
- Resolving conflicts: for files this worktree did not intentionally modify, prefer the main version. Never
  take the worktree side wholesale without checking every hunk.

## Workflow

1. State the approach and the files you'll touch before writing code; wait for confirmation.
2. Implement full-stack changes in one pass — command + registration + DTO + i18n (EN) + UI.
3. Run the definition-of-done gate and fix failures before presenting results.

## Delegation (T3 Code `delegate_task`)

Run child agents on the **Pi harness** by default (`providerInstanceId: "pi"`) — it carries every model provider we
use. Model routing within Pi (IDs as `orchestrator_capabilities` lists them):

- **Fable** → `anthropic/claude-fable-5-1`.
- **Opus** → `anthropic/claude-opus-5-5` (operator decision 2026-09-25: 5.5 replaces 5 everywhere; the Pi option is
  `thinking`, lowest tier `low`). Round 19 (operator 2026-09-27): **Opus 5.5 runs `high` across the board** for
  implementation children.
- **GPT-6 Astra** → `openai/gpt-6-astra` (the OpenAI seat on multi-model review panels, adversarial reviews and
  audits; requires a live `openai` login in Pi — Sign in with ChatGPT; `openai-codex` is Pi's legacy provider, never use it).
- **GPT-6** (other uses) → `openai/gpt-6.1-sol` / `openai/gpt-6-luna` (operator 2026-09-27: the 5.6 family is
  retired; use 6). Luna is the cheapest model in the catalog: only for very basic, tightly-scoped tasks (e.g. a
  mechanical single-surface class migration), and always with high thinking.
- **Cortex** gateways: only when Alex asks for them.

The routing above is the default; confirm with Alex per session — it shifts with 5-hour and weekly usage limits.
Choose thinking per task: `low`/`medium` for routine work (research, scouting, implementation, reviewing small or
medium changes), `high` for reviewing large-scale work; ask if unsure. Adversarial review uses a different model
than the implementer.

Children read AGENTS.md's Workflow step 1 literally and stop for confirmation: **pre-approve it in the brief**
("state your approach and continue"). Tell them `.scratch/` is tracked and that `docs/agents/issue-tracker.md`
§ Delegated children applies: they may edit tickets and write handoffs, never archive, `git mv`, or commit under it.

Launch children with `runtimeMode: "full-access"` (or `inherit`) so they run unattended. Never use
`approval-required` for delegated work — it gates every child tool call on an operator prompt. Express
read-only/no-run constraints in the task brief instead.

## Deeper context (pointers)

- `README.md` — user-facing overview, supported-tools table, FAQ.
- `CHANGELOG.md` — most reliable record of dependency/stack changes.
- `docs/design-inputs/` — dated briefs derived from external research (upstream diffs, field surveys); BACKLOG items cite them by
  § as their source pointer. The evidence behind each brief lives in the Quartermaster repo (Plane `QM-58`); fix a wrong claim
  there and regenerate the brief, never the brief alone.

## Agent skills

### Issue tracker

Issues live as local markdown files under `.scratch/<effort>/`, tracked in git. **Start every session by reading
`.scratch/BACKLOG.md` — the only cross-effort queue — then the newest note (by filename date) in `.scratch/handoffs/`.**
Read `docs/agents/issue-tracker.md` § Lifecycle before scheduling a BACKLOG item, declaring an effort finished, archiving
anything under `.scratch/`, or adding an item that belongs to no effort.

### Handoffs

Write handoffs to `.scratch/handoffs/<YYYY-MM-DD>-<slug>.md` (overrides the handoff skill's temp-dir default — `/tmp` is
purged between sessions). A handoff points at `BACKLOG.md`, never embeds it.

### Triage labels

Default label vocabulary — role names used as-is (`needs-triage`, `needs-info`, …). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.
