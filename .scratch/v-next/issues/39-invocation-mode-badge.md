# 39: Invocation-mode badge in My Skills

Status: resolved

Type: task (research first, then implement)
Source: fog item "surface each skill's invocation mode in My Skills UI" (map); target release 1.2.1

## Question to answer first

What SKILL.md frontmatter does the backend already parse (`core/skill_discovery.rs`, and anything
else that reads frontmatter — `skill_files.rs`, `installer.rs`, the frontend markdown metadata table
from 0.4.1)? Record the fields, where they are parsed, where they are stored (DB column vs.
computed at list time), and which DTO carries them to the UI.

Then, against the current Agent Skills spec (agentskills.io) and Claude Code's skill docs (primary
sources, cite URLs), establish the frontmatter keys that determine **who can invoke a skill**:
`disable-model-invocation`, `user-invocable` (or whatever the current names are), and their defaults.
Derive the invocation modes to show, e.g. *user + model* (default), *user only*, *model only*.

## What to build

A small badge on each skill card/row in **My Skills** showing the invocation mode, sourced from the
skill's SKILL.md frontmatter.

- Backend: parse the invocation keys wherever frontmatter is already parsed; expose one typed field
  (an enum — e.g. `InvocationMode { UserAndModel, UserOnly, ModelOnly }` deriving `specta::Type`) on
  the managed-skill DTO. Prefer computing at list time from the central copy's SKILL.md over adding
  a DB column, unless the existing design already persists frontmatter (then follow it — check
  `migrate_legacy_db_if_needed` in `core/skill_store.rs` before any schema change). Missing/absent
  frontmatter ⇒ default mode, never an error.
- Frontend: badge in the My Skills list (`src/components/skills/…`), DTO type re-exported through
  `src/components/skills/types.ts` (never import `src/bindings` directly), i18n keys in **both** `en`
  and `zh` (`src/i18n/resources.ts`; the parity test enforces it), tooltip explaining the mode.
  Default mode may be rendered as no badge or a muted badge — decide and record.
- Tests: core table test for the frontmatter → mode mapping (including missing keys and malformed
  values); hook test only if a hook changes.

## Acceptance criteria

- [ ] Ticket `## Findings` section answers the frontmatter question with file:line pointers and cites the spec for the invocation keys.
- [ ] `InvocationMode` (or equivalent) crosses IPC via the generated binding; `cargo test` regenerates `src/bindings/index.ts` and the diff is committed.
- [ ] Badge visible in My Skills; en + zh strings; no hardcoded UI text.
- [ ] `npm run version:check && npm run check` green.

## Findings

### 1. Frontmatter the backend already parses

Only two fields, `name` and `description`, and there is exactly one parser:

- `src-tauri/src/core/skill_discovery.rs:270` `parse_skill_md` → thin `ok()` wrapper over
  `parse_skill_md_with_reason` (`:275`), a hand-rolled line scanner (no YAML crate): requires `---` on
  line 1, reads `name:` (`:297`) and `description:` (`:302`, incl. `|` / `>` block scalars), requires a
  closing `---`, and reports failures as stable tokens `read_failed` / `invalid_frontmatter` /
  `missing_name` (`:283`, `:332`, `:336`). Values are unquoted by `clean_frontmatter_value` (`:338`).
- Callers: `skill_discovery::inspect` (validity + name/description for git/local listings),
  `core/install_finalize.rs:259` (name/description at install time). `core/skill_files.rs` does **not**
  parse frontmatter — it only lists (`:19`) and reads (`:58`) files. `core/installer.rs` goes through
  `skill_discovery`.
- Storage: `name` is a `skills` column from the schema's start; `description` was added by migration V2
  (`core/skill_store.rs:252`, `ALTER TABLE skills ADD COLUMN description`). Nothing else from
  frontmatter is persisted — no generic metadata column, so **no schema change was needed** for the
  invocation keys and `migrate_legacy_db_if_needed` is untouched.
- DTO to the UI: `ManagedSkillDto` (`src-tauri/src/commands/mod.rs:624`), built in
  `get_managed_skills_impl` (`:682`) from `SkillRecord` (`core/skill_store.rs:114`) — it carries
  `central_path`, so list-time frontmatter reads are cheap and always fresh.
- Frontend: `SkillDetailView.tsx:246` `parseFrontmatter` is an independent, display-only re-parse (the
  0.4.1 metadata table); it renders every key generically and feeds nothing back into the DTO.

### 2. The invocation keys (primary sources)

- Claude Code, *Extend Claude with skills* — <https://code.claude.com/docs/en/skills> ("Control who
  invokes a skill" + Frontmatter reference):
  - `disable-model-invocation` — "Set to `true` to prevent Claude from automatically loading this
    skill. Use for workflows you want to trigger manually with `/name`. **Default: `false`**."
  - `user-invocable` — "Set to `false` when only Claude should invoke the skill: Claude Code hides it
    from the `/` menu and doesn't run it when you type `/name`. **Default: `true`**."
  - "By default, both you and Claude can invoke any skill."
  - Boolean fields "accept `yes`, `no`, `on`, `off`, `1`, and `0`" besides `true`/`false`; frontmatter is
    read "only when the opening `---` is the file's first line", and malformed YAML degrades to empty
    metadata rather than an error.
  - Setting **both** is the documented way to hide a skill from everyone: "With `user-invocable: false`,
    you can't invoke the skill, but Claude still can. To keep Claude from invoking it through the Skill
    tool, set `disable-model-invocation: true`."
- Agent Skills specification — <https://agentskills.io/specification>: the frontmatter table is `name`,
  `description`, `license`, `compatibility`, `metadata`, `allowed-tools` (experimental). **It does not
  define any invocation key**; adding the Claude Code scheme is still an open proposal
  (<https://github.com/agentskills/agentskills/issues/105>). So the two keys are a Claude Code extension
  the spec tolerates, and unknown keys are harmless elsewhere.

Derived modes (4, not 3 — the both-set combination is a real, documented state):
`user + model` (default) · `user only` (`disable-model-invocation: true`) · `model only`
(`user-invocable: false`) · `neither` (both).

## Answer

Commit: `a22c0831e8a560cc75ef9a2946fdc09ec64a21e3` on branch `t39` (worktree
`~/.worktrees/skills-hub-t39`, not merged).

### Design decisions

1. **Computed at list time, no DB column.** Frontmatter beyond `name`/`description` is not persisted, so
   `get_managed_skills_impl` reads the central copy's SKILL.md per skill. Always fresh (editing SKILL.md
   updates the badge on the next list) and no migration; `migrate_legacy_db_if_needed` untouched.
2. **Lives in `core/skill_discovery.rs`**, the module that already owns SKILL.md frontmatter — so no new
   `core/mod.rs` line (also keeps the t37 merge trivial) and `installer.rs` untouched.
3. **Four variants, not three.** `InvocationMode { UserAndModel (default), UserOnly, ModelOnly, Neither }`
   (`serde(rename_all = "kebab-case")` → `"user-and-model" | "user-only" | "model-only" | "neither"`).
   Both keys set is Claude Code's documented recipe for hiding a skill from user *and* model; collapsing
   it into one of the other three would misreport it.
4. **Never an error.** No frontmatter, no closing `---`, unreadable/absent SKILL.md, or an unrecognised
   boolean spelling ⇒ that key keeps its documented default. Booleans accept
   `true/false/yes/no/on/off/1/0` (case-insensitive, quotes stripped); indented lines are skipped so a
   key nested under `metadata:` cannot be mistaken for a top-level one.
5. **Badge decision: the default mode renders NOTHING.** Most skills are `user + model`, so a badge on
   every card would be pure noise and would dilute the signal the ticket wants (which skills are
   restricted). Restricted skills get a small pill next to the name with icon + short label and a
   `title`/`aria-label` tooltip naming the responsible frontmatter key: *User only* (accent-soft,
   `User` icon), *Model only* (warning-soft, `Bot` icon), *Not invocable* (muted, `EyeOff` icon).

### Files

- `src-tauri/src/core/skill_discovery.rs` — `InvocationMode`, `invocation_mode_for_dir`,
  `parse_invocation_mode`, `parse_frontmatter_bool`.
- `src-tauri/src/commands/mod.rs` — `ManagedSkillDto.invocation_mode` + computation in
  `get_managed_skills_impl` (wiring only).
- `src/bindings/index.ts` — regenerated by `cargo test` (committed).
- `src/components/skills/InvocationModeBadge.tsx` (new), `SkillCard.tsx` (badge in the header row),
  `types.ts` (`InvocationMode` re-export).
- `src/App.css` — `.invocation-badge` (+ `.user-only` / `.model-only` variants).
- `src/i18n/resources.ts` — `invocationMode.{userOnly,modelOnly,neither,*Tooltip}` in `en` **and** `zh`.
- `src/hooks/useExploreState.ts`, `src/hooks/useSkillLibrary.test.ts` — DTO literals gained the new
  required field (no behaviour change).
- `CHANGELOG.md` — `[Unreleased] → Added`.

### Tests

- `core::skill_discovery::tests::invocation_mode_maps_every_frontmatter_shape` — 19-case table: no
  frontmatter, empty file, unterminated frontmatter, keys absent, explicit permissive values, every
  truthy/falsy spelling, quoted value, both-set, malformed/empty values, key nested under `metadata:`,
  key after the closing `---`.
- `core::skill_discovery::tests::invocation_mode_for_dir_defaults_without_readable_skill_md` — missing
  SKILL.md ⇒ default; case-insensitive `Skill.md` lookup honours the restriction.
- No hook behaviour changed, so no new frontend test (existing 98 pass).

### Gate

`cargo test --all`: **326 passed**, 0 failed. `npm run version:check`: Version OK (1.2.0).
`npm run check`: eslint clean · vitest **98 passed** (9 files) · typescript-7 build + vite (704 modules)
clean · `cargo fmt --check` clean · `cargo clippy --all-targets --all-features -D warnings` clean ·
`cargo test` 326 passed. Not verified in a running window (a `tauri:dev` run would touch the operator's
live skill library); the badge is pure presentation over a gate-verified typed field.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
