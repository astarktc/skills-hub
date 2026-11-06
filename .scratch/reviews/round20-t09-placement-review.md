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

---

# Round 2: re-review of fix pass 1 (`4824ef6`)

- **Reviewer:** GPT-6 Astra (high), 2026-11-06. The first attempt was stopped by OpenAI's content filter; the retry ran
  with neutral wording.
- **Verdict:** **BLOCK**.

## Round-1 findings

| # | Status | Evidence |
|---|---|---|
| 1 | Partly fixed, still open | Rows naming independent paths are separated now, and both round-1 reproductions pass. Two new routes to unowned ownership remain (findings 5 and 6); finding 7 is a side effect of the fix. |
| 2 | Closed | `useSyncOrchestration.ts:768–784` toggles the whole folder; tests at `useSyncOrchestration.test.ts:1174–1273` cover on and off for either member, before and after migration. |
| 3 | Closed for rows naming the same artifact | `project_sync.rs:206–217` (`deploy_adapter` resolved through placement); `propagation.rs:412–422`. The Deep Agents Code test switch is correct: a group's capability stays its own registry fact (`placement.rs:328–340`, `tests/placement.rs:244–264`). |
| 4 | Closed for current reports | `artifact_removal.rs:175–193`, `reportOutcome.ts:397–427` and `:704–722`. Old stored reports are finding 8. |

## New findings

### 5. P1: equal inodes don't prove that two names are one Deployment

`removal_fence.rs:917–922` treats two present paths as the same artifact whenever their inodes match, and that grouping
grants overwrite permission and settlement (`project_sync.rs:913–917`, `:954–963`). Two different names can be hard
links to the same symlink inode.

Reproduced:
1. Make `current` a link into the Library, then hard-link it as `previous`.
2. Seed an owned `trae/current` row and a never-synced `trae_cn/previous` row.
3. Resync. Only `current` is replaced, yet `previous` is marked owned and synced.
4. Unassign. The never-written `previous` link is deleted.

### 6. P1: grouping two absent paths lends overwrite permission to whatever appears there later

Absent paths are grouped by parent and leaf name before the write (`removal_fence.rs:923–934`,
`propagation.rs:209–214`), and a sibling's ownership becomes an unconditional overwrite flag (`propagation.rs:259–280`;
`sync_engine.rs:191–205`). When a foreign folder appears at that path after the grouping, it is replaced with Library
bytes (reproduced with a deterministic interleaving). Two spellings naming one empty slot at observation time is not
evidence that a later occupant is ours.

### 7. P2: Enable silently deletes an independent disabled record whose folder still exists

`project_sync.rs:1028–1037` deletes every disabled independent assignment, before the driver's sync succeeds. A
disabled row can be a never-owned Keep marker over an existing foreign folder. Reproduced: the old folder still
exists, the record is gone, and nothing reports it. This contradicts ticket 09's boundary that collapsing legacy rows
belongs to the redeploy.

### 8. P2: a required `RowRef.folder_key` drops stored 1.3.4 Library-move reports

`artifact_removal.rs:147–164`, `library_move.rs:244–250` and `:476–485`: a 1.3.4 report fails with `missing field
'folder_key'` and is dropped. A bare `#[serde(default)]` is wrong: it yields `""` and makes the TypeScript field
`folder_key?: string`. Instead, add a narrow decoder for stored reports that fills the key from scope + tool, keep the
field required on the wire, and add a stored 1.3.4 report fixture that includes Amp and Kimi rows.

## The implementer's open items

- **(a) Independent stale copies wait for ticket 10:** acceptable, but finding 7 must not erase the records the
  redeploy will need.
- **(b) Stored reports:** must fix (finding 8).
- **(c) My Skills dots:** out of scope; ticket 13 owns them.

**Gate on the branch:** `npm run check` and `cargo test --all` passed (1,398 frontend tests; 1,061 Rust tests, 1
ignored), with no binding drift. Four probe assertions failed, matching findings 5–8.

## Disposition (orchestrator, 2026-11-06)

Fix pass 1 inferred ownership from physical identity. That is the wrong direction: physical identity may make the app
more cautious, but it must never grant ownership. Fix pass 2 removes the inference instead of patching it:

- **Driver row.** The folder's driver is chosen among its rows, preferring a row that owns its artifact, so overwrite
  permission is always the driver's own and is never lent by a sibling.
- **Shared settlement.** After a successful write, settlement extends only to rows whose recorded `target_path` is
  byte-identical to the path just written. That path is now the app's artifact, so recording those rows as its owners
  is true.
- **Everything else is independent.** Aliases, hard links and absent-slot matches are independent rows, left untouched
  for the redeploy (tickets 10 and 11). `same_artifact` may still narrow a decision (refuse, de-duplicate a display) but
  never widens one.
- **Enable** leaves independent rows as they are. The folder's enabled or disabled state is read from the driver and
  its byte-identical siblings only.
- **Stored reports:** add the compatibility decoder from finding 8.

---

# Round 3 (Astra): review of fix pass 2 (`c52ff4d`)

- **Reviewer:** GPT-6 Astra (high), 2026-11-06, briefed with the principle only.
- **Verdict:** **BLOCK** under that principle.
- **Origin of the findings:**
  - Finding 9 is a regression this branch introduced.
  - Findings 10 and 11 predate the PR: the same behaviour is on main at `07c4550`.

## Findings

### 9. P2: global Enable overwrites an independent legacy record that holds the folder key (regression on this branch)

`admit_folders` correctly deletes only the disabled rows on the driver's path (`skill_disable.rs:439–455`). But the
batch then receives folder keys, not the driver's row (`:527–537`). A successful sync always records under the folder
key (`global_sync.rs:145–156`), and the upsert replaces whatever row holds that key (`skill_store.rs:1590–1595`).

Reproduced:
1. An owning `kimi_cli` row records `.config/agents/skills/alpha`.
2. A disabled, never-synced `amp` Keep marker records `independent`, over a folder the app didn't create.
3. Disable the shared folder, then Enable it.

The `amp` row becomes `alpha / owned / synced`. Its old path and Keep answer are lost. The existing test only covers
the reversed keys.

**Fix:** carry the driver's row identity and recorded path through the global write and its settlement. If an
independent record holds the folder key, keep recording under the driver's legacy key until the redeploy reconciles
the two. Add the reversed-key test.

### 10. P1: a successful Disable keeps the right to delete whatever later appears at the path (pre-existing)

Ownership is `synced_at.is_some()` (`skill_store.rs:211–213`), and Disable keeps `synced_at` at both scopes
(`:1651`, `:1685`, `:1721`, `:1742`). Removal therefore plans the path and deletes whatever is there
(`artifact_removal.rs:593–597`, `:659–671`).

Reproduced: deploy, Disable (the folder is removed), put a different folder at that path, Delete the skill. The
different folder is deleted. The existing test `an_owned_disabled_target_is_still_removed_by_delete` pins this
behaviour on main.

**Fix:** clear ownership when Disable removes the artifact or confirms it is absent, and keep it when the removal
fails. Existing disabled rows need conservative handling on upgrade. Replace the old test with tests that a different
folder put there later is kept, at both scopes.

### 11. P1: an owning row's old timestamp still authorises overwriting a folder that replaced its copy (pre-existing)

Resync and Propagation grant overwrite whenever the driver `owns_artifact()` (`project_sync.rs:876–878`;
`propagation.rs:255`, `:407–430`). That checks only the timestamp, not what is at the path now.

Reproduced: assign to a project, delete the link, put a different folder there, resync. It is overwritten and the
report says `synced=1 failed=0`.

**Fix:** check what is at the path before replacing it. A link must still point at its expected Library copy; a copy
needs evidence tied to the content that was deployed. Anything different or unverifiable stays put and goes to the
overwrite ask. Check again at the point of replacement.

## The implementer's residuals

| # | Residual | Ruling |
|---|---|---|
| 1 | The folder-key upsert loses an independent record | Must fix in ticket 09 (finding 9). |
| 2 | A disabled former owner keeps `synced_at`, so Delete removes a later occupant | Must fix under the principle (finding 10, pre-existing). |
| 3 | An owning row replaces whatever is at its own path | Must fix under the principle (finding 11, pre-existing). |

## Earlier findings

| # | Status |
|---|---|
| 1 | The original reproductions stay closed; finding 9 is a separate problem. |
| 2 | Closed. |
| 3 | Closed for rows on the same path. |
| 4 | Closed. |
| 5 | Closed: `same_artifact` no longer grants anything, and the hard-link test passes. |
| 6 | Closed as a sibling-lending finding; finding 11 is the separate timestamp rule. |
| 7 | Closed at project scope; the global analogue is finding 9. |
| 8 | Closed. |

**Fixture check (registry scenario):** turning Kimi or Amp on or off moves both together, Pi stays selected, and the
deploy offer still appears.

**Gate:** `npm run version:check && npm run check` passed (1,399 frontend tests; 1,068 Rust tests, 1 ignored), and
`cargo test --all` passed. Three throwaway probe tests reproduced findings 9–11.
