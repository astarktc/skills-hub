# 06 — Two independent design critiques + accessibility audit

Status: ready-for-agent
Blocked by: 04
Spec: `.scratch/round18/spec.md` — D1, D5, D6, D10, ticket 06.

## Work

Two children, different models (Astra high; Opus 5.5 high), same brief, no knowledge of each other, each producing
one file: `.scratch/round18/review/critique-astra.md` / `critique-opus.md`. A third child (or the Astra child in a
second pass) produces `audit.md`.

Critique brief: run impeccable `critique` (read `.pi/skills/impeccable/reference/critique.md` fully; `operate.md`;
`craft-floor.md`) over **every surface of the incumbent app** in fixture mode (`npm run dev:fixture`, scenarios
`rich` and `failures`, dark and light), captured with `preview_*`: My Skills list + card, Skill detail, Add flow
(GitHub pick, local pick), Projects + matrix, Explore + explore detail, Settings, Import/onboarding, Tool config
modal, notifications, every confirmation. Heuristic scores per surface; findings ranked by severity with a
screenshot path each; a "what to keep" list (the incumbent has real strengths — name them). Judge against
`PRODUCT.md`, the direction contract (`.impeccable/surfaces/src-components-skills-skillslist-tsx.md`) and the
Emil checklist (`.pi/skills/emil-design-eng/SKILL.md` § Review Checklist, Before/After table format for motion
findings). `ui-ux-pro-max`'s `references/quick-reference.md` priority table is the rubric for the a11y/interaction
rows. Do **not** propose a new look — the world is decided; propose what each surface must do inside it.

Audit brief: impeccable `audit` (`reference/audit.md`) — keyboard completion of the six core flows, focus
visibility, AA contrast both themes (compute, don't eyeball), reduced-motion, 960×640 behaviour, tab order in
modals, ARIA on custom controls. Output `audit.md` as a table: surface · check · pass/fail · evidence path.

## Result — Opus

File: `.scratch/round18/review/critique-opus.md`. Screenshots (local, gitignored):
`.scratch/round18/evidence/critique-opus/` (44 PNGs). Whole app **17/40 (Poor)**; per surface from My Skills 13/40
to Settings 29/40. Both raw-key leaks confirmed and placed (Import "Found in claude_code" ×31; matrix headers
`CLAUDE_CODE`/`AGENTS_SKILLS`/`PI`/`WINDSURF` plus the cell aria-labels).

Top 5:

1. **P0** — "Uninstall from tool directories" (My Skills toolbar) removes every Sync target in the library in one
   click. It asks for no confirmation, gives no counts and offers no undo, and auto-sync stays on.
2. **P0** — My Skills misreports state. A central-missing skill shows 7 green pills. Skills whose upstream fetch
   fails look healthy. Failed targets are colour-only and can hide in "+N more". Healthy is loud and exceptions are
   quiet.
3. **P1** — A tool-pill click or the link icon silently undeploys, with no confirmation or undo. The pill vanishes,
   so it cannot be clicked back.
4. **P1** — Skill detail shows no deployment truth: no targets, health, projects or fixes. It is a file viewer that
   replaces the list, where the contract needs an inspector.
5. **P1** — Modal as first thought. Add stacks 4 dialogs at the overwrite ask. Refresh (all) blocks the app under
   "Installing Skills…". One Esc closes every stacked dialog, and there is no focus trap or return.
