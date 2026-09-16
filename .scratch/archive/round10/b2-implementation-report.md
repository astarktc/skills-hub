# B2 implementation report — Manifest

## Result and checkout

Implemented end to end and committed in the authorized isolated checkout only.

- Checkout: `~/Projects/skills-hub/.scratch/round10/worktrees/manifest`
- Branch: `r10b/manifest`
- Base: `a824caf45fffb0bd5c38318ba10b2ffa0ca11c62`
- HEAD: `6adab087f313413a767358527c38d069d0b3fb1d`
- Commits:
  1. `37b33640cfc210b5f8f5466b469227d196327d4e` — `refactor(manifest): consolidate reads and preserving invocation edits`
  2. `6adab087f313413a767358527c38d069d0b3fb1d` — `fix(manifest): align complete fences across reads edits and presentation`
- Final `git status --porcelain`: empty. Final base-to-HEAD `git diff --check`: clean.
- No main/other-worktree writes, binding handoff, app/dev run, live library/database testing, child, merge, rebase, push, release or tag. All fixtures use temporary directories/stores. Cargo jobs limited to 4.
- `.scratch` evidence/report remains ignored, not committed or force-added.

## Chosen module interface

`src-tauri/src/core/manifest.rs` now owns the existing read grammar, invocation type, scalar helpers, case-insensitive manifest lookup, preserving writer, and required-I/O error mapping. The directory scan/admission ladder remains in `skill_discovery`; Edit rows and orchestration remain in `skill_edits`; final naming and atomic settlement remain in `install_finalize`.

Read doors (existing spelling retained to minimize cross-lane churn):

```rust
pub(crate) fn find_skill_md(dir: &Path) -> Option<PathBuf>;
pub(crate) fn parse_skill_md(path: &Path) -> Option<(String, Option<String>)>;
pub(crate) fn parse_skill_md_with_reason(path: &Path)
    -> Result<(String, Option<String>), &'static str>;
pub fn invocation_mode_for_dir(dir: &Path) -> InvocationMode;
pub fn parse_invocation_mode(text: &str) -> InvocationMode;
pub(crate) fn read_manifest(path: &Path) -> anyhow::Result<String>;
pub(crate) fn read_text(path: &Path) -> std::io::Result<String>;
```

Preserving write doors, moved rather than reimplemented:

```rust
pub fn read_invocation_lines(text: &str) -> InvocationLines;
pub fn write_invocation_mode(text: &str, mode: InvocationMode) -> String;
pub fn restore_invocation_lines(text: &str, base: &InvocationLines) -> String;
pub fn apply_to_file(path: &Path, edit: impl FnOnce(&str) -> String)
    -> anyhow::Result<()>;
```

`InvocationLines::mode()` and the four persisted fields are unchanged:
`disable_model_invocation: Option<String>`, `user_invocable: Option<String>`,
`had_frontmatter: bool`, `repeated_lines: Vec<(usize, String)>`.
No new schema, edit kind, YAML library, arbitrary-edit feature or serialization facility was introduced; the existing `apply_to_file` closure seam was retained.

Fence detection, scalar helpers, line editing helpers and typed I/O mapping are private. Lookup moved unchanged with the parser so Manifest does not depend back on Discovery. Discovery retains thin re-exports of `find_skill_md`, `parse_skill_md`, `parse_skill_md_with_reason`, and `InvocationMode`; unused mode-function re-exports were removed after compiler feedback. No parallel parser/writer remains.

### Skill-lock rationale

Q21's original shorthand is not literally SKILL.md parsing: `skill_lock::parse_lock_file` reads `.skill-lock.json`. It now calls Manifest's small shared `read_text` adapter, as metadata/default-mode/required reads do. `skill_lock` still owns JSON deserialization, source/subpath interpretation, home-fenced symlink enrichment and `.ok()?` optional failure policy. No SKILL.md interpretation or surfaced optional-lock error was invented. Required Edit reads alone wrap failures (including invalid UTF-8) as `SkillManifestIo` / `SKILL_MANIFEST_IO` with the underlying I/O error retained.

### Presentation adapter and deliberate fence changes

`SkillDetailView.tsx` imports the extracted pure `parseFrontmatter` from `src/lib/manifestPresentation.ts`. No component state, IPC, styles, visual feature or JSX tests changed.

Both Rust reads/writes and the TS adapter recognize complete column-zero `---` lines, accepting trailing whitespace and CRLF. Opening indentation and delimiter prefixes are not fences. Scalar-indented fences and later body fences are preserved. TS calculates body offsets using unnormalized raw lines, normalizes CRLF only in metadata scalar lines, and preserves the existing leading-blank-line removal for presentation. Empty/no-entry and unfinished frontmatter remain raw body, as before.

The first commit preserves the original Rust/TS grammar. The second isolates the Rust opening `trim()` → `trim_end()` fix and TS prefix/offset/CRLF fixes. Rust already rejected closing prefixes and scalar-indented closing fences; those cases are regressions, not claimed new Rust fixes.

## Behavioral evidence / red-green

All log paths below are under this checkout's `.scratch/round10/evidence/`. Every shell command began by changing into the authorized checkout. The test commands were:

```sh
cd ~/Projects/skills-hub/.scratch/round10/worktrees/manifest
CARGO_BUILD_JOBS=4 cargo test --manifest-path src-tauri/Cargo.toml core::frontmatter_edit::tests
CARGO_BUILD_JOBS=4 cargo test --manifest-path src-tauri/Cargo.toml core::manifest::tests
CARGO_BUILD_JOBS=4 cargo test --manifest-path src-tauri/Cargo.toml replaces_top_level_lines_in_place_and_appends_only_missing_keys
CARGO_BUILD_JOBS=4 cargo test --manifest-path src-tauri/Cargo.toml complete_column_zero_fences_match_presentation_corpus
npm run test -- src/lib/manifestPresentation.test.ts
```

1. **Preserving baseline and extraction.** Original writer: 6/6 tests passed (`01-preserving-before.log`). Extracted Manifest (including migrated parser tests): 12/12 passed (`02-preserving-extracted.log`); extracted unchanged TS parser: 2/2 (`03-presentation-extracted.log`). No parser tests were duplicated in Discovery.
2. **Writer mutation bites a literal assertion.** Temporarily negated `key.value(mode)` only when replacing an existing line. The exact literal assertion in `replaces_top_level_lines_in_place_and_appends_only_missing_keys` failed, exit 101 (`04-writer-mutation-red.log`): actual `user-invocable: false\r\n` versus expected `user-invocable: true\r\n`, with unrelated bytes unchanged. Restored the source; Manifest 12/12 green in `05-extraction-manifest.log`. This was a runtime assertion failure, not an import/compiler failure.
3. **Real old-source fence red.** Added the nine literal cases in `src/lib/manifestPresentation.corpus.json`, consumed directly by Rust `include_str!` and TS tests. Old Rust failed `indented opening is body`: actual `Some(("alpha", None))`, expected `None`, exit 101 (`07-fences-old-rust-red.log`). Old TS failed five cases (opening prefix, closing prefix, trailing whitespace, CRLF body/scalar offsets, prefix before real closer), exit 1; six tests passed (`08-fences-old-ts-red.log`).
4. **Fix green.** Manifest 13/13 and TS 11/11 passed (`09-fences-rust-green.log`, `10-fences-ts-green.log`).
5. **Explicit source reversion after the fix.** Saved fixed files inside `.scratch`, replaced both production files with their exact `git show 37b3364:<path>` content, retained the new compiling tests, reran the same commands. Rust again failed the literal indented-opening assertion, TS again had five failures (`12-reverted-source-rust-red.log`, `13-reverted-source-ts-red.log`). An EXIT trap restored both fixed files. Full final gates then passed. No mutation or reverted source remains.
6. **Consumer/compatibility regressions.** The shared corpus now exercises real Discovery → Finalize → Catalog consumers as well as Manifest reads, capture, no-op, edit and restore; no Manifest stubs. New optional-read tests cover missing files, invalid UTF-8, directory-as-unreadable-text, and malformed JSON. Manifest filesystem test proves permissions survive a real edit and same-mode no-op preserves inode, mtime and bytes. Existing rename-failure cleanup, hidden-temp identity ignore, later-frontmatter-addition, duplicate-key, mixed-newline, empty and unfinished-header tests remain.
7. **Persisted base and failure-atomic replay.** `persisted_invocation_base_clears_and_replays_without_a_schema_change` inserts literal pre-extraction JSON into a real temporary `SkillStore`, checks its effective base mode, clears to literal original CRLF/comment/duplicate-key/body bytes, replays an Update to literal edited bytes, verifies the persisted JSON fields, then clears byte-exact again. Existing ADR-0004 SQLite edit/hash failure rollback cases remain; the same test now also covers invalid UTF-8 acquired bytes. Each failed Update restores bytes + skill row + Edit row, removes backup debris, and a clean retry succeeds. The existing typed direct-Edit/replay invalid-UTF8 tests also pass.

## Exact verification gates

Targeted extraction commands ran the following filters individually:
`core::manifest::tests`, `core::skill_discovery::tests`, `core::skill_lock::tests`,
`core::install_finalize::tests`, `core::skill_edits::tests`, `core::skill_catalog::tests`.
Results after extraction: 12, 22, 9, 24, 13, 7 passing respectively (`05-extraction-*.log`).
After added regressions: Manifest 15, lock 10, Edit 14 passed (`11-regression-*.log`); the final full run includes the later invalid-UTF8 rollback extension.

Final commands (all exit 0):

```sh
cd ~/Projects/skills-hub/.scratch/round10/worktrees/manifest
CARGO_BUILD_JOBS=4 cargo fmt --manifest-path src-tauri/Cargo.toml --all
CARGO_BUILD_JOBS=4 cargo test --manifest-path src-tauri/Cargo.toml --all
CARGO_BUILD_JOBS=4 npm run version:check && CARGO_BUILD_JOBS=4 npm run check
git diff --check a824caf45fffb0bd5c38318ba10b2ffa0ca11c62..HEAD
git diff a824caf45fffb0bd5c38318ba10b2ffa0ca11c62..HEAD -- src/bindings/index.ts
git status --porcelain
```

- `14-cargo-test-all.log`: **619 passed**, no failures/ignored; binary and doc-test targets also green.
- `15-version-check.log`: **Version OK (1.2.11)**. The task targets the next release, but release/version edits were explicitly excluded, so no bump.
- `16-full-check.log`: ESLint green; **15 Vitest files / 294 tests passed**; TypeScript-7 `tsc.js -b` + Vite build green; Rust format check green; `cargo clippy --all-targets --all-features -- -D warnings` green; **619 Rust tests passed**.
- `cargo test` regenerated bindings; final bindings diff is empty. InvocationMode wire spelling/shape unchanged. No generator output was hand-edited.
- Build emits existing bundle-size and ineffective dynamic-import warnings; not failures and outside this scope.
- Active changed-path LSP batch: 15 files, zero error diagnostics, 4 confirmed clean / 11 inconclusive (4 timeout, 7 silent-on-clean). Final focused re-probe: 6 files, zero error diagnostics, 5 confirmed clean / 1 silent-on-clean inconclusive. Earlier auxiliary hints concern inherited scalar loops/indexing; no blocking error.
- Required final `lens_diagnostics mode=all` with checkout filter reported no cached diagnosed files and two stale omissions. It is **not** proof of cleanliness. Active probes plus full compiler/test gates above are the verification; cache/probe limitations are explicit.

## Scope, deviations and review

No substantive spec deviation. The ticket's corrected facts match this base: `.skill-lock.json` is JSON, Rust already has column-zero closing handling but trims opening indentation, and TS used `startsWith`/`indexOf`, not the Rust rule.

Two additional files beyond the named ownership list:
- `src-tauri/src/core/skill_files.rs`: one comment renames the retired module; the temp ignore predicate is unchanged.
- `src/lib/manifestPresentation.corpus.json`: one shared literal cross-language fixture prevents corpus drift; test-only, not imported by production UI.

All other changed paths are in the assigned ownership: Manifest + moved tests; retired frontmatter_edit implementation/tests; Discovery parser/lookup extraction + parser-test move; mod declarations; finalize/catalog imports; Edit wiring + tests; lock read adapter + tests; detail parser extraction + helper/test. No installer, commands, hooks, report folds, i18n, versions, changelog, AGENTS, CONTEXT or ADR changes.

Manual two-axis review against base (no review children, per instruction):
- **Standards:** no blocking finding. Explicit roots, guard ownership, errors, generated wire types, i18n/no-new-copy and scope restrictions preserved. Production Edit changes are import/qualification-only plus formatter layout. No YAML expansion or normalization of write bytes.
- **Spec:** no missing required feature identified. Tests demonstrate the goal backward: readers/consumers agree, a pre-existing saved Edit clears and replays byte-exact, required errors remain typed while optional reads stay permissive, and failing Update reverts all three state owners before retry. Frontend uses its pure presentation adapter with the same delimiter corpus.

Integration risks/limits: parent must reconcile B3's shared `skill_edits`/`skill_catalog` and `core/mod.rs` hunks, regenerate bindings on the integrated branch and run the integrated full gate. No live Tauri visual testing was attempted (prohibited). Existing intentionally limited metadata/YAML scalar semantics and duplicate restoration positioning were preserved, not expanded for Edit V2. No Windows-specific new behavior was introduced or separately exercised here; the existing atomic writer was moved unchanged.
