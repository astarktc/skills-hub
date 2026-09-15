# Round-7 review — Skill Edit V1 (`r7/skill-edit-v1`, `ac8b514...a261e89`)

Reviewer: `opus`. Verified at HEAD: `cargo test --all` 567 passed, `npm run test` 241 passed,
`npm run build` clean, `npm run version:check` OK (1.2.6), `git status --porcelain` empty after
`cargo test` (no binding drift). One commit; additions plus small wiring, no v1.2.6 hunk reverted.

## Standards

**No hard violations found.** Every invariant I checked holds:

- Mutation guard (AGENTS.md "Sync-target mutation"): `skill_edits::set_invocation_override` wraps its
  own body in `mutation_guard::serialized` and calls only unlocked seams inside (`propagate_unlocked`,
  store reads); no entry point calls another. `replay_unlocked` is `pub(crate)` and documents that the
  caller holds the guard.
- Command in `collect_commands!`, `#[tauri::command] + #[specta::specta]`, `spawn_blocking`,
  `commands/` stays wiring-only, bindings committed and clean after `cargo test`.
- `invocation_override: InvocationOverrideDto | null` — wire-accurate `| null`, not `?`.
- i18n: every new key in both `en` and `zh`. No backend user-facing prose; the two `.context(...)`
  strings guard conditions the UI cannot produce, which AGENTS.md explicitly permits.
- `frontmatter_edit` / `skill_edits` declared in `core/mod.rs`; `frontmatter_edit` is pure
  `&str → String` with one `apply_to_file` adapter; no `dirs::*` / `AppHandle` in core.
- Presentation stays pure and single-implementation: the badge's two 4-arm ternaries became
  `INVOCATION_LABEL_KEY` / `INVOCATION_TOOLTIP_KEY` in `skillPresentation.ts` — this *removes* a
  Repeated Switches smell.

Baseline smells (judgement calls):

1. **Duplicated Code — `commands/mod.rs:1217`.** `InvocationOverrideDto { mode, base_mode, conflict }`
   is a field-for-field clone of `core::skill_edits::InvocationOverride`, while its neighbour
   `SkillRefreshStatusDto::Refreshed { edit_conflict: Option<crate::core::skill_edits::InvocationEditConflict> }`
   puts the *core* type on the wire directly. Two rules for two adjacent types. *Fix*: drop the mirror
   and use `InvocationOverride` (AGENTS.md allows DTOs living in `core/`), or add a conflict Dto — one rule.
2. **Primitive Obsession / Mysterious Name — `frontmatter_edit.rs`.** Both keys are addressed by a bare
   `usize`: `fn line(&self, key: usize)`, `found[key]`, `values[key]`, `KEYS[key]`,
   `key_index(line) -> Option<usize>`. *Fix*: a two-variant `Key` enum so `line(Key::UserInvocable)` reads.
3. **Round trip through serde to get a string — `skill_edits.rs:110`.**
   `serde_json::to_value(mode)?.as_str().context("invocation mode must be a string")?.to_string()`.
   *Fix*: give `InvocationMode` an `as_key()`, as `SkillEditKind` already has, and use it on both the
   write and the `edit_mode` read side.

## Spec

**(a) Missing / partial.** Nothing material. Ticket tests (a)–(h) all exist in
`core/tests/skill_edits.rs`; the frontmatter corpus is property-style over 4 modes × 14 inputs and
covers every listed shape (no frontmatter; neither/one/both keys; odd order; nested indented same-name
keys; CRLF; content after `---`; duplicate top-level keys; unterminated block), asserting
`parse(write(t,m)) == m`, `restore(write(t,m), read(t)) == t` and the `write(t, parse(t)) == t` no-op.
Two partials:

- E9: *"add a one-skill variant of `managed_skill_catalog`, or filter"*. Filter was taken
  (`skill_edits.rs:143`, `managed_skill_catalog(store)?.into_iter().find(...)`) — it builds the **whole**
  catalog (a filesystem read plus two queries per skill) **inside the Mutation guard** to return one
  entry. Permitted, but it stalls every other mutation on an O(n) sweep per Save.
- E10: *"Never touch indented/nested lines"*. `header_end` closes the block at the first line whose
  **trim** is `---`, so an indented `---` inside a block scalar (`description: |`) is read as the fence
  and appended keys land inside the scalar. This matches the inherited `parse_invocation_mode` /
  `parse_skill_md_with_reason` rule, so read and write stay self-consistent and the round trip still
  restores — but the writer now acts on that boundary.

**(b) Scope creep.** All small and defensible. `had_frontmatter` + `repeated_lines` (declared deviation
2) are required for the ticket's byte-identical "create → clear". Save enabled on an unchanged-but-
conflicted override (deviation 1, `InvocationModeModal.tsx:21`) is **right**: the ticket's own banner
says *"Keeping your override re-bases it"*, and with Save disabled there is no way to re-base except
clear-and-re-set. Beyond the ticket's "closable warning" there are two extra toasts —
`invocationEdit.refreshCompletedWithEdits`, and `invocationEdit.warningTitle` reused as the
single-Update success-toast title (`useSkillLibrary.ts:580`). `CONTEXT.md` gained a whole new
`**Update**` entry where the ticket asked only to *cross-link*; there was none to cross-link, so that
reading is fair. Files outside the expected list are all forced: `skill_discovery.rs` (`Deserialize`,
needed by `edit_mode`), `useExploreState.ts` (one type-forced `invocation_override: null`),
`SkillsList.tsx` (pass-through), `skillPresentation.ts` (pure maps).

**(c) Implemented but wrong.** None. Traced and confirmed: replay sits between `finalize_update` and
`propagate_unlocked`, and `record_hash` re-hashes **after** the rewrite and upserts the skill row
**before** Propagation reads it (`installer.rs:389-390`). `edit.base_value = to_string(&upstream)` runs
on every replay (so a later clear restores *current* upstream), `edit.conflict |= upstream_mode != base_mode`
persists an unresolved conflict across unchanged Updates, and `set(Some)` writes `conflict: false` while
keeping `base_value` — exactly E6's re-base. Restore and Re-point both reach the seam
(`repoint_and_update → refresh_managed_skills → apply_one_unlocked → finalize_and_propagate_unlocked`),
asserted by `restore_and_repoint_replay_the_edit`. Deletion cascades: `ON DELETE CASCADE` plus
`PRAGMA foreign_keys = ON` on every connection (`skill_store.rs:1373`) — the same mechanism
`skill_targets` uses; Detach keeps the row and correctly keeps the edit. `CentralPathMissing` reuse is
what the ticket named, and the reported path is `<central>/SKILL.md`, so the copy is not misleading.
Badge: `<button>` with `stopPropagation`, `aria-haspopup="dialog"`, `aria-label` + "Overridden",
`:focus-visible`, disabled on `central_missing` (`.skill-card` is a plain `<div>` with no `onClick`, so
click-through was never possible). Hook replaces the DTO by id with no success-path refetch, and closes
only after `await`, so a failure leaves modal and list untouched — both asserted.

## Summary

### Blocking
None. No documented-standard breach, no missing or incorrect spec requirement, and no path that
corrupts a `SKILL.md`, desyncs row vs bytes, or loses an operator's edit in normal operation. Both
declared deviations are correct. The declared limitation (no finalize rollback on replay failure)
leaves a self-consistent state: if `apply_to_file` fails, bytes and the finalize-written hash both say
"upstream" and the skill is reported `Failed`.

### Follow-up
1. `skill_edits.rs:143` — swap the full `managed_skill_catalog(store)` sweep for the ticket's one-skill
   variant; today every Save does O(skills) filesystem reads while holding the global mutation mutex.
2. `frontmatter_edit.rs::header_end` — an indented `---` inside a frontmatter block scalar is read as
   the closing fence. Inherited rule, but the writer now mutates bytes on it; narrow it to a
   non-indented `---` in `frontmatter_edit`, `parse_invocation_mode` and `parse_skill_md_with_reason`
   *together* (changing one alone would desync read from write).
3. `skill_edits.rs::replay_unlocked` — bytes are written before `upsert_skill_edit`; if that upsert
   fails, `base_value` stays stale and a later clear restores *stale* upstream text. This is the only
   row-vs-bytes window I found; ordering the store write first closes it.
4. `restore_invocation_lines` deletes the entire frontmatter block whenever `base.had_frontmatter` is
   false, assuming the writer created it. Unreachable for a Managed skill today (`installer.rs:623`
   admits only a `SKILL.md` with a closed block carrying `name:`), but an out-of-band hand edit between
   set and clear would drop that block. Consider gating removal on the block still holding only the two
   keys the writer wrote.
5. Smells 1–3 above: the `InvocationOverrideDto` duplicate, the `usize` key indexing, and the
   `serde_json::to_value(...).as_str()` mode-key round trip.
6. `useSkillLibrary.ts:580` — reusing `invocationEdit.warningTitle` ("{{name}}: invocation Edit
   conflict") as a *success* toast title for a single Update reads oddly beside the identical
   warning-entry title (`:267`); a dedicated string would be clearer.
