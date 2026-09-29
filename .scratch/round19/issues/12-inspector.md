# 12 — The inspector: ladder, per-tool list + grid toggle, repairs, Edit, project assignment, copyable source, Files, sheet collapse

Status: ready-for-agent
Blocked by: 06, 09, 11
Spec: `.scratch/round19/spec.md` — D3, D15, D18, D22, D23. Sources: `critique-astra` F09, F03/Q3; `critique-opus` P1-2;
`flows.md` Flow 5; `audit.md` Detail rows. Absorbs D8's "detail health ladder + copyable source" and "per-row /
inspector project assignment".

## Work

- **Placement**: right third of My Skills for the focused row (surface state's `focused`); below the breakpoint
  (ticket 11's number) a **sheet** over the list; focus rules per D23/D21. `SkillDetailView.tsx` is replaced; its pure
  helpers (`buildTree`, `getLang`, `formatSize`) move to `src/lib/skillFiles.ts` with tests (arch candidate 12) and
  the file loading into `useSkillDocument(skillRef)` — fed a small document value, so Explore's preview (19b) can reuse
  it without forging a `ManagedSkill`.
- **Order of sections** (the operational question first):
  1. **Health** — every row of the fold with its repair inline: Restore (central missing) · Re-point / Detach (source
     missing) · Retry (fetch failed) · resolve Edit conflict (opens Edit with the plain choice "Keep yours (User only)"
     / "Use the skill's (User & model)") · finish Delete (D16's report dialog).
  2. **Targets** — vertical per-tool list (D22): mark · Tool label · sync mode (link/copy) · failure text in product
     words · inline **Retry failed** / **Overwrite and retry** (`TARGET_EXISTS`, through the ask) / **Disable** /
     **Enable**; the click-to-sync **grid** behind a toggle (persisted preference). A toggle-off shows "Removed from
     Pi · Undo" for ~5 s (undo = enable).
  3. **Upstream** — source line from the fold (label · href · **copy** button; local skills show the recorded folder,
     D15), provenance (imported: "Imported from Cursor · no upstream — Refresh skips it", D18), recorded ref/SHA,
     "Last updated"; **Update** and **Change source…** here.
  4. **Edits** — invocation mode state, conflict if any, Edit.
  5. **Projects** — assignments with status; **Assign to project…** (door verb `assignToProject`, project picker with
     Tool choice honouring the project's configured Tools; the matrix stays the per-project view).
  6. **Files** — tree + Markdown/code reader (`prose.css`), **expand** to full width in place and collapse back
     without losing the focused row; `list-style` restored in prose.
  7. Footer: **Remove** (danger, D16 dialog).
- **Re-point modal** (D15): pre-filled with the current source; on submit calls `preview_repoint_source` first and
  shows "replaces *X*'s bytes with **Y** (subpath @ ref)" with a name mismatch highlighted and an explicit Continue;
  multi-skill target opens the Add picker (ticket 16's picker component if landed, else the existing one); errors
  inline by the field, never a toast over the modal.
- **Detach** gets a consequence-stating confirm ("stops following ~/…; Skills Hub's copy becomes the source; Refresh
  will skip it"); the provenance line afterwards reads "Detached from ~/…" (if the backend lacks the field, record it
  in the Result as a follow-up for BACKLOG — do not invent it).
- Every control keyboard-reachable; the inspector is a landmark (`aria-labelledby` the skill name); Tab enters after
  the row.
- Migration rule: files join `migrated`; `detail-*` and `tree-*` rules leave `App.css`; ceiling lowered.

## Gate (D31)

Astra audit: keyboard transcript row → inspector → repair → back to row; sheet at 960×640 with focus rules; contrast;
`local-notes` shows its missing folder and Re-point pre-filled; `docx` shows Restore under Health and no filled marks;
Re-point to a mismatched name requires Continue. `npm run check` green.

## Result

(child fills in)
