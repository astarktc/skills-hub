# Handoff — 2026-09-24 · round 17 pushed as 1.2.17; rounds 16 and 17 await the operator's smoke

The queue is `.scratch/BACKLOG.md` — this note does not repeat it.

## Where things stand

- **Round 17 pushed**: main @ `747efbe` (rebased onto origin's featured-skills commits), tag `v1.2.17`,
  `release.yml` run 35952962348 **green** — release published 2026-09-24T03:59Z with all five targets, macOS/Linux
  `.sig`s and `updater.json`; CI run 35953036720 green. Ticket 04 is done (`747efbe`).
- What shipped (spec: `.scratch/round17/spec.md`): D1 the onboarding scan honours "only scan selected tools"
  (`OnboardingScanScope`, resolved once at the seam, carried by `ImportPolicy` into the import); D2 a
  skills-only footprint is not an installed tool (`is_installed_in` walk, virtual groups exempt); D3 the one
  sync seam raises an overwrite ask on `TARGET_EXISTS` and retries with per-pair overrides — identical content
  still replaced silently; BACKLOG #42 absorbed.
- **Astra review** (`.scratch/round17/review/astra-review.md`): fix-then-ship; all five applied in ticket 05
  (`88e8885`, pre-rebase sha recorded in the ticket): M2 became a registry correction — `relative_detect_dirs`
  slice, Amp `.config/amp`, Kimi `.kimi-code`/`.kimi` — because the vendor's two Kimi doc sites disagree;
  M1 plan reloads on `scanScopeRevision` + fresh fetch on Review; S1 ask ownership; S2 enumeration errors read
  as installed; S3 thrown sync reloads.
- Gate at push: `npm run version:check && npm run check` green; cargo 669, vitest 406, eslint 0 warnings,
  bindings unchanged (no wire change this round).
- **Open**: round 16 ticket 08 (Windows junction smoke, `ready-for-human`); round 17 needs only the operator's
  smoke of the spec's Closure checklist. Both rounds stay live until then.
- BACKLOG: `Now` / `Later` / `Parked` empty; Future efforts #26 #37–#41; next free **#43**.

## Exact next step

1. Operator installs v1.2.17 and smokes `round17/spec.md` § Closure checklist (phantom tools gone from the
   Tool config modal — on the operator's machine Kimi and Amp now correctly read *not installed*; Review & Import
   scoped; sync onto a differing occupied folder → the ask, Overwrite / Keep existing; identical → silent).
   The modal's layering above the loading overlay was verified only by source/CSS — watch for it.
2. Operator runs round 16 ticket 08 on the Windows host; then close **both** efforts per
   `docs/agents/issue-tracker.md` § Lifecycle (`## Closure — <date>` blocks, extract-then-archive, archive README
   rows, delete this note).
3. Next work: a Future efforts pick (#37 UI audit is the natural first — grill, not code). #40 (harness
   detection audit) gained evidence this round: detect roots were wrong for two of 45 entries; expect more.

## Gotchas learned this session

- **Vendor docs disagree with themselves** (Kimi: `~/.kimi` vs `~/.kimi-code`). A single detect dir per tool
  was the wrong model; `relative_detect_dirs` is a slice now. When #40 audits the rest, fetch both the old and
  the current doc site per tool.
- Fixtures that "install" a tool with a bare `create_dir_all(detect_dir)` are exactly the footprint D2 rejects;
  `tool_adapters::mark_installed_in` is the only fixture door. Several fixtures also relied on "installing Amp
  installs Kimi" — a shared *detect* dir was never a real fact.
- A confirmation raised **inside** `runAction` sits under the loading overlay (z 2000); the modal needs
  `backdropClassName="modal-backdrop-over-loading"` (z 2001) and must not disable on `loading`.
- The frontend caches the onboarding plan; any backend change that makes the plan depend on settings needs a
  frontend invalidation signal (`scanScopeRevision`) — Astra caught this by probing the hook, not by reading.
- Astra as reviewer again found a control-flow hole no test covered (M1) and a real-world regression by fetching
  vendor docs (M2). Keep the pairing; brief the reviewer to fetch primary sources for registry facts.
