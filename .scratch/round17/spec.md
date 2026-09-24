# Round 17 — honest detection, scoped scan, overwrite ask → 1.2.17

Status: open
Opened: 2026-09-23

Absorbs BACKLOG **#42** (overwrite ask on `TARGET_EXISTS`, recorded and absorbed in this opening commit). The two
defects were found by the operator while smoking 1.2.16: ego-browser's installer dropped
`~/.<tool>/skills/ego-browser → ~/.local/share/ego/ego-skills` into 25 tool dirs that had never existed, and
Skills Hub reported all 25 tools installed and listed 27 copies of one skill in Review & Import despite "Only scan
for existing skills within selected tools" being on.

Baseline: 1.2.16 (`d8218d0`, tag `v1.2.16`, release published with all five targets). Main @ `a1a3b94`.
Round 16 stays live until its ticket 08 (Windows junction smoke) closes; nothing here depends on it.

## Problem (verified against `main` @ `a1a3b94`)

1. **The scan-scope setting never reaches the scan.** `scan_selected_tools_only` is read only by
   `src/hooks/useSyncOrchestration.ts:35` (`filterRelevantNewlyInstalled`), which suppresses the "new tools
   detected" popup. `core/onboarding.rs:65` (`build_onboarding_plan_in_home`) loops every adapter where
   `is_installed_in()` holds and sees neither the setting nor the selection. The checkbox promises a scope the
   backend does not apply.
2. **Installedness is `detect_dir.exists()`** (`core/tool_adapters/mod.rs:802`). A `~/.kiro/` whose only content
   is `skills/ego-browser` counts as Kiro installed. Every one of the 25 false positives has that shape. Any skill
   deployer (`npx skills add`, ego, Skills Hub itself when a selected tool is later uninstalled) leaves it, so
   the fault is not ego-specific. Consequences: the Tool config modal badges phantom tools "(installed)";
   `record_installed_tools` persisted 25 phantom keys; the onboarding scan scans them.
3. **A first sync onto an occupied target is a dead end.** `BatchSyncPolicy { overwrite, overwrite_if_same_content,
   overrides }` (`core/global_sync.rs:200–208`) is honoured by the engine, but every frontend call site passes at
   most `{ overwriteIfSameContent: true }` — `overwrite: true` appears nowhere. A target with *different* content
   settles as a `TARGET_EXISTS` row and the fold renders "Please remove it and try again". Refresh is the only
   workaround (Propagation always overwrites, `core/propagation.rs:248`), and only for a pair that already has a
   target row. `handleSyncSkillToAllTools` (`useSkillLibrary.ts:240`) does not even pass the same-content rule.

## Goal

The scan honours the setting; a skill deployer's footprint is not a tool; a first sync onto an occupied target
with different content asks before overwriting, while identical content keeps being replaced silently (operator
requirement, 2026-09-23). Released as 1.2.17.

## Decisions (accepted by the operator 2026-09-23)

- **D1 Scan scope is resolved at the seam, applied in core.** `core/onboarding.rs` gains
  `OnboardingScanScope::{Installed, Selected(Vec<String>)}`; `build_onboarding_plan(home, central, store, scope)`
  scans the installed adapters under `Installed`, exactly the named keys under `Selected` (an absent dir yields
  nothing; `total_tools_scanned` counts the scope). `commands/mod.rs::get_onboarding_plan` reads
  `scan_selected_tools_only` + the global selection and builds the scope: on + configured → `Selected`; otherwise
  `Installed` (the frontend's existing fallback for the popup, kept symmetrical). No wire change.
- **D2 A skills-only footprint is not an installed tool.** `is_installed_in` = detect dir exists **and** it is not a
  *skills-only footprint*: walking from the detect dir along the components of the skills dir, every directory on
  the way holds exactly one entry — the next component. An empty detect dir stays installed (unchanged);
  `~/.pi/agent/{settings,skills}` stays installed; `~/.kiro/skills/x` alone is not. **Virtual-group entries are
  exempt**: their detect dir *is* the convention (`~/.agents` legitimately holds only `skills/`), so presence
  alone counts, as today. Stronger per-tool signals remain Future effort #40. `global_sync` already skips
  undetected tools, so a selected tool reduced to a footprint is now reported as a skip — the pre-existing rule
  applied honestly.
- **D3 The overwrite ask lives in the one sync seam.** `useSyncOrchestration.syncSkillsToTools` runs the batch;
  if the report carries `TARGET_EXISTS` rows it raises one confirmation (a `useOverwriteConfirmation` building
  block shaped like `useSharedDirConfirmation`, rendered by `OverwriteModal` through the Modal shell, wired in
  `App.tsx`) listing skill → tool → path. **Confirm** re-runs one batch over the affected skills × affected
  tools with a per-pair `overrides` entry for each asked pair and the caller's same-content rule, and replaces
  exactly the asked rows in the report; **Cancel** returns the first report unchanged (the rows stay
  `TARGET_EXISTS` failures with copy that no longer says "remove it and try again"). Call sites are untouched
  except that `handleSyncSkillToAllTools` adopts `{ overwriteIfSameContent: true }` so identical content never
  reaches the ask. The ask happens under the loading overlay, so the modal layers above it (`z-index` rule) and
  does not disable on `loading`; the action message reads "waiting for confirmation" meanwhile. The confirm click
  is the operator action that authorises the second batch — still one batch per action.

## Tickets

| # | Ticket | Lane |
| --- | --- | --- |
| 01 | `01-backend-scan-scope.md` — D1 | Rust |
| 02 | `02-backend-footprint-is-not-installed.md` — D2 | Rust |
| 03 | `03-frontend-overwrite-ask.md` — D3 | TS |
| 04 | `04-docs-and-release.md` | orchestrator |
| 05 | `05-review-fixes.md` — Astra review M1 M2 S1 S2 S3 | both |

## Review (Astra, 2026-09-23 — `review/astra-review.md`)

Verdict fix-then-ship; every finding applied in ticket 05, each in the fuller form where the smallest fix would
have bent a rule: M2 became a registry correction (`relative_detect_dirs` slice; Amp `.config/amp`, Kimi
`.kimi-code`/`.kimi`) rather than a presence-only exemption for `.config/agents`; M1 reloads the plan on a
configuration revision **and** on Review; S1 gives the ask explicit ownership (ref, one-shot, displacement,
unmount); S2 makes any enumeration error read as installed; S3 takes the "thrown requests also reload"
precedent rather than synthesising report rows.

## Closure checklist (operator smoke on the installed 1.2.17)

- Tool config modal no longer badges ego's footprint dirs "(installed)" (any you have not deleted yet).
- Review & Import with the scan setting on lists only skills from the four selected tools; with it off, every
  detected tool.
- Sync a skill onto a tool where a foreign folder of the same name sits: differing content → the ask; Overwrite
  replaces it and the row turns Synced; Cancel leaves it and the toast says so. Identical content → replaced
  silently, no ask.
