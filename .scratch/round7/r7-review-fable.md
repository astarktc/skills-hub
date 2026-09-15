# Round-7 review — Skill Edit V1 — reviewer: fable

Reviewed `ac8b514..a261e89` (one commit, 26 files). Verified at HEAD: `cargo test skill_edits` 6/6, `cargo test frontmatter_edit` 3/3, `npm test` 241/241, `npm run build` green, `src/bindings/index.ts` not dirty after `cargo test`. `git diff ac8b514..HEAD -- src-tauri/src src/` deletions are only the intentional badge/hook/CSS rewrites — nothing from v1.2.6 reverted.

## Standards

**Hard violations of a documented standard: none found.** Checked against AGENTS.md invariants: command has `#[tauri::command] + #[specta::specta]` and is in `collect_commands!`; `commands/` stays wiring-only; `invocation_override: InvocationOverrideDto | null` is wire-accurate; `InvocationMode` gains `Deserialize` for the input side; en + zh keys both present (14 each); new core module declared in `core/mod.rs`; components import types only from the `types.ts` shim; presentation maps moved into `skillPresentation.ts` (pure, single-implementation); `set_invocation_override` wraps its own body in `mutation_guard::serialized` and calls only `propagate_unlocked` + readers inside it; the store uses `PRAGMA foreign_keys = ON` so the `ON DELETE CASCADE` on `skill_edits` mirrors `skill_targets`; typed `SkillEditKind` reaches core; `CONTEXT.md` entries follow the `_Avoid_` convention.

Baseline smells (all judgement calls):

1. **Duplicated Code / inconsistent DTO policy** — `skill_edits::InvocationOverride` and `commands::InvocationOverrideDto` are byte-identical shapes with a hand-mapped `From`, while `InvocationEditConflict` (same module) crosses the wire directly as a core `specta::Type` in `SkillRefreshStatusDto::Refreshed`. Pick one policy (AGENTS.md explicitly allows DTOs living in `core/`); delete one of the two override types.
2. **Primitive Obsession** — `skill_edits.rs`: `serde_json::to_value(mode)?.as_str().context("invocation mode must be a string")?` and `edit_mode` via `serde_json::from_value(Value::String(..))` to turn the enum into its kebab key. `SkillEditKind` got `as_key`; `InvocationMode` deserves `as_key()`/`from_key()` too.
3. **Feature Envy under the lock** — `set_invocation_override` ends with `managed_skill_catalog(store)?.into_iter().find(..)`: it builds every skill's entry (all `SKILL.md` reads + target rows) inside the mutation guard to return one. The ticket allowed "or filter", so not a breach; a `managed_skill_entry(store, id)` seam is the deeper fix.
4. **Mysterious control flow** — `useSkillLibrary.runSingleUpdate`: `let hasEditConflict = false;` mutated inside the action and read by the `successToast` thunk. `handleRefresh` gets the report as the toast argument; returning the report (instead of `?? true`) would give the same shape here.
5. **Duplicated test** — `tests/frontmatter_edit.rs::every_mode_round_trips_without_frontmatter` is a strict subset of the corpus test (`""`, `"# Body\r\n"` already loop all four modes).
6. **Cyclic module dependency** — `skill_edits` imports `skill_catalog::managed_skill_catalog`; `skill_catalog` imports `skill_edits::invocation_override`. Compiles, but the catalog now depends on the thing it decorates.
7. `manifest()` + `read_to_string` + `apply_to_file` read `SKILL.md` twice per operation; `read_to_string(&path)?` in `set` carries no path context (a permission error would surface as bare "Permission denied" under `OTHER`).
8. `InvocationEditConflict` is re-exported from `types.ts` but no component/hook names it.

## Spec

**(a) Missing / partial:** nothing material. E3–E11 are all present: writer + corpus round-trip (`tests/frontmatter_edit.rs` loops the ticket's corpus × 4 modes, asserts parse and byte-exact restore, plus "writing the mode the file already has is a no-op"); table DDL matches the ticket verbatim (+ FK cascade); command shape; replay in `finalize_and_propagate_unlocked` between `finalize_update` and `propagate_unlocked` with `content_hash` recomputed *after* the rewrite and the skill row upserted before Propagation reads it; Restore and Re-point both reach the seam (`repoint_and_update` → `refresh_managed_skills` → `apply_one_unlocked`; traced, and tested in `restore_and_repoint_replay_the_edit`); tests (a)–(h) all present; DTO `| null`; badge `<button>` with `stopPropagation`, `aria-label`, `aria-haspopup`, focus-visible CSS; modal on the shared shell, closes only on success; hook replaces DTO in place (test asserts one `getManagedSkills` call and referential identity of the untouched sibling); EN + ZH copy; CONTEXT.md **Edit**/**Fork** match E11 with Update/Refresh/Re-point cross-links. Only a "property test" in the QuickCheck sense is absent — the ticket's wording ("for every mode × a corpus") is satisfied.

**(b) Not asked for (all benign):** `ON DELETE CASCADE` on the new table (stronger than "delete the rows"); a new **Update** glossary entry (none existed to cross-link from); `had_frontmatter` + `repeated_lines` in `base_value` (declared deviation 2 — required for the ticket's own "create → clear is byte-identical" rule with an empty upstream block, and for the duplicate-key corpus case); Save enabled on an unchanged-but-conflicted override (declared deviation 1 — **correct**: it is the only way to re-base while keeping the same mode, and the backend clears `conflict` on that path); the Refresh completion toast turning `kind: "warning"` when only conflicts occurred; `useExploreState.ts` gains `invocation_override: null` on its placeholder DTO (required by tsc, minimal).

**(c) Implemented but questionable:**
- `replay_unlocked`: `edit.conflict |= upstream_mode != base_mode` then `conflict.then_some(..)` — a persisted conflict is re-emitted on *every* later unchanged Update with `base_mode == upstream_mode`, so the panel says "upstream changed its invocation mode to X" each time. Spec E6 says the flag persists (test asserts it); re-reporting is a reading of it, but the copy overstates ("changed").
- Upstream converging *to* the override's mode still flags a conflict (base ≠ upstream, override == upstream). Spec-literal; UX-wise it could auto-resolve.
- `SignalError::CentralPathMissing` for "dir present, `SKILL.md` gone" (`manifest()`) surfaces as "The skill's central copy is missing from the Skills Hub library: …/SKILL.md". Typed (good), slightly misleading path; acceptable.
- Badge `disabled` for `central_missing`: pre-empts E8's typed refusal, and a disabled button drops its `title` tooltip in some engines.
- Writer on a no-frontmatter CRLF file creates the block with `\n` (mixed endings until cleared). Restore is exact; cosmetic.

## Summary

**Blocking:** none. I could not construct a corpus case that corrupts a `SKILL.md` or leaves row and bytes disagreeing on a success path: the parser and writer agree on header detection (`trim() == "---"`, unclosed `---` = no frontmatter, indented lines skipped, duplicate keys last-wins vs. all-replaced), positions of replaced lines are stable so the positional `repeated_lines` restore is sound, and re-basing happens on every replay before the rewrite.

**Follow-up (ordered by value):**
1. Partial-failure ordering in `set_invocation_override`: file is rewritten *before* `upsert_skill_edit`; if the upsert fails the edit is on disk with no row and a stale `content_hash`, and the next Update silently drops it. Upsert-then-write (a row without bytes self-heals on the next replay) is the safer order. Same class as the declared replay limitation, but this one is cheap to fix.
2. `apply_to_file` uses `fs::write` in place (crash-truncation window). `finalize_update` documents "failure-atomic, not crash-atomic", so this matches the repo's bar; a temp-file + rename would exceed it at trivial cost.
3. Re-emission of a persisted conflict (Spec (c) first bullet) — either carry `conflict` only when newly detected or reword the warning to "still in conflict".
4. Standards smells 1–4 (DTO duplication, enum key round-trip through serde_json, whole-catalog build under the guard, mutable toast flag).
5. Drop the redundant `every_mode_round_trips_without_frontmatter` test and the unused `InvocationEditConflict` re-export; add path context to the `read_to_string` in `set`.
