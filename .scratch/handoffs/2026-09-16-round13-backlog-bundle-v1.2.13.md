# Handoff — 2026-09-16 · round 13 (backlog bundle) closed, released as 1.2.13

The queue is `.scratch/BACKLOG.md` — this note does not repeat it.

## Where things stand

- **Round 13 is done and archived** (`archive/round13/`): BACKLOG #03 #05 #09 #11 #13 #14 #19 #20 #31 shipped across
  five parallel Fable 5.1 lanes (A docs `49f2f9a`, C core `dd29ecc`, B CI `7f9ac17`, D store `dbc0cdb`, E full-stack
  `6280df2`), adversarial review by GPT-6 Astra (Standards + Spec) applied in `fa4ae9f`, released as **1.2.13**
  (`c927683`) together with round 12's three unreleased fixes.
- **Release:** the push that carries `c927683` triggers `auto-tag.yml` → `release.yml`. Verify the tag `v1.2.13` and
  the Release run went green — this is the first release build under the SHA-pinned, least-privilege workflows (#31).
  If `release.yml` fails on a pin/permission, fix and ship as 1.2.14; `ci.yml` was exercised by the same push.
- Gate at close: `npm run version:check && npm run check` green; `cargo test --all` 645; vitest 357.
- No live effort. Next free BACKLOG number **#34**.

## Exact next step

1. Confirm the v1.2.13 GitHub release built (all five targets) and Alex smoke-tested the installed build — pay
   attention to Remove Project / Configure Tools toasts (#14) and the local picker's disabled manifest-less
   `.claude/skills` rows (#11).
2. Then BACKLOG "Now": #02 (wave C — one report representation + ADR-0001 amendment; own release) and #04
   (`UpdateRequest` hardening) are the remaining large items; #10, #12, #15, #16 are medium.

## Gotchas learned this session

- **Parallel `delegate_task` children in one checkout work** when lanes are file-disjoint, children never commit, and
  the parent commits per lane by path. cargo's target-dir lock serializes their builds (expected slowness). Two
  hazards seen: (1) any child's `cargo test` regenerates `src/bindings/index.ts` with *another* lane's DTOs — brief
  children to leave it alone if the diff is not theirs (one child `git checkout`ed it; the owning lane regenerated it
  later, no loss); (2) a parent `cargo fmt` pass touches other lanes' in-progress files — run it only after all code
  lanes report.
- Bash `for n in 01..09` with `${SHA[$n]}` fails on `08`/`09` (octal) — use `case` on strings.
- The git listing is by design the installable set (`is_installable`); "listed but invalid" is a local-picker concept
  only (CONTEXT.md **Skill candidate** now says so).
- `release.yml` top-level `permissions` is now `contents: read`; the publishing job carries its own `contents: write`.
  Lowering it further breaks `softprops/action-gh-release`.
