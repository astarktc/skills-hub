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

## Comments

- 2026-09-29 — from tickets 02/05: rename `list_all_skill_target_paths` to say "owned" (it now returns only rows the app materialised); the `replaced` group's `TARGET_EXISTS` rows route into ticket 04's one ask — decide whether the review's "Replace" button is the confirmation or raises the ask (05 chose not to force-overwrite from the plan because a copy can change between review and import).

- 2026-09-30 — from ticket 15: the strip light exists — `StripMarkIcon` + `useSyncLight` fade a mark that *becomes* synced
  on screen from opacity 0 over 120 ms (`@starting-style`, never on mount; reduced motion shows it settled). The rare
  delight (DESIGN.md § Empty and first-run) only has to add the stagger: a per-mark `transition-delay` (20 ms × index)
  for the imported rows, once. It is one of the two permitted rare moments in DESIGN.md § Motion.

- 2026-09-30 — from ticket 16: Add no longer says "Import" anywhere (the `outcome.done.installed` count reads "N added";
  an Add candidate's failure title is `add.failedTitle`); `errors.importFailedTitle` is Import's alone now. The one
  candidate picker (`skills/add/CandidatePicker.tsx`) — search, "Select all shown", rows named by the candidate,
  unavailable rows disabled with "Can't be added: {reason}" in words — fits the review's invalid-manifest rows and
  group checkboxes (D14) if a `mode` for variants is added. `ImportModal` still uses `.inline-checkbox` (its last user
  in `App.css`). Import keeps `HeaderProgress placement="dialog"` at its foot; `progressInDialog` in App lists Add,
  Import, Re-point and Notifications only (the pickers are gone).
