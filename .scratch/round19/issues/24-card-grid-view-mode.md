# 24 — Card-grid view mode designed inside the world

Status: ready-for-agent
Blocked by: 19
Spec: `.scratch/round19/spec.md` — D32; contract § Unresolved (grid); round-18 D2 ("both list and card-grid view
modes exist in the new world"). Sources: `critique-astra` F07. Phase 19b.

## Work

- One grid card = the same row model: name · marks after the name · description (2 lines, then ellipsis with a title)
  · strip + N/M · scope · switch · overflow; **no clipped Tool labels ever** (the strip is marks, labels are accessible
  names + the inspector); same selection, same keyboard model (`j/k` moves across the grid by reading order), same
  group headers with the singleton fold; auto and dense densities as the two grid sizes through the lens's view
  mode; persisted preference keys kept (`preferences.ts` compat contract).
- Grid uses the primitives and tokens only; no new tokens unless `DESIGN.md` gains them in the same change.

## Gate (D31)

Astra audit on `rich` in both densities: nothing clipped at 960×640; keyboard reading order; contrast; `npm run check`.

## Result

(child fills in)

## Comments

- 2026-11-02 — Vocabulary: read Tool/Sync/Refresh as Harness/Deploy/Update — ticket 32.
- 2026-11-04 — operator grill (autonomous run through 31 → 20 ∥ 21 → 34 → 24 → 25 → 26; results reviewed at the end): grid still wanted in the T3-style dark world (round-18 D2 stands).
- 2026-11-04 — from ticket 31's standards review (S3): the group header is inline in `SkillsList.tsx` (selection, collapse, identity, summary, initials legend, Deploy all); extract it as a private header component when the grid reuses it — the grid must not copy the block.
