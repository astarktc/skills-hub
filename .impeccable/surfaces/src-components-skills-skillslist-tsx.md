---
version: 1
slug: "src-components-skills-skillslist-tsx"
primary_target: "src/components/skills/SkillsList.tsx"
related_targets: ["src/components/skills/SkillCard.tsx","src/components/skills/FilterBar.tsx","src/components/skills/SkillDetailView.tsx","src/App.tsx"]
---

## Surface

My Skills — the app's primary surface (list of the library grouped by source, inspector for the focused skill, bulk
rail). Visitor mode: **Operate**. Extends to Explore, Projects, Settings and every modal as the one visual world.

## Audience, job, constraints

Alex (and people like him) at a dark desk beside T3 Code and a terminal; the job is "read the library's state at a
glance, deploy/undeploy ergonomically, find things". Constraints: Restrained colour, WCAG AA both themes, keyboard
completion of every core flow, ≥960×640 with one structural collapse, no network fonts, no state library, `t()` for
every string. See PRODUCT.md.

## Direction contract

Decided 2026-09-24 after a head-to-head bake-off of the two top cards (`.scratch/round18/prototypes/{patch-bay,
tracker-list}/`; the roll assigned Patch Bay, seed key 4a508bda; the operator pinned the hybrid below — a user-pinned
direction beats the roll). Code-led.

THESIS: The library is a keyboard-first tracker: grouped rows whose "Synced" column shows *which* tool is missing,
every action reachable from ⌘K, detail in a split pane. It refuses the category default — card grid, status pills,
one blue accent, white ground — and it refuses the rack costume (top rail, engraved caps, hairline-everything,
"patched" vocabulary).

OWN-WORLD: Tracker List's shell and palette — left sidebar, ground `#0e0f12`, panel `#15171b`, raised `#1d2025`,
lines `#262a31`/`#343942`, text `#e6e7ea`/`#a6abb4`/`#858b95`; iris accent `#9690ff` (light `#5a4fdc`, fill `#6a61e8`)
used only for selection, focus and the one primary action; status ok `#3ddc84` / stale `#f5b400` / failed `#ff6b6b`
as state only, healthy is silent. Light mode is Tracker List's (white-family ground), a derivation not a redesign.
System UI stack + `ui-monospace` for identifiers; sizes 12/13/14/16, tabular numerals; 4px base; 36px rows, headers
and palette items; radii 4/6/10; motion ≤120ms opacity/transform on palette, bulk bar and switch only; reduced-motion
removes it. Borrowed from Patch Bay, on its terms: the **per-tool strip** in the Synced column (one mark per
installed tool; hollow = not synced, filled = synced, × = failed; near-invisible when N/N), the **bulk rail** with
`x select · Esc clear` hints, and the **click-to-sync per-tool grid** as an option inside the inspector. Vocabulary is
the product's: synced · not synced · failed · stale (target drift) · disabled · update available (new, separate
channel) — never "patched".

STORY: Open the app → the sidebar says where you are; the list says what's wrong (only exceptions carry a mark, to
the right of the name); the Synced strip says where; the inspector says why and offers the fix inline (Retry failed ·
Overwrite and retry · Update · Edit · Disable) with a Health / Upstream / Edits / Last synced ladder; ⌘K does
anything from anywhere; select rows → the rail deploys, assigns, disables, deletes in one motion.

FIRST VIEWPORT (1440×900, dark): sidebar 200 (My Skills · Explore · Projects; theme, shortcuts, settings at the
foot). Header row: title + count, exception summary ("3 failed targets · 3 stale · View 5 skills with issues"),
Refresh all, primary Add skill. Filter bar: search (`/`), chips All · Issues · Edited · Disabled, Grouped/Flat
toggle. Sticky group header per source: name · count · state summary · Deploy all; singleton sources fold into
"Individual sources" with a muted inline source label. Rows: checkbox · name (+ Edit pencil) · description · Synced
strip + N/M · scope (globe / folder+N) · switch · overflow. Right third: inspector for the focused row. Bulk rail
rises from the foot on ≥1 selection. Below ~1200 the inspector becomes a sheet.

FORM: Tracker List (the pick card) hybridised with Patch Bay (the assigned card); seed key 4a508bda; both prototypes
kept as evidence under `.scratch/round18/prototypes/`.

SIGNATURE INTERACTION: ⌘K palette that reaches every skill and every action, including bulk actions on the current
selection; j/k · x · Enter · Esc row navigation; the Synced strip lighting on sync completion (≤120ms).

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md,
and every shipping raster carrying its provenance.

## Unresolved

- Card grid as a second view mode (operator wants both list and grid) — designed inside this world in round 19,
  not here.
- Whether the inspector's per-tool grid (click-to-sync) is shown by default or behind a toggle.
