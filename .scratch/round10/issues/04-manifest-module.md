# 04: Wave B2 — one Manifest module for reads and byte-preserving writes

Status: resolved

**What to build:** Skill metadata, invocation-mode reads and Edit writes share one Manifest implementation, so an Edit preserves unrelated bytes and parsing agrees with what the existing skill detail view displays. This prepares Edit V2; it does not add an Edit V2 feature.

**Blocked by:** None (can start immediately from integrated wave A). Parallel with B1 (#03) and B3 (#05); parent isolates worktrees and owns final integration/review.

**Completed:** integrated on `main`; final reviewed candidate `23458bb`. Includes pre-upgrade indented-header Edit compatibility and independent corpus expectations found in review. Evidence: `.scratch/round10/b2-implementation-report.md`, `review-fixes-report.md`, `r10b-review-opus-recheck.md`. v1.2.12 prepared, not published.

**Release / lane:** v1.2.12 / B2 (#5). Source verified on `main` at `a824caf45fffb0bd5c38318ba10b2ffa0ca11c62`. Implementer: Pi `openai-codex/gpt-6-astra`, high. Review: one model seat, Anthropic Opus 5 high replacing exhausted Fable; parent resolves the exact model ID.

## Read first / working contract

Read `AGENTS.md`, `CONTEXT.md`, ADR-0001/0003/0004, `.scratch/round10/decisions.md` Q9/Q21, wave-A tickets 01/02 as history, and `.scratch/round7/backlog.md` 10/16. Use `implement`, `codebase-design`, `pi-lens-lsp-navigation`, `tdd`, and React best-practices for the narrow frontend follow-up. State your approach and continue: pre-approved, no confirmation pause. Search only this checkout. No app/dev runs, live library/DB access, children, release edits or commits unless separately authorized. Tests use temporary files/roots/stores. `.scratch/` is gitignored; NEVER `git add -f`.

If the ticket is factually wrong about the code, say so in your report and adapt.

Prefer targeted edits and re-read after writing.

## Problem / verified evidence

- `core/skill_discovery.rs` owns discovery **and** `header_end`, `parse_skill_md[_with_reason]`, invocation mode type/read/parse and scalar helpers. `header_end` checks the opening line with `trim()` but the closing line with `trim_end() == "---"`. The existing closing-fence fix correctly ignores indented `---` in block scalars; the opening rule is still inconsistent.
- `core/frontmatter_edit.rs` imports discovery's parser/fence helper and owns `InvocationLines`, `read_invocation_lines`, `write_invocation_mode`, `restore_invocation_lines`, `apply_to_file` and `read_manifest`. It is already a byte-preserving writer: same-mode edits are no-ops, unrelated lines stay intact, clear restores recorded lines, and atomic rename preserves permissions. This is a consolidation, not permission to replace it with whole-document YAML serialization.
- `core/skill_edits.rs` reads/writes through `frontmatter_edit`; serialized `InvocationLines` is persisted in `SkillEditRecord.base_value`. `replay_unlocked` updates the Edit row before bytes, and rollback is paired with finalize per ADR-0004. These persisted field spellings/semantics must survive the module move.
- `core/install_finalize.rs::read_skill_md_meta` currently composes `find_skill_md` and `parse_skill_md` and returns `(None, None)` on unusable metadata. Both `finalize_install` and `finalize_update` call it.
- **Correction to Q21 shorthand:** `core/skill_lock.rs::parse_lock_file` reads `.skill-lock.json` with `read_to_string`, then serde JSON; it does not parse SKILL.md or invocation frontmatter. `try_enrich_from_skill_lock_with_home` performs the symlink/home-fenced provenance lookup. Route its read through Manifest as requested, but keep the JSON schema/provenance logic and its missing/unreadable/malformed → `None` policy in `skill_lock`. Do not invent SKILL.md parsing in that module or make a lock-file failure a user-visible command failure.
- `core/skill_catalog.rs::build_entry` calls `invocation_mode_for_dir`; absent/unreadable manifest falls back to default mode. Catalog target-query failures still fail the catalog.
- **Fourth fence correction:** `src/components/skills/SkillDetailView.tsx::parseFrontmatter` currently uses `raw.startsWith("---")` and `raw.indexOf("\n---", 3)`, not `trim()`. It accepts delimiter prefixes such as `---suffix`; its offsets assume LF, and it has its own scalar parsing. The backlog's opening-`trim()` note describes Rust, not this TS function.

## Settled decisions / implementation

1. **Q9/Q21: `core/manifest.rs`, reads AND byte-preserving writes.** Move manifest grammar/metadata/invocation parsing and the writer's implementation into this deep module. Its interface owns reading a manifest, reading metadata/mode, capturing/restoring invocation lines and applying a byte-preserving invocation edit; implementation details such as fences/key scanning/I/O mapping are private. Keep directory discovery/admission and scan ladders in `skill_discovery`, Edit orchestration/row settlement in `skill_edits`, and final-name/atomic settlement in `install_finalize`.
2. `skill_discovery` becomes a Manifest consumer. Thin compatibility re-exports for existing parser/type imports are permitted to avoid cross-lane churn; they must contain no second parser/policy. `install_finalize::read_skill_md_meta` reads via Manifest. `skill_lock::parse_lock_file` uses the shared read half (a small text-read adapter, or a manifest-owned lock read if the interface needs it); JSON decoding, source derivation and optional enrichment remain owned by `skill_lock`. Do not create a generic file-service abstraction or silently change error policy to justify this historical wording.
3. Fold `frontmatter_edit.rs` into Manifest as the write half: migrate tests rather than duplicating the old implementation. Preserve serialized `InvocationLines` fields, duplicate-key restoration, comments/spacing/newlines/body, no-op identity, absent/empty/unfinished frontmatter behavior, permission preservation and `.skills-hub-manifest-*` cleanup/ignore behavior. No new schema or arbitrary-edit facility. Future Edit kinds should reuse the module, not be implemented now.
4. Finish the existing fence follow-up: recognize opening and closing fences as complete column-zero `---` lines (trailing whitespace/CRLF tolerated), not indented lines or delimiter prefixes. Indented `---` stays scalar content; later body fences stay body. Characterize existing behavior first and explicitly isolate deliberate opening/prefix/CRLF fixes from the behavior-preserving extraction. Do not broaden the supported YAML language or change metadata fallback/name policy as incidental cleanup.
5. Move the frontend's existing pure `parseFrontmatter` presentation function into a small testable pure helper (e.g. `src/lib/manifestPresentation.ts`) and make `SkillDetailView` import it. Align delimiter recognition with the backend using the same literal corpus; no new IPC call, visual feature, state hook or JSX tests. Preserve existing metadata/body rendering outside these fence fixes. Rust cannot be imported into TS: this is a tested cross-language presentation adapter, not a claim that the UI shares Rust execution.
6. Keep typed `SKILL_MANIFEST_IO` on required Edit reads/writes (including invalid UTF-8), optional metadata/default-mode/lock reads permissive as they are today. Keep ADR-0004 replay/rollback semantics and ADR-0003 provenance intact.

## Ownership / explicit do-not-touch

Rust paths below are relative to `src-tauri/src/`.

- **Own:** new `core/manifest.rs`, new/migrated `core/tests/manifest.rs`, retired `core/frontmatter_edit.rs` and `core/tests/frontmatter_edit.rs`; manifest parsing/type/helper portion of `core/skill_discovery.rs` and corresponding parser tests in `core/tests/skill_discovery.rs`.
- **Narrow allowances:** `core/mod.rs` add Manifest/remove old writer declaration; `core/install_finalize.rs` metadata imports/read helper only; `core/skill_lock.rs::parse_lock_file` read adapter only and its read-policy tests; `core/skill_edits.rs` Manifest imports/calls only, not outcome/guard/settlement logic; `core/skill_catalog.rs` invocation type/read imports only. `core/tests/skill_edits.rs` import renames and Manifest compatibility/regression assertions only, preserving propagation/admission fixtures.
- **Consumer allowances:** parser/type import or qualification updates in `installer.rs`, `onboarding.rs`, `commands/mod.rs` and directly affected tests, only when compatibility re-exports cannot keep consumers stable. `src/bindings/index.ts` generator-only if moving the Rust type changes output; `InvocationMode` wire spelling must not change. Keep these imports narrow and report every extra file.
- **Frontend own:** `src/components/skills/SkillDetailView.tsx::parseFrontmatter` extraction/import and the new pure helper + `.test.ts`. No other component behavior/style changes. No `useAddSkillFlow`, `useSkillLibrary` or report folds.
- **B1 owns:** git intent/resolution/candidate policy, preview publication, git sections of `installer.rs`/its tests, the two `useAddSkillFlow` resolution arguments, and the `GitSourceResolution` shim export. Do not change discovery traversal/admission to satisfy Manifest tests. B1 may touch installer imports; use compatibility exports and parent symbol-level reconciliation.
- **B3 owns:** mutation result DTOs/commands/catalog assembly, Edit return shape, `useSkillLibrary`, `reportOutcome`, mutation tests and i18n. In `skill_edits` B2 changes only Manifest wiring, B3 only return adaptation if needed. Prefer B3's envelope assembly outside Edit's guarded body to avoid overlap.
- **Inevitable overlap:** `core/mod.rs` declaration lines, `commands/mod.rs` type imports, `installer.rs` imports and tests, `skill_edits.rs` imports/calls/tests, generator output. Parent must rebase/review shared hunks and regenerate bindings; never accept a whole shared file from a worktree.
- **Excluded:** Edit V2 implementation, new YAML library/grammar, content-identity policy and warn-dedupe cleanup, broad `UpdateRequest` cleanup, malformed `global_selected_tools`, round11 C3/C4, wave C #7 report unification, #9, versions/changelog/domain/context/ADR edits and unrelated module cleanup. Parent adds **Manifest** to `CONTEXT.md` and updates invariants separately.

## Acceptance

- [x] One Manifest implementation owns metadata/mode reads and invocation byte edits; discovery, finalize and Edit consume it without a parallel parser/writer.
- [x] `skill_lock` reads through the agreed module but retains JSON/provenance ownership and optional failure behavior; finalize retains name/description fallback.
- [x] Existing persisted Edit bases deserialize and clear/replay byte-for-byte; no schema/version change.
- [x] Complete column-zero fence rule agrees across Rust reads/writes and frontend presentation; indented scalar fences and body remain intact, LF/CRLF and delimiter prefixes covered.
- [x] Required I/O remains typed; optional reads still degrade as before; rename failure cleans temporary scratch and preserves original bytes.
- [x] Replay failure restores bytes + skill row + Edit row and retry succeeds; no change to propagation, admission or mutation locking.
- [x] No new UI feature or unrelated cleanup; independent lane gate passes.

## Tests / gates / red-green proof

Test at Manifest's interface with literal input/output fixtures, then the existing discovery/finalize/Edit/catalog consumers. Verified corpus in `tests/frontmatter_edit.rs::corpus_round_trips_preserving_unrelated_bytes_and_no_op` covers missing/empty/unfinished blocks, comments, duplicate keys, quoted booleans, mixed CRLF/LF, nested metadata and body fences. Preserve/extend `indented_fence_is_description_content_for_every_parser_and_writer`, `rename_failure_preserves_bytes_and_cleans_temp`, `clear_preserves_later_frontmatter_additions` and `abandoned_temp_is_not_content`. Add literal assertions for opening indentation, `---suffix` at either end, trailing whitespace and CRLF body slicing; use a pure TS helper test, not JSX.

Preserve `tests/skill_lock.rs::parse_valid_lock_file_returns_entries` and enrichment fixtures; add/retain missing, invalid UTF-8/unreadable and malformed JSON fallback tests with temporary files only. Keep `tests/skill_edits.rs::failed_update_replay_restores_bytes_skill_and_edit_then_retry_succeeds` (SQLite trigger faults) and invalid-UTF8 coverage. Do not turn consumer tests into tests that only mock Manifest returning the expected answer.

Prove the test bites: revert your source change, confirm the test fails, restore.

For the behavior-preserving extraction/corpus, **green on old code is expected**; deliberately mutate an edit output, fence decision or restore step, confirm a literal assertion fails, restore. For actual fence fixes, run the same regression against old source for a genuine red, then new source for green. A missing import/renamed module is not behavioral red; preserve a compiling adapter during the reverse check. Record commands, named failed assertion and final green; leave no mutation behind.

Run targeted Rust filters `manifest`, `skill_discovery`, `skill_lock`, `install_finalize`, `skill_edits`, `skill_catalog`; run the pure TS helper test; run `cargo test --all`, then `npm run version:check && npm run check`. Use the build script's TypeScript-7 compiler, not bare `tsc`. Active LSP for changed source when available; final `lens_diagnostics mode=all`, noting coverage limits. Regenerate/review bindings via `cargo test`, re-read edits, report exact results. Parent repeats full gate after integration.

## Completion report

Give Manifest's chosen interface, compatibility exports removed/retained, lock-file read adaptation rationale, persisted-base compatibility proof, fence corpus red/green proof, unchanged replay/byte guarantees, gate results, ownership deviations and worktree/branch/HEAD. Identify facts that differed from this ticket rather than broadening scope silently.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
