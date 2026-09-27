# Round 18 — visual-world bake-off brief (shared by both prototypes)

Two independent throwaway prototypes of the **My Skills** surface of Skills Hub, one per candidate visual world.
Same fixture data, same constraints, same deliverables — only the world differs. The operator compares them and
locks one; the loser is kept as evidence. Nothing here touches `src/`.

## Question the prototype answers

"Does this world carry the operator's real library — one huge source beside several tiny ones, ~80 skills, 7 tools,
mixed sync states — with **state readable at a glance, no pill clutter, dense but AA-legible, dark first** — and does it
still feel like *a practical tool that happens to be beautiful*, sitting beside T3 Code, Linear and Raycast?"

## Hard constraints (from PRODUCT.md and the grill — not negotiable)

- **Operate mode.** Familiarity is earned; strangeness without purpose is the failure mode. No display fonts in UI
  labels, no custom scrollbars, no reinvented form controls, no orchestrated page-load choreography.
- **Restrained colour**: neutrals + one accent. The accent means *live / selected / primary action* only. Status colours
  (success / warning / error) mean state only. Never accent or saturated colour on an inactive element.
- **Dark is designed first**; light is a derivation switched with a toggle (`data-theme`), both must pass **WCAG AA**.
- **Density**: the library is ~80 skills; the operator wants *more skills per screen*, fewer pills, no repeated repo
  labels inside repo groups. A repo name appears once — on its group — never per row.
- **Grouping**: repo groups are the default, collapsible (state persisted in memory is enough), with sticky headers.
  Sources contributing **one** skill fold into a single **"Individual sources"** group whose rows carry a muted inline
  source label (the one place a source label survives on a row). A **flat mode** toggle shows one list with a source
  column.
- **Row content**: name · one-line description · **"Synced N/M"** as a compact visual (N lit of M installed tools) ·
  scope (global / per-project count) · enable/disable switch · overflow. Full tool list lives in the inspector/hover,
  not the row. Rows carry a selection checkbox; selecting ≥1 raises a **bulk rail** (Sync to tools · Assign to
  project · Disable · Delete) — one selection model.
- **Group header**: name · count · state summary · **Deploy all** for the group (#39).
- **Inspector / detail pane** for the focused row: source line (copyable), health, targets as a per-tool grid,
  **Projects** section with per-project checkboxes (assign from My Skills), Edit.
- **Issues affordance**: an aggregate "N skills have sync issues → View issues" and an `Issues` chip in the filter bar.
- **Motion**: near-invisible on frequent actions (select, filter, collapse: ≤150 ms, opacity/transform only);
  `prefers-reduced-motion` honoured; no `transition: all`.
- **Keyboard**: rows focusable, `j/k` or arrows move focus, `x` toggles selection, `Enter` opens inspector, visible
  focus ring. This must actually work in the prototype.
- **Type**: system UI stack (`-apple-system, BlinkMacSystemFont, "Segoe UI", Inter, sans-serif`) and a mono stack for
  paths/identifiers. **No network font imports.** Fixed rem scale, ratio ≤1.2, body ≥13px, tabular figures for counts.
- **Window envelope**: designed at 1440×900; must hold at **960×640** with one structural collapse (inspector becomes
  a sheet or the rail collapses) — no horizontal scroll.
- **Icons**: inline SVG (Lucide-style, 16px, `stroke-width` 1.5–1.75). Never emoji.
- **Quality-bar peers**: T3 Code (density, dark palette, type, tone — colour only where it means something), Linear
  (cleanliness), Raycast settings panes. Upstream `qufei1993/skills-hub` is a domain reference, not a look to copy.

## Fixture data (use exactly this shape; invent nothing structural)

Installed tools (7): Claude Code, Codex, Pi, Cursor, Kimi, Amp, Gemini CLI. Projects (3): `skills-hub`,
`quartermaster`, `agent-skills`.

Sources and skills (name · one-line description · synced/7 · state · scope):

**mattpocock/skills** (28) — e.g. `ts-error-fixer` · "Diagnose and fix TypeScript errors by category" · 7/7 · ok ·
global; `vitest-migration` · 5/7 · **stale** (upstream update available) · global; `react-perf-audit` · 7/7 · ok ·
global + 2 projects; `zod-schema-first` · 6/7 · **failed** on Cursor (TARGET_EXISTS) · global; `prisma-review` · 0/7 ·
**disabled** · —; …fill the rest with plausible TS/React/Node names, mostly 7/7 ok, a few 6/7, two with local
**Edits** (pencil marker).

**anthropics/skills** (9) — `docx`, `pdf`, `pptx`, `xlsx`, `frontend-design`, `mcp-builder`, `webapp-testing`,
`canvas-design`, `artifacts-builder`; all 7/7 ok, global; `frontend-design` also in 1 project.

**obra/superpowers** (12) — `brainstorming`, `tdd`, `systematic-debugging`, `verification-before-completion`,
`writing-plans`, `executing-plans`, `subagent-driven-development`, `condition-based-waiting`, `root-cause-tracing`,
`defense-in-depth`, `using-git-worktrees`, `finishing-a-development-branch`; 7/7 ok except `tdd` **stale** and
`using-git-worktrees` 4/7 with **2 failed** (Kimi, Amp: TOOL_NOT_WRITABLE).

**vercel-labs/agent-skills** (6) — `react-best-practices`, `composition-patterns`, `web-design-guidelines`,
`next-app-router`, `edge-runtime`, `turbo-monorepo`; 7/7 ok.

**Individual sources** (fold): `alexstark/handoff` (1) · `ego-browser/skill` (1) · `pi-lens` (1, local folder
`~/Projects/pi-lens/skill`) · `t3-code/orchestration` (1, **stale**) · `some-org/ui-ux-pro-max` (1, 7/7, Edited).

Totals: 61 skills · 7 tools · 3 projects · issues: 3 failed targets across 2 skills, 3 stale.

## Deliverables (all under `.scratch/round18/prototypes/<world>/`, `<world>` = `patch-bay` or `tracker-list`)

1. `index.html` — **one self-contained file**, opens by double-click (`file://`). Styling: either plain CSS with custom
   properties as tokens, or Tailwind v4 browser build via
   `<script src="https://cdn.jsdelivr.net/npm/@tailwindcss/browser@4"></script>` with tokens in a
   `<style type="text/tailwindcss">@theme { … }</style>` block (preferred — it dry-runs the styling architecture
   round 19 will adopt). **Never** `cdn.tailwindcss.com`. Fixture data as an inline JS array; the DOM is rendered from
   it; toggles (theme, grouped/flat, collapse, select, inspector) mutate in-memory state and re-render. No frameworks,
   no persistence, no build step.
2. `NOTES.md` — ≤40 lines: the decisions you made where the world was silent, what felt wrong while building, what
   you'd change, and the exact tokens you ended with (palette, type scale, spacing, radii, motion).
3. Screenshots into `.scratch/round18/evidence/<world>/`: `dark-1440.png`, `light-1440.png`, `dark-960.png`, and
   `dark-selected.png` (2 rows selected → bulk rail up, one row open in the inspector). Capture with the t3
   `preview_*` tools (open `file://` path, resize, snapshot with `save=true`, copy the PNG). Inspect each capture once
   before you finish; a blank or half-rendered capture is not evidence.

## Process

- State your approach in three lines and continue — pre-approved, do not stop for confirmation.
- Before writing any UI, read `.pi/skills/impeccable/reference/craft-floor.md` and
  `.pi/skills/impeccable/reference/operate.md` (absolute: `/Users/alexstark/Projects/skills-hub/.pi/skills/impeccable/reference/`).
  Read `PRODUCT.md` at the repo root. Read the `emil-design-eng` skill's Review Checklist
  (`/Users/alexstark/Projects/skills-hub/.pi/skills/emil-design-eng/SKILL.md`, section "Review Checklist") for motion.
- Build fully, then **one** batched screenshot round, fix what it shows in one batch, confirm with at most one more
  round, stop. No open-ended self-polish.
- Run `.pi/skills/impeccable/scripts/impeccable detect --json .scratch/round18/prototypes/<world>/index.html` once at
  the end; fix mechanical findings; list the rest in NOTES.md.
- Do not edit anything outside `.scratch/round18/prototypes/<world>/` and `.scratch/round18/evidence/<world>/`.
  Do not commit. Do not touch `src/`, `PRODUCT.md`, or the tracker.
- Commit to your world completely. Do not hedge toward the other candidate or toward the category default (white
  cards, pills, blue). A safe rendition is the known failure.
