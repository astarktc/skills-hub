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

---

# Round 3 (Sol): an independent review of `c52ff4d`

- **Reviewer:** GPT-6.1 Sol (high), 2026-11-06, in parallel with Astra.
- **Verdict:** **BLOCK**.

## Findings

### 101. P1: an Overwrite answer can be applied to a different row's path than the one it asked about (regression on this branch)

Enable picks a driver row and clears its ownership (`project_sync.rs:983–1008`). Collision resolution then picks the
folder's driver again instead of finding the row that raised the collision (`:1074–1108`). The frontend retry carries
project, skill and folder, not the assignment ID or the path it asked about (`useSyncOrchestration.ts:466–474`).

Reproduced:
1. An owning but disabled `trae_cn` row records `previous`, and a never-synced `trae` error row records `alpha`. Both
   paths hold folders the app didn't create.
2. Enable raises `TARGET_EXISTS` for `previous`, and the operator answers Overwrite.
3. The retry picks `trae` and overwrites `alpha`, which the ask never showed.

### 102. P1: a historical `synced_at` authorises deleting or replacing a new foreign folder (pre-existing)

Same class as Astra's findings 10 and 11, reproduced both ways:
- Disable, then a folder appears at the path, then Delete: the folder is deleted.
- An owned link is replaced by a different folder, then resync: the folder is replaced and the report says `Synced`.

### 103. P2: a successful global batch repurposes an independent record that holds the folder key (regression on this branch)

The same finding as Astra's 9, reproduced through a successful global sync rather than Enable.

### 104. P2: the project total shows an error from an independent row that resync never settles (regression on this branch)

The listing shows one row per folder (`project_sync.rs:1331–1337`), and resync acts only on the driver
(`:841–899`), but the project's status total still counts every raw row (`skill_store.rs:1909–1929`). The result is
a Synced matrix next to an Error project total, and resyncing never clears it.

### 105. P2: the sync confirmation counts a shared folder twice (regression on this branch)

`CountConfirmModal.tsx:106–109` maps the selected harnesses to folder keys without de-duplicating, and
`bulkCounts.ts:225–236` multiplies by that list. With Amp and Kimi selected, the confirmation says `added: 2`; the
backend writes one Deployment.

**Gate:** `npm run check` and `cargo test --all` passed (1,399 frontend tests; 1,068 Rust tests, 1 ignored). Five
Rust probes and one TypeScript probe reproduced the findings above.

## Disposition after round 3 (orchestrator, 2026-11-06)

The two reviewers converge. Every finding the branch introduced (101, 9/103, 104, 105) and every finding of rounds
1–2 has one cause: ticket 09 changed how mutations treat existing rows, by having them act on the rows of a folder as
a group. Three fix passes made that grouping safer, and each review round found a new way it goes wrong.

**Fix pass 3 takes that change out of ticket 09.**

1. **Mutations act per raw row, exactly as on main.** Sync, resync, Propagation, collision answers, Disable/Enable and
   Delete each act on a row at its own recorded path, with its own ownership and its own settlement. Remove the driver
   row and every settlement shared between rows. A collision answer applies to exactly the row and path it was asked
   about.
2. **What ticket 09 still changes:**
   - a new row is created under its folder key;
   - one write no longer adds a row for every harness sharing the folder;
   - a row's capability is resolved through placement for its folder;
   - the selection works by folder;
   - removal reports carry a folder key, and stored 1.3.4 reports are decoded.
3. **Grouping by folder is presentation and counting only.**
   - A folder's state is one pure fold over all its rows, and the listing and the project total use that same fold.
   - The fold leaves no state that no action can clear, because per-row resync settles every row, as on main.
   - Counts and confirmations de-duplicate folder keys (finding 105).
4. **Collapsing the existing rows into one row per folder** stays with the redeploy (tickets 10 and 11). The ticket's
   wording is amended to match.

**Findings 10, 11 and 102** describe released behaviour, not regressions. They go to a new ticket, 18 (Deployment
ownership means the current occupant). It is blocked by ticket 09 and blocks ticket 10, because the redeploy needs
the same check: whatever is at a path right now must be verified as ours when the app acts on it.

---

# Round 4: review of fix pass 3 (`228190d`)

- **Reviewers:** GPT-6 Astra (high) and GPT-6.1 Sol (high), working independently, 2026-11-06.
- **Both verdicts:** **FIX-THEN-SHIP**. Neither found a P0 or P1. Neither found a row lending ownership to another, and
  finding 101 is closed. Ticket 18 keeps the historical-`synced_at` findings.

## Astra (201–206): mutations that still differ from `main`

- **201. P2: global sync writes an existing row at its stale recorded path.** It should write at `main`'s destination:
  the planned registry root plus the current skill name (`global_sync.rs:375–386`, `:540–562`, `:115`). Reproduced on
  both commits: the PR creates a destination outside any harness folder.
- **202. P2: global Enable adds a lasting `pending` state** (`EnableAdmitted`) that survives an ordinary
  not-installed skip. `main` deleted the disabled rows (`skill_disable.rs:451–466`, `:483–504`;
  `skill_store.rs:1658–1672`).
- **203. P2: global Propagation admits an uninstalled old row** because a sibling harness of the folder is installed.
  On `main` such a row was `ToolNotInstalled` (`propagation.rs:234–248`). Reproduced on both commits.
- **204. P2: a disabled sibling hides a surviving error**, and the project total reads empty while a link is still on
  disk (`placement.rs:439–481`, `project_sync.rs:1274`, `skill_store.rs:1959`).
- **205. P2: per-folder counting is missing from the Update/Edit (Propagation), Import and Library-move reports**
  (`propagation.rs:101`; `reportOutcome.ts:373–389`, `:675–709`, `:1281`, `:1794–1802`, `:1923`).
- **206. P2: dropping a configured project harness keeps its independent old assignment and artifact** when another
  configured harness shares the folder. On `main`, its removal ran (`project_ops.rs:289–305`).

Astra's module-by-module conformance table is in the child thread. Every other module conforms.

## Sol (301–305): what the surfaces show

- **301. P2:** the same as 204. After a partial Disable, the inspector offers Enable where it should offer to repair
  the failed removal (`skillPresentation.ts:1103–1106`, `:1160–1190`).
- **302. P2: the single row standing in for a folder loses facts the confirmations need.** A Disable confirmation can
  count 0 while Disable removes a deployed copy (`placement.rs:473–477`, `skill_catalog.rs:170–175`,
  `bulkCounts.ts:79–95`, `skillPresentation.ts:629–654`).
- **303. P2: with only Kimi installed, the inspector refuses Retry or Enable on the Amp/Kimi folder as "not
  detected".** The backend can reach the folder through Kimi (`skillPresentation.ts:1084–1097`, `:1187–1190`).
- **304. P2:** the same as 205 (Propagation outcomes).
- **305. P2: the fixture's project totals count raw rows**, while its listing and production fold per folder
  (`fixtures/backend.ts:689–710`, `:733–734`).

## Rulings on the implementer's open items

- The recorded path (Astra) is finding 201, to fix. Sol had accepted it.
- The first-pass conflict on an old pair, and two rows in one ask: both reviewers accept these. Don't regroup
  mutations to remove them.
- Disabled outranking an error: fix (204/301).
- Enable keeping rows: Astra rules fix (202); Sol had accepted it.
- Propagation's missing `folder_key`: fix (205/304).

**Gate (both reviewers):** `npm run check` and `cargo test --all` passed (1,403 frontend tests; 1,071 Rust tests, 1
ignored), with no binding drift. Probes reproduced every finding.

## Disposition (orchestrator, 2026-11-06)

Fix pass 4 applies one rule to Astra's 201–203 and 206:

- **A row keyed by its own folder key is a folder row.** It follows the folder rules: the folder's reachability and
  its capability through placement.
- **A row with any other key is a legacy row.** It behaves exactly as on `main` in every mutation: destination,
  installedness, the Enable lifecycle and project-tool removal.
- **Destinations follow `main` for every row:** the planned root plus the current skill name.
- **Enable doesn't change a folder-key record it wasn't asked about.** It carries each row's identity through an
  in-memory first-sync plan rather than through a lasting `pending` state.

For display and counts (204/301, 302, 303, 205/304, 305):

- The fold returns facts about the whole folder instead of one stand-in row:
  - the displayed status, with failure ahead of muted, as `skillHealth` already orders it;
  - the disabled intent, kept separately;
  - whether any live row owns an artifact Disable would remove.
- `PropagationOutcome` gains `folder_key`.
- Update/Edit, Import and Library move fold per folder.
- Detection in the inspector checks every harness in the folder.
- The fixture's totals use the shared fold.

---

# Round 5: review of fix pass 4 (`108ab53`, delta from `228190d`)

- **Reviewers:** GPT-6 Astra (high) and GPT-6.1 Sol (high), working independently, 2026-11-06.
- **Convergence bar:** only a P0 or P1, or a legacy row mutating differently from `main`, blocks the merge.
- **Both verdicts:** **SHIP**. No blocking findings.

## Astra

- **Rule check: pass.** `placement::is_folder_row` is `own_folder(scope, key) == Some(key)`; unknown keys count as
  legacy. `folder_reachable` and `row_reachable` are used correctly in:
  - the global batch;
  - Propagation;
  - artifact removal's locatable check;
  - project configuration.

  No other mutation path needs them.
- **201, 202, 203 and 206 are closed.** Seven differential probes give identical results on `07c4550` and `108ab53`.
- **The rewritten regression tests still protect what Astra 9, Sol 103 and Sol 101 were about,** under `main`'s
  destination policy.
- **Enable's in-memory first-sync plan matches `main`** at pair grain and at skill grain.
- **N1 (P2, matches `main`; goes to ticket 10).** Dropping Trae CN whose legacy row records the same path as the kept
  Trae row removes the shared artifact. Trae then reconciles to `Missing`, and a resync restores it. Keep this as
  permanent redeploy coverage.

## Sol

- **301–305 are closed.**
  - Errors now outrank a disabled sibling, and the inspector offers "Finish disabling".
  - The `deployed` fact keeps confirmations right.
  - `isFolderDetected` is threaded through the inspector.
  - Every report family counts per folder.
  - The fixture's totals use the shared fold.
- **Every displayed failure has a way to clear it.**
- **Non-blocking items, all going to ticket 13:**
  - **NB1 (P2).** A folder with one synced row and one never-synced error row still gets first-deploy wording
    (`skillPresentation.ts:1147`). The wording should come from the folder's facts.
  - **NB2 (P2).** In a partly disabled project folder, "Sync this" clears the failure, but the run then reports "Not
    deployed" while the copy stays on disk (`projectPresentation.ts:103–140`, `reportOutcome.ts:475–483` and
    `:980–981`).
  - **NB3 (P2).** Retrying a failed pair Disable briefly shows "Removed … · Undo"
    (`TargetsSection.tsx:75–97`, `:169–185`). Show Undo only once the refreshed folder facts confirm the Disable
    finished.

**Gate (both):** `npm run version:check && npm run check` passed (1,421 frontend tests; 1,085 Rust tests, 1 ignored),
and `cargo test --all` passed.

**CI:** `rust-windows` clippy failed on an unused `skill` parameter outside `cfg(unix)` in a test helper
(`tests/project_sync.rs`). The orchestrator fixed it in `4696eb2`.
