# 04 — One install batch command for Add; `TARGET_EXISTS` consolidated across Add, re-assert and import

Status: ready-for-agent
Blocked by: 01
Spec: `.scratch/round19/spec.md` — D12, D20 (grill Q10, Q14). Source: `flows.md` Flow 1 gaps 2, 6; `critique-astra` F06.

## Why

The picker installs in a **frontend loop**, one candidate at a time, each raising its own overwrite ask under four
stacked dialogs; bytes have landed before the ask; Cancel does not stop the loop; Refresh re-assert and Onboarding
import collisions get no ask at all. AGENTS.md's "one batch command per operator action" holds for sync but not Add.

## Work

### Backend

- **`install_selection` batch command** (`core/install_batch.rs`, declared in `core/mod.rs`): input = source
  (`Git { url, resolution: GitSourceResolution }` from the listing, or `Local { root }`), `candidates: Vec<{ subpath,
  name_override? }>`, `tools: Vec<ToolId>`, `overwrite_if_same_content`, `overrides: Vec<{ skill_name, tool, decision:
  Overwrite | Keep }>`, `deploy: bool`. Per candidate under the guard: acquire → finalize → first sync through
  `global_sync`'s unlocked seam. Answers **one** report: per-candidate `Installed { skill } | Failed { error } |
  Cancelled`, and per (skill, Tool) target rows including `TargetExists` rows **collected, not asked** — the frontend
  raises one ask and calls **the same command again** with `overrides` for only the collided candidates (already
  installed candidates are re-sent as `already_installed` and skipped, or better: the retry uses
  `sync_skills_to_tools` with overrides, since the skills now exist — choose the one that keeps *one* ask and *one*
  report shape, and record the choice in the Result).
- **Cancel**: honours the `CancelToken` **between candidates**; remaining candidates answer `Cancelled` (ticket 03's
  variant). Progress over a `Channel` ("Adding (i/n) name").
- `install_git_selection` / `install_local_selection` are removed from `collect_commands!` when the frontend no longer
  calls them (ticket 16 does the frontend; **this ticket keeps them registered** and adds the batch beside them so
  the build stays green — note it in the Result so 16 deletes them).
- **Re-assert collisions**: `refresh.rs`'s `reassert_auto_sync` already reports `TARGET_EXISTS` rows as failures with no
  ask; classify them so the report row is distinguishable as **askable** (the row carries the typed `TARGET_EXISTS`
  error already — confirm the frontend can select exactly those rows and retry them through `sync_skills_to_tools`
  with per-pair overrides; if the report lacks the pair identity, add it).
- **Import divergent siblings**: `onboarding_import` reports them today as both a warning and a `TARGET_EXISTS`
  failure (`flows.md` Flow 3 gap 7). One row, `TARGET_EXISTS`, askable the same way; the double report goes.
- Every `TARGET_EXISTS` row carries what the ask displays: `target_path`, the existing folder's `modified_at`, and
  `same_content: bool` (the frontend never re-derives it).

### Frontend minimum

- `useOverwriteConfirmation` learns a list shape (several pairs, per-row Keep/Overwrite, one confirm) while keeping
  its `request()`-resolves pattern; the existing single-pair callers keep working. The Add flow itself is rewired in
  ticket 16 — **do not** rewrite `useAddSkillFlow`/`useCandidatePick` here beyond what the build needs.
- `useSyncOrchestration.syncSkillsToTools` passes `Keep` decisions through (ticket 02's `Keep` override) so declined
  pairs record as disabled.

### Fixture

- `install_selection` handler with the same collision behaviour; the `failures` scenario seeds two colliding
  candidates in one repo; cancelled tail producible with `latency=3`.

### Tests

- Batch of three with one collision → one report, two installed, one `TargetExists` row, no bytes written to the
  collided target; retry with `Overwrite` replaces; retry with `Keep` yields a `disabled` row (via 02); cancel after
  candidate 1 → rows 2–3 `Cancelled`; re-assert collision row is askable; import divergent sibling reported once.

### Docs

AGENTS.md § Invariants "Target fan-out is backend-owned" gains the install batch bullet; CONTEXT.md if a term moves.

## Review

Astra @ high: guard ordering per candidate, no partial-finalize leak on cancel, the retry path's idempotence.

## Result

(child fills in)

## Comments

- 2026-09-27 — round-18 extract: the fixture does not produce `TARGET_EXISTS` on re-assert for some seeded occupied targets (`prisma-review` → Cursor/Codex; `round18/review/flows.md` § Fixture gaps) — fix under "Fixture" here.
