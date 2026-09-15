# Round-4 review — `fable` — `f057190` (v1.2.3) … `7fcec94`

Diff reviewed: `git diff f057190...HEAD` (44 commits, 72 files). Standards sources: `AGENTS.md`, `CONTEXT.md`, ADR-0001/0002/0003, `docs/agents/*.md`. Spec: `.scratch/round4/spec.md` + tickets 01–09. Fresh check run: `cargo test --all reclassif` → 5/5 ok.

## Standards

**Hard (documented standard)** — none found. Every new command has `#[tauri::command]` + `#[specta::specta]` + a `collect_commands!` entry; every new DTO derives `specta::Type` and reaches components via the `types.ts` shim; all 7 new `SignalError`s have a `CommandError` arm, a `describeCommandError` branch and EN + ZH keys; core takes explicit roots (`reclassify_legacy_imports(store, home, central_dir)`; `lib.rs` resolves them); Sync-target mutations stay behind existing guarded entry points (`repoint_local_source`/`detach_from_source` are store-only); removal still goes only through `artifact_removal`. Vocabulary follows `CONTEXT.md` (Provenance, Unlocatable, "skipped" vs "not a member").

**Judgement calls (baseline smells)**

- **Duplicated Code** — the Tool-label lookup ``t(`tools.${key}`, { defaultValue: key })`` appears in `SkillCard.tsx`, `SkillDetailView.tsx` and `commandError.ts`, and the "Managed here · Imported from {tool}" line is composed twice (card + detail). `AGENTS.md` "Frontend presentation logic … lives once" points at a `toolLabel(key, t)` in `skillPresentation.ts`.
- **Duplicated Code** — `onboarding_import.rs`: `apply_one_unlocked` and `sync_imported_unlocked` each do `group.variants.iter().find(|v| v.path == selection.chosen_path)`; one `chosen_variant(group, selection)` would serve both.
- **Primitive Obsession** — `apply_one_unlocked`: `.map(|variant| variant.tool.as_str()).unwrap_or_default()` turns "not found" into `imported_from_tool = Some("")`; `admit` makes it unreachable, but the empty-string sentinel would render as "Imported from ". An `Option` or a `.expect` with the invariant is more honest.
- **Mysterious Name** — `legacy_reclassification.rs` has `tool_owning_path`, `tool_shaping_path` beside the registry's `tool_holding_path`; three near-synonyms for two different questions ("under this home's Tool dir" vs "has a Tool-dir shape under any prefix"). Rename the shape one (e.g. `tool_dir_shape_of`).
- **Prose stringly errors** — `LinkChain::follow` depth bound and `require_local` bail plain `anyhow` (→ `OTHER`), with a subpath / skill name in the message. Sanctioned by ADR-0001 as the "safety valve", but both are conditions the code can name.
- **Panic in core** — `project_sync::sync_assignment_target`: `.expect("a live skill name always locates the artifact")`. True by construction today; the ticket admits it mirrors an existing `expect`. Fine, noted.
- **Dangling references** — code comments cite `spec Q5`, `ticket r4/06`, `round 4, ticket 09` (ADR-0003) — `.scratch/` is gitignored and "spec Q5" now means two different things (round-3 toast lifetimes in `useStatusReporter.test.ts`, round-4 reclassification in `lib.rs`). Pre-existing pattern (9 hits at base), but it is getting ambiguous.
- **Doc drift** — ADR-0003 "Consequences": "Refresh (all) selects its members through one backend predicate (`provenance::is_refreshable`)" — after ticket 09 the batch rule is `refresh_eligibility`; `is_refreshable` is the Update/listing half.
- **Migration atomicity** — `ensure_schema` incremental branch: `ALTER TABLE … ADD COLUMN` then `pragma_update(user_version)` are separate statements on an autocommit connection; a crash between them leaves a DB that fails every later launch with "duplicate column". Same shape as before; wrapping the branch in one transaction would close it.

## Spec

**(a) Missing / partial**

- Ticket 04 acceptance "`describeCommandError` is imported by the reporter (and `commandError.ts` tests) only" is ticked but not met: `src/components/skills/SettingsPage.tsx:5` still imports it (ticket comment admits it). Small: pass `formatError` as `SkillDetailView` now does.
- Spec "Implementation Decisions": "Refresh membership is one predicate … consulted by Refresh (all), single Update and the UI's Update affordance." Shipped as two (`refresh_eligibility` for the batch; `is_refreshable` + `unlocatable_state` for Update and the DTO), and the card re-combines two flags (`skill.refreshable && !unlocatable`). Deviation is documented (ticket 09 #3) and internally consistent — but the one-rule promise is not literally kept.
- Q8 API path: only a *leaf* `type: symlink` is followed. A link on a *component* (`skills -> ../pkg/skills`, subpath `skills/foo`) answers 404 from the Contents API, which `classify_fast_path_failure` now raises typed and never retries as a clone — so the clone adapter, which does follow component links, is never reached. The five operator rows are leaf links, so today's library is fine.

**(b) Not asked for**

- Ticket 01: `open_log_folder` on Linux/Windows now `open_path`s the dir instead of revealing it in its parent — a behaviour change beyond "make the `.app` rule a pure function". Harmless, but undocumented in the spec.
- Ticket 09 `repoint_local_source` also validates `require_skill_md` and `tool_holding_path` — sensible, but Q7 only says "new `source_ref` → the single-skill Update".

**(c) Looks implemented but wrong / unsafe**

- **Legacy reclassification rule (iii) misclassifies a genuine `local` skill (data safety).** Q5(iii): "does not exist **and** has the shape of a Tool skills-dir path (… any *home* prefix — covers migrated WSL/Windows rows)"; US 9 / ticket 07: "a `local` skill whose folder is genuinely my own … left exactly as it is … whether or not the folder currently exists." `tool_shaping_path` accepts *any* prefix, and `.claude/skills`, `.agents/skills`, `.cursor/skills` … are also project-scope dirs. `~/Projects/<repo>/.agents/skills/<x>` (this very repo has one) or a worktree's `.claude/skills/<x>` added as a `local` skill and absent at launch (worktree removed, branch switched, volume unmounted) → rewritten `imported`, `source_ref` dropped, `imported_from_tool = claude_code` (false history), and only the *count* is logged — irreversible without knowing the old path. For a path under this `home`, rule (i) (`tool_holding_path`, lexical `starts_with`, no existence needed) already gives the full answer, so rule (iii) should only ever consider paths **not** under `home`; and each reclassified row (name + former `source_ref`) should be logged. The guard test's "vanished" case is `home/gone/skill` — no Tool shape — so this is untested.
- **`LinkChain::follow` lets a backslash target escape on Windows (US 4).** Only `/` is a separator: a hostile blob `..\..\..\x` is pushed as *one* segment, resolves to `a/b/..\..\..\x`, and `repo_dir.join(..)` on Windows (where `\` is a separator) resolves outside the cache before `copy_src.exists()` / `copy_dir_recursive`. Normalise `\`→`/` (or refuse any `\`) before splitting; the same sweep should cover `symlink_on_path`'s target.
- `detach_from_source` is offered for `source_missing` even when the central copy is *also* gone (spec: both-gone reports `source_missing`); Detach then yields an `imported` row with no source and no central — only Remove works. Hide Detach (or refuse it) when `central_path` is absent.

Verified OK (scrutiny list): `ensure_schema` V9 + `user_version` fix — a v7-reading v8 DB re-runs idempotent V8 SQL, adds the column once, records 9; second launch is a no-op (test covers 8→9→9). `refresh_eligibility`: `All` settles Unlocatable up front, so `reassert_auto_sync_unlocked` (per *finalized* skill) can never mint a target for them; imported skills are never members; `Ids` reaches acquire, which refuses `imported` typed before any central check and rebuilds a missing central (Restore). Cache entry only widens (`widened_to` union). Import take-over: a divergent sibling in a policy Tool meets `overwrite: false` + `overwrite_if_same_content` (hash differs) → `TARGET_EXISTS`, never overwritten; a `None` fingerprint is treated as divergent. Error contract complete for all 7 codes; the only prose carrying a (possibly absolute) path is `errors.symlinkEscapesRepo` interpolating `{{target}}` — same pattern as the pre-existing `pathOutsideToolDirs`.

## Summary

**Blocking (fix before v1.2.4)**
1. `core/legacy_reclassification.rs::tool_shaping_path` / `classify` — rule (iii) fires for vanished project-scope Tool-shaped paths under the operator's own home (irreversible `source_ref` loss, false found-in history). Restrict rule (iii) to paths outside `home`; log each reclassified row.
2. `core/repo_subpath.rs::LinkChain::follow` — backslash-separated targets bypass the `..` accounting; on Windows the resolved path escapes the checkout and is read. Normalise or refuse `\`.

**Follow-up**
- `SkillCard.tsx`: hide/refuse Detach when the central copy is also missing (dead-row outcome).
- `SettingsPage.tsx` still imports `describeCommandError` (ticket 04 box ticked, not met).
- API adapter: component-level symlink → typed 404, never falls to the clone that would follow it.
- `errors.symlinkEscapesRepo` interpolates `{{target}}` into prose; move to `withDetail`.
- ADR-0003 consequence names `is_refreshable` as the batch predicate; it is `refresh_eligibility`.
- `ensure_schema` incremental branch: run migrations + `user_version` in one transaction.
- Duplicated Tool-label lookup ×3 / "Managed here" line ×2 → `skillPresentation.ts`; duplicated chosen-variant lookup in `onboarding_import.rs`; empty-string `found_in_tool` sentinel; `tool_shaping_path` naming; ambiguous `spec Q5` comments.
- `open_log_folder` non-macOS behaviour change (open vs reveal) — record it in the CHANGELOG.

Totals: Standards 0 hard / 9 judgement; Spec 3 partial, 2 unasked, 3 wrong-or-unsafe (2 blocking).
