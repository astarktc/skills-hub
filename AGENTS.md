# Skills Hub — agent context

Tauri 2 + React 19 desktop app that installs AI Agent Skills once and syncs them to many AI coding tools.
Canonical agent context for every harness. `CLAUDE.md` imports this file.

## Commands

```bash
npm run dev              # Vite dev server (port 5173, strict)
npm run tauri:dev        # Tauri dev window (frontend + backend) — see live-data warning below
npm run build            # node node_modules/typescript-7/lib/tsc.js -b && vite build
npm run lint             # ESLint
npm run check            # lint + build + rust:fmt:check + rust:clippy + rust:test
npm run version:check    # verify the 3 version files are in sync
npm run version:set X.Y.Z   # bump all 3 version files (never hand-edit)

cd src-tauri && cargo test <filter>              # single test / module by name substring
cd src-tauri && cargo test <filter> -- --nocapture   # show println!/dbg! output
```

**Two TypeScript compilers are installed**: `typescript` (~6.0.3) and `typescript-7` (`npm:typescript@^7.0.1-rc`).
`npm run build` uses **typescript-7** explicitly. A bare `npx tsc --noEmit` type-checks with the *wrong*
compiler and can disagree with the build — always type-check via `npm run build`.

`npm run rust:test` runs `cargo test`; CI runs `cargo test --all` (includes all workspace targets).
Prefer `--all` locally when touching Rust to match the gate.

## Definition of done

```bash
npm run version:check && npm run check
```

`npm run check` **omits the version gate** that CI enforces (`.github/workflows/ci.yml`), so run both.
A version desync has shipped before (commit `f98bf9b`, "sync Cargo.toml version to 1.1.7").

## Environment gotchas

- **Dev runs mutate the operator's REAL skill library — there is no sandbox.** Dev and release builds share
  both the central repo (`~/.skillshub`, `core/central_repo.rs`) and the app database
  (`~/Library/Application Support/com.skillshub.app/skills_hub.db`, from `identifier` in `tauri.conf.json`).
  Any sync/install/delete action in `npm run tauri:dev` writes the operator's live global skills and
  creates/removes real symlinks under `~/.claude/skills`, `~/.pi/agent/skills`, etc.
  Do destructive testing only with explicit permission; the `central_repo_path` setting can repoint the
  central repo, but the database path is not overridable.
- `.claude/skills/` and `.agents/skills/` are **hardlinked to each other** (same inodes) and gitignored —
  editing a skill file in one silently edits the other.
- Rust source carries **Chinese comments** in places (upstream fork heritage); this is expected, not corruption.
- The Rust crate is `app_lib`, not the package name — import as `app_lib::...`.

## Invariants — touch X, then also update Y

- **Version**: `package.json` + `src-tauri/tauri.conf.json` + `src-tauri/Cargo.toml` must match. Use
  `npm run version:set X.Y.Z` (`scripts/version.mjs`); never edit a version field by hand.
- **New Tauri command**: define it in `src-tauri/src/commands/` (`mod.rs` or `projects.rs`) **and** register it
  in `src-tauri/src/lib.rs` under `generate_handler!`. Unregistered commands fail only at runtime.
- **DTO changes**: `src/components/skills/types.ts` mirrors the Rust DTOs in `commands/`. Update both sides.
- **New AI tool adapter**: add the `ToolId` variant and the `default_tool_adapters()` entry in
  `core/tool_adapters/mod.rs` (plus the `project_relative_skills_dir()` arm), **and** add a row to the
  README supported-tools table. The Rust arms are compiler-enforced; the README table is not — check it
  matches the `ToolId` variant count whenever adapters change.
- **UI strings**: add keys to **both** `en` and `zh` in `src/i18n/resources.ts`. No hardcoded UI text.
- **New `core/` module**: declare it in `src-tauri/src/core/mod.rs`.
- **DB schema change**: consider `migrate_legacy_db_if_needed` in `core/skill_store.rs` — a migration path exists.

## Ambiguity resolution

- **No state-management library** (no Zustand/Redux/Context). State lives in plain `useState` inside
  per-world hooks (`src/hooks/`, `components/projects/useProjectState.ts`), each returning that world's
  data + actions; `src/App.tsx` is the binder that composes the hooks and passes props down. Cross-world
  needs flow through interfaces App passes into hooks (e.g. the shared `useStatusReporter` surface, the
  sync seam) — hooks never import each other. Refresh data by re-invoking the relevant command
  (e.g. `invoke('get_managed_skills')`) after a mutation.
- **`commands/` is wiring only** (DTO conversion, error formatting); business logic goes in `core/`, which is
  independently testable. Async commands wrap sync work in `tauri::async_runtime::spawn_blocking`.
- **Error wire contract** — commands return `Result<T, CommandError>` (`commands/error.rs`), a serde
  internally-tagged enum (`{ code: "TOOL_NOT_WRITABLE", tool, path }`). Core raises typed conditions
  as `SignalError` (`core/errors.rs`) through `anyhow` chains; `CommandError::from_anyhow` recovers
  them by downcast at the command seam — never encode error conditions in message strings. The TS
  mirror is generated by ts-rs into `src/bindings/` on `cargo test` (commit it; CI diff-guards it).
  All user-facing error copy lives in the frontend's `describeCommandError` (`src/commandError.ts`).
  Adding a variant: Rust variant + regenerated binding + a `describeCommandError` branch + i18n keys.
  See `docs/adr/0001-tagged-command-error-contract.md`.
- **`src/Layout.tsx` and `src/Dashboard.tsx` are dormant by design** — not imported by `main.tsx` or `App.tsx`.
  Leave them alone unless explicitly wiring them up.
- Styles live in `src/App.css` / `src/index.css`. There are no CSS Modules — don't add the pattern.
- Path handling must support `~` expansion (`expand_home_path()` in the backend).
- Sync uses a triple fallback: symlink → junction (Windows) → copy.
- **Global sync fan-out is backend-owned.** `sync_skills_to_tools` (one batch command; core engine in
  `core/global_sync.rs`) handles installedness filtering, shared-dir dedupe, overwrite policy (batch
  default + per-(skill,tool) overrides), DB record fan-out, and per-target results; progress streams
  over a Tauri `Channel`. There is no per-pair sync command — never loop one in the frontend. Per-target
  failures/skips are report data (`SyncTargetStatusDto`), not command errors. The shared-skills-dir
  grouping reaches the UI only as `ToolInfoDto.shared_with`; do not re-derive it from `skills_dir`.

## Do not

- Never hand-edit version numbers (use `version:set`).
- Never "fix" the Cursor adapter to use symlinks — Cursor does not support symlink/junction skill dirs, so
  `sync_dir_for_tool_with_overwrite` forces copy mode for it deliberately (`core/sync_engine.rs`).
- Never commit `.claude/`, `.agents/`, `.gsd/`, `.mcp.json`, or `docs/conversation-logs/` (all gitignored).
- Don't refactor, reformat, or "improve" code unrelated to the requested change.
- Git uses vendored-openssl and HTTP uses rustls-tls on purpose — don't switch to system SSL.

## Worktree safety

Parallel worktrees branched from a stale base can silently revert changes already merged to main.
This cost 18 features in v1.1.4 (`5e1f42e` "restore 18 features lost during parallel worktree merges").

- Before merging a worktree branch: `git rebase main`, then review `git diff main...HEAD` for unexpected
  deletions in files the worktree never meant to touch.
- Highest-risk shared files: `core/tool_adapters/mod.rs`, `core/installer.rs`, `core/project_sync.rs`,
  `commands/mod.rs`, `src/App.tsx`.
- After merging: `git diff <pre-merge-sha>..HEAD -- src-tauri/src src/` and confirm no unintended reverts,
  then run the full gate. A green build proves structural integrity, **not** feature completeness — spot-check
  functions other worktrees recently added.
- Resolving conflicts: for files this worktree did not intentionally modify, prefer the main version. Never
  take the worktree side wholesale without checking every hunk.

## Workflow

1. State the approach and the files you'll touch before writing code; wait for confirmation.
2. Implement full-stack changes in one pass — command + registration + DTO + i18n (EN & ZH) + UI.
3. Run the definition-of-done gate and fix failures before presenting results.

## Deeper context (pointers)

- `README.md` — user-facing overview, supported-tools table, FAQ.
- `CHANGELOG.md` — most reliable record of dependency/stack changes.

## Agent skills

### Issue tracker

Issues live as local markdown files under `.scratch/<feature>/` in this repo. See `docs/agents/issue-tracker.md`.

### Triage labels

Default label vocabulary — role names used as-is (`needs-triage`, `needs-info`, …). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.
