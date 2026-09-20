# 02 Repoint dead doc links

Status: done — pending parent commit
Lane: A
Source: BACKLOG #20

`docs/releases/**` cite deleted files (`docs/system-design*.md`, `docs/requirements/skills-aggregation-repo.md`, `ExploreCard.tsx`, `SettingsModal.tsx`); archived round-10 reports under `.scratch/archive/round10/` cite removed `worktrees/…` lanes (evidence now under `.scratch/archive/round10/evidence/<lane>/`). Find with rg, repoint to the surviving successor or mark as '(removed in vX)'. Edit in place; do not move files.

## Done when

`rg` for each dead path returns only intentional historical mentions.

## Comments

- 2026-09-16 (lane A child) — Edited in place, no moves. `docs/releases/`: `v0.1-v0.2/system-design{,.zh}.md` cross-links repointed to their sibling files (archived there since v0.3.0, ddcc7b8); `v0.3.0/plan-explore-page-redesign.md` `ExploreCard.tsx` marked *(never created: rendered inline by `ExplorePage.tsx`)* — the file never existed in history; `v0.3.1`+`v0.4.0/skills-aggregation-repo.md` `docs/requirements/…` marked *(never created: this document is the requirements doc)* — never existed in history; `v0.3.1`+`v0.4.0/plan-in-app-update.md` `SettingsModal.tsx` (×2 each) marked *(removed in v0.4.0 — now `SettingsPage.tsx`)* (deleted in f7b2091). `plan-settings-page.md`'s `modals/SettingsModal.tsx → SettingsPage.tsx` heading describes the migration itself — intentional historical mention, left. `.scratch/archive/round10/`: checkout lines in b1/b2/b3/review-fixes reports, the lane table in `wave-b-execution.md` and the checkout line in `wave-b-review-disposition.md` annotated *(worktree removed after merge; evidence under `.scratch/archive/round10/evidence/<b1|b2|b3|review-fixes>/`)*. Remaining `worktrees/` hits are verbatim command lines in code blocks (covered by the report-level note), the review brief's search instruction, and the "merged worktrees cleaned" log line — all intentional. Verified with `rg -n 'worktrees/' .scratch/archive/round10` and `rg -n 'docs/system-design|requirements/skills-aggregation-repo|ExploreCard\.tsx|SettingsModal\.tsx' docs/releases`. Note: `.scratch/archive/round10/evidence/` is gitignored (`**/evidence/`) and not present in this checkout — the pointer follows the ticket text.
