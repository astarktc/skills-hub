# 21 — Import review: unchecked conflicts and managed names, Replace/Leave, remove-originals checkbox, counts, variant details, Tool labels, first-run empty state

Status: ready-for-agent
Blocked by: 05, 19
Spec: `.scratch/round19/spec.md` — D13, D14 (grill Q11, Q12); absorbs BACKLOG **#46**'s UI half. Sources: `flows.md`
Flow 3; `critique-astra` F08; `critique-opus` P1-5; `audit.md` Import rows. Phase 19b.

## Work

- Review dialog on the new primitives: header counts "8 skills · 11 copies in 8 tools"; groups as rows; **Match**
  quiet (not amber), **Conflict** the exceptional row; conflict groups and **managed-name** groups start unchecked;
  managed-name groups offer "Replace *Cursor*'s copy with the managed skill" / "Leave"; conflict variants show
  description + modified time + Tool label, radios named by Tool + path; invalid manifests listed disabled with the
  reason in words.
- **Policy line + checkbox** before the button: auto-sync on — "Your identical originals in *Cursor, Claude Code* are
  replaced by links to the managed copy"; auto-sync off — "Remove my 9 identical originals from *Cursor, Claude Code*"
  checkbox (default = current policy); the button names the action ("Import and sync" / "Import and remove 9
  originals" / "Import"). Report lists removed / left-in-place originals per Tool; divergent siblings are one row and
  route into the consolidated ask (ticket 04).
- After a partial import: failed groups deselected and annotated; imported groups gone; the dialog stays.
- **First-run**: the empty state *is* the invitation (ticket 11's design); a large import completing gets D6's one
  deliberate moment (brief, does not gate interaction, removed under reduce).
- Migration: `ImportModal` joins `migrated`; its rules leave `App.css`.

## Gate (D31)

Astra audit on `first-run`: keyboard-only review with a conflict chosen by arrows and a managed-name group set to
Replace; auto-sync-off path shows the checkbox and the button text changes; counts match the plan; contrast;
`npm run check`.

## Result

(child fills in)
