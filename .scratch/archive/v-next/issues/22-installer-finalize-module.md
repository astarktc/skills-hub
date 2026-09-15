# 22: One installer finalize module + typed `SkillExists` + typed GitHub status

Status: resolved

Type: task
Blocked by: 20

## What to build

Unanimous review #2 finding (all three panelists, verified; also the old scan's Finding 5 / map fog item): `core/installer.rs` (1,951 lines, the only tier the epic never touched) hand-rolls "materialise a skill into the central repo and record it" in four flows instead of calling it. At `943f85c`: the 14-field `SkillRecord` literal at `:90` (`install_local_skill`), `:187` (`install_git_skill`), `:1379` (`install_git_skill_from_selection`), plus the update rebuild at `:869`; the SKILL.md-preferred rename dance duplicated verbatim at `:159-183` and `:1348-1371` (both carrying the same "fixes #28" comment) — and the copies have drifted (description re-read sits *outside* the exists-guard in one, *inside* it in the other); the collision `bail!("skill already exists in central repo: {:?}")` at `:61`, `:147`, `:1324`.

Two riders that belong in the same campaign:

- **The collision crosses the wire as prose.** `src/commandError.ts:65-77` string-matches the message and regex-extracts the skill name from Rust's `{:?}` Debug path formatting; `core/tests/installer.rs:218` asserts on the prose too. ADR 0001 names this smell explicitly. (Ticket 11 banked this.)
- **GitHub HTTP status is discarded then sniffed back.** `core/github_download.rs:136-168::check_github_response` has `status` in hand, raises typed `RateLimited` only when the reset header parses, and otherwise throws the code away as prose; `installer.rs:1690-1707` then does `err_msg.contains("404")` / `contains("403")` to reconstruct typed signals (losing the reset ETA → `reset_minutes: 0`). Decided in grilling: classify at the origin.

Deepen:

- One entry, e.g. `finalize_install(store, central_dir, staged_dir, provenance/NameIntent) -> Result<InstallResult>` owning: collision check (typed), SKILL.md-preferred rename, description extraction, content hash, `SkillRecord` construction, upsert. The four flows shrink to "acquire bytes into a staging dir, then call it" (the update path stages then swaps). Decide the exact interface; record it in the Answer.
- `SignalError::SkillExists { name }` + `CommandError` variant + regenerated binding (`cargo test`) + `describeCommandError` branch + EN/ZH i18n keys (the ADR recipe; `COMMAND_ERROR_CODE_MAP` will force the frontend branch). Delete `describeOther`'s sniff. The installer test asserts the typed variant.
- `check_github_response` raises a typed error carrying the status (and `reset_minutes: Option<i64>`); `fetch_skill_files` matches by downcast; both `contains(...)` branches deleted. Extend the existing mock-server tests (`github_download.rs:297-375`) to assert variants; the 404 path gets its first test.
- Preserve wire codes for existing conditions; `GITHUB_SKILL_NOT_FOUND` / `RATE_LIMITED` keep their meaning.

## Acceptance criteria

- [ ] Exactly one `SkillRecord` construction for new installs (update may share or reuse it); zero duplicated rename/collision blocks; four entry points call the finalize interface.
- [ ] `"skill already exists in central repo"` no longer exists as prose anywhere (Rust or TS); `SKILL_EXISTS` is a wire variant handled in `describeCommandError` with EN+ZH copy; bindings regenerated and committed.
- [ ] Zero `.contains("404"|"403"|"Not Found"|"Forbidden")` in `installer.rs`; GitHub status classification is typed at the origin with tests.
- [ ] `npm run version:check && npm run check` green.

## Answer

Landed green in `71b1515` (Fable 5.1 child, medium thinking; rebased cleanly over ticket 23; orchestrator-verified; 268 cargo + 62 vitest, full gate green on main).

**Finalize interface** (new `core/install_finalize.rs`):
```rust
pub struct StagingDir            // StagingDir::new_in(central_dir) → ".skills-hub-staging-<uuid>" sibling of the final path;
                                 // RAII: removed on drop if not consumed; move_into = same-fs rename with copy fallback
pub enum NameIntent { UserProvided(String), Derived(String) }
pub struct SkillProvenance { source_type, source_ref, source_subpath, source_revision }  // ::local(path) / ::git(url, subpath, revision)
pub fn ensure_name_available(central_dir, name) -> Result<PathBuf>   // typed SignalError::SkillExists
pub fn finalize_install(store, central_dir, staged, name: NameIntent, provenance) -> Result<InstallResult>
pub fn finalize_update(store, record, staged, revision: Option<String>) -> Result<SkillRecord>
```
Flows own only *acquire* (bytes into `staged.path()`); name preference (SKILL.md `name` beats a derived name, operator-provided always wins), collision, move, description, hash, upsert live once. `ensure_name_available` is exposed so a doomed install never downloads; `finalize_install` re-checks as the authority. Exactly one `SkillRecord {` for new installs (+ one struct-update in `finalize_update`); `installer.rs` −409 lines; three install entry points + the update path call it.

**Typed `SkillExists`**: `SignalError::SkillExists { name }` → `CommandError::SkillExists` (`SKILL_EXISTS`) → regenerated binding → `describeCommandError` branch reusing `errors.skillExistsInHubNamed`; `describeOther` deleted (`OTHER` passes the message through); dead `errors.skillExistsInHub` key removed from EN+ZH. Zero occurrences of the old prose in Rust or TS; tests assert the typed variant at core, wire, and frontend layers.

**Typed GitHub status**: `GithubApiError { status: u16, reset_minutes: Option<i64>, url }` raised by `check_github_response` for every non-success status; `fetch_skill_files` downcasts (404 → `GithubSkillNotFound`, 403 → `RateLimited` with the real ETA — was always 0). Zero `contains("404"|"403"|…)` in the installer. Tests: 403+ETA, 403 without header, 404, 502, `.context()` survival.

**Bonus fixes surfaced by the consolidation**: the multi-skill subpath backfill in the update path was clobbered by the rebuilt record (stale clone) — now carried through; failed installs no longer leave partial dirs in the central repo. CONTEXT.md gained **Staging dir** and **Finalize (install)**.

**Known residue** (pre-existing, scoped out): `finalize_update` removes the old central dir before moving staging in — a failure at the move loses content as before (could become move-old-aside → move-new → delete-old). No end-to-end test for the `fetch_skill_files` 404/403 mapping because `download_github_directory` hardcodes `api.github.com`. A crash mid-install could leave a `.skills-hub-staging-*` dir in the central repo (dot-prefixed; nothing lists it).

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
