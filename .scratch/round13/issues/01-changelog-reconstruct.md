# 01 Reconstruct CHANGELOG entries 1.2.7–1.2.11

Status: open
Lane: A
Source: BACKLOG #09

CHANGELOG.md jumps from 1.2.12 to 1.2.6. Tags v1.2.7..v1.2.11 exist. Reconstruct each entry from `git log vX..vY` (exclude `chore: update featured-skills.json`), dated from the tag's commit date, in the existing style (one-line summary + Fixed/Changed/Added bullets). Do not fabricate — if a range has nothing user-visible, say so in one line.

## Done when

Five headings exist in order between 1.2.12 and 1.2.6; every bullet traces to a commit.

## Comments
