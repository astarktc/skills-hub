# 01 Harden `UpdateRequest` and the byte adapters

Status: ready-for-agent
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
