# 07: Legacy rows that were really imports are reclassified once on upgrade

Status: done — 816bf13

**What to build:** On the first launch after upgrading, every Managed skill recorded as `local` whose source path was really a Tool's skills directory becomes `imported`, so the rows that failed every Refresh stop failing without the operator doing anything. Three rules, applied to the stored source path: it is inside any Tool's skills dir under the operator's home; or it resolves through a symlink into the central repo; or it does not exist **and** has the shape of a Tool skills-dir path (matches an adapter's relative skills dir under any home prefix — migrated Windows/WSL rows). A `local` skill whose folder is the operator's own, outside every Tool dir, is left exactly as it is, whether or not the folder currently exists. The pass runs once per launch as a core function with explicit roots, is idempotent, and logs how many rows it changed; the schema version does not change.

Source: `../spec.md` Q5 and the "Store" facts.

**Blocked by:** 06 (`imported` provenance)

- [x] Test: a `local` row pointing inside a Tool skills dir under the temp home → `imported`, source cleared, found-in Tool set from the path
- [x] Test: a `local` row whose path is a symlink into the central dir → `imported`
- [x] Test: a `local` row whose path does not exist but ends in a Tool's relative skills dir + name (e.g. a `/mnt/c/Users/x/.claude/skills/foo` shape) → `imported`
- [x] Test: a `local` row whose path is an ordinary folder — existing or not — is untouched; `git` and `imported` rows are untouched
- [x] Test: running the pass twice changes nothing the second time
- [x] Startup calls the pass after `ensure_schema` with roots resolved at the command seam (no `AppHandle` in core)
- [x] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Status reconciliation: ready-for-agent → done — 816bf13. Evidence: Cited 816bf13 integrated as daf5ee6; fbeaff1,b9054c0,08f37d1 extend rules/startup. src-tauri/src/core/legacy_reclassification.rs:50–51 rewrites local import rows and clears source_ref; later shape safety lives at :162.

### 2026-09-05 — implementation (branch r4/07-legacy-reclassification)

**Shipped** (6 commits on top of main `16d4617`):
- `816bf13` feat(store): new core module `core/legacy_reclassification.rs` (declared in `core/mod.rs`) — `pub fn reclassify_legacy_imports(store: &SkillStore, home: &Path, central_dir: &Path) -> Result<usize>`. Rule (i): a `local` row whose `source_ref` is a strict descendant of `skills_dir_in(home, adapter)` for any registry adapter (first in registry order when Tools share a dir, e.g. Amp/Kimi) → `imported`, `source_ref = None`, `imported_from_tool = Some(adapter.key())`. Written through the plain `upsert_skill` with `..record` — timestamps/status/central_path untouched.
- `fc4f9f4` feat(store): rule (ii) — `symlink_metadata` says link **and** `canonicalize(source)` starts with `canonicalize(central_dir)` (both canonical, for macOS `/var` → `/private/var`). Found-in Tool is read off the path's shape when it has one, else `None` (the UI already tolerates `null`).
- `d019b09` feat(store): rule (iii) — `!source.exists()` (follows links, so a dangling link counts as missing) **and** the path's `\`/`/`-normalised components contain an adapter's `relative_skills_dir` as a contiguous run ending exactly one component before the end. Matches `/mnt/c/Users/x/.claude/skills/foo` and `C:\Users\x\.pi\agent\skills\win`.
- `276f9ba` test(store): untouched guard — an existing own folder, a vanished own folder, an **existing** folder under another user's `.claude/skills` (shape matches, but it exists and is not under this `home` → left alone), a `git` row and an `imported` row all survive byte-identical.
- `34313ca` test(store): idempotency — second run returns 0 and every row (including `updated_at`) is unchanged.
- `121a264` feat(startup): `lib.rs` `.setup` — inside the existing `Ok(central)` arm right after `ensure_central_repo`, i.e. after `ensure_schema`: `home_dir()` (from `core::environment`) + the already-resolved `central` → the pass; `log::info!("reclassified {} legacy local skill(s) as imported", n)` when n > 0, `log::warn!` on error (startup never fails on it). No `AppHandle` enters core.

Tests: `core/tests/legacy_reclassification.rs` ×5 (temp home / central / DB; seam = the core fn, verified through `store.get_skill_by_id` / `list_skills`).

**Gate**: `npm run version:check` OK (1.2.3); `npm run check` green — vitest 13 files / 194 tests, build OK, rustfmt clean, clippy `-D warnings` clean, `cargo test` **489/489** (was 484); `cargo test --all` 489/489. No DTO change → `src/bindings/index.ts` untouched; worktree clean.

**Deviations**
1. None from the spec's three rules. One small enrichment: rule (ii) matches also record a found-in Tool when the link's own path has Tool shape (a link sitting in a Tool dir that is not under this `home`); a link with no Tool on its path gets `imported_from_tool = None`.
2. Rule (i) requires a *strict* descendant (the skills dir itself is not "inside"), unlike `ensure_path_within_tool_dirs`' `starts_with` — a `local` row pointing at the Tool dir itself is nonsense either way, and a strict check never mislabels it.
3. Reclassified rows keep their `updated_at` — the skill's bytes did not change, only how the record is described.

**Notes for the orchestrator**
- **Ticket 08 overlap**: rule (i) lives as a private helper `tool_owning_path(home, path) -> Option<&'static str>` over the registry's public `default_tool_adapters()` + `skills_dir_in`. If 08 added a public "which Tool owns this path" predicate to `tool_adapters`, the two should be unified in a follow-up (delete mine, call the registry's). No file conflict expected: I touched no file 08 owns (`installer.rs`, `tool_adapters/mod.rs`, frontend untouched).
- Merge touchpoints: `core/mod.rs` (+1 line, alphabetical after `installer`), `lib.rs` (one block inside the `Ok(central)` arm), plus two new files.
- The pass runs *before* `app.manage(store)`, so it is complete before any command can observe the rows — the first listing after upgrade already shows "Managed here".
- Operator smoke test (after merge, `tauri:dev` on the real library): the log should say `reclassified N legacy local skill(s) as imported` on first launch and nothing on the second; the 7 rows from the spec's problem statement should read "Managed here" with no Update button, and Refresh (all) should stop reporting them.

### 2026-09-05 — orchestrator, at merge
- Unified rule (i) onto ticket 08's registry fn: `tool_owning_path` now calls `tool_adapters::tool_holding_path` (strict-descendant filter kept). Commit on this branch before the ff-merge. Rule (iii) stays its own function — it matches a Tool-dir *shape under any home prefix*, which a `home`-anchored registry fn cannot answer.
