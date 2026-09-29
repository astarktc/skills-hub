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
