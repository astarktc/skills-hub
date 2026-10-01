# 19 — Docs, CHANGELOG, AGENTS.md invariants, `version:set 1.3.0`, release; operator VoiceOver smoke

Status: ready-for-agent
Blocked by: 15, 16, 17
Spec: `.scratch/round19/spec.md` — D31, D32; § Closure checklist (19a).

## Work

- **CHANGELOG 1.3.0** block: the new world (tokens, primitives on Base UI, `App.css` ratchet), My Skills (rows, strip,
  exception summary, Issues chip, group Deploy all, singleton fold), inspector, selection + bulk rail, Disable as the
  durable un-deploy (and the removal of "Uninstall from tool directories"), persisted skill health, composite
  headlines and `declined`/`cancelled`, notifications by run, header progress, Add as one surface with one ask and
  picker search, Projects keyboard model and cell states, coexistence, min window, pre-paint theme, EN-only copy
  pass on touched strings. Migration notes: schema additions (`disabled`, health columns), the three unsync commands
  replaced, `install_*_selection` replaced.
- **AGENTS.md**: every invariant the tickets changed is re-read and made true (styling bullet, Dialog primitive,
  state-management bullet with the new modules, the fan-out bullet with the install batch, the sync-target mutation
  bullet with disable/enable, Commands with `styles:check`, the removed "(round-19 world ticket lands …)" clause);
  README supported-tools table untouched unless adapters changed (they did not).
- **CONTEXT.md**: *Disabled*, *Skill health*, *Inspector*, *Selection*, *Run report* as terms; the removal of
  "unsync" as an operator verb.
- `docs/adr/0006`: § 7 marked accepted (D2); `DESIGN.md` linked from README's contributor section.
- `npm run version:set 1.3.0`; `npm run version:check && npm run check`; commit; `git fetch && git rebase
  origin/main` (take origin's `featured-skills.json` if it conflicts); push `main` — the push is the release.
  Watch `auto-tag.yml` → `release.yml` (five targets + `updater.json`); record run ids here.
- **Operator smoke list** (D31 human half), written here for Alex: install the build; both themes; the six core flows
  keyboard-only with VoiceOver; `prefers-reduced-motion: reduce` on; window refuses <960×640; Disable survives
  Refresh on the real library (one skill, one Tool, reversible); select-all → Disable on all Tools shows the count and
  is **cancelled**; Add of a repo with a collision raises one ask.

## Result

(child fills in; the orchestrator adds the smoke outcome)

## Comments

- 2026-10-01 — from ticket 17 (CHANGELOG / README inputs): **Sync All skips a project whose folder is missing** —
  one line "archived-app: folder not found — Update path or Remove" instead of a failure per assignment, and it no
  longer recreates the missing folder's Tool dirs (it used to: `sync_engine` creates parents). New per-cell **Sync
  this** (`resync_project_assignment`) for stale / missing / failed assignments. The Projects surface: keyboard project
  selection, Tool labels + "not detected" in the matrix, cell states as mark + word (legend), "also global" cells
  enabled (D17), Assigned · Issues chips, a header select below 1200 px, a "statuses may be out of date — Refresh view"
  line, Remove project closes with a kept copy shown as a failed cell (ADR-0002). Project batch toasts read as the count
  line; a disabled skip now tones a project re-sync "skipped" (was success).
