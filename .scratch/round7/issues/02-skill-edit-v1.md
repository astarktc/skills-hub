# 02: Skill Edit V1 — invocation-mode override (spec E3–E11)

Status: done — a261e89

**Lane:** B (Astra medium)  **Blocked by:** 01 (merged — `ac8b514` already carries the icon-only badge).
**Files (expected):** `src-tauri/src/core/{frontmatter_edit (new),mod,skill_store,skill_catalog,installer,refresh,errors}.rs`,
`src-tauri/src/core/tests/{frontmatter_edit (new),refresh,skill_edits (new)}.rs`, `src-tauri/src/commands/{mod,error}.rs`,
`src-tauri/src/lib.rs` (`collect_commands!`), `src/bindings/index.ts` (regenerated), `src/components/skills/{types,InvocationModeBadge,SkillCard}.tsx`,
`src/components/skills/modals/InvocationModeModal.tsx` (new), `src/hooks/useSkillLibrary.ts` (+ test), `src/App.tsx`,
`src/commandError.ts`, `src/i18n/resources.ts` (en + zh), `src/App.css`, `CONTEXT.md`.
**Do NOT touch:** `install_finalize.rs` beyond what E5/E10 strictly need (prefer none — replay lives one level up),
`git_acquisition.rs`, `github_download.rs`, `artifact_removal.rs` executor, `propagation.rs` internals, `sync_engine.rs`.

## Problem
Invocation mode (`user-and-model` / `user-only` / `model-only` / `neither`) is read at list time from the central copy's
`SKILL.md` frontmatter (`disable-model-invocation`, `user-invocable`; `skill_discovery::parse_invocation_mode`) and never
stored. Operators cannot change it without hand-editing the central copy, and any hand edit is silently wiped by the next
Update/Refresh (`finalize_update` replaces the central copy wholesale, rename-aside). Only Claude Code honours these keys;
other Tools ignore unknown frontmatter, so the edit is safe to write into the one shared copy.

This is V1 of a future **Fork** capability (keep upstream, layer operator edits, replay them on every update, flag
conflicts). One edit kind now; the data model must survive more kinds without a rewrite.

## Decision

### Mechanism (E3, E10) — `core/frontmatter_edit.rs` (new; declare in `core/mod.rs`)
Line-based writer that pairs with `parse_invocation_mode`. API shape (names are yours; keep the module pure — `&str` in,
`String` out — with one thin `apply_to_file(path, …)` wrapper):
- `read_invocation_lines(text) -> InvocationLines` — the current top-level `disable-model-invocation` / `user-invocable`
  lines (each `Option<String>`, verbatim incl. trailing whitespace) and the mode they parse to.
- `write_invocation_mode(text, mode) -> String` — replace existing **top-level** key lines in place (same position),
  append missing ones before the closing `---`, create a `---\n…\n---\n` block at the top when the file has none. Never
  touch indented/nested lines, never reorder other keys, preserve the rest byte-for-byte (line endings included).
- `restore_invocation_lines(text, base: &InvocationLines) -> String` — put the recorded lines back exactly (or remove
  the keys when the base recorded absence). Removing the last key of a block the writer itself created must remove the
  block too, so a "create → clear" round trip is byte-identical.
- Property/round-trip tests (`core/tests/frontmatter_edit.rs`): for every mode × a corpus of inputs (no frontmatter;
  frontmatter with neither key / one key / both keys / keys in odd order / nested indented keys with the same names /
  CRLF / trailing content after `---`), `parse_invocation_mode(write(text, mode)) == mode` and
  `restore(write(text, mode), read(text)) == text`. Also: writing the mode the file already has is a no-op (bytes equal).

### Persistence (E4, E5) — `skill_store.rs`
New table `skill_edits(skill_id TEXT NOT NULL, kind TEXT NOT NULL, value TEXT NOT NULL, base_value TEXT NOT NULL,
conflict INTEGER NOT NULL DEFAULT 0, applied_at INTEGER NOT NULL, PRIMARY KEY (skill_id, kind))`, created with
`CREATE TABLE IF NOT EXISTS` in the schema init the way `hidden_explore_skills` was added (that IS the migration path — a
pre-existing DB gets the table on open; confirm `migrate_legacy_db_if_needed` needs nothing more). V1 `kind =
"invocation_mode"`; `value` = the mode key string (`user-only` …); `base_value` = the recorded `InvocationLines` serialised
as JSON (the two optional verbatim lines) so clearing restores upstream text exactly and `content_hash` returns to
upstream's. Store API: `get_skill_edit(skill_id, kind)`, `upsert_skill_edit`, `delete_skill_edit`, and delete the rows
when the skill row is deleted (same place `skill_targets` rows go). A typed `SkillEditRecord` in core; `kind` is a small
enum with `as_key`, not a bare string in core code.

### Command (E8, E9) — `commands/mod.rs` + `lib.rs`
`set_skill_invocation_override(skill_id: String, mode: Option<InvocationMode>) -> Result<ManagedSkillDto, CommandError>`
(`#[tauri::command] + #[specta::specta]`, listed in `collect_commands!`, `spawn_blocking`). Core entry point (e.g.
`core/skill_edits.rs::set_invocation_override`, a **mutation entry point** wrapping its own body in
`mutation_guard::serialized`; commands carry no lock state):
1. Load the record; if the central copy is gone → `SignalError::CentralPathMissing { path }` (already exists — reuse, no
   new variant). All provenances are eligible, incl. imported and `source_missing`.
2. `Some(mode)`: if no edit row exists, `base_value := read_invocation_lines(current SKILL.md)`; if one exists, keep its
   `base_value` (the base is upstream's text, which the operator's previous edit already displaced). Write the mode,
   upsert the row (`conflict = 0` — choosing a mode resolves a conflict, i.e. re-bases: when the row was in conflict,
   `base_value` := the lines recorded by the last replay, see below).
   `None`: if a row exists, `restore_invocation_lines(base_value)` and delete the row; if none, no-op.
3. Recompute `content_hash`, upsert the skill row (`updated_at`), then `propagate_unlocked` for that skill (existing
   seam) so copy-fallback targets get the bytes. Propagation's per-target failures are report data; log them here (the
   command answers with the DTO, not a report — V1 keeps the command shape the spec fixed).
4. Return the single skill's catalog entry as `ManagedSkillDto` (add a one-skill variant of `managed_skill_catalog`, or
   filter — do not duplicate the entry-building).

### Replay after finalize (E6) — `installer.rs::finalize_and_propagate_unlocked`
This one seam is where Update, Refresh, Restore and Re-point all land (verify `unlocatable::repoint_and_update` reaches it
too). Between `finalize_update` and `propagate_unlocked`:
- If the skill has an `invocation_mode` edit: read the **freshly landed** central `SKILL.md`'s lines (`upstream_lines`) and
  parse their mode; compare with the mode parsed from the row's `base_value` (**base mode**). If they differ → the edit
  still wins, set `conflict = 1`. Either way, `base_value := upstream_lines` (so a later clear restores *current* upstream,
  never stale text), write the recorded mode, recompute `content_hash`, upsert the skill row. Then Propagation as today.
- `UpdateOutcome` gains `edit_conflict: Option<InvocationEditConflict { base_mode, upstream_mode, override_mode }>`;
  `SkillRefreshStatus::Refreshed` carries it; `SkillRefreshStatusDto::Refreshed` mirrors it (specta). Frontend: the
  Refresh/Update warning toast/panel (`useSkillLibrary` entries builders) lists "<skill>: upstream changed its invocation
  mode to X; your override (Y) is kept" — closable warning like the other per-skill warnings, i18n en+zh.
- A replay failure (write/hash/upsert) is a finalize failure for that skill (`SkillRefreshStatus::Failed`) — the central
  copy already holds upstream bytes; do not attempt rollback of finalize (out of scope, note it in Comments).
- Tests (`core/tests/refresh.rs` or a new `skill_edits.rs` test module): (a) Update with an override and unchanged
  upstream mode → override persists, no conflict, `content_hash` ≠ upstream's; (b) upstream changes its own mode → override
  persists, `conflict` set, report carries it, `base_value` now equals the new upstream lines; (c) clear after (b) →
  central bytes == freshly acquired upstream bytes, `content_hash` == upstream's, row gone; (d) Restore of a
  `central_missing` skill replays; (e) Re-point replays; (f) `set` refuses `central_missing` with the typed error;
  (g) `set` on an imported skill works and never conflicts; (h) Propagation runs after `set` (a copy-fallback target
  receives the new bytes — reuse the existing test double for a non-symlink target if one exists).

### DTO (E9) — `skill_catalog.rs` + `commands/mod.rs`
`ManagedSkillEntry` gains `invocation_override: Option<InvocationOverride { mode, base_mode, conflict }>` (read the edit
row at list time; `base_mode` parsed from `base_value`). `ManagedSkillDto` mirrors it as
`invocation_override: InvocationOverrideDto | null` (wire-accurate `| null`, not `?`). `invocation_mode` stays the
effective mode (still read from the central copy — it now reflects the edit). Re-export the new DTO from the skills shim
`types.ts`.

### UI (E7) — badge + modal + hook
- `InvocationModeBadge` becomes a `<button type="button" class="invocation-badge <mode> …">` (keep the pill look; add
  `:hover`/`:focus-visible` affordance in `App.css`). When `invocation_override` is set: an `overridden` modifier class
  with a small marker (e.g. a 6px dot or a `Pencil` 9px icon in the pill's corner), tooltip gains
  " — Overridden; the skill itself says <base label>"; when `conflict`: a `conflict` modifier (warning colour) and the
  tooltip says upstream changed. `aria-label` = label (+ "overridden"). Card click-through: the badge's click must not open
  the detail view (`stopPropagation`).
- `InvocationModeModal.tsx` (new, on the shared `Modal` shell like `GitRepointModal`): title "Invocation mode — <skill>";
  radio list: the four modes (label + one-line explanation, reuse the existing `invocationMode.*Tooltip` keys) plus
  "Follow the skill's own setting (currently: <base label>)"; a one-line note "Only Claude Code honours these keys today;
  other tools ignore them"; a warning banner when `conflict` ("The skill's upstream changed its own setting to X after you
  overrode it. Keeping your override re-bases it; 'Follow the skill's own setting' clears it."); Save / Cancel; Save
  disabled when the selection equals the current state. Save calls the hook action; the modal closes on success, stays
  open on failure (same pattern as Re-point).
- `useSkillLibrary`: owns `invocationEditSkillId` (+ open/close), `setInvocationOverride(skillId, mode | null)` via
  `runAction` → `invokeTauri("setSkillInvocationOverride", …)`, applies the returned DTO in place (replace by id in
  `managedSkills`) — no refetch on success. `App.tsx` binds the modal. `SkillCard` passes `onInvocationClick`.
  Hook test: set → DTO replaced in place; failure → list untouched, modal stays open.
- i18n: every new string in **both** `en` and `zh` (translate zh yourself in the neighbouring style). New
  `describeCommandError` copy only if you add a variant (you should not need one — `CENTRAL_PATH_MISSING` exists).

### Glossary (E11) — `CONTEXT.md`
Add **Edit**: "an operator change layered on a Managed skill's central copy, recorded in `skill_edits` and replayed after
every finalize (Update / Refresh / Restore / Re-point); on conflict — upstream changed the same thing — the Edit wins and
the row is flagged until the operator re-chooses or clears. V1 kind: invocation mode." Reserve **Fork** ("future:
whole-copy operator variant of an upstream skill; not built"). Cross-link from **Update**/**Refresh**/**Re-point** entries
with one clause each ("…then replays Edits").

## Out of scope
Any second edit kind; a pristine upstream copy; diff/merge UI; a backup sweep; `repointDoor` body (backlog 7 — leave it).

## Gate
`npm run version:check && npm run check` green (vitest, cargo `--all`, lint, build, fmt, clippy); `src/bindings/index.ts`
regenerated and committed, not dirty after `cargo test`. Conventional commits on your branch, one per layer is fine
(`feat(edit): frontmatter writer`, `feat(edit): skill_edits store + replay`, `feat(edit): override command + DTO`,
`feat(edit): invocation modal + badge`, `docs(context): Edit / Fork`). Do NOT push/merge/rebase. Do NOT run
`npm run tauri:dev` (mutates the operator's real skill library). `.scratch/` is gitignored — never `git add -f` it.
Paste `## Comments` in the final message: what you built, every deviation from this ticket and why, files touched outside
the expected list, and known holes.

## Comments

- 2026-09-15 — Status reconciliation: ready-for-agent → done — a261e89. Evidence: git log v1.2.6..v1.2.7 finds a261e89; src-tauri/src/core/skill_edits.rs:102 persists the override and :164 replays it; src-tauri/src/core/tests/skill_edits.rs:112 covers update plus exact clear.
