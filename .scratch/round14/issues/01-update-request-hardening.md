# 01 Harden `UpdateRequest` and the byte adapters

Status: claimed
Lane: A
Source: BACKLOG #04 (from `archive/round7/backlog.md` #18)

`src-tauri/src/core/skill_update.rs`. Four sub-items, all backend, no wire change, no frontend:

1. **Collapse `UpdateBytes::{GitAcquired, RestoreRebuild}`.** Identical payload `{ staged, revision }` and identical
   handling (`:142–143` match both arms to the same tuple; `:264–271` choose between them on `restore`, which nothing
   downstream reads). One variant (e.g. `Acquired { staged, revision }`); `UpdateRequest::local` (`:76`) and
   `acquire_update` (`:263`) stop branching on `restore` for the variant. Keep the doc comment honest about Restore.
2. **Drop the `unreachable!()`** at `:145`. Restructure the match in `apply_unlocked` so `EditInPlace` and the staged
   variants are separated by the type system, not by a second match with a dead arm (e.g. match once on
   `request.bytes` with the staged arms calling a shared `settle_staged(store, &current, staged, revision)` helper).
3. **Private `UpdateRequest` fields + constructors.** `expected`, `record`, `bytes`, `repoint` become private. Callers
   today: `skill_edits.rs:114` builds a literal for Edit (`EditInPlace`, `repoint: false`, `expected == record`);
   `unlocatable.rs:102` and `commands/tests/mutation_results.rs:262` use `UpdateRequest::local`; `refresh.rs:170,241`
   and `core/tests/refresh.rs` use `acquire_update`. Add `UpdateRequest::edit(record, clear)` for the Edit path;
   `local` and `acquire_update` stay the other doors. `apply_unlocked` keeps reading the fields (same module). The
   round-7 note's `SourceProposal { ref, subpath, type }` idea: adopt it **only** if it removes the `record` clone
   without widening the surface — otherwise say why not in the ticket comment. Do not change admission semantics
   (`:118–127` stale check) — tests in `core/tests/skill_update.rs` and `core/tests/refresh.rs` pin them.
4. **Reuse `installer::ensure_installable_skill_dir`** (`installer.rs:189`, currently private) instead of the inlined
   `is_skill_dir` + `SkillInvalid { missing_skill_md }` at `:218–223`. Make it `pub(crate)`; same error token.

Read AGENTS.md **Update / Refresh** bullet and `docs/adr/0004-*` first. TDD where a behaviour could drift: a test
that the Restore path (central copy absent) still finalizes at the recorded path after the variant collapse, if none
pins it already (check `core/tests/skill_update.rs` and `core/tests/refresh.rs` before adding).

Do not touch `src/bindings/index.ts` (no DTO changes here — if `cargo test` regenerates it with another lane's diff,
leave it). Gate: `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all`.

## Done when

One staged variant; no `unreachable!`; `UpdateRequest` cannot be built by literal outside `skill_update.rs`;
`ensure_installable_skill_dir` is the single manifest gate for install and Re-point; `cargo test --all` green.

## Comments

### 2026-09-20 — lane A (implementation)

Done, all four sub-items, backend only, no wire change (`src/bindings/index.ts` unchanged after `cargo test --all`).

Files: `src-tauri/src/core/skill_update.rs`, `skill_edits.rs`, `installer.rs`, `tests/skill_update.rs`, `tests/refresh.rs`.
`unlocatable.rs`, `refresh.rs` and `commands/tests/mutation_results.rs` needed no change — they only use the
constructors, never a field.

1. `UpdateBytes::{GitAcquired, RestoreRebuild}` → one `Acquired { staged, revision }`. `UpdateRequest::local` and
   `acquire_update` no longer probe the central copy for a `restore` flag (`acquire_update` keeps
   `ensure_central_repo` — the staging dir still needs its parent). Enum doc says why Restore is not a fourth adapter.
2. `apply_unlocked` matches `request.bytes` once; `Acquired`/`LocalFolder` call a private `settle_staged(store,
   &current, staged, revision)` (finalize + Edit replay, ADR-0004). No `unreachable!`.
3. `UpdateRequest` fields are private; doors are `local`, new `edit(record, clear)` (used by `skill_edits.rs`), and
   `acquire_update`. **`SourceProposal` judgement call — adopted**, as a module-private
   `struct SourceProposal { source_ref, source_subpath, source_type }` replacing `record: SkillRecord`:
   `apply_unlocked` reads exactly those three fields, so the type now says what the doc comment used to promise
   ("carries only proposed source changes") — apply *cannot* overwrite an unrelated field. It removes the full
   `record.clone()` from all three constructors (`expected` now takes `record` by move; the proposal clones three
   strings via `SourceProposal::unchanged(&record)`), and it widens nothing: no new `pub(crate)` item, the constructor
   signatures are unchanged, and tests reach it as the child module they already are (`acquired.record.source_*` →
   `acquired.proposal.source_*`). Admission (`expected` stale check) untouched.
4. `installer::ensure_installable_skill_dir` is `pub(crate)` and is the Re-point manifest gate in `acquire_update`
   (same `SkillInvalid { reason: "missing_skill_md" }` token).

Tests (TDD): added `local_restore_rebuilds_the_central_copy_at_the_recorded_path` (`tests/skill_update.rs`) — pins
the `UpdateRequest::local` Restore door whose branch was removed; verified green *before* the refactor and after.
The git side was already pinned by `git_restore_rebuilds_the_central_copy_and_its_dangling_link`. Tightened
`git_repoint_non_skill_directory_never_replaces_a_working_skill` (`tests/refresh.rs`) from `SkillInvalid { .. }`
to the `missing_skill_md` token so the shared gate is asserted, not assumed. `every_byte_adapter_settles_and_reports_propagation`
still covers `git`/`local`/`edit`/`restore` (restore now = same `Acquired` request with the central copy removed).

Evidence: `cargo clippy --all-targets --all-features -- -D warnings` clean; `cargo test --all` → 646 passed,
0 failed (was 645 + 1 new); `cargo test skill_update` → 11 passed. `rustfmt --check` clean on the five edited files
(whole-crate `cargo fmt` left to the parent). `lens_diagnostics`: the only blocker is yamllint on
`.github/workflows/auto-tag.yml` (lane B's file).
