# 02 Repoint dead doc links

Status: open
Lane: A
Source: BACKLOG #20

`docs/releases/**` cite deleted files (`docs/system-design*.md`, `docs/requirements/skills-aggregation-repo.md`, `ExploreCard.tsx`, `SettingsModal.tsx`); archived round-10 reports under `.scratch/archive/round10/` cite removed `worktrees/…` lanes (evidence now under `.scratch/archive/round10/evidence/<lane>/`). Find with rg, repoint to the surviving successor or mark as '(removed in vX)'. Edit in place; do not move files.

## Done when

`rg` for each dead path returns only intentional historical mentions.

## Comments
