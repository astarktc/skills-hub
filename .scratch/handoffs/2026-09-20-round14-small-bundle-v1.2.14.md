# Handoff — 2026-09-20 · round 14 (small bundle) closed, released as 1.2.14

The queue is `.scratch/BACKLOG.md` — this note does not repeat it.

## Where things stand

- **1.2.13 verified** at session start: `release.yml` run 35485634640 green on the SHA-pinned workflows; operator
  smoke passed for #14 toasts and #11 local-picker rows (staged fixture at `/tmp/smoke`, operator's to clean up along
  with the `smoke-good` skill it installed into the live library).
- **Round 14 is done and archived** (`archive/round14/`): BACKLOG #04 #10 #32 #33 #34 shipped across three parallel
  Fable 5.1 lanes (B CI `8cf55c3`, C UI `13eab13`, A core `78ff47f`), Astra review with no findings, released as
  **1.2.14** (`af55d70`).
- **Release:** the push carrying `af55d70` triggers `auto-tag.yml` → `release.yml`. This is the **first run on the
  bumped Actions majors** (checkout v7, setup-node v7, upload/download-artifact v7/v8, gh-release v3). Watch: CI's
  new `cargo test --release --all propagation` step (16 tests) then the drift check; auto-tag's push + dispatch on
  checkout v7's credential-file path; Release's "Download workflow artifacts" step (download-artifact v8 makes digest
  mismatch fatal) and the published release carrying all five platform assets, every `.sig`, and `updater.json`.
  `update-featured-skills.yml` is only exercised by its nightly cron.
- Gate at close: `npm run version:check && npm run check` green; `cargo test --all` 646; vitest 358.
- No live effort. Next free BACKLOG number **#35**.

## Exact next step

1. Confirm the v1.2.14 GitHub release built (all five targets) — if a major bump breaks it, fix and ship 1.2.15.
   Operator smoke: Add skill button on My Skills opens the Local/Git modal; a fresh-launch Remove Project with a kept
   target names the tool by label.
2. Then BACKLOG "Now" is **#02 alone** — wave C report unification + ADR-0001 amendment, its own round and release.
   Scope note already in `archive/round14/spec.md`: it is **five** report DTOs, not four (`InvocationEditReportDto`).

## Gotchas learned this session

- A backlog line can be stale by the time it is picked up: #10's "30 s poll" had already been removed. Read the
  cited file:line before writing the ticket, and narrow in the spec's Decisions (round 14 D2).
- A parent hypothesis in a ticket ("gh-release v3 changed `files` globbing") is a prompt for the child to verify,
  not a fact — lane B correctly disproved it against the release notes.
- Three file-disjoint lanes + parent `cargo fmt` after all lanes = no formatting collisions (fmt was a no-op).
