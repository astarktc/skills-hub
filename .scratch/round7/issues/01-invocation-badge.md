# 01: Invocation badge — icons only, always shown, tooltip carries the text

Status: done — d7afd4f

**Lane:** A  **Files:** `src/components/skills/InvocationModeBadge.tsx`, `src/App.css`
(`.invocation-badge*` only), `src/i18n/resources.ts` (en + zh). Frontend only; no Rust, no bindings.

## Today
`InvocationModeBadge` returns `null` for `user-and-model`; for the other three modes it renders icon + label text with
`title`/`aria-label` = the long explanation. Used once, `SkillCard.tsx:136`.

## Decision (spec E1/E2)
- Always render one pill:
  - `user-and-model` → `<User/>` + `<Bot/>` side by side (two 11px icons in the pill), **neutral/quiet** styling
    (new `.invocation-badge.user-and-model` rule — muted colour, no accent) so restricted modes still stand out;
  - `user-only` → `<User/>`; `model-only` → `<Bot/>`; `neither` → `<EyeOff/>` — existing colours.
  - No label text inside the pill.
- `title` = `"<label> — <explanation>"` (e.g. "User only — Only you can invoke this skill…"); `aria-label` = the label.
- i18n: add `invocationMode.userAndModel` ("User & model") and `invocationMode.userAndModelTooltip` ("Both you (via
  /skill-name) and the AI agent can invoke this skill — the default.") to **both** `en` and `zh` (translate the zh
  yourself in the style of the neighbouring keys). Existing `userOnly`/`modelOnly`/`neither` labels stay (they feed the
  tooltip prefix now).
- Keep the pill's DOM shape simple (one `<span class="invocation-badge <mode>">` with icon children) — round-7 ticket 02
  will make it clickable and add an "overridden" marker; leave room, don't pre-build it.
- Check the card layout at narrow widths still fits (the pill is now always present); adjust `.invocation-badge`
  padding/gap if needed. No other CSS changes.

## Tests / gate
Components have no JSX tests by design (AGENTS.md). `npm run lint && npm run test && npm run build` green. Do NOT run
`npm run tauri:dev`. If you want a visual check, `npm run dev` + a browser is fine (no backend needed for the badge).

Commit on your branch (`feat(badge): icon-only invocation badge for every mode`). Do NOT push/merge/rebase. `.scratch/`
is gitignored — never `git add -f` it. Paste `## Comments` in the final message.

## Comments

- 2026-09-15 — Status reconciliation: ready → done — d7afd4f. Evidence: git log v1.2.5..v1.2.6 finds d7afd4f: this round7 badge ticket shipped early in v1.2.6, before Edit in v1.2.7. src/components/skills/InvocationModeBadge.tsx:16 renders the default dual-icon mode.
