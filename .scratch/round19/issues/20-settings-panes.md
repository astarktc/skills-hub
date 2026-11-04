# 20 — Settings panes; Configure Tools moves into Settings → Tools; theme radio group; named header controls

Status: ready-for-agent
Blocked by: 19
Spec: `.scratch/round19/spec.md` — D24 (grill Q18). Sources: `critique-astra` F16, Settings read; `critique-opus` Q8,
P2-6; `audit.md` Settings rows. Phase 19b.

## Work

- Panes inside one Settings surface (a vertical pane list, not a second app navigation): **General** (appearance as a
  radio group with `aria-checked`, zoom, auto-sync policy — moved here from the My Skills toolbar), **Tools** (global
  Tool selection = the effective target set, with the "selected but not detected" rule explained inline; **scan
  scope** visibly distinct; the effective summary shown on save; `ToolConfigModal` retired — the My Skills header and
  ⌘K deep-link to `#/settings/tools`), **Storage & cache** (central repo path, git cache TTL, clear now), **GitHub**
  (token presence, set/clear — the keychain assurance kept), **Updates** (from `useAppUpdate`), **Diagnostics** (log
  folder, version).
- Every control labelled; the header gear named; light helper text on the measured token; auto-save vs explicit save
  disclosed consistently.
- Migration: `SettingsPage`, `ToolConfigModal` (deleted) join `migrated`; `settings-*`, `tool-*` rules leave `App.css`.

## Gate (D31)

Astra audit: keyboard through every pane; theme changed by arrow keys; Tools pane explains Windsurf's presence;
contrast; 960×640; `npm run check`.

## Result

(child fills in)

## Comments

- 2026-09-30 — from ticket 15: the legacy `.settings-toggle-knob` now slides on the tokens (`--motion-fast`, `--ease-out`)
  instead of 200 ms `ease`; the pane migration should replace it with the `Switch` primitive (which also offers
  `focusableWhenDisabled`). `.settings-select` / `.settings-theme-btn` lost their `transition: all` (instant hover).

- 2026-09-30 — from ticket 16: `shared/ToolConfigModal.tsx` (Configure Tools) migrated onto the primitives (Checkbox /
  Field / Badge / Button; joins `migrated`), because the `pick-*` and `tool-pick-list` / `tool-filter-toggle` rules it
  used had to leave `App.css` with the Add pickers. Behaviour unchanged (detected-only filter, a selected undetected
  Tool stays listed "not detected", scan-only toggle). When Configure Tools moves into Settings → Tools, carry it as is.
  Add's Tool scope ("Edit Tools" in Add's summary) is a session choice over the same catalog, not the saved selection.

- 2026-10-27 — from ticket 18's think-aloud (`18-operator-think-aloud.md` § Findings, P3·Q1, P5·Q3):
  - **Auto-sync's new home.** Asked to turn it off, the operator looked in the header, then Configure Tools, then found
    it in Settings. He agrees with the move. Consider a one-time "Auto-sync moved to Settings" hint for upgraders, or a
    link from Configure Tools.
  - Auto-sync drives the re-assert behind BACKLOG #71, and nothing on the main screen says it is on.
  - **A selected Tool that is no longer detected** (Windsurf in the fixture) puts a permanent hollow mark in every strip.
    He understood why, but it's standing noise. Tools could flag "not detected — remove from the selection?".

- 2026-11-02 — **grill decisions that reshape this ticket (spec D37, D47, D48, D49, D50; vocabulary D45 — read
  "Tool" here as Harness, "Sync" as Deploy, "Auto-sync" as Auto-deploy; ticket 32 renames the strings first):**
  - **Settings is a dialog, not an area (D37).** A large modal on the layer stack (max ≈ 880 × 640, capped at 90 vw /
    85 vh, not resizable — D50), pane tabs on the left: General · Harnesses · Appearance · Updates · Diagnostics. Opened
    from the sidebar's Settings item; Escape closes; focus returns there. `#/settings/<pane>` leaves the hash and the
    settings area/page is removed (`SettingsPage.tsx` becomes the dialog's body). Keyboard shortcuts adopts the same
    frame. Configure Tools becomes the **Harnesses** pane.
  - **Harnesses pane = per-harness expandable rows (D47)**: detected or not, the directory in use, the deploy mode in
    use, "remove from selection" for an undetected one (the Windsurf noise above). No speculative options — the
    AGENTS-standard-dir preference is BACKLOG #83, gated on research.
  - **Adding a harness with auto-deploy on asks once (D48)**: "Deploy your N skills to Cursor now?" → one global batch,
    one overwrite ask; declined or auto-deploy off → nothing (hollow marks; Deploy all per group). Never automatic.
  - **Auto-deploy's state shows where it acts (D49)**: Add's summary, the Import review, this pane — not My Skills.
    The 2026-10-27 "nothing on the main screen says it is on" concern is answered by ticket 33 (auto-deploy no longer
    changes anything on Update), not by a header indicator. The upgrader hint ("Auto-sync moved to Settings") is
    dropped: 1.3.0 already moved it.
- 2026-11-02 — Vocabulary: read Tool/Sync/Refresh as Harness/Deploy/Update — ticket 32.
- 2026-11-04 — operator grill (autonomous run through 31 → 20 ∥ 21 → 34 → 24 → 25 → 26; results reviewed at the end): the sidebar's Theme menu stays beside the Appearance pane (quick toggle; D37 does not remove it).
