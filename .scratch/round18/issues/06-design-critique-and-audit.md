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
