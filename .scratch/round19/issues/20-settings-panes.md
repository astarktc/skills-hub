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
