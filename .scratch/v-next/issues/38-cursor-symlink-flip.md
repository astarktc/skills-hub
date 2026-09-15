# 38: Flip Cursor to `supports_symlink: true`

Status: resolved

Type: task
Blocked by: 36
Source: fog item "Is Cursor's `supports_symlink: false` obsolete?" — resolved by research `../assets/research-cursor-symlinks.md` (Opus 5, 2026-09-02)

## What to build

The reason encoded in the Cursor registry entry (`core/tool_adapters/mod.rs:191-198`) is obsolete:
Cursor IDE ≥ 2.5 (Feb 2026, staff-confirmed) and Cursor CLI (changelog 2026-06-22, "Skills found
through symlinks") both discover symlinked skill directories under `~/.cursor/skills`; project scope
never was broken. Latest datapoint IDE 3.15.19 (Aug 2026). Docs never mention symlinks, so this is
observed behaviour, not a documented contract — the `supports_symlink` capability stays as the
single revert lever (decision: ticket 36 kept the mechanism for exactly this).

Flip the entry to `true`, and update every place that states the old fact:

- `core/tool_adapters/mod.rs` — the entry + its comment; the field doc (`false forces copy mode …
  (Cursor)`) becomes tool-agnostic.
- `core/tests/tool_adapters.rs::only_cursor_lacks_symlink_support` → assert no registry entry lacks
  symlink support (the capability mechanism is still exercised by
  `core/tests/sync_engine.rs::any_adapter_without_symlink_support_is_copied`).
- `core/tests/sync_engine.rs::cursor_sync_forces_copy` — premise now false; delete (mechanism test
  above covers it).
- `AGENTS.md` do-not bullet — rewrite: the capability is the lever, never a `"cursor"` string check;
  group capability rule from ticket 36 stays.
- `CONTEXT.md` capability term — drop the Cursor example or past-tense it.
- `README.md` FAQ "Why is Cursor sync always copy?" — replace with the new fact + minimum version.
- `CHANGELOG.md` `[Unreleased]` → `### Changed`; note that already-synced Cursor copies are left in
  place until re-synced with overwrite (a non-overwrite sync reports the existing dir as a skip).

Unverified by any source: symlinked `SKILL.md` *files* (we symlink directories, so n/a) and
Windows junctions on Cursor. Noted in the README line as "Cursor ≥ 2.5".

## Acceptance criteria

- [ ] `rg "supports_symlink: false" src-tauri/src` → only the arbitrary literal in the
  `scan_tool_dir_skips_app_support_path` fixture (or none).
- [ ] No test or doc still claims Cursor cannot follow symlinks.
- [ ] `npm run version:check && npm run check` green.

## Answer

Commit `1224910` on `main` (orchestrator, no worktree).

- `core/tool_adapters/mod.rs`: Cursor entry → `supports_symlink: true`; field doc made tool-agnostic.
- Two things grew beyond the ticket, both forced by the flip:
  1. **Seven tests used Cursor as the copy-mode route.** No shipped entry is copy-only any more,
     so `tool_adapters::test_overrides::shadow(adapter)` (a `#[cfg(test)]`, thread-local,
     leaked-to-`'static` shadow consulted first by `adapter_by_key`) lets tests exercise a
     copy-only record without a fake registry entry. `project_sync` tests get a
     `copy_only_tool()` helper; `global_sync::cursor_gets_copy_mode` →
     `copy_only_capability_gets_copy_mode`.
  2. **Latent bug surfaced:** four "sync fails when the source is gone" tests only passed because
     copy mode fails on a missing source — symlink mode happily created a dangling link and
     recorded `Synced`. `sync_dir_hybrid_with_overwrite` now bails with
     `SignalError::InvalidPath { reason: "missing" }` before touching the target; new test
     `hybrid_sync_refuses_missing_source`. Those four tests are now mode-agnostic.
- `only_cursor_lacks_symlink_support` → `every_adapter_supports_symlink`; `cursor_sync_forces_copy`
  deleted (mechanism still pinned by `any_adapter_without_symlink_support_is_copied`).
- Docs: AGENTS.md do-not rewritten (capability is the lever, never a name check; group rule kept),
  CONTEXT.md capability term, README en + zh FAQ, CHANGELOG `[Unreleased]` (Changed + Fixed).
- Gate: 324 cargo (−1 +1) / 98 vitest; bindings unchanged.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
