# 05 Add Skill entry point on My Skills

Status: ready-for-agent
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
