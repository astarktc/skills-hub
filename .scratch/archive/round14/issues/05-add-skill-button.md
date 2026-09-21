# 05 Add Skill entry point on My Skills

Status: done — 13eab13
Lane: C (after 04)
Source: BACKLOG #34 (operator 1.2.13 smoke)

`addFlow.handleOpenAdd` (the Local / Git add modal) is reachable only from the Explore page's "Manual" button
(`src/App.tsx:380` `onOpenManualAdd`; copy `manualAdd` / `manualAddHint`). The My Skills toolbar
(`src/components/skills/FilterBar.tsx`, props `onRefresh` / `onUnsyncAll` / `onConfigureTools` / sort / group / view)
has no way to add a skill, so the operator detours through Explore.

Add a primary **Add Skill** button to the My Skills toolbar: new `onAddSkill: () => void` prop on `FilterBar`, App
wires `addFlow.handleOpenAdd` (same handler Explore uses — one flow, one modal). Placement: leading position in the
toolbar's action group, primary style, before Refresh; reuse existing button classes from `src/App.css` — no new CSS
pattern, no CSS Modules. Copy: new i18n key in **both** `en` and `zh` in `src/i18n/resources.ts` (e.g.
`addSkillButton: "Add skill"` / `"添加 Skill"` — the modal title key `addSkillTitle` already exists at `:102`/`:719`;
reuse its wording). Paste raw CJK; Edit `oldText` must be verbatim.

Check the toolbar still wraps sanely at the narrowest layout the app supports (open `npm run dev` and resize; the
Tauri window's min size is in `src-tauri/tauri.conf.json`). Read the impeccable skill's guidance on toolbar
hierarchy if you need to choose between primary and secondary styling — one primary per toolbar.

No tests required beyond `npm run lint && npm run build` (components get no JSX tests by design); if you touch
`useAddSkillFlow`, its tests must stay green.

## Done when

My Skills toolbar has an Add Skill button opening the same Local/Git modal as Explore's Manual; EN+ZH keys; gate green.

## Comments

### 2026-09-21 — lane C (Fable)

Done.

- `src/components/skills/FilterBar.tsx`: new prop `onAddSkill: () => void`; a `btn btn-primary` button with a lucide
  `Plus` (14px, matching the sibling icons) and `t("addSkillButton")`, `disabled={loading}` like its neighbours, placed
  **first** in `.filter-bar-right` — before Auto-sync / Configure Tools / Uninstall / search / Refresh. It is the only
  primary in the toolbar (impeccable: one primary per toolbar, accent reserved for the primary action; the toolbar had
  zero primaries before). No new CSS — existing `.btn` / `.btn-primary` / `.btn:disabled`.
- `src/App.tsx`: `onAddSkill={addFlow.handleOpenAdd}` — the same handler Explore's Manual button gets via
  `onOpenManualAdd`; one flow, one modal. `useAddSkillFlow` untouched.
- `src/i18n/resources.ts`: `addSkillButton: "Add skill"` (en) / `"添加 Skill"` (zh), next to `addSkillTitle` in both
  locales, same wording.

Visual check (Vite dev on port 5174 via `VITE_DEV_PORT` — another project held 127.0.0.1:5173; t3-code `preview_*`,
no Tauri backend so the list is the empty placeholder; `tauri.conf.json` sets no `minWidth`, default window 1920):

- 1920 wide: one row, Add skill leads the right group —
  `/Users/alexstark/.t3/userdata/browser-artifacts/browser-screenshot-localhost-mube5ozg-14ce32f3.png`
- 800 wide: the right group wraps onto two tidy rows under the sort/view row, Add skill first —
  `/Users/alexstark/.t3/userdata/browser-artifacts/browser-screenshot-localhost-mube5ybl-f0400a93.png`
- 1280 wide: right group already wrapped to a second row before this change (it was ~1440px of content); unchanged in
  kind.
- Clicking the button opened the modal (`Add skill ✕ Local Folder / Git Repository …`); toggling to 中文 rendered
  `添加 Skill` on the same `btn btn-primary`; language restored to EN afterwards (it is a persisted preference).

Evidence: `npm run lint` clean; `npm run test` 15 files / 358 passed; `npm run build` (typescript-7) ✓;
`lens_diagnostics mode=all` no errors. Dev server stopped after the check.

- 2026-09-20 (parent) — closed `done — 13eab13`.
