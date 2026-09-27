# Round 18 — see and judge (UI effort phase A) → 1.2.18

Status: open
Opened: 2026-09-24

Opens Future effort **#37** (overall UI audit) as a two-round effort: round 18 *judges* (enablers, decisions, findings),
round 19 *builds* (the new visual world and the workflow features). Absorbs BACKLOG **#43** and **#44** (tickets 01, 02)
and the operator's ZH-removal decision (ticket 03). Records the visual-world decision and the styling-architecture
decision so round 19 starts from settled ground.

Baseline: 1.2.17 (`747efbe`, tag `v1.2.17`). Main @ `9ef00ff`. Rounds 16 (ticket 08) and 17 (closure smoke) stay
live awaiting the operator; nothing here depends on them.

## Problem (verified against `main` @ `9ef00ff`)

1. **The UI is the 2025 vibe-coded default** — white ground, rounded cards, status pills, one blue accent (operator,
   2026-09-24). Measured: `src/App.css` 3,696 lines behind 482 `className` sites (Tailwind v4 is wired but used at
   ~10 sites); 22 `transition: all`; 0 `prefers-reduced-motion`; 0 `@media`; fonts fetched from Google at launch; no
   minimum window size. No `PRODUCT.md`/`DESIGN.md` existed.
2. **Operator frictions**: many tool pills per card; repo pills repeated inside repo-grouped lists; the grouped
   layout wastes space when one 28-skill source sits beside five single-skill sources; no bulk assignment; no
   project assignment from My Skills; Explore "could be better".
3. **Agents cannot see the app.** `npm run dev` dies at the first `invoke`; `tauri:dev` mutates the operator's real
   library and cannot be screenshot by `preview_*`. Every UI review so far was done blind.
4. `#43`: `tool_adapters/mod.rs` names Augment's global dir `.augment/rules`; the vendor's is `.augment/skills`.
5. `#44`: the GitHub token is a plaintext settings row (`core/settings.rs`).
6. The ZH locale is an upstream inheritance nobody on either side can read or verify; the "both en and zh"
   invariant doubles every copy change for no reader.

## Goal

By the end of round 18: agents can run and screenshot a fixture-backed app in the browser; the visual world, the
styling architecture and the frontend's architectural debts are decided and written down; two independent design
critiques, an accessibility audit and a task-flow review exist as findings; #43, #44 and ZH removal have shipped as
**1.2.18**; and a grilled **round 19 spec** exists. Round 18 lands no new-world UI.

## Decisions (accepted by the operator 2026-09-24)

- **D1 Replace the visual world once, then refine every surface into it.** 80/20: 80 % workflow/feature/consistency
  refinement, 20 % redesign. Operate mode; Restrained colour; **dark designed first**, light a first-class
  derivation; "practical, not flashy". Quality-bar peers: T3 Code (density, palette, type, tone), Linear
  (cleanliness), Raycast settings panes; upstream `qufei1993/skills-hub` as a *domain* reference — a specific
  item may be borrowed when genuinely ideal, with our own spin.
- **D2 The world is the Tracker List / Patch Bay hybrid** recorded as the direction contract in
  `.impeccable/surfaces/src-components-skills-skillslist-tsx.md` (see § Bake-off). Product vocabulary only —
  never "patched". Healthy is silent; only exceptions carry a mark, after the name. Both **list and card-grid** view
  modes exist in the new world (round 19 designs the grid).
- **D3 Styling architecture** (ADR, ticket 08): Tailwind v4 idiomatic — tokens in `@theme` (one place; that *is*
  the world), utilities in JSX, and a deep **primitives module** `src/components/ui/` whose props (`variant`,
  `tone`, `size`) are the interface; **shadcn** supplies the *behavioural* primitives only (Dialog, Popover,
  DropdownMenu, Tooltip, Checkbox, Switch, Tabs, Command), copied in and restyled to our tokens. `App.css` shrinks
  to zero surface by surface with a CI guard. Reverses AGENTS.md's "styles live in App.css / no CSS Modules" rule.
  Radix vs Base UI beneath shadcn: decided in the ADR after reading the `shadcn` skill.
- **D4 Fixture-backed browser mode** at the existing `invokeTauri` seam: stateful in-memory, every command in the
  generated `commands` table (a missed one fails `npm run build`), simulated `Channel` progress, scenarios via
  `?scenario=` (rich · empty · first-run · failure-heavy), gated by `import.meta.env` so production tree-shakes it.
  Served by Vite (`preview_*` refuses `file://` — both prototype builders hit this).
- **D5 Accessibility is a hard gate** for every round-19 ticket: keyboard-only completion of core flows, visible
  focus, WCAG AA both themes, `prefers-reduced-motion`, minimum window 960×640 with one structural collapse.
- **D6 Motion**: crisp and near-invisible on frequent actions; deliberate delight only on rare moments (first run,
  a large import completing, an install landing). Emil's checklist is the review rubric.
- **D7 EN only.** Remove `zh` (ticket 03); keep i18next and `t()`; retire the parity test; broader localisation is
  BACKLOG #61.
- **D8 Round-19 scope reserved now** (not opened): #52 enable/disable + multi-select bulk (sync-to-tools, assign-to-
  project); #39 group-level Deploy all; per-row/inspector project assignment; #49 issues banner + chip; "Synced N/M"
  per-tool strip; detail health ladder + copyable source; sticky install summary in Add; #47 picker search; #46
  `SKILL.md`-gated scan; passive title-bar update affordance; singleton fold + flat mode; ⌘K palette. Out: sectioned
  sidebar (§E4), Updates page (§E6), home/doctor surface (#58), #62 upstream check (needs #59 first).
- **D9 Sequencing**: round-19 children run **sequentially** on the styling hotspots (`App.css`/`@theme`,
  `src/components/ui/`); parallelism only across disjoint surfaces after the world ticket lands.
- **D10 Explore** gets a full critique like the other surfaces (operator: "could be better").
- **D11 Versioning**: round 18 ships as **1.2.18**; round 19 (the new world + workflow features) ships as **1.3.0**.
- **D12 Token migration is automatic** on first launch of 1.2.18 (no opt-in); the row is deleted only after the
  keychain write is read back (ticket 02).

## Bake-off (2026-09-24)

`impeccable concept-seed --scope direction --mode operate` (key `4a508bda`) assigned **The Patch Bay**; the pick card
was **The Tracker List**; challengers Mono Grid / Module Mosaic (competitive), Signal Bench / Lineup Sheet / Fold
Sequence / Tension Column (declined, each donating one raise). Decision page payload:
`.scratch/round18/evidence/direction-payload.json` (local). The operator locked Patch Bay by a slim margin, then had
both top cards prototyped head-to-head by independent Opus 5.5 children (`.scratch/round18/prototypes/BRIEF.md`,
`patch-bay/`, `tracker-list/` — tracked; screenshots under `evidence/` are local-only).

Verdict (operator + orchestrator): **Tracker List primary** — shell (left sidebar), darker palette, iris accent,
inspector's Health / Upstream / Edits / Last synced ladder with Retry failed · Edit · Disable, vertical per-tool
target list, ⌘K palette, light mode. **Patch Bay borrows** — the per-tool Synced strip (shape-encoded: hollow /
filled / ×; near-invisible at N/N), the bulk rail with `x`/`Esc` hints, the click-to-sync per-tool grid as an
inspector option. **Rejected**: top-rail shell, engraved caps, hairline-everything, lighter panel ground ("Proxmox
console" line), "patched/unpatched" vocabulary, colour-only jack states, amber-on-jacks for stale.

Two findings the prototypes surfaced about the *product*, not the design: (a) both rendered "stale = update
available", a state the backend cannot produce — our `stale` means target drift (`project_sync::
list_assignments_with_staleness`); a passive upstream check is BACKLOG **#62**; (b) neither had a **skill-level
health** channel (unlocatable source, fetch failure, Edit conflict) distinct from target health — round 19 must.

## Tickets

| # | Title | Depends on | Ships in 1.2.18 |
|---|---|---|---|
| 01 | Augment global skills dir `.augment/skills` (#43) | — | yes |
| 02 | GitHub token → OS keychain behind `CredentialStore` (#44) | — | yes |
| 03 | Remove the ZH locale; EN-only invariant; gitignore `.pi/` | — | yes |
| 04 | Fixture-backed browser mode (D4) | — | dev-only, tree-shaken |
| 05 | Frontend architecture review (`improve-codebase-architecture`, `src/` only) | — | findings |
| 06 | Two independent design critiques + audit on the fixture app (impeccable `critique` + `audit`) | 04 | findings |
| 07 | Task-flow and state review of the six core flows (`product-design-and-ux`) | 04 | findings |
| 08 | ADR: styling architecture (D3) + Radix/Base UI decision | 05 | docs |
| 09 | Grill on findings → `round19/spec.md` | 05 06 07 08 | ready-for-human |
| 10 | Docs, CHANGELOG, `version:set 1.2.18`, release | 01 02 03 04 | release |

## Execution

Children on Pi: implementation **Opus 5.5** (`anthropic/claude-opus-5-5`, thinking medium), reviews/critiques **Astra**
(`openai-codex/gpt-6-astra`, high) and, for ticket 06's second critic, Opus 5.5 (high). (Opus 5 → 5.5, operator 2026-09-25.) Every child: state approach and
continue (pre-approved); `.scratch/` tracked; § Delegated children applies; `runtimeMode: full-access`; children brief
their reviewer to fetch primary sources for registry facts. Ticket 06/07 children read `PRODUCT.md`, the surface brief
and the prototypes before judging; they judge the *incumbent* app for findings, not the prototypes.

## Closure checklist

- 1.2.18 published with five targets; `~/.augment/skills` scanned on the operator's Mac; token stored in Keychain
  (settings row gone after first launch); UI shows no ZH option.
- `npm run dev:fixture` serves a walkable app; `preview_snapshot` captures every surface in both themes.
- `.scratch/round18/review/` holds: `architecture.html` + `architecture.md`, `critique-astra.md`, `critique-opus.md`,
  `audit.md`, `flows.md`.
- `docs/adr/0006-styling-architecture.md` merged; AGENTS.md styling rules updated.
- `round19/spec.md` written and accepted by the operator.
