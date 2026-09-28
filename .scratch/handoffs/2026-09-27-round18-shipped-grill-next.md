# Handoff — 2026-09-27 · round 18 shipped as 1.2.18; ticket 09 (the grill) is the only thing left

The queue is `.scratch/BACKLOG.md` — this note does not repeat it. Round 18's record is `.scratch/round18/spec.md` plus its
ten tickets (each carries a `## Result`); the findings are under `.scratch/round18/review/`.

## Where things stand

- **v1.2.18 is released** (tag by `auto-tag.yml`, `release.yml` run 36358980829: five targets + `updater.json` green; CI
  green). Main is at the release + docs commits, pushed. The operator smoke list is in ticket 10's Result — **not yet done**:
  install the build, expect one Keychain prompt (Always Allow), Settings says "A token is saved", `github_token` row and any
  `*.cleanup_pending` marker gone from `skills_hub.db`, `~/.augment/skills` holds the relocated targets, no ZH option.
- Tickets 01–08 and 10 are `implemented`. **Ticket 09 (grill → `round19/spec.md`) is `ready-for-human`** and needs Alex.
- Rounds 16 (ticket 08, Windows) and 17 (closure smoke) still await the operator; unrelated.
- Model routing changed this session: **Opus 5.5 replaces Opus 5** everywhere (AGENTS.md § Delegation, operator 2026-09-25).

## What the grill has to work from (all committed)

- `review/architecture.md` (+ `.html`): 13 candidates; Strong = surface-state module, set-based skill-action entry point, pure
  health/source fold, styling (with three D3 corrections), list lens, action lifecycle.
- `docs/adr/0006-styling-architecture.md`: **Base UI, not Radix**, under shadcn; `@theme` tokens (named set), `prose.css` the
  one guard exemption, settings world owns the resolved theme; primitives-a11y-test exception only *proposed*. Accept or
  overturn at the grill.
- `review/critique-astra.md` (18 findings, coverage ledger) and `review/critique-opus.md` (17/40, 44 screenshots) — blind,
  independent; they converge on: global unsync without a checkpoint, healthy decoration outranking problems (skill-level
  health missing), detail as a file viewer instead of an inspector, no multi-select operating model, four-deep modal stack,
  raw Tool keys, colour-only matrix.
- `review/flows.md`: six flows; no-row failures leave no trace; Cancel real for only 5 commands; unsync undone by
  `reassert_auto_sync`; `reportOutcome` precedence (conflict › failure) wrong for batches, "Keep existing" reported as an
  error; import auto-sync-off deletes originals behind an unchanged button; D8 mapped onto flows; 12 questions.
- `review/audit.md`: **every D5 clause fails today**, with measured contrast and focus/keyboard transcripts; per-clause fixes
  for the ADR-0006 primitives.
- Reserved round-19 scope is spec D8; the § E borrows and BACKLOG #46 #47 #49 #52 and #39 stay listed until `round19/spec.md`
  absorbs them.

## Exact next step

1. Run the ticket-09 grill with Alex (`grilling` skill): open with the convergent findings above, the ADR-0006 Base UI call,
   the four policy questions (unsync under auto-sync; batch fold precedence; import auto-sync-off originals; one selection
   across surfaces vs per surface), and the two open ticket-05 questions (inspector vs route; addressable screens for
   fixture deep-links). Write `.scratch/round19/spec.md` + tickets; absorb the BACKLOG items D8 names; close round 18 per
   `docs/agents/issue-tracker.md` § Lifecycle (archive is the orchestrator's, after the operator accepts the spec).
2. Operator smoke of v1.2.18 (list above). If the Keychain migration misbehaves, `bc84841`/`4f6e312` are the commits.

## Gotchas learned this session

- **Children in one working tree**: run a single fixture Vite server from the orchestrator (5175 `strictPort` — a second
  child's server just fails), and know that `cargo test` regenerating `src/bindings/index.ts` and any `src/` edit hot-reload
  every reviewer's preview page and reset fixture state. Sequence tickets that share files (02 waited for 03).
- **t3 `preview_*` in delegated (hidden) tabs**: the page only runs while a `preview_evaluate` is in flight; one call at a
  time; drive a flow inside one evaluate; force a repaint before capture; and **two children's preview tabs share browser
  storage** (theme/view flipped under one critic). Now recorded in AGENTS.md § Environment gotchas.
- Astra's adversarial reviews paid for themselves three times on ticket 02 (migration overriding a later Save; plaintext left
  in freed SQLite pages; lost retry signal). Keep fix-then-ship with a bounded re-review for anything touching secrets.
- Two children chose the bigger design when the ticket allowed it and were right: ticket 01's relocation module (option (a)
  refuted by test), ticket 03 removing the header switcher the ticket didn't name. Brief for "prove the cheap option first".
- Fixture fidelity is a review input: the first fixture answered success where Rust answers report failures; the fixed one
  (`5fbc3c5`) mirrors the producers, and `OTHER` (not `TOOL_NOT_WRITABLE`) is the honest code for a Propagation write failure.

## Suggested skills for the next session

`grilling` (ticket 09), `to-spec` / `to-tickets` (round 19), `domain-modeling` (any ADR the grill produces), `handoff`.
