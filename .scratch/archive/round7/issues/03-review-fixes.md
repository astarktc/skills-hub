# 03: Round-7 review fixes (panel: Fable 5.1 / Opus 5 / Astra — reports `r7-review-*.md`)

Status: done — fb71b1b

**Blocked by:** 02 (done, `a261e89`).  **Files:** `src-tauri/src/core/{frontmatter_edit,skill_edits,skill_catalog,errors,skill_discovery}.rs`,
`src-tauri/src/core/tests/{frontmatter_edit,skill_edits}.rs`, `src-tauri/src/commands/{mod,error}.rs`, `src/commandError.ts`,
`src/hooks/useSkillLibrary.ts` (+ test), `src/i18n/resources.ts` (en + zh), `src/components/skills/types.ts`. Bindings regenerate via `cargo test`.
**Do NOT touch:** `installer.rs`, `refresh.rs`, `install_finalize.rs`, `skill_discovery.rs` beyond an `as_key`/`from_key` pair on `InvocationMode`.

## Blocking (data safety — all three reviewers converge on B1/B2)

### B1 — persist the edit row BEFORE rewriting bytes (Astra P2, Fable F1, Opus F3)
`skill_edits.rs::set_invocation_override` writes `SKILL.md` via `apply_to_file` and only then `upsert_skill_edit`; `replay_unlocked`
likewise writes bytes, then `upsert_skill_edit`, then `record_hash`. If the row write fails, bytes carry the override with no (or a
stale) `base_value`: a retry captures edited bytes as "upstream" and Clear can never restore the original (E5). Reorder both paths:
**capture base → upsert edit row (conflict/base_value/applied_at as computed) → write bytes → recompute hash + upsert skill row.**
A row without matching bytes self-heals on the next replay (the replay rewrites from the row); the reverse never does. On the
`None` (clear) path: **restore bytes → delete row → hash** is already the safe order (a row surviving a failed restore is replayed
again next Update) — keep it, but state the invariant in the module doc: *the edit row is the source of truth; bytes follow it.*
Tests: (a) inject a failure on the bytes side (make `SKILL.md`'s parent dir read-only on Unix, `#[cfg(unix)]`) after the row is
written → `set` returns Err, row exists with the correct `base_value`, bytes untouched, and the next `replay_unlocked` lands the
override; (b) if the store exposes a cheap fault seam (or one can be added test-only, e.g. `PRAGMA query_only = 1` via a
`#[cfg(test)]` helper), assert a failed upsert leaves bytes untouched; otherwise document why not in Comments.

### B2 — atomic manifest write (Astra P1, Fable F2)
`frontmatter_edit::apply_to_file` uses `std::fs::write` in place (truncate then write): ENOSPC / a crash mid-write leaves an empty
or partial `SKILL.md`, unrecoverable from the two-key base. Write to a sibling temp file (`.SKILL.md.skills-hub-<uuid>` or similar
hidden name — check `is_hidden_dir_name`/discovery ignore rules so a leftover is never scanned as content, and `IGNORE_NAMES` in
`content_hash.rs` so it never hashes), `sync_all`, then `rename` over the original; remove the temp on any failure. Test: a
rename-target failure leaves the original bytes intact and no temp file behind (Unix: read-only parent dir prevents rename —
combine with B1's test fixture).

### B3 — typed error for manifest I/O failures (Astra S1)
A read-only `SKILL.md`, permission-denied parent, or non-UTF-8 upstream bytes are operator/upstream-causable and today surface as
`OTHER` (AGENTS.md error contract). Add `SignalError::SkillManifestIo { path: String, detail: String }` (raise it from
`apply_to_file`'s read/write/rename and from the `read_to_string` in `set`/`replay`; keep the io error as the anyhow source chain),
the `CommandError` arm (`commands/error.rs`, internally tagged), a `describeCommandError` branch, and EN + ZH keys (message names
the path; `detail` is diagnostics, not copy). Follow the `PATH_OUTSIDE_TOOL_DIRS` worked example. Unit test the downcast like
`rollback_rename_failure_names_and_preserves_backup` does.

## Also (small, in scope)
- A1 (Fable S3, Opus F1): add `skill_catalog::managed_skill_entry(store, id) -> Result<Option<ManagedSkillEntry>>` sharing the
  entry-building with `managed_skill_catalog` (one private `build_entry`), and use it in `set_invocation_override` so a Save no longer
  builds every skill's entry under the mutation guard. This also breaks the `skill_edits` ↔ `skill_catalog` import cycle only if you
  move `invocation_override` reading into the catalog's `build_entry` via the store directly — do that if it is a one-liner, else leave.
- A2 (Fable spec-c #1): the Refresh/Update report should carry `edit_conflict` only when the conflict is **newly detected** in that
  replay (`upstream_mode != base_mode` this time), not on every later unchanged Update; the persisted `conflict` flag (badge/modal
  state) is unchanged. Adjust the test that asserts persistence so it asserts the flag persists AND the second report carries `None`.
- A3 (Opus F6): give the single-Update success toast its own title key (`invocationEdit.updateCompletedWithConflict` or similar)
  instead of reusing `invocationEdit.warningTitle`; EN + ZH.
- A4 (Fable S4): `useSkillLibrary.runSingleUpdate` — replace the mutable `let hasEditConflict = false` captured by the toast thunk
  with the report value flowing through the return (mirror `handleRefresh`'s shape). Hook test still green.
- A5 (Fable S1, Opus S1): one DTO policy — drop `InvocationOverrideDto` and put `core::skill_edits::InvocationOverride` on the wire
  directly (it already derives `specta::Type`), matching how `InvocationEditConflict` crosses. Re-export from the `types.ts` shim.
- A6 (Fable S2, Opus S3): `InvocationMode::as_key()` / `from_key()` (kebab strings, one table) replacing the `serde_json` round trips
  in `skill_edits.rs`; keep the `Deserialize` derive for the command input.
- A7 (Opus S2): `frontmatter_edit.rs` — a two-variant `Key` enum instead of `usize` indexes into `KEYS`/`found`/`values`.
- A8 (Opus F4): `restore_invocation_lines` removes the whole block when `base.had_frontmatter` is false — gate removal on the block
  containing only the writer's two keys; otherwise just remove the key lines. Add the corpus case.
- A9 (Fable S5/S8): drop the redundant `every_mode_round_trips_without_frontmatter` test; drop the unused `InvocationEditConflict`
  re-export from `types.ts` unless A5 makes it used.

## NOT in scope (append to `.scratch/round7/backlog.md` as items 10–11)
- Opus F2: `header_end` treats an indented `---` inside a block scalar as the closing fence — inherited from `parse_invocation_mode` /
  `parse_skill_md_with_reason`; must change all three together.
- Fable spec-c #2: upstream converging *to* the override's mode still flags a conflict (spec-literal; could auto-resolve).

## Gate
`npm run version:check && npm run check` green, `src/bindings/index.ts` regenerated and not dirty. Conventional commits on
`r7/skill-edit-v1` (one per B-item preferred, one for the A-items is fine). Do NOT push/merge/rebase. Do NOT run `npm run tauri:dev`.
`.scratch/` is gitignored — never `git add -f`. Paste `## Comments` in the final message (what changed, deviations, the B1 fault-seam
decision, gate counts).

## Comments

- 2026-09-15 — Status reconciliation: ready-for-agent → done — fb71b1b. Evidence: git log v1.2.6..v1.2.7 finds fb71b1b; src-tauri/src/core/skill_edits.rs:207 persists the row before the write, and src-tauri/src/core/manifest.rs:441–443 syncs the temp then renames it. Manifest now owns the former frontmatter writer.
