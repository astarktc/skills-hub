# 22 — Explore as tracker rows: Not-in-library filter, Preview primary, scope summary, installed → inspector, contrast + search label

Status: ready-for-agent
Blocked by: 19
Spec: `.scratch/round19/spec.md` — D25 (grill Q19); round-18 D10. Sources: `critique-astra` F11, Explore reads;
`critique-opus` P2-1, Q6; `audit.md` Explore row. Phase 19b.

## Work

- Rows through the list lens (search with an associated label; chips **Not in library** (default) · **Installed** ·
  **Hidden**); row = name · source · description · popularity as a secondary numeric · installed mark; row focus opens
  the **preview** in the inspector (the `useSkillDocument` module from ticket 12, fed the Explore document — no forged
  `ManagedSkill`); **Install** as one action with an inline Tool-scope summary that stays visible; an installed
  candidate's row and preview route to the **managed inspector** (surface state `focus({kind:'skill', id})`), never a
  second Install; the preview's acquisition time is labelled as such, never as freshness.
- Search mode line rewritten (featured vs online; minimum query length as inline hint, not a paragraph).
- Hide moves to the overflow; Unhide from the Hidden chip.
- Dark-mode Install/Installed pairs on tokens (the audit's 2.14:1 / 1.82:1 are the regression list).
- Migration: `ExplorePage` joins `migrated`; `explore-*` rules leave `App.css`.

## Gate (D31)

Astra audit: keyboard-only search → focus a row → preview → Install with scope shown; an installed row opens the
managed inspector; contrast both themes; 960×640; `npm run check`.

## Result

(child fills in)

## Comments

- 2026-09-30 — from ticket 14 (recorded by the orchestrator; the 14 child noted it only in its Result): the Explore preview leaves focus on the main landmark rather than its own heading when it opens (`ExplorePreviewView.tsx`, which now uses `useSkillDocument`). Move the arrival focus to the preview's heading (`tabindex="-1"`, D21) and return to the row on close, the way the inspector does.
