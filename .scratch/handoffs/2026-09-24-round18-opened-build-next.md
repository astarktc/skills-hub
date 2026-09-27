# Handoff — 2026-09-24 · round 18 opened; next session dispatches the build

The queue is `.scratch/BACKLOG.md` — this note does not repeat it. Everything decided this session is in
`.scratch/round18/spec.md` (D1–D12, bake-off record) and its ten tickets; read those, not this note, for content.

## Where things stand

- Main @ `1cfff60` + the uncommitted spec/ticket-02 amendments this note is committed with (**local only, not
  pushed** — nothing bumps the version yet). Rounds 16 (ticket 08, Windows) and 17 (closure smoke) still await the
  operator; unrelated.
- **Round 18 = "see and judge"**, phase A of the UI effort (#37). Ships **1.2.18**. Round 19 = build, ships **1.3.0**.
- Artifacts written: `PRODUCT.md` (root); direction contract
  `.impeccable/surfaces/src-components-skills-skillslist-tsx.md` (7 blocks, seed `4a508bda`, code-led); two
  prototypes `.scratch/round18/prototypes/{patch-bay,tracker-list}/` (+ `BRIEF.md`); screenshots local-only under
  `.scratch/round18/evidence/` (gitignored — `open` the prototypes' `index.html` to see them live).
- Tickets 01–05 are `ready-for-agent`, independent, disjoint files → dispatch all five at once. 06/07 wait on 04;
  08 on 05; 09 is the operator grill; 10 is the release.

## Exact next step

1. Dispatch 01, 02, 03, 04, 05 as async `delegate_task` children on Pi, `anthropic/claude-opus-5`, thinking medium,
   `runtimeMode: full-access`, your own `clientRequestId` (`r18-t01-v1` …). Paste the ticket file **and** the spec's
   Decisions section into each brief; pre-approve AGENTS.md step 1; tell them `.scratch/` is tracked and
   § Delegated children applies. Tell 04's child that `preview_*` refuses `file://` (serve via Vite).
2. Astra (`openai-codex/gpt-6-astra`, high) reviews 02 and 04 when they land (fix-then-ship, like round 17); 01/03/05
   get an orchestrator read. Brief reviewers to fetch primary sources for registry facts (ticket 01: Augment docs).
3. When 04 lands: dispatch 06 (two blind critics: Astra high + Opus 5 high, then the audit) and 07. When 05 lands: 08.
4. Ticket 10 after 01–04 (+ their reviews): `version:set 1.2.18`, gate, rebase, push. Then the ticket-09 grill with the
   operator → `.scratch/round19/spec.md`.

## Gotchas learned this session

- The impeccable decision page's "inspiration" images are catalog stock, not renders of the concept — the operator
  had to judge from text + palette. **Prototype the top two before locking a world**; that's what actually decided it.
- Both prototype builders hit the same wall: t3 `preview_*` won't open `file://` and resizes to 1280×800; serve a
  URL and use agent-browser for exact-size captures.
- My brief invented "stale = update available"; our `stale` means target drift. Two backlog items came out of that
  (#62 upstream check, and skill-level health as a round-19 requirement). Check domain words against `CONTEXT.md`
  before they reach a child.
- Impeccable's "could a stranger guess it from the category" test is an anti-slop check on the generator, not a
  product-differentiation goal; for a personal tool the operator's taste is the brief and familiarity is legitimate.
- Keychain on ad-hoc-signed macOS builds re-prompts after updates (ticket 02 handles it; CHANGELOG must say so).
- `.pi/` (skills mirror) is untracked and unignored until ticket 03 lands — don't `git add -A`.

## Suggested skills for the next session

- `implement` (driving the tickets), `code-review` (per landed ticket, Standards + Spec), `impeccable` (tickets
  06/08 briefs — `critique.md`, `audit.md`, `craft-floor.md`, `operate.md`), `product-design-and-ux` (ticket 07
  brief), `improve-codebase-architecture` + `codebase-design` (ticket 05 brief), `domain-modeling` (ticket 08 ADR),
  `grilling` (ticket 09), `handoff` at session end.
