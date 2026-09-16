# Handoff — 2026-09-15 · tracker lifecycle adopted, v1.2.12 released

The queue is `.scratch/BACKLOG.md` — this note does not repeat it.

## Where things stand

- **v1.2.12 released** (tag at `f4fd7f6`, Release run 35035055428, all five targets). Operator smokes by installing the
  build. Wave B of round 10 is closed; **wave C (BACKLOG #02) is the next architectural slice** and has no effort yet.
- **Every prior effort is archived** under `.scratch/archive/` (11 efforts, 100 tickets, all terminal; closure block in
  each; rows in `archive/README.md`). There is no live effort. `.scratch/` is tracked in git as of `7b5e160`.
- **CI (`ci.yml`) was re-enabled** after five months disabled by hand, and is **green** as of run 35038633345
  (web job needed Node 22 for jsdom 30; the April failures were Rust-on-Linux and no longer reproduce).
- **AGENTS.md was audited** (180 claims tested; 8 falsehoods, 7 partials fixed; Release section and five gotchas added).
  A fresh-context probe of the pointer wording ran after the rewrite — see the session log for its result.

## Exact next step

1. Pick from BACKLOG "Now": #02 (wave C) is the largest; #06/#07/#08 are small, related, and could form one
   "round 12" effort with a spec.

## Gotchas learned this session (already promoted into AGENTS.md)

- `main` is routinely behind `origin/main` (nightly featured-skills commit) — rebase before push.
- A push that bumps `package.json` *is* the release (auto-tag → release.yml).
- After an Xcode update, `cargo` fails at link until `sudo xcodebuild -license accept`.
- A filtered `cargo test <name>` does not regenerate `src/bindings/index.ts`.
