# 01 Reconstruct CHANGELOG entries 1.2.7–1.2.11

Status: done — 49f2f9a
Lane: A
Source: BACKLOG #09

CHANGELOG.md jumps from 1.2.12 to 1.2.6. Tags v1.2.7..v1.2.11 exist. Reconstruct each entry from `git log vX..vY` (exclude `chore: update featured-skills.json`), dated from the tag's commit date, in the existing style (one-line summary + Fixed/Changed/Added bullets). Do not fabricate — if a range has nothing user-visible, say so in one line.

## Done when

Five headings exist in order between 1.2.12 and 1.2.6; every bullet traces to a commit.

## Comments

- 2026-09-16 (lane A child) — Inserted `## [1.2.11]`…`## [1.2.7]` (all dated 2026-09-08 from `git tag --format='%(creatordate:short)'`) between 1.2.12 and 1.2.6 in CHANGELOG.md; `[Unreleased]` untouched. Style follows the existing entries (no inline hashes); every bullet traces to a commit in `git log vX..vY` excluding `chore: update featured-skills.json`:
  - 1.2.7: a261e89 (Added: in-app invocation-mode Edit, persist/replay, conflict banner), fb71b1b (Fixed: manifest bytes preserved, `SKILL_MANIFEST_IO`).
  - 1.2.8: a247958, 1e79c92 + 27e0481, 4bbedc0 + 0779a41, 3190623.
  - 1.2.9: 1ce39fc + 73d82f4, 75728dc, 7ff30a5, ace8725; internal: d24b6bf, 5516eab, 76ca354, 365a3f8. (7a7c519's doc-comment tweak not listed — not user-visible.)
  - 1.2.10: c8bb3d4 + 300f89b, e8022ec + 9bec136, 13b2445, 3db3adc; internal: 0f1bc91, d1fe15f, 444f817 + c7bf1c2.
  - 1.2.11: Added a0f2c1d + 14932af; Changed 7af11fb + 6f35f7d + 9fc2b71 + 7dcc4cd; Fixed 86cfd63 + c909028, 525e970, 7dcc4cd + 5e98143, 7dcc4cd. Merge commits (b564738, a05e0a6, 370d56f, 5b29181) and 61f477e (docs) not listed.
  Note: the tag-date rule makes 1.2.6 (tagged 2026-09-07) and 1.2.7–1.2.11 all read 2026-09-08 — the existing 1.2.6 heading already says 09-08 and was left as is.

- 2026-09-16 (parent, post-review) — Spec review (Astra) found the 1.2.11 clause "unknown keys are no longer dropped
  silently on save" traced to no commit in the range (`c909028` changes picker visibility only). Clause removed.

- 2026-09-16 (parent) — closed `done — 49f2f9a`; review fixes in fa4ae9f; released as 1.2.13 (c927683).
