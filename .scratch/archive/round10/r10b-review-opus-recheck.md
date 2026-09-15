# Wave B v1.2.12 — focused re-review of accepted review fixes (single seat, Opus 5)

Target: `/Users/alexstark/Projects/skills-hub`, main checkout, HEAD **`23458bb6ff29c1a3bb453a623904cb85fe80dfa8`** (verified by `git rev-parse`), working tree clean at review time (so the parent's final `cargo test` left no binding drift). Original integrated review HEAD `960e5d5`, wave base `a824caf4`.

`git diff 960e5d5...HEAD` = **8 files, +364/−19**: `src-tauri/src/core/manifest.rs`, `core/skill_edits.rs`, `core/tests/manifest.rs`, `core/tests/skill_edits.rs`, `src/lib/manifestPresentation.corpus.json`, `src/lib/manifestPresentation.test.ts`, plus `AGENTS.md` and `CHANGELOG.md`. That matches the claim "original reviewed code unchanged outside the six fix files + two docs" — verified, not assumed. Commits: `3d3a2dd` (changelog), `c1ff64c` (fix), `23458bb` (invariant).

Rebase fidelity checked independently: the six source files at HEAD hash **exactly** to the SHA-256 fingerprints recorded in `review-fixes-report.md` for the pre-rebase commit `a25f38e`, and `git diff a25f38e c1ff64c` is `CHANGELOG.md` only (the interleaved `3d3a2dd`). So the child's gate logs apply to byte-identical source.

Read-only seat: no cargo/npm/bun/build/generator/LSP run here; no app, dev server, operator DB, library or Tool dirs touched; no network. Scope limited to the accepted fixes, their blast radius, the changelog clarification, and the standing of the deferred findings. Approved decisions were not reopened.

## Standards

**STANDARDS4 (corpus equates Rust header presence with TS metadata presence) — RESOLVED.**
`src-tauri/src/core/tests/manifest.rs:7-19` now derives `Case` with independent `had_frontmatter`, `rust_metadata` and `rust_error` literals, and line 45 asserts `base.had_frontmatter == case.had_frontmatter` instead of `case.meta.is_some()`; line 28 adds a per-language name-validity assertion (`parse_skill_md_with_reason(...).err() == case.rust_error`). `src/lib/manifestPresentation.test.ts:6-7` records that `meta`/`body` are presentation-only expectations. Crucially the corpus now *contains* the divergence case the finding predicted: `manifestPresentation.corpus.json:2-10` `"---\n---\nBody\n"` with `hadFrontmatter: true`, `rustError: "missing_name"`, `meta: null`. Two more independent cases were added (comment-only CRLF header; header with `description` but no `name`). I hand-checked all three against both implementations: TS `parseFrontmatter` (`src/lib/manifestPresentation.ts:46`, `keys.length === 0` → `meta: null`, body = raw) and the Rust parsers, including the shared loop's follow-on assertions (`write_invocation_mode` no-op at the case mode, edit-to-`UserOnly` then `restore_invocation_lines` back to `raw`, discovery/finalize fallback name). All three are consistent with the code; no production metadata/display policy changed. Test evidence: `evidence/review-fixes/corpus-coupling-red.log` (old inferred expectation `false` vs actual `true` on the empty-header case).

**STANDARDS1 (unserviceable listing intent) — still valid, still non-blocking, untouched.** `src-tauri/src/core/git_acquisition.rs:509` still bails with prose for `Intent::Listing`; no production caller can reach it. Unchanged by these commits.

**STANDARDS2 (Update command calls the batch command) — still valid, still non-blocking, untouched.** `src-tauri/src/commands/mod.rs:818-840` unchanged; ticket 05 §2 expressly allows it.

**STANDARDS3 (`refreshProgress` name) — still valid, cosmetic, untouched.** `src/hooks/useSkillLibrary.ts:131` unchanged.

**No new Standards finding in the fix blast radius.** Checked deliberately, all clean:
- The compatibility grammar is *one* private gate (`manifest.rs:321`) that delegates the closing-fence rule to the existing strict `header_end` via a read-only projection — there is no second parser or second fence policy in the module.
- `write_invocation_header` (`manifest.rs:363`) is a pure extraction: byte-identical to the body previously inlined in `write_invocation_mode`, and to the historical writer at `a824caf:core/frontmatter_edit.rs:124-148`.
- `write_persisted_invocation_mode` is `pub(crate)` with exactly one caller (`skill_edits.rs:155`); `restore_invocation_lines`/`write_invocation_mode` keep their existing callers. Grep across `src-tauri/src` + `src` confirms `had_frontmatter`, `read_invocation_lines`, `restore_invocation_lines`, `write_invocation_mode` are consumed only inside `manifest.rs` and `skill_edits.rs`. Blast radius is contained to Edit settlement.
- No `commands/`, DTO, binding, i18n, catalog, Propagation, admission, guard or sync surface changed; AGENTS invariants (fan-out ownership, mutation guard, specta generation, typed error contract, core-root resolution) all still hold.

## Spec

**SPEC1 (pre-upgrade Edit over an indented opening fence) — RESOLVED; every accepted requirement verified against source, not assumed.**

1. *Byte-for-byte clear of an old saved Edit.* `manifest.rs:398` adds `.or_else(|| legacy_edit_header_end(&lines, base))`; because the legacy gate requires `base.had_frontmatter == true`, the `!had_frontmatter` strip branch at `:401` cannot fire, so restore keeps `lines[0]` (the real `  ---` bytes), replaces first-occurrence key lines with the persisted lines, replays `repeated_lines` by absolute index, and re-emits `lines[end..]` verbatim. That body is character-identical to the historical `restore_invocation_lines` (`a824caf:core/frontmatter_edit.rs:154-178`); only the fence gate differed, and the gate is now restored for exactly this input. **Independent provenance check:** I re-derived the `LEGACY_*` fixtures (`tests/skill_edits.rs:614-616`) by hand from the historical `a824caf` reader/writer — `header_end` = 6, `user_invocable = "user-invocable: 'no'  \r\n"`, `repeated_lines = [[4,"user-invocable: yes\r\n"]]`, `disable_model_invocation = "disable-model-invocation: false\r\n"`, and the `Neither` write reproduces `LEGACY_EDITED` byte-for-byte including the `---\t\r\n` closing and the LF body fence. That corroborates `historical-fixture-proof.log` without re-running the (now removed) probe test.
2. *No global relaxation of the new grammar.* The gate at `manifest.rs:321` needs a persisted `had_frontmatter` **and** the former opening spelling at document start; the closing fence, delimiter prefixes and scalar rules go through the unchanged strict `header_end` (`:32`). `parse_skill_md*`, `parse_invocation_mode`, `invocation_mode_for_dir`, `read_invocation_lines` and discovery/finalize are untouched. Guarded by `tests/manifest.rs:108-128` (body-located block, `---suffix` at either end, indented closing, unfinished block → all no-ops) and by the `fresh` half of that test (same bytes, `had_frontmatter:false` → body).
3. *Replay captures/uses current upstream.* `skill_edits.rs:173` (`read_invocation_lines` of the freshly finalized manifest) and `:208` (`write_invocation_mode`, strict) are unchanged; only `settle_direct_unlocked` passes a base. `skill_update.rs:135/148` confirms the persisted writer is reachable only from `UpdateBytes::EditInPlace`, never from the git/local/restore/re-point paths. `tests/skill_edits.rs:731-762` runs two consecutive Updates over both a canonical and an indented upstream, asserts the exact bytes and the *new* base JSON each time (no synthetic-header stacking, legacy base gone), then clears back to the **current** upstream bytes.
4. *No schema / new field / migration walker / real data.* `InvocationLines` (`manifest.rs:246-252`) is unchanged, so the persisted JSON shape is identical; nothing scans the body or rewrites stored rows at startup.
5. *Typed required I/O; failed clear/re-choose retains the restoration base.* `apply_to_file`/`manifest_io` untouched; `tests/skill_edits.rs:770-800` (unix, `0o555`) asserts `SkillManifestIo`, unchanged bytes, retained `LEGACY_BASE`, no `.skills-hub-manifest-*` sibling, and a successful retry + clear. `:803-834` covers the deleted and invalid-UTF-8 manifest, keeping the row identical.
6. *ADR-0004 rollback still restores bytes + skill row + Edit row, retry works.* `failed_update_replay_restores_bytes_skill_and_edit_then_retry_succeeds` is now parameterised over `(legacy, fault)` for all three SQLite/hash/UTF-8 faults (`tests/skill_edits.rs:414-425`), with the legacy row seeded from the literal historical JSON. Sensitivity shown by `replay-rollback-mutation-red.log`.
7. *Fresh indented input without a legacy base.* `tests/skill_edits.rs:764-... ` asserts the canonical prepended header, a `had_frontmatter:false` base, a re-choose, and an exact clear round-trip back to `LEGACY_ORIGINAL`.

Sensitivity evidence read to completion (not re-run): `source-reversal-red.log` — with **both** production files restored from `960e5d5`, the real clear test fails on the byte assertion (`left` retains the forced keys), not on a compile error; `rechoose-mutation-red.log` — routing re-choose to the strict writer produces the synthetic LF header above the CRLF block; `historical-fixture-proof.log`; `preserving-corpus-mutation-red.log`.

**SPEC2 (Q5 changelog consequence) — RESOLVED and accurate.** `CHANGELOG.md:14` now names both consequences. Verified against source: `src/hooks/useAddSkillFlow.ts:444-450` hands off to the picker whenever no `target` is given and `candidates.length !== 1` (previously a folder-skill listed only itself → auto-install), and `:435-438` resolves a `target_match.kind === "resolved"` nested candidate that previously fell through to `errors.skillNotFoundInRepo`. No behavior change in the commit — docs only.

**SPEC3 (missing 1.2.7–1.2.11 sections) — unchanged, still deferred.** `CHANGELOG.md:7` (`1.2.12`) still sits directly above `:28` (`1.2.6`). Pre-existing; noted, not re-argued.

**New observations in the fix's blast radius — none is a regression; recorded for the record.**

- *(consequence, deliberate)* For the legacy corpus, a **re-choose** now writes inside the indented block, which no consuming tool reads as frontmatter, so the newly chosen mode reaches Tools only after the next Update canonicalises the file; at `960e5d5` the (defective) synthetic block did take effect while stranding the old keys and deleting the row on clear. This is the trade the disposition selected — byte-preserving reversibility over effectiveness on an already-malformed manifest — and the tests pin it (`tests/skill_edits.rs:679-681` deliberately assert `invalid_frontmatter` / `UserAndModel` for the file). Row and bytes stay mutually consistent under the legacy grammar and clear restores exactly, so there is **no data loss, wrong byte or row divergence**: not a blocker.
- *(pre-existing at `960e5d5`, not introduced here)* For the same corpus the catalog's `invocation_mode` (strict read → `user-and-model`) differs from `invocation_override.mode` shown beside it (`src/components/skills/SkillCard.tsx:141-142`, `modals/InvocationModeModal.tsx:16-21`). That divergence is inherent to the accepted strict grammar, not to these three commits; the fix makes the row honest so the next Update converges.
- *(cosmetic, optional, explicitly not a new requirement)* `CHANGELOG.md:19` states the fence rule but not its user-visible consequence for an already-installed skill whose `SKILL.md` starts with an indented fence (its name/description/mode now fall back in the detail view). Symmetric to the accepted Q5 clarification; raise only if the parent wants that symmetry — otherwise leave it, per "avoid scope expansion".
- *(completeness)* The restore fallback also opens for a *modern* base (`had_frontmatter: true`) whose central file was later mangled externally to an indented opening: clear then rewrites the recorded key lines inside that block instead of no-oping. That is exactly pre-wave behavior and strictly better than `960e5d5` (no-op + row deletion), so it is not a regression.

**Docs.** `AGENTS.md:155-161` now records the compatibility invariant ("only a persisted Edit base authorizes the former indented opening for clear/re-choose. Fresh reads and upstream replay stay strict") and the corpus's independent per-language expectations — both match the code as read. `CONTEXT.md:35-36`'s Manifest definition stays correct at its level of abstraction; ADR-0004 needs no amendment (replay/rollback production code is unchanged).

## Summary

- **Standards: STANDARDS4 resolved; STANDARDS1/2/3 still valid, unchanged, non-blocking; 0 new findings.**
- **Spec: SPEC1 resolved (all seven accepted requirements verified against source, with independently re-derived fixture provenance); SPEC2 resolved and accurate; SPEC3 unchanged/deferred; 0 new regressions — four recorded consequences/observations, all non-blocking.**
- **Merge verdict: OK with notes** at HEAD `23458bb6ff29c1a3bb453a623904cb85fe80dfa8`. The compatibility fix is narrow, persisted-base-gated, reachable only from direct Edit settlement, and leaves the strict grammar, replay, rollback, schema, catalog, Propagation and admission untouched.

### Gate evidence read (not run by this seat)

Parent's final logs at the main checkout, written after the HEAD commit (`23458bb` 04:49:57; logs 04:51) and read to completion:
- `.scratch/round10/final-version.log` — "Version OK (1.2.12)".
- `.scratch/round10/final-check.log` — `RUN v4.1.11 /Users/alexstark/Projects/skills-hub`; eslint → vitest **15 files / 347 tests passed** → typescript-7 `tsc -b` + vite build → `cargo fmt --all --check` → `clippy --all-targets --all-features -D warnings` → `cargo test` **640 passed / 0 failed**.
- `.scratch/round10/final-cargo-all.log` — `cargo test --all` **640 passed / 0 failed**.
- Working tree clean afterwards ⇒ no `src/bindings/index.ts` drift.
The child's own gate (`evidence/review-fixes/final-*.log`, same 347/640) ran in the worktree on byte-identical source, per the SHA-256 match above.

### Verification limits (reviewer)

- No build, test, generator or LSP run by this seat; no diagnostics claim of my own. All pass/fail statements come from the logs named above.
- **No live smoke test**: no app, dev server, operator DB, real skill library, Tool directory, network acquisition or Windows runtime was exercised. The permission-failure coverage is unix-only (macOS host); Windows uses the unchanged I/O adapter.
- The child's `historical_writer_produces_literal_upgrade_fixtures` probe is intentionally absent from HEAD, so fixture provenance is not reproducible from committed source; I substituted a manual derivation from `a824caf:core/frontmatter_edit.rs` and it matches the literals exactly.
- Mutation red logs were read, not re-executed.
- Reviewer evidence is not publication authority; the parent owns integration and release.
