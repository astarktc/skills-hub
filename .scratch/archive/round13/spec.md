# Round 13 — bundle the backlog into 1.2.13

Status: closed
Opened: 2026-09-16

Absorbs BACKLOG #03 #05 #09 #11 #13 #14 #19 #20 #31 (lines left BACKLOG.md in the opening commit).
Stretch (not absorbed, still in BACKLOG): #10, #15. Deferred: #02, #04, #16.

## Decisions

- **D1 (#05):** Option A — the DB row's `content_hash` is the source of truth for a skill's content; hand-editing
  `~/.skillshub/<skill>` is unsupported. Every source kind already has a sanctioned edit door (git → upstream/fork,
  Update/Restore; local path → source path, Re-point/Update; imported/central-only → in-app Edit). Record as ADR-0005
  + pointer in CONTEXT.md **Content identity**. No code change.
- **D2:** Parallel Fable children, one per file-affinity group; children do not commit, do not touch CHANGELOG
  `[Unreleased]`, do not archive/`git mv` under `.scratch/`. Parent commits per ticket, adds CHANGELOG entries, runs
  adversarial review (Astra), releases 1.2.13.

## Tickets

01 changelog-reconstruct (#09) · 02 doc-links (#20) · 03 adr-content-identity (#05) · 04 workflow-hardening (#31) ·
05 touch-portability (#19) · 06 content-identity-warn-once (#03) · 07 listing-validity (#11) ·
08 insert-transition (#13) · 09 project-removal-outcomes (#14)

## Lanes

A: 01 02 03 · B: 04 05 · C: 06 07 · D: 08 · E: 09

## Closure — 2026-09-16

Shipped as **1.2.13** (`c927683`; lanes `49f2f9a` A, `dd29ecc` C, `7f9ac17` B, `dbc0cdb` D, `6280df2` E; review
fixes `fa4ae9f`). All nine tickets `done`. Gate: `npm run version:check && npm run check` green; `cargo test --all`
645 passed; vitest 357. Adversarial review: GPT-6 Astra, Standards + Spec axes (one hard doc mismatch, two judgement
calls, one P2 narrowed by decision, one P3 CHANGELOG clause removed — all applied in `fa4ae9f`).

Residue:
- **→ BACKLOG #32:** bump GitHub Actions to current majors (checkout v7, setup-node v7, upload-artifact v7,
  download-artifact v8, gh-release v3) — lane B stayed within existing majors so CI behaviour did not change.
- **→ BACKLOG #33:** `toolLabelById` on the remove-project path falls back to raw tool keys when `toolStatus` has not
  been loaded (it loads on the add/tool-config flow) — the kept-project toast can show `claude` instead of `Claude Code`.
- **Dropped by name:** a git-side listed-but-invalid DTO for manifest-less `.claude/skills/` children (ticket 07
  narrowed: the git listing is the installable set; install can never accept such a dir); a literal end-to-end
  command-envelope test for the kept-project removal (core + fold + mocked-hook coverage is the repo's test policy);
  sharing SQL between `TargetTransition::Recorded` and the test-only `upsert_skill_target` fixture (fixture is
  `#[cfg(test)]`); `.scratch/archive/round10/evidence/` pointers written by ticket 02 refer to a gitignored tree.
