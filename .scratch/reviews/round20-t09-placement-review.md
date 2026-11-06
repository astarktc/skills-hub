# Round 20 t09 — placement and one row per folder: adversarial review (round 1)

- **PR:** #21 (`t3/placement-per-folder-rows`)
- **Commit reviewed:** `cf229bb` against `origin/main` `07c4550`
- **Reviewer:** GPT-6 Astra (high), 2026-11-06
- **Verdict:** **BLOCK**, until the ownership regression is fixed.

## Findings

### 1. P1: grouping rows by folder hands ownership from one artifact to another, so a delete can run unconfirmed

Sharing a folder key does not prove that two legacy rows describe the same artifact. Project rows keep their own
`skill_name`, which is used to locate the artifact after a rename.

- **Overwrite permission:** granted from any sibling row's ownership (`project_sync.rs:841–848`).
- **Settlement:** `mirror_settlement` marks every sibling synced (`project_sync.rs:922–944`), and Propagation mirrors
  ownership the same way (`propagation.rs:385–410`).

Reproduced in two Rust tests:

1. **Silent overwrite.** Seed an owned `trae_cn` row under `previous-name` and a never-synced `trae` row whose
   current-name path holds a different foreign folder. Resync borrows `trae_cn`'s ownership, silently replaces the
   foreign folder and reports `Synced`.
2. **Deletion after a mirrored settlement.** With the keys reversed, the canonical row syncs and `SyncCompleted` is
   mirrored onto the never-synced sibling. A later unassign then deletes the foreign folder.

The removal fence can't catch this: the foreign folder sits inside a legitimate project skills folder, and the fault
is the invented ownership.

**Fix:** keep folder grouping separate from artifact ownership. Share overwrite permission or settlement only between
rows proven to name the same physical artifact. Keep independently located legacy rows as they are until the
redeploy (ticket 10) reconciles them. Add both cases as regression tests.

### 2. P1: once Amp or Kimi is selected, Settings can't switch either off

- `setHarnessSelected` deletes only the clicked harness key (`useSyncOrchestration.ts:762–776`).
- The backend saves the selection as the union of folders and reads it back expanded to every harness each folder
  serves (`settings.rs:343–377`, `:209–211`, `:443–451`).

Reproduced through the real hook: start with `global_selected_folders_v2 = ["amp"]` and switch Kimi off. The result
is `["amp", "kimi_cli"]` where `[]` was expected.

**Fix:** apply a selection change to the whole folder through `ToolInfoDto.folder_key`, or carry the folder selection
through the hook. Add on and off tests for either member, including after migration.

### 3. P2: project Propagation can skip a legacy copy that needs updating

- Propagation acts on one representative row per folder (`propagation.rs:369–376`); that row's adapter, mode and
  ownership alone decide whether new bytes are needed (`:445–470`).
- Resync reads the deploy capability from the legacy member key, not from placement (`project_sync.rs:715`).

Reproduced: a canonical `trae` row recorded `Symlink` and a sibling `trae_cn` row recorded `Copy` over a real copy
with old bytes. The result is `Skipped { LinkFollowsSource }` and the copy keeps its old bytes.

**Fix:** resolve the folder's capability through placement, and decide whether new bytes are needed from every
legacy row on the same physical artifact. Combine this with finding 1's physical-artifact check; never copy
ownership between rows to solve it.

### 4. P2: removal reports still count legacy sibling rows as separate Deployments

- Folder-scoped planning includes both sibling rows (`artifact_removal.rs:346–359`).
- `removalCounts` counts `target.rows.length` (`reportOutcome.ts:398–407`).

Reproduced: one removed artifact with `amp` and `kimi_cli` row references folds to `outcome.done.disabled {count: 2}`.
That disagrees with the one-folder catalog and the confirmation counts. Failed entries also stay one per row.

**Fix:** keep the raw row references for settlement, and give presentation counts and failure de-duplication a
folder identity owned by the backend. The frontend still infers nothing from paths.

## The implementer's five reported limits

| Limit | Assessment |
|---|---|
| Amp and Kimi switches toggle together | The coupling is fine, but the implementation fails: off snaps back on (finding 2). |
| Project configuration keeps harness keys | Acceptable at ticket 09's scope; tickets 11 and 15 take over. |
| Legacy project rows use the member's capability | Must fix (finding 3). |
| A corrupt v1 selection keeps naming the old key until saved | Acceptable. |
| No fixture or browser check | Not blocking, but the "no user-visible change" claim is unproven; exercise the switches after fixing. |

## Verified sound

- **Placement** is pure: it does no filesystem or store reads.
- **Folder-key tests** cover every shared dir, including group ownership and the registry-order fallback.
- **New rows** are written once per folder; the per-sharer fan-out is retired.
- **Ordinary legacy siblings** that name the same artifact are de-duplicated in listings and counts.
- **Dropping one project harness** while another still uses the folder leaves the folder's Deployments intact.
- **Selection migration:**
  - v2 takes precedence over v1;
  - an explicit empty selection is preserved;
  - the v1 translation is idempotent;
  - unknown keys are pruned;
  - a corrupt value stays distinguishable from an unconfigured one.
- **Bindings** carry the new fields with no drift.
- **Guards:** no entry point newly calls another.
- **Rewritten tests** were rewritten, not deleted.

**Gate on the branch:** `npm run version:check && npm run check` passed (1,387 frontend tests, 1,046 Rust tests, 1
ignored). The adversarial probes failed as described above.

## Disposition (orchestrator, 2026-11-06)

All four findings go to a fresh fix child on the PR branch. Re-review after the fix.
