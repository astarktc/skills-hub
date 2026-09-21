# Round 14 — small backend/CI/UI bundle into 1.2.14

Status: closed
Opened: 2026-09-20

Absorbs BACKLOG #04 #10 #32 #33 #34 (lines left BACKLOG.md in the opening commit).
Deliberately not absorbed: #02 (wave C report unification — its own round on this smoked baseline; note it must
cover **five** report DTOs, `InvocationEditReportDto` included), #12 #15 #16 (real, not urgent).

Baseline: 1.2.13 (`c927683`) verified — `release.yml` run 35485634640 green on the SHA-pinned workflows (all five
targets + `updater.json`), operator smoke passed for #14 toasts and #11 local-picker rows.

## Decisions

- **D1:** Same execution shape as round 13 D2 — parallel Fable 5.1 children via `delegate_task`, file-disjoint lanes,
  children never commit, never touch CHANGELOG `[Unreleased]`, never archive/`git mv` under `.scratch/`, never touch
  `src/bindings/index.ts` unless the diff is theirs. Parent commits per lane by path, runs `cargo fmt` only after all
  code lanes report, adversarial review by GPT-6 Astra (Standards + Spec), releases 1.2.14.
- **D2 (#10):** the ticket's second half is already done — the 30 s `try_serialized` poll is gone
  (`tests/propagation.rs:534` uses `serialized` directly). #10 narrows to the `--release` CI run only.
- **D3 (#10 + #32):** both touch `ci.yml`, so they share one lane (B), sequential, one writer.
- **D4 (#33):** the projects world must not import the sync hook (worlds never import each other at runtime). Fix at
  the binder: App passes `sync.toolLabelById` (already exported, i18n-resolved via `tools.<key>`) into `ProjectsPage`;
  `state.toolStatus` remains the source for the tool-config modal only.

## Tickets

01 update-request-hardening (#04) · 02 release-ci-run (#10) · 03 actions-majors (#32) · 04 tool-labels (#33) ·
05 add-skill-button (#34)

## Lanes

A: 01 · B: 02 03 (sequential) · C: 04 05

## Closure — 2026-09-20

Shipped as **1.2.14** (`af55d70`; lanes `8cf55c3` B, `13eab13` C, `78ff47f` A). All five tickets `done`. Gate:
`npm run version:check && npm run check` green; `cargo test --all` 646 (+1); vitest 358 (+1); bindings unchanged.
Adversarial review: GPT-6 Astra, Standards + Spec — **no findings on either axis**, verified all five action SHAs
independently and the Restore path through `finalize_update`.

Residue:
- None to BACKLOG. Ticket 03's hypotheses about gh-release v3 breaking `files`/draft defaults were wrong (v3.0.0 is
  the Node 24 runtime only) — recorded in that ticket, no follow-up. download-artifact v8 makes digest mismatch
  fatal — a release-run behaviour to watch on the 1.2.14 build, not a queue item.
- Operator smoke of 1.2.14: Add skill button on My Skills; a fresh-launch Remove Project kept-target toast says
  `Claude Code` not `claude` (ticket 04 was not exercised against the real backend).
