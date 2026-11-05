# Ticket 34 — second full backend audit

**Verdict: FAIL.** Two commit-blocking ownership sequences remain. The ordinary F1/F2 fixes and the successful-write part of F3 work, and all requested standard gates pass. Adversarial tests nevertheless reproduce lost ownership after a failed relocation cleanup and after **Keep → Enable**. Physical-path onboarding exclusion and duplicate prevention also remain incomplete.

## Scope and evidence

Audited the uncommitted diff and untracked first report in `/Users/alexstark/.t3/worktrees/skills-hub/t3-dd600575`, branch `t3/registry-truth-pass`, HEAD `8b42ce5fcd9dd82aadf59c51dd9e98478f6bae93`. Did not compare against the subsequently moved main. Read AGENTS.md, the first audit, the entire ticket/Result/fix passes, the research brief, BACKLOG/newest handoff, ADR-0002, and the implementation/tests/docs identified in the brief.

**No source fixes or commits. This report is the only worktree file deliberately written.** Authorized binding exports reproduced identical bytes. Additional Rust tests were written only in `/tmp/t34-audit2-scratch`, a copy outside the worktree. Production Rust in that copy is identical to the reviewed worktree; the only differences are two added test files and their module declarations in `core/tests/target_relocation.rs`, verified with `diff -rq`. Tests use temporary homes/databases, not the operator's library. The scratch build reused the worktree's Cargo target cache, not its source files.

Executed evidence is distinguished below from code traces. The final adversarial run is **14 tests: 9 passed, 5 failed**, covering four findings. Sources: `/tmp/t34-audit2-scratch/src-tauri/src/core/tests/audit2.rs` and `audit2_more.rs`; output: `/tmp/t34-audit2-adversarial-final.log`. These temporary files are supplemental evidence, not durable ticket deliverables; reproductions and expected fixes are recorded here.

## First audit — status

| Finding | Status | Fresh evidence / remaining boundary |
| --- | --- | --- |
| **F1 — destination Disable gate** | **CLOSED** | `target_relocation.rs:181–204` calls `DisabledGate::blocks_group` for the destination's layout-derived sharers. Preview and move use that same admission. Both destination-marker regression tests pass in both complete suites. Additional `audit2_skill_grain_disabled_blocks_preview_and_move` passes: owned Cursor row + skill-level disabled timestamp → preview 0, summary `{relocated:0,left_in_place:1}`, old artifact untouched. Disabled source markers are never candidates; nothing clears markers. |
| **F2 — all-owner inventory** | **CLOSED** | `artifact_removal.rs:409–464` inventories all skills' global rows and resolved project assignments through removal planning. `same_artifact_path` canonicalizes **parents**, retaining the leaf name. Both project-overlap regressions pass. Additional tests pass for another skill's global owner recorded with a canonical parent and for a project marker with no `synced_at`, which correctly does **not** protect the old artifact. Existing home-as-project tests exercise macOS `/var` versus `/private/var` spelling. |
| **F3 — partial relocation loses ownership on later mutation** | **STILL OPEN** | Owned-path Propagation partitions and successful first-sync relocation are fixed: original regressions pass, and scratch tests pass for true copy-only adapters, live links, both layout directions, and Delete of both owned partitions. However the collision **Keep** branch bypasses the successful-write safeguard; subsequent Enable drops the old ownership record. Executed F11 below. A separate non-owner Propagation partition also creates a duplicate, F13. |
| **F4 — onboarding attribution / physical exclusion** | **STILL OPEN** | Ordinary shared-dir attribution now works: `readers` is populated, and a Codex-owned copy at the identical `.agents/skills` spelling is excluded under either scope. But `managed_target_key` only normalizes lexical components (`onboarding.rs:294–309`), and directory dedupe uses lexical `PathBuf` equality (`:151–167`). Two scratch tests fail for a Tool skills dir symlinked to another Tool's dir: managed copy re-offered, unmanaged copy counted twice. F12. |
| **F5 — retired labels** | **CLOSED** | `skillPresentation.ts:148–154` provides catalog → retained translation fallback; App uses it at `:174,178`, chips at `skillPresentation.ts:534`. Full Vitest run passes the new retired-label/strip tests. Retired keys remain undetected but readable, not resurrected as selectable registry entries. |
| **F6 — frontend gate** | **CLOSED for this audit run** | Fresh `npx vitest run src`: exit **0**, 53 files / 1,152 tests, no unhandled teardown errors. The first audit's failures remain valid history; this does not prove their timing-dependent cause is cured or that main caused them. |
| **F7 — generated whitespace** | **CLOSED** | Export normalization at `src-tauri/src/lib.rs:273–286`; `git diff --check` exit **0**. Both filtered exports exit 0 and all before/after SHA-256 values agree. |
| **F8 — deferral / docs / counts** | **CLOSED as disposition** | Ticket fix pass 2 explicitly records Kilo/Xum project-path deferral to orchestrator-owned BACKLOG #87; registry comments repeat it. This stale-base worktree does not itself contain #87, and main was deliberately not audited. README now distinguishes verified/secondary/unknown evidence and qualifies cross-read duplication; CHANGELOG counts independently recount to **27 readers / 24 movers**. Registry-truth moves are distinguished from preference moves. Remaining behavioral exceptions are F10/F13, not evidence that the prose/count corrections were omitted. |
| **F9 — snapshot provenance / contention** | **STILL OPEN, P2 wording only** | `every_snapshot_citation_exists_in_the_brief` **really reads the brief** (`tests/tool_adapters.rs:807–860`) and checks section/row/implication anchors and evidence tags. Both full Rust runs pass, with no contention flake. The snapshot doc correctly admits this is hand-transcribed and citation-bound, not content-bound. However the registry header at `tool_adapters/mod.rs:337–341` still claims a registry edit without a **brief edit** fails. Editing registry + snapshot while retaining existing citations still passes. Remove that stale guarantee (say “without a snapshot edit”), or enforce content/revision correspondence if the ticket's original stronger guarantee is retained. No registry-fact mismatch was found. |

## New findings

### F10 — P0 blocks commit: cleanup failure is forgotten after the row moves

**Executed reproduction:** `audit2_removal_failure_is_retried_or_reported_next_launch` uses a real permission failure, not a mocked production function:

1. Create an owned legacy Augment link at `home/.augment/rules/skill`.
2. Set its parent directory to mode `0500`; the destination remains writable.
3. Invoke the public launch relocation entry. Destination is created and the row is settled there, but unlinking the source fails.
4. Restore the parent to `0700`; invoke launch relocation again.

```text
first=RelocationSummary { relocated: 1, left_in_place: 0 }
second=RelocationSummary { relocated: 0, left_in_place: 0 }
old_exists=true, old_owned=false
assertion failed: source is neither retried nor reported on next launch
```

**Code:** `target_relocation.rs:284–303` settles `SyncCompleted` at the destination and increments `relocated` **before** cleanup; `:326–336` only logs removal failure. `artifact_removal.rs:770–798` deliberately has no row settlement for this seam. Next launch's admission sees only the current destination, so it cannot discover the leftover source. There is one target row, not duplicate rows, and its destination exists; nevertheless the old artifact has become unowned. Delete can remove the new artifact/central record and leave the old link dangling (or leave an old copy usable and invisible).

This best-effort limitation existed in the earlier relocation implementation; expanding it to this live-library migration does not satisfy the requested interrupted-move recovery or ADR-0002. It is now exercised against real failing removal, not merely noted.

**Exact proposed fix:** persist recoverable relocation state **before** creating the destination, retaining the old and intended new artifact identities through cleanup. A durable pending-relocation/cleanup record must be admitted under the same guard, included in ownership/removal planning, and retried at launch. Atomically settle the target path and the pending state so a crash cannot erase either artifact's ownership. On source-removal failure retain an error/retryable record and count/report it as incomplete, not unqualified relocated success. Clear pending state only after removal succeeds or another owner deliberately retains the source. Do not “fix” this by deleting first or by retaining only the old row while forgetting an already-written destination. Add this permission-failure test plus restart/crash-boundary tests to production tests.

### F11 — P0 blocks commit: Keep → Enable orphans the former deployment and falsely shows it disabled

**Executed reproduction:** `audit2_keep_then_enable_never_orphans_the_owned_former_copy`:

1. Cursor owns a live link at `.cursor/skills/skill`; `.agents/skills/skill` contains different, foreign bytes.
2. Save preference on. Relocation correctly refuses the destination and retains Cursor's old row/artifact.
3. Deploy Cursor with the overwrite answer **Keep**.
4. Enable Cursor. The destination still contains foreign bytes, so Enable reports `TARGET_EXISTS`.

```text
Keep outcome: Declined
Cursor row: path=.cursor/skills/skill, status=Disabled, synced_at=Some(1)
old_exists=true

Enable outcome: Failed { TargetExists { path=.agents/skills/skill, same_content:false } }
Cursor row: path=.agents/skills/skill, status=Disabled, synced_at=None
old_exists=true, old_owned=false
assertion failed: Keep followed by Enable orphaned a live old deployment
```

**Code:** `global_sync.rs:586–608` routes Keep straight to `DisabledRecorded`, not the new owned-path/relocation guard. `skill_store.rs:1598–1625` preserves the existing old path and `synced_at` but sets status disabled. `skill_disable.rs:381–384` then deletes that disabled row before first-sync; the failed Enable restores a new non-owning marker at the destination (`:455–478`). No operation removes or transfers ownership of the old artifact.

The strip is dishonest **already after Keep**: `syncedStrip` (`skillPresentation.ts:1055–1058`) shows disabled despite the old Cursor deployment still being live. After Enable the old copy is no longer even represented by a row. This is an uncovered branch of F3, not a lock failure.

**Exact proposed fix:** extend the superseded-owned-path rule to **Keep**, not just successful `Recorded`. If a declined destination belongs to a pair still owning a different former artifact, settle removal of that owned old artifact through Artifact removal before declaring the pair disabled; never touch the foreign destination that the operator kept. Failure must retain the old row/path as `error` with ownership and a typed failed outcome, not Declined. Only successful old-artifact settlement may produce the non-owning destination marker. Also harden Enable so it cannot blindly delete a disabled row that still owns a present superseded artifact; recover it through the same ownership-aware relocation/removal mechanism. Add `refused move → Keep → Enable → Delete` regression tests, including old-artifact removal failure.

### F12 — P1 fix before the UI half: physical-path onboarding still uses lexical keys

**Executed tests:**

- `audit2_owned_copy_is_excluded_through_a_symlinked_tool_dir`: a managed **copy** exists at `.cursor/skills/skill`, with a Cursor-owned row. `.claude/skills` is a directory symlink to `.cursor/skills`. Scan `Selected([claude_code])`. Actual `copy_count=1`, expected 0; the owned copy is offered as a managed-name replacement under the Claude spelling.
- `audit2_unmanaged_copy_is_deduped_through_a_symlinked_tool_dir`: same directory alias without the owned row, scan both Tools. Actual `copy_count=2`, expected 1.

**Code:** `onboarding.rs:108–114,163–167,283–309`. Dropping the Tool from the exclusion key fixed attribution, but `path.components().collect()` does not resolve symlinked parents or `/var` aliases. `scanned_dirs` likewise stores the as-written dir. The reader roster is then derived only from the attributed registry dir, not the physical alias relationships.

**Exact proposed fix:** reuse/extract the parent-canonicalized artifact identity from Artifact removal (canonical parent + leaf, not canonicalized artifact symlink referent) for managed exclusion. Canonicalize existing scan roots for directory dedupe, retain a display/operation path and first-in-scope attribution, and union in-scope installed readers of aliased roots into one variant. Keep a lexical fallback when resolution fails. Use the same identity rule for invalid-entry exclusion. Add both alias tests under Installed and Selected scopes. Do not canonicalize whole artifact symlinks into central and thereby conflate distinct deployments with one referent.

### F13 — P1 fix before the UI half: a non-owner Propagation partition can create a second copy without any warning

**Executed reproduction:** `audit2_propagation_nonowner_does_not_add_second_cursor_copy`:

- Cursor owns a copy in `.cursor/skills/skill`; Codex has a never-synced error row for the absent `.agents/skills/skill`.
- Temporarily remove the central directory, save preference on, and confirm relocation reports one left-in-place row. Restore central bytes (the state Update can establish by reacquisition).
- Run Propagation. It first-syncs the non-owner Codex partition at `.agents/skills/skill`, updates Cursor in place, and reports both Synced. Both artifacts remain.

```text
cursor_path=.../.cursor/skills/skill
codex_path=.../.agents/skills/skill
both artifacts exist; both outcomes Synced
assertion failed: Update created the own-dir plus AGENTS duplicate
```

**Code:** `propagation.rs:283–306` partitions owned rows at stored paths and non-owners at the current root. Each partition is independently written/settled at `:297–331,405–456`. No post-settlement relocation or incomplete-layout signal exists. Ownership is preserved here (unlike F10/F11), and Delete still can remove both, but the preference's duplicate-prevention goal is not achieved. Cursor reads both directories. The strip has one healthy mark per recorded Tool, not per physical artifact; Cursor and Codex both read as synced, and there is no relocation-pending/duplicate indication. Other installed siblings may also be recorded by the first-sync partition (Augment in the executed fixture).

**Exact proposed fix:** before first-syncing a non-owner partition into the effective root, detect owned members of that effective group still occupying superseded paths. Until their relocation can settle safely, defer that new partition with an explicit typed relocation-pending skip and expose the incomplete state/repair, rather than silently introducing another artifact. Explicit Deploy can use the existing guarded relocation path after F10/F11 are fixed. Preserve the already-correct owned-path partitions and the invariant that Propagation is the target-row writer on an Update path; do not call a guarded relocation entry from inside Update. If product policy instead permits this duplicate, record that decision and give the operator a visible incomplete-relocation state—the current healthy strip and unconditional prevention claim are insufficient.

## Remaining requested checks

### Lock graph and first-sync behavior

Executed the requested `rg -n 'serialized\(|try_serialized\(' src-tauri/src/core` and read the relevant bodies. **No nested acquisition found.**

| Entry / path | Lock and relocation trace |
| --- | --- |
| Global batch | `global_sync.rs:671` acquires once → `sync_skills_to_tools_unlocked` → `relocate_synced_skills` (`:716`) → **`relocate_skill_unlocked`** (`:744`). |
| Install batch | `install_batch.rs:222` acquires per settlement → `settle_unlocked` → global batch unlocked (`:302`) → same unlocked relocation seam. |
| Import / replacement | `onboarding_import.rs:242,248` acquires around apply/replace → global batch unlocked (`:371,539`) → same unlocked relocation seam. |
| Enable | `skill_disable.rs:144` acquires once → `enable_unlocked` / `first_sync_global` → global batch unlocked (`:452`) → same unlocked relocation seam. |
| Project sync / project Enable | Project guarded entries use their unlocked project seams / low-level sync; **no path into global relocation**. `first_sync_assignments_unlocked` does not take another guard. |
| Launch | `lib.rs:181–189` calls `relocate_former_global_targets`; that entry takes one guard (`target_relocation.rs:88`) and calls `relocate_unlocked` → `relocate_skill_unlocked`. |
| Preference save | `apply_global_layout` takes one guard (`:106`), writes setting, calls `relocate_unlocked`. |
| Relocation cleanup | `artifact_is_owned` → removal **planning**, not a guarded removal entry; removal uses `remove_superseded_artifact_unlocked`, then low-level sync-engine removal. |
| Preview | Command `commands/mod.rs:413–432` uses `spawn_blocking` and calls read-only `preview_relocation` directly, **outside** the mutation guard. It neither queues on the guard nor calls `global_layout`/latches settings. An executed fresh-store preview test confirms no setting write. It is an advisory count, not an atomic future promise. |

Additional test `audit2_first_sync_public_entry_completes_both_layouts` calls the **public guarded** global entry (not the fixer's unlocked test door) with Cursor owning a present superseded artifact. Both preference-on and preference-off cases complete: **Synced**, old source removed, row at destination. Thus the named first-sync deadlock does not occur. On normal success this is neither a typed skip nor an in-place sync: it writes the current root, holds back `Recorded` for the old owner, then relocates through the unlocked seam. The strip is honest on that successful path. Collision without an answer remains a typed `TARGET_EXISTS`; Keep is F11. Cleanup failure and suppressed relocation summaries are F10.

### Ownership, launch scope, recovery, and counting

- **F1 counts:** destination-disabled candidates contribute preview 0 and `left_in_place=1`; disabled/non-owning source markers are not candidates and remain unchanged. Skill-grain Disable is also blocked, including an error owner remaining from a failed Disable.
- **F2 ownership:** global rows of every skill and project assignment owners protect the old physical artifact. Never-synced project markers do not. Parent-only canonicalization avoids confusing two different symlinks pointing into the same central source. Ordinary same-skill Kimi/Amp and both project-overlap tests pass.
- **F3 owned partitions:** scratch test `audit2_owned_partitions_copy_and_link_keep_paths_and_delete_both` passes for **actual copy-only adapter shadows** (both written in place as Copy) and symlink adapters/rows (both `LinkFollowsSource`, paths unchanged). Both see new central bytes; Delete removes both artifacts. The non-owner case is F13.
- **Existing unset launch:** only registry-truth candidates move: Codex, Goose, OpenHands, Antigravity, Kilo, Xum, Kimi, OpenClaw's former names, plus the pre-existing Augment-rules correction. Ordinary Cursor/Pi/Gemini/Amp own-dir targets do not move on the default. No preference is saved; offer remains pending.
- **Important correction to the audit prompt's “exactly Codex” assertion:** the six-harness scratch seed was run **twice**. With Kimi already at `.kimi-code/skills`, exactly Codex moves: summary `{1,0}`, second launch `{0,0}`, pending true. With Kimi at the **pre-change registry path** `.config/agents/skills` beside Amp, **Codex and Kimi** move: summary `{2,0}`, second `{0,0}`, pending true; Amp's original remains. This is the authorized Kimi registry correction, not a preference move or an unsafe Amp relocation. The real machine was not read; which case describes it depends on its recorded Kimi path. Do not promise “only Codex” without that qualification.
- **Kimi former-dir correction to prompt:** `.config/agents/skills` is **included**, not excluded, at `tool_adapters/mod.rs:479`. The ticket forbids doing so *blindly*, not literally under every implementation. Eligibility resolves each row's own adapter; all-owner cleanup preserves Amp/project artifacts. The guarded implementation passes the ordinary hazard tests.
- **Re-key:** `a_former_names_row_is_rekeyed_to_openclaw_and_relocates` passes for both retired keys × both layouts and a second invocation. Same row id, no duplicate key row. Existing OpenClaw row blocks re-key: preview excludes, summary left-in-place, retired row remains removable.
- **Divergent destination:** additional test confirms preview **1 attempted row**, actual `{relocated:0,left_in_place:1}`, original row/path retained. Preview does not hash/probe all future materialization failures. UI confirmation must say “would attempt to move,” not guarantee N successful moves. Permission failure after destination settlement is F10, not a successful retry case.

### Onboarding readers and registry truth

The reader interpretation is correct **against the supplied brief**. Contrary to the audit question's parenthetical, that brief explicitly says **Goose also reads `~/.claude/skills`** (§1 row 38, Summary 7, I1, S10). `other_global_reads` (`mod.rs:295–302`) chains declared cross-read dirs and the verified AGENTS dir; `reads_global_dir` (`:305–306`) also admits the own dir. It does not indiscriminately treat every AGENTS reader as a Claude reader.

Executed roster test, with every registry entry available for filtering:

- `.claude/skills`: **cursor, claude_code, opencode, amp, kimi_cli, augment, continue, crush, goose, codewhale**.
- `.agents/skills`: **agents_skills + 27 verified harness readers** (28 entries total).
- `.config/agents/skills`: **amp, kimi_cli, crush**.

The plan narrows that roster to in-scope installed readers, plus its attributed Tool (`onboarding.rs:237–253`). Kimi's legacy-conditional cross-reads and Continue's weaker evidence remain limitations inherited from the brief's modeling; “can read” is safer UI wording than promising every installed version actively loads every copy. Physical aliases remain F12.

Independent Node extraction of the current registry, enum, README and snapshot produced:

```text
enumVariants: 47
registry: 47
readme: 47
snapshot: 47
verifiedReaders: 27
movedReaders: 24
allSymlink: true
readmeParity: true
snapshotOrder: true
```

All 47 global literals were checked against the brief/snapshot, including uncertain rows deliberately retained rather than promoted to verified AGENTS readers. OpenClaw is present, `.openclaw/skills`, AgentsWins, former names/dirs attached; only Clawdbot/MoltBot are retired. Four verified `since` facts remain Codex `rust-v0.95.0`, Pochi `2a9ff73 (#1360)`, Roo `PR #11181`, Gemini `PR #18151`. Crush remains **Unknown**, not OwnWins. Windsurf is held. WorkBuddy is global-only: no project catalog entry, gitignore pattern, or resolved project target; configuration rejects it before writes. Kilo/Xum project roots remain explicitly deferred; global roots are corrected. No new tool-name special case appears in sync/relocation, and all filesystem roots are explicit inputs.

### Commands, DTOs, and docs

- Both preference commands are in `collect_commands!` (`lib.rs:27–28`), annotated `tauri::command`/`specta`, execute sync work in `spawn_blocking`, and map errors to `CommandError`. Setter delegates the lock to core and loads the returned settings after settlement.
- Generated bindings have required booleans for `prefer_agents_standard_dir`, `hide_harness_specific_dirs`, and `agents_standard_offer_pending`; `agents_standard_read: AgentsStandardReadDto | null`, `since: string | null`, `also_reads: string[]`, `shadowed_by_own_dir: boolean`. Counts are `number`, not nullable. Shim exports are present.
- General `SettingUpdate` contains hide but **not prefer**; the extra scratch serde test confirms `{key:"prefer_agents_standard_dir",value:true}` is rejected. Thus no general-settings bypass of relocation.
- `shadowed_by_own_dir` (`catalog.rs:120–149`) probes the filesystem under explicit home and only for OwnWins readers moved off their own root. It checks matching recorded deployment basenames and an unrecorded own copy using `symlink_metadata`. This is not a complete duplicate detector: managed old copies and aliases are outside that flag's guarantee.
- README/CHANGELOG evidence levels, 47-row parity, cross-read caveat and 27/24 counts are now accurate. The unchanged Chinese README does not exist; no finding invented. CONTEXT's destination-disabled, all-owner cleanup and stored-path-partition descriptions match their normal paths. Its “first deploy never re-points an owned row” safety promise is still incomplete under Keep/Enable (F11), and clean completion cannot be claimed after cleanup failure (F10). Preference duplicate prevention needs F13's limitation/repair addressed before UI copy promises it.
- UI, fixture layout simulation, screenshots and real-library release smoke remain deferred to the UI/release half; this audit did not drive the app or mutate live deployments. Whole-operation layout immutability and the first report's renamed/symlinked import-original policy remain disclosed prior follow-ups, not silently proved here.

## Verification

Commands below ran in the audited worktree unless explicitly marked scratch. Logs are under `/tmp/t34-audit2-*.log`.

| Command | Exit / fresh result |
| --- | --- |
| `cd src-tauri && cargo test --all` — run 1 | **0**, 887 passed, 0 failed; 14.52 s test execution. |
| Same — run 2 | **0**, 887 passed, 0 failed; 24.67 s. Both runs also completed binary/doc-test targets with zero tests. |
| `cd src-tauri && cargo clippy --all-targets -- -D warnings` | **0**. |
| `cd src-tauri && cargo fmt --check` | **0**. |
| `npx vitest run src` | **0**, 53 files, 1,152 tests; Node `v26.10.0`. No unhandled errors. |
| `npx tsc --noEmit -p tsconfig.app.json` | **0**. |
| `git diff --check` | **0**, including a repeat after the scratch runs. |
| `cd src-tauri && cargo test export_bindings` — run 1 | **0**, 1 passed / 886 filtered. |
| Same — run 2 | **0**, 1 passed / 886 filtered. |
| Binding SHA-256 before tests, after both exports, and after scratch verification | Identical: `478bb067f1a60e6ef2889fffbe76640f26eee9543cfeb0a1531f998e06827ae5`. |
| Node mechanical enum/registry/README/snapshot count and key comparison | **0**, 47/47/47/47, 27 readers, 24 movers, parity/order true. |
| Scratch: `CARGO_TARGET_DIR=/Users/alexstark/.t3/worktrees/skills-hub/t3-dd600575/src-tauri/target cargo test audit2 -- --nocapture` from `/tmp/t34-audit2-scratch/src-tauri` | **101**, **9 passed / 5 failed**, 887 original tests filtered; final log `t34-audit2-adversarial-final.log`. The five failed assertions are the two alias cases, non-owner duplicate case, cleanup recovery case, Keep/Enable case. |
| `diff -rq src-tauri/src /tmp/t34-audit2-scratch/src-tauri/src` | **1 as expected**: only the two new scratch test files and their module declarations differ. No production-function modification was used to produce the failures. |

The final pi-lens cache check reported no errors; the active probe of the three scratch Rust test paths was **inconclusive** (no language-server answer), not a confirmed-clean result. Actual Cargo compilation/execution above is the verification for those tests.

Scratch setup initially failed compilation because two root-relative `include_str!` assets had not been copied; those were copied unchanged. An initial cross-skill-owner test incorrectly reused a UNIQUE central path; its fixture was corrected to a distinct central path and the test passes. Neither setup error is counted as a product finding. The final 14-test result above is the reviewed evidence.

## Final verdict

**FAIL.** Destination Disable admission, all-owner cleanup protection, normal stored-path Propagation, unlocked first-sync relocation, restored OpenClaw, settings defaults, typed wire fidelity, labels, documentation counts and every requested standard gate are substantially sound. But the migration still forgets an old live artifact when source removal fails, and the Keep/Enable collision path can erase that artifact's ownership while showing the harness disabled. Those two executed P0s block commit. Resolve them with durable ownership-preserving settlement, then fix the physical-alias onboarding and non-owner duplicate paths before the UI half; do not equate the green 887-test suite with a clean live-library migration audit.
