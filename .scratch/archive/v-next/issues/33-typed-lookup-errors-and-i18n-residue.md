# 33: Type the remaining prose lookup errors; close the i18n residue

Status: resolved

Type: task
Source: review #3 H1, H2, J9 (Opus; verified) — [verdict](../assets/review-3/verdict.md)

## What to build

### Prose error conditions still cross the wire

ADR 0001 / AGENTS.md: "never encode error conditions in message strings"; `CommandError::Other { message }` is the safety valve, and after ticket 22 deleted the `describeOther` sniff, `src/commandError.ts:110` returns `e.message` verbatim — English-only prose reaching a ZH user. Sites (all in core, several beside a typed `SignalError::NotFound` for the same lookup):

| Site | Message | Note |
|---|---|---|
| `core/project_ops.rs:199` | `bail!("unknown tool: {}")` | **new in batch** (t28 `configure_project_tools`) |
| `core/project_sync.rs:41, 252, 462` | `unknown tool: …` | pre-existing shape, moved by t25 |
| `core/project_ops.rs:112, 237, 333` | `project not found: …` | t13 typed 7 sites; these 3 survived / were moved |
| `core/project_sync.rs:276` | `project not found: …` | |
| `core/project_sync.rs:250` | `skill not found: …` | |
| `core/gitignore.rs:226` | `project directory does not exist: …` | 8 lines below a typed `NotFound` |
| `core/project_ops.rs:90, 318` | `path is not a directory: …` | |

Decide the variant set — probably `NotFound` (existing; check its fields cover project/skill), a new `UnknownTool { tool }`, and either `NotFound`/`InvalidPath { path }` for the directory cases. Per the ADR checklist: Rust variant + `cargo test` (regenerates binding) + `COMMAND_ERROR_CODE_MAP` + `describeCommandError` branch + EN/ZH keys. Also sweep `commands/` and other `core/` modules for any `anyhow!("... not found")` / `bail!("unknown ...")` conditions a caller could hit; leave genuine internal invariants as prose.

### i18n

- `errors.skillNotFoundInRepo` exists only in `en` (`src/i18n/resources.ts:269`); add the `zh` key. Pre-existing, but t29 rewrote its only emitter (`useAddSkillFlow.ts:474`).

### Test fidelity

- `useProjectState.test.ts:249,258` rejects with `{ code: "INTERNAL" }` — not a `CommandError` code (`CommandError::internal` yields `OTHER`). Use a real code so the convergence path exercises the wire contract; consider typing test fixtures as `CommandError` so this fails the build next time.

## Acceptance criteria

- [ ] Zero user-reachable `anyhow!`/`bail!` prose conditions for not-found / unknown-tool / bad-path in `core/project_ops.rs`, `core/project_sync.rs`, `core/gitignore.rs`; each is a `SignalError` variant with a `describeCommandError` branch and EN+ZH copy.
- [ ] Every `en.errors.*` key has a `zh` twin (add a vitest that diffs the two key sets so this is enforced).
- [ ] `useProjectState.test.ts` fixtures use real `CommandError` codes.
- [ ] `src/bindings/index.ts` regenerated and committed; `npm run version:check && npm run check` green.

## Answer

Branch `t33` (worktree `~/.worktrees/skills-hub-t33`), commit `36f08dd`.

### Variant decisions

Two new variants, both `SignalError` + `CommandError` (identical shape), plus reuse of the
existing `NotFound { kind, id }`:

- `UnknownTool { tool }` — a tool key that matched no `TOOL_ADAPTERS` entry. Its own variant
  rather than `NotFound { kind: "tool" }` because the frontend copy differs (a registry-key
  mismatch is a bug/stale-config signal, not a deleted row) and the field is the key itself.
- `InvalidPath { path, reason }` — one variant for both bad-path classes, with `reason` a
  machine token the frontend localizes (`missing`, `not_a_directory`), following the
  `SkillInvalid { reason }` precedent. Avoids two near-identical variants and leaves room for
  future path refusals without another wire change.
- `NotFound { kind, id }` reused unchanged for every project/skill lookup — its fields already
  cover them, and 4 sites in these files already used it (the prose sites were survivors).

### Sites changed

| File | Was | Now |
|---|---|---|
| `core/project_ops.rs` (112, 237, 333) | `project not found` | `NotFound{project}` |
| `core/project_ops.rs` (90, 318) | `path is not a directory` | `InvalidPath{not_a_directory}` |
| `core/project_ops.rs` (199) | `unknown tool` | `UnknownTool` |
| `core/project_sync.rs` (41, 252, 462) | `unknown tool` | `UnknownTool` |
| `core/project_sync.rs` (250) | `skill not found` | `NotFound{skill}` |
| `core/project_sync.rs` (276) | `project not found` | `NotFound{project}` |
| `core/gitignore.rs` (226) | `project directory does not exist` | `InvalidPath{missing}` |
| `commands/projects.rs` (176) | `skill not found` (sweep) | `NotFound{skill}` |
| `core/installer.rs` (342) | `skill not found` (sweep, update flow) | `NotFound{skill}` |
| `core/skill_store.rs` (721) | `project not found` (sweep, `update_project_path`) | `NotFound{project}` |

Left as prose deliberately: `commands/mod.rs:658` (`path is not under a known tool skills
directory`) is a safety refusal for a path the UI sources from its own listings — an internal
invariant, not a lookup; `core/global_sync.rs:327` unknown-tool becomes per-target *report*
data (`Failed` outcome text), never a `CommandError`, so typing it buys nothing; `installer.rs`
`source path not found` / `central path not found` / `subpath not found in repo` are
staging-internal invariants behind already-validated input.

### Frontend

`COMMAND_ERROR_CODE_MAP` gained `UNKNOWN_TOOL`/`INVALID_PATH` (compiler-derived from the
regenerated union), `describeCommandError` gained both branches with an
`INVALID_PATH_KEYS` reason→key table and a generic fallback; EN+ZH copy for
`errors.unknownTool`, `errors.invalidPathMissing`, `errors.invalidPathNotADirectory`,
`errors.invalidPath`. `src/bindings/index.ts` regenerated by `cargo test` and committed.

### i18n parity

The gap was wider than `errors.skillNotFoundInRepo`: `zh` was missing 61 key paths — that one
error key, 6 top-level sync keys, and the entire `projects` namespace (54 keys), which carried
an explicit `// projects: deferred to future phase (English fallback active via i18next)`
comment. Rather than weaken the guard to `errors.*`, all 61 were translated into Chinese and
the deferral comment removed. New `src/i18n/resources.test.ts` asserts full deep-key-path
parity in both directions (plus a spot-check that the ticket's new error keys exist in both).

### Test fidelity

`useProjectState.test.ts` fixtures are now declared `const … : CommandError` (with
`CommandError` re-exported from `components/projects/types.ts` per the shim rule), so a
fabricated code fails `npm run build`; the fake `{ code: "INTERNAL" }` became a real
`{ code: "INVALID_PATH", path, reason: "missing" }` — exactly what the backend now raises when
a project dir vanishes between registration and the ignore write.

### Test counts / gate

- Rust: 317 → **321** passing (`cargo test --all`). New: `from_anyhow_recovers_unknown_tool_and_invalid_path_through_context`,
  `assign_and_sync_raises_typed_unknown_tool`, `resync_project_raises_typed_not_found_for_unknown_project`,
  `update_project_path_raises_typed_not_found_for_unknown_project`. Converted from
  message-substring to typed downcast: `register_rejects_non_dir`,
  `project_update_rejects_a_project_whose_directory_is_gone`,
  `configure_tools_rejects_unknown_tool_before_writing_anything`,
  `assign_skill_to_project_tool` unknown-tool case.
- Frontend: **93 passing / 9 files** (`npm run test`), including 3 new `commandError.test.ts`
  cases and 2 new `resources.test.ts` cases.
- `npm run version:check` → Version OK (1.1.9); `npm run check` (lint + vitest + build +
  rustfmt + clippy -D warnings + cargo test) green; `git status` clean.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
