# Round 7 — Invocation badge redesign + Skill Edit V1 (invocation-mode override)

Grilled 2026-09-08; every decision is the operator's ruling. Base: `main` after the round-6 fix lane lands.

## Context (from the operator)
Edit V1 is the seed of a future **Fork** capability: keep the upstream skill copy, layer operator edits on top, keep
receiving upstream updates on Refresh, replay the edits git-style, and flag "merge conflicts" when upstream changed the
same thing. V1 = one edit kind (invocation mode). Design so the data model survives that future without a rewrite.

## Facts
- Invocation mode is derived at list time from the **central copy's** `SKILL.md` (`skill_catalog.rs:56` →
  `skill_discovery::invocation_mode_for_dir` / `parse_invocation_mode`, line-based reader of `disable-model-invocation`
  and `user-invocable`). Never stored. Four values: `user-and-model`, `user-only`, `model-only`, `neither`.
- Every Tool target is a symlink to the central copy (all adapters `supports_symlink: true`); Propagation handles the
  copy-fallback case. Update/Refresh replaces the central copy wholesale (`finalize_update`, rename-aside).
- Only Claude Code defines these two keys; other Tools ignore unknown frontmatter.
- Badge: `src/components/skills/InvocationModeBadge.tsx`, used once in `SkillCard.tsx:136`; renders nothing for
  `user-and-model`, else icon + label text with `title` = explanation. CSS `.invocation-badge{,.user-only,.model-only}`
  in `App.css:422–445`.

## Decisions
- **E1 (badge rendering)**: always show one pill. `user-and-model` → `User`+`Bot` icons side by side, neutral/quiet
  styling; `user-only` → `User`; `model-only` → `Bot`; `neither` → `EyeOff`. No label text inside the pill.
- **E2 (tooltip)**: `title` = "<label> — <explanation>"; `aria-label` = label. New `invocationMode.userAndModel` +
  `userAndModelTooltip` keys in `en` and `zh`.
- **E3 (mechanism)**: edit in place — rewrite the two frontmatter keys in the central copy's `SKILL.md`; every Tool sees
  it through the symlink; record the edit in the DB and **replay it after every finalize** (Update / Refresh / Restore /
  Re-point). No pristine second copy in V1.
- **E4 (persistence)**: new table `skill_edits(skill_id, kind, value, base_value, conflict, applied_at)`, one row per
  (skill, kind); V1 `kind = "invocation_mode"`. Migration via the existing `migrate_legacy_db_if_needed` path.
- **E5 (byte-exact restore)**: `base_value` stores the original frontmatter **lines** for the two keys (or their absence)
  so clearing the edit restores upstream text exactly and `content_hash` returns to upstream's.
- **E6 (refresh + conflict)**: after finalize, replay the edit. If upstream's own mode (parsed from freshly acquired
  bytes) differs from the **base mode** recorded at edit time: the edit still wins, the row is flagged `conflict`, the
  Refresh report carries it as typed per-skill report data (panel warning), the badge shows a conflict state. Resolving =
  re-choose (re-bases) or clear.
- **E7 (control)**: clicking the badge opens a Modal (app Modal shell): radio list of the four modes + "Follow the skill's
  own setting (currently: X)"; one-line note that only Claude Code honours these keys today; conflict banner when flagged.
  Overridden badge carries a small marker; tooltip adds "Overridden — the skill itself says X".
- **E8 (eligibility)**: all provenances incl. imported (no upstream → never conflicts) and Unlocatable `source_missing`
  (central exists); refuse `central_missing` with a typed error.
- **E9 (command)**: `set_skill_invocation_override(skillId, mode | null) -> ManagedSkillDto`; under the mutation guard:
  rewrite frontmatter → upsert/delete edit row → recompute `content_hash` → run Propagation for that skill (existing seam).
  DTO gains `invocation_override: { mode, base_mode, conflict } | null`; `invocation_mode` stays the effective mode.
- **E10 (frontmatter writer)**: new `core/frontmatter_edit.rs` (declare in `core/mod.rs`): replace existing top-level key
  lines in place, append missing ones before the closing `---`, create a frontmatter block when absent, never touch
  indented/nested lines; round-trip property test against `parse_invocation_mode`.
- **E11 (glossary)**: CONTEXT.md term **Edit** — "an operator change layered on a Managed skill's central copy, replayed
  after every Update; V1 kind: invocation mode". **Fork** reserved for the future whole-copy model.
- **E12 (sequencing)**: ticket A (badge) starts immediately in its own worktree (Astra low); ticket B (Edit V1, Astra
  medium) starts after the round-6 review-fix lane merges (shared files: `install_finalize.rs`, `errors.rs`,
  `commands/`, `resources.ts`). B rebases over A (both touch `InvocationModeBadge.tsx`).

## Ticket map
| Lane | Ticket | Model |
| --- | --- | --- |
| A | `issues/01-invocation-badge.md` | Astra low |
| B | `issues/02-skill-edit-v1.md` | Astra medium |

## Closure — 2026-09-15

Shipped: v1.2.6 (badge); v1.2.7 / 51dcae6 (Edit V1). Tickets: 3 terminal (3 done), 0 still open (none).
Residue → BACKLOG: #03, #04, #05, #26, #29. Dropped by name: none.
