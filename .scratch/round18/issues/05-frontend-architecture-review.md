# 05 — Frontend architecture review (`src/` only)

Status: implemented
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

## Result

Reviewed at `e4639fe` (read via `git archive`, not the working tree). Deliverables:

- `.scratch/round18/review/architecture.html`: 13 candidate cards with before/after diagrams, a top recommendation,
  and open questions. Tailwind comes from the jsdelivr browser@4 build and Mermaid 11 from jsdelivr. All 10 Mermaid
  diagrams were verified to render in the preview browser.
- `.scratch/round18/review/architecture.md`: the same candidates as text, ranked, each with files and line ranges,
  problem, solution, benefits and strength.

Ranked candidates (by leverage against D8):

1. Surface state (route, selection, inspector target, dialog) as one module — Strong.
2. Skill action set: every verb acts on N skills through one door — Strong (depends on 1).
3. A pure fold for skill health and the source line (health ladder, #49, Synced N/M, copyable source) — Strong, and
   the cheapest.
4. Styling: tokens plus a `src/components/ui` primitives module (D3, evaluated) — Strong, with three corrections:
   - the tokens already exist (518 `var()` reads, 24 literals);
   - `App.css` keeps a prose layer, so the guard needs an exemption;
   - theme resolution is not a module.
5. Library lens (query, sort, group, singleton fold, view mode, visible selection), shared by My Skills, the matrix,
   the pickers and the palette — Strong.
6. Action lifecycle: one Outcome executor (5 hand copies today, and `ProjectsPage` drops warnings), per-subject
   pending instead of a global overlay — Strong.
7. Project world with an app lifetime and one skill catalog — Worth exploring (Strong if inspector project
   assignment ships).
8. One app-update module (the `useUpdateChecker` and `SettingsPage` machines, plus the title-bar affordance) — Strong
   (small).
9. A native platform seam beside `invokeTauri` (folder picker implemented 5×, updater, Channel, zoom) — Worth
   exploring; it touches ticket 04.
10. Add flow exposes intents and keeps form state internal (10 raw setter members; Explore install runs through a
    trigger effect) — Worth exploring.
11. Tool catalog split out of `useSyncOrchestration` (30 members, 2 unused outside the hook) — Worth exploring.
12. Detail file browser as a hook plus pure module fed a skill document — Speculative.
13. The `App.tsx` binder's growth — evaluated as a symptom of 1, 6, 8 and 10 (the deletion test finds pass-through).
    Speculative as a standalone candidate.

Top recommendation: grill 1 together with 2. Candidate 3 can run in parallel, and D9 already sequences 4 first.

Open questions for ticket 09:

- Is the detail an inspector (a selection) or a route?
- Is there one selection across surfaces, or one per surface?
- Should surface state be addressable (a hash route) for fixture-mode deep links?
- Should primitives get role/ARIA tests despite the "no component tests" rule?
- Is the prose layer exempt from the `App.css` guard?
- Keep the one-action-at-a-time rule, or allow concurrent inline actions?
- Does project assignment from My Skills ship in round 19?
- Should the platform seam fold into ticket 04's door or sit beside it?
