# 05 — Frontend architecture review (`src/` only)

Status: ready-for-agent
Spec: `.scratch/round18/spec.md` — ticket 05.

## Work

Run the `improve-codebase-architecture` skill (read its SKILL.md and HTML-REPORT.md; `codebase-design` vocabulary:
module / interface / depth / seam / adapter / leverage / locality / deletion test) scoped to `src/` — the 613-line
`App.tsx` binder, `SkillDetailView` (597), `SettingsPage` (449), `AssignmentMatrix` (449), the hooks, `App.css`'s
482 class sites, `reportOutcome.ts`, `skillPresentation.ts`. Rust core is **out of scope** (already deepened).
Read `CONTEXT.md` and `docs/adr/` first; do not re-litigate ADRs 0001–0005.

Deliver `.scratch/round18/review/architecture.html` (self-contained; Tailwind via
`https://cdn.jsdelivr.net/npm/@tailwindcss/browser@4`, never `cdn.tailwindcss.com`; Mermaid from jsdelivr; dark
cards get a light inset for diagrams) **and** `architecture.md` (the same candidates as text, ranked, each with
files / problem / solution / benefits / strength). Include as candidates at minimum: the styling module (D3 in the
spec — evaluate it, don't assume it), the `App.tsx` binder's growth, the modal/route state, and any hook whose
interface is as wide as its body. Stop before proposing interfaces; the grill (ticket 09) picks.
