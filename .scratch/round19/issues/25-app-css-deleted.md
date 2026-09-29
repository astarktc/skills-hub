# 25 — `App.css` deleted; the guard flips to "must not exist"; `Layout.tsx` / `Dashboard.tsx` disposition

Status: ready-for-agent
Blocked by: 20, 21, 22, 23, 24
Spec: `.scratch/round19/spec.md` — D32; ADR-0006 §5–§6. Phase 19b closure.

## Work

- Every remaining `App.css` rule is either migrated with its surface (there should be none — list them if any and
  migrate), moved to `prose.css` under the prose root, or deleted with the alias block. `git rm src/App.css`;
  `scripts/check-styles.mjs` rule 1 becomes "`src/App.css` must not exist"; `styles-baseline.json` drops
  `appCssCeiling`.
- `src/components/Layout.tsx` and `src/pages/Dashboard.tsx` (dormant by design) would render unstyled: propose
  deletion to the operator in the Result with the evidence that nothing imports them; delete only on the operator's
  word (AGENTS.md says it is an operator call).
- AGENTS.md: the styling bullet loses every `App.css` sentence; ADR-0006 gains a dated "`App.css` deleted" note.

## Gate

`npm run check` (guard passes on the absence); fixture walk of every surface in both themes shows no unstyled element.

## Result

(child fills in)
