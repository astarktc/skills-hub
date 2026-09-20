# Round 13 — bundle the backlog into 1.2.13

Status: open
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
