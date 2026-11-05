# Ticket 34 — backend Spec review and adversarial audit

**Verdict: FAIL.** Three commit-blocking ownership/Disable defects remain in the interaction between relocation and the existing mutation paths. The ordinary registry/default-setting cases pass, but the successful tests do not exercise these sequences.

Reviewed the uncommitted diff on `t3/registry-truth-pass`, HEAD `8b42ce5fcd9dd82aadf59c51dd9e98478f6bae93`, in `/Users/alexstark/.t3/worktrees/skills-hub/t3-dd600575`. Read AGENTS.md, BACKLOG, newest handoff, the entire ticket including both Result passes and all Comments, and the research brief. No source edits or commits. This report is the only repository file written by the reviewer. Test-generated bindings were checked for additional drift.

**Evidence distinction:** test results below are executed results. Adversarial sequences labelled **code trace** are constructions against the actual implementation, not newly executed Rust regression tests; adding tests would breach the source-read-only brief. The proposed regression cases are deliberately explicit so the fix pass can reproduce them.

## Spec

### (a) Missing / partial

- Ticket: **“Add a table test … match a checked-in snapshot derived from the brief, so a registry edit without a brief edit fails.”** A useful 47-row, hand-maintained literal snapshot exists, with citations and independent expected values; changing one registry fact fails it. It does not read or checksum the brief, however: changing registry and snapshot together without touching the brief passes. See F9.
- Brief I6: **“project dir `.kilo/skills`”**; I7: **“project `.xum/skills`.”** Global corrections are implemented, but project mappings remain `.kilocode/skills` and `.mux/skills`. The Result explicitly discloses the hold pending project relocation; no operator Comment approves that hold. Treat this as a documented scope decision still needed, not a stealth global-registry omission. See F8.
- The UI, offer presentation, collapse preference, warnings, fixture layout simulation, screenshots and real-library release smoke remain outstanding, as explicitly handed off. That is appropriate for this **backend-half** review, not grounds to fail this half by itself.
- The requested “once per operation” preference read is met by an individual global sync batch, but not literally by every composite operation: import reads it once in the sync and again in original settlement, and re-reads per group; Enable reads it for the group and again in first-sync. See verification §4.

### (b) Scope creep

- Ticket's registry work does not request dropping onboarding attribution to all but the first harness. The new scan dedupe changes the wire's meaning without adding a reader roster, and its exclusion is still keyed by `(tool, path)`. One managed physical copy can now appear as unmanaged; F4.
- `settle_superseded_originals` is a reasonable supporting change for **“the harness-specific deployment stops”**, not intrinsically scope creep. Its actual boundary is broader than the audit prompt's proposed “near misses”: an identical symlink original can be removed, and a renamed import can remove its differently named original. These are not excluded by its predicates. See §6; do not claim these non-removal proofs exist.
- Wider first-sync recording for installed but unselected readers is **not** new policy: it applies the existing D34 shared-dir rule to a larger directory group. Its presentation consequences are substantial but authorized; §5.

### (c) Implemented-but-wrong

- Ticket: **“existing harness-dir targets relocated … by the same relocation rule … on save.”** AGENTS.md: **“Only Enable re-materialises”** and disabled shared-dir siblings block fan-out. Relocation applies only the source row's Disable gate, not the destination group's: F1.
- Ticket: **“never removes the artifact when another installed sharer's row points at it.”** The new check is only over this skill's global rows. It misses a live project assignment on the same physical path: F2.
- Ticket: **“byte-identical replace, else leave + log.”** Leaving rows on different actual paths is supported by relocation, but subsequent Propagation groups them using their new theoretical directory, then writes every row to one driver's actual path. This loses ownership of the other artifact: F3.
- Result: **“i18n labels stay so a stored row still shows a readable name.”** The deployed strip/inspector label route uses the current Tool catalog and falls back to the raw key, not those retained translations: F5.

## Findings

### F1 — P0 blocks commit: relocation can re-enable a disabled destination sharer

**Evidence:** `src-tauri/src/core/target_relocation.rs:143–165` rejects only a source row that is disabled/owns nothing; `:183–214` checks skill-grain Disable and re-key collision; `:216–250` materialises without checking disabled members of the **destination** group. In contrast `src-tauri/src/core/global_sync.rs:564–587` explicitly blocks any disabled sharer via `DisabledGate`.

Command: `rg -n 'DisabledGate|fn blocks|let layout' src-tauri/src/core/global_sync.rs` → `564:struct DisabledGate`, `582:fn blocks`, `622:let layout = settings::global_layout(store)`. Reading `relocate_unlocked` and `materialise` finds no equivalent destination check.

**Code trace:** preference off; Cursor has an owned live `~/.cursor/skills/x`; Codex has a disabled marker at `~/.agents/skills/x`, which is absent. This is legal under own dirs because the two groups differ. Save preference on. Codex's disabled row is skipped. Cursor's live row passes admission, `materialise` writes `~/.agents/skills/x`, and Cursor's old copy is removed. Codex now reads an artifact the operator disabled, while its row still says disabled. No Enable occurred. The same hazard exists for a first-launch Codex former-dir move into an AGENTS group disabled through another member.

**Exact proposed fix:** before materialising each move, derive the destination group from the supplied layout and apply the same skill/pair-disabled admission as global sync (including disabled rows that own nothing). Leave a blocked source row/path intact and report/log it as left in place; use the same eligibility predicate for preview. Do not silently clear disabled markers. Add tests for Cursor→AGENTS with disabled Codex/agents marker, and Codex's registry-truth move with a disabled destination sharer.

### F2 — P0 blocks commit: cleanup can delete a project assignment's artifact

**Evidence:** `target_relocation.rs:261–284` queries only `store.list_skill_targets(&skill.id)` and searches that vector for another owner. `project_ops.rs:151–181` admits the home directory as a project (canonical directory + duplicate check, no home prohibition). `artifact_removal.rs:409–420` already provides an ownership-aware inventory including resolved project assignments.

Command: `rg -n 'still_recorded|recorded_target_paths' src-tauri/src/core/target_relocation.rs src-tauri/src/core/artifact_removal.rs` → `target_relocation.rs:269` and `artifact_removal.rs:409`. The former does not call the latter or enumerate project assignments.

**Code trace:** register `home` as a project, assign skill `x` to project Augment, and have the same skill globally deployed to Augment. Both legitimately resolve to `home/.augment/skills/x`. Save preference on. The global row moves to `home/.agents/skills/x`; the remaining global rows no longer record the old path; cleanup deletes `home/.augment/skills/x`. The project assignment remains owned and reports its old successful state until reconciliation, but its artifact was removed by a global preference change. A project rooted at `home/.config/agents` with an OpenClaw project assignment (project dir `skills`) also overlaps the Kimi/Amp old path.

Uninstalled global owners do **not** slip past the check: it does not require installedness. Never-synced global markers do not protect a path, correctly, because they own nothing. Project owners (and an owner on another skill) do slip past it.

**Exact proposed fix:** after row settlement, check ownership of the old physical path across **all** global rows and resolved project assignments before cleanup, not only this skill's global rows. Reuse/extend artifact removal's ownership-aware inventory; preserve alias handling consistently with project canonicalization. Add home-as-project and `.config/agents`-as-project overlap tests, asserting the project artifact and assignment survive both relocation directions.

### F3 — P0 blocks commit: partial relocation followed by Update can orphan a still-owned copy

**Evidence:** `propagation.rs:230–241` forms the group by effective registry keys, without requiring equal `target_path`; `:274–310` selects an installed driver and determines overwrite; `:344–375` writes at `driver.target_path` and settles **all** installed rows at that path. `skill_store.rs:1088` lists rows `ORDER BY tool ASC`. The new preference and new structural sharing make formerly separate directories one group even when relocation deliberately cannot finish.

Command: `rg -n 'ORDER BY tool ASC' src-tauri/src/core/skill_store.rs` → `1088`; source inspection at the ranges above shows no stored-path partition.

**Code trace:** with preference off, Codex owns a copy at `.agents/skills/x`, Cursor owns a separate copy at `.cursor/skills/x`. The AGENTS copy has locally divergent bytes. Turning on the preference persists `true`, but correctly leaves Cursor at its old path because the destination is not byte-identical. Later Update runs Propagation: Codex sorts before Cursor, both now belong to the AGENTS group, and copy mode requires writing bytes. Propagation writes Codex's path and changes **both** target rows to that path. Cursor's old artifact still exists but has no row. Delete can no longer clean it; Cursor can show two copies. Even without Codex, a later successful first-sync can replace an old-path row through `TargetTransition::Recorded` without removing its old owned artifact (`global_sync.rs:139–150`, `skill_store.rs:1524–1546`).

**Exact proposed fix:** make stored-path ownership survive an incomplete migration. Propagation must partition owned rows by actual artifact path and update each still-owned artifact without changing another row's path merely because its Tool shares an effective root. First-sync/Recorded must not silently replace a different owned target path; retry migration through the guarded relocation seam or retain both ownership records until cleanup has settled. Add `partial relocation → Update → Delete` and `partial relocation → successful deploy → Delete` regression sequences. Assert no old artifact loses its owner.

### F4 — P1 fix before the UI half: scan dedupe loses harness attribution and can resurface managed copies

**Evidence:** `onboarding.rs:145–161` scans each own directory once and gives it the first in-scope adapter. `OnboardingVariant` still has one `tool` (`:17–24`), no roster. Managed exclusions remain `(tool, path)` at `:97–101`, `:253–269`. The new test `a_shared_skills_dir_is_scanned_once` passes, but tests only the reduced copy count, not preserving ownership/reader information.

Executed: `cargo test --all` → `test core::onboarding::tests::a_shared_skills_dir_is_scanned_once ... ok` (both runs).

**Code trace:** a managed Codex copy at `.agents/skills/x` has only a Codex row, e.g. after registry relocation on a filesystem that uses copy fallback. Under Installed scan scope, the virtual AGENTS entry is first and installed because `.agents` exists. Scan attributes the copy to `agents_skills`, whose `(tool,path)` key is not excluded by the Codex row; the copy is offered again as a managed-name replacement. Under Selected(Codex), it is correctly excluded. Links into central happen to be filtered separately, so a symlink-only test hides this.

An unmanaged shared copy likewise names only `agents_skills` under Installed scope, not Codex/Goose/OpenHands. With auto-sync off its original report names just that one tool. With auto-sync on the batch can record installed sharers correctly, but the plan/variant and per-request outcome still do not enumerate every consuming harness.

**Exact proposed fix:** retain one physical-copy entry, but exclude owned physical paths independent of attribution and carry its in-scope/installed reader keys explicitly (or derive a documented roster from the catalog in the review). Preserve reader attribution in the report without duplicating physical removal work. Add tests with a managed **copy** owned only by Codex under Installed and Selected scopes, plus an unmanaged shared copy with multiple readers.

### F5 — P2 note: retained translations do not give retired deployments their claimed labels

**Evidence:** `useToolCatalog.ts:49–56` builds `toolLabelById` only from registered catalog entries. `App.tsx:177` uses `toolLabelById[id] ?? id`. `skillPresentation.ts:1024–1026` appends target rows absent from the selected set using that callback; `:518–519` has the same raw-key fallback for chips. Thus an unrekeyable row appears as **`clawdbot` / `moltbot`**, undetected, not the retained “Clawdbot” / “MoltBot” translation (`resources.ts:423–432`). It remains visible/removable, so this is a presentation/handoff inaccuracy rather than lost data.

Command: `rg -n 'labelOfTool|toolLabelById\[|order.push' src/App.tsx src/lib/skillPresentation.ts` → `App.tsx:177`, `skillPresentation.ts:1026`.

**Exact proposed fix:** route the missing-catalog fallback through the existing `skillPresentation.toolLabel(t, id)` rule, or provide a backend retired-label surface without resurrecting retired selectable rows. Add a pure strip/chip test with a retained retired key. Update the Result claim if leaving raw keys intentionally. Saved retired selections are silently pruned (existing unknown-key policy); an informational retired-harness notice is optional, not required by this ticket.

### F6 — P1 verification gate: fresh frontend suite is not green

**Executed evidence (Node v26.10.0):**

```text
npx vitest run src                    # run 1, exit 1
Test Files 53 passed (53)
Tests      1147 passed (1147)
Errors     3 errors
ReferenceError: window is not defined
  react-dom-client.development.js:18936
  scheduler.development.js:45
originated in src/components/ui/sonner.test.tsx

npx vitest run src                    # run 2, exit 1
Test Files 53 passed (53)
Tests      1147 passed (1147)
Errors     2 errors
[same teardown exception]

npx vitest run src/components/ui/sonner.test.tsx  # exit 0
Test Files 1 passed (1); Tests 3 passed (3)
```

A Node comparison against `git show 8b42ce5:src/components/ui/sonner.test.tsx` reports `entire file identical to base: true`. This is evidence of an unchanged test implicated in a full-suite teardown failure, **not** proof that the backend diff causes it or that main would fail identically. Do not repeat “full gate green” based only on the passing assertion count.

**Exact proposed fix:** reproduce the full suite under CI's Node 22, then ensure the toast test unmounts/drains scheduled React work before jsdom teardown (in its owning change if pre-existing). Require an exit-0 complete suite; do not suppress unhandled errors globally. The current report must retain the failed fresh run even if a retry passes later.

### F7 — P2 note: generated bindings fail `git diff --check`

**Executed:** `git diff --check` exits 2, reporting trailing whitespace at `src/bindings/index.ts:963,965,967,969,1372` (the generated documented `Precedence` union and extended `SettingUpdate` union). `cargo test export_bindings` passes and reproduces the same bytes: SHA-1 before/after `cef12c1e1b3f19e71353ba30d03f71513d8828ac`. No additional bindings drift occurred.

**Exact proposed fix:** normalize trailing whitespace in the binding export pipeline, then regenerate and re-run its drift test and `git diff --check`; do not hand-edit a generated DTO/union. Alternatively explicitly document an approved generated-file whitespace exception rather than claiming this check passes.

### F8 — P2 scope/documentation notes to resolve before claiming ticket completion

1. **Project corrections held:** I6/I7 specify `.kilo/skills` / `.xum/skills`; registry lines `912` / `705` still use `.kilocode/skills` / `.mux/skills`, and the project table pins those old values. A missing project relocation facility explains why existing assignments cannot be blindly moved; it does not establish that new assignments should use the old dirs indefinitely. **Fix:** obtain/record an explicit scope deferral with a follow-up owner, or implement project-path migration before correcting these mappings. Goose/OpenHands' old project dirs are documented compatible, so retaining those is less concerning.
2. **Overclaims:** README says “Every path is checked against … own docs or source” despite WorkBuddy/UNKNOWN rows being secondary/unverified; it also says “so no harness sees a skill twice,” although Claude+Cursor cross-reading still duplicates. CHANGELOG's “Pi and 14 more” names 11 readers + 14 = 25, whereas the registry has 27 verified readers (or 24 moved readers if excluding the three already-global sharers). **Fix:** distinguish verified corrections from retained uncertain rows, qualify duplicate prevention as own-dir-versus-AGENTS only, and use a correctly defined count or remove it. Clarify that unconditional registry-truth moves still occur for existing installs even while the preference offer waits.
3. **No Chinese README exists in this checkout:** `git ls-files '*README*'` returns only `.scratch/archive/README.md` and `README.md`; `find . -maxdepth 3 -iname '*readme*zh*' -not -path './node_modules/*'` returns nothing. There is no `docs/README.zh.md` to call stale. No invented stale-row finding.

### F9 — P2 test-contract notes: snapshot provenance and contention

- `BRIEF_SNAPSHOT` (`tests/tool_adapters.rs:737–786`) is a manually maintained literal with citations, not dynamically generated from `TOOL_ADAPTERS` at test runtime. It is therefore **not runtime-circular**, and it does catch accidental single-sided edits. Independent review of the rows found no incorrect global path/read/precedence transcription. However its citation is marked `allow(dead_code)` and not checked, and the brief is not consumed. **Fix:** accurately describe this as a manually reviewed brief-derived snapshot, or bind it to a brief revision/digest if “a brief edit is required” is to be an enforced guarantee.
- Requested Rust flakes did **not** reproduce: both complete runs passed 878 tests. A Node source-body comparison with `8b42ce5` reports identical `listing_reconciles_when_no_mutation_is_in_flight`, `list_reconciled`, and `missing_status_when_source_absent`. The first two use 100 × 50 ms polling of the process-global try-lock. New preference-save/launch tests (`the_preference_relocates_a_readers_targets_both_ways_on_save`, `the_launch_pass_never_moves_a_readers_target_on_the_unset_default`, `saving_the_preference_on_an_existing_install_relocates_and_ends_the_offer`, `declining_the_offer_saves_own_dirs_and_ends_it`) and new import/configure tests add guarded I/O contention; most row-relocation tests call the unlocked seam and do not. **Fix:** isolate these no-contention tests from unrelated process-global mutations or provide a deterministic test-only synchronization seam; a larger monotonic polling deadline is a mitigation, not proof of fairness. Do not increase the production reader's blocking behavior.

## Verification against every requested area

### 1. Registry versus the brief

Mechanical Node extraction of the enum, every registry literal, README keys and snapshot emitted:

```text
ToolId variants: 47 README rows: 47
registry literals: 47
snapshot rows: 47
in-memory sensitivity: Codex registry BothShown -> OwnWins;
  snapshot remains BothShown; assert_eq!(adapter.reads_agents_standard_globally, expected.reads...) fails
brief consumed by snapshot test: false
```

The in-memory mutation changes only a string copy of Codex's literal; the failure conclusion follows the actual test's equality assertion, not a modified repository/test run. The normal Rust snapshot test passed in both complete runs.

**All 47 global rows checked** (the brief does not newly establish every unchanged detector or symlink capability; those were checked against baseline/invariants rather than falsely called vendor-verified). `—` means no verified AGENTS read; U/O/A/B = Unknown/OwnWins/AgentsWins/BothShown. Paths are home-relative.

| Key | Global dir after change | AGENTS precedence | Brief row / disposition |
| --- | --- | --- | --- |
| agents_skills | .agents/skills | — | §1 #1, convention |
| cursor | .cursor/skills | U | #2 |
| claude_code | .claude/skills | — | #3, I12 |
| codex | .agents/skills | B | #4, §3, I2 |
| opencode | .config/opencode/skills | U | #5 |
| antigravity | .gemini/config/skills | — | #6, I3 |
| amp | .config/agents/skills | O | #7 |
| kimi_cli | .kimi-code/skills | U | #8, I4; current Kimi Code order unknown |
| augment | .augment/skills | O | #9 |
| openclaw | .openclaw/skills | A | #10; restored by fix-pass Comment |
| copaw | .copaw/skill_pool | — | #11, uncertain |
| cline | .cline/skills | U | #12; secondary since-version omitted correctly |
| codebuddy | .codebuddy/skills | — | #13 |
| command_code | .commandcode/skills | O | #14 |
| continue | .continue/skills | — | #15 |
| crush | .config/crush/skills | U | #16: listed order, collision rule undocumented |
| junie | .junie/skills | U | #17 |
| iflow_cli | .iflow/skills | — | #18 |
| kiro_cli | .kiro/skills | — | #19 |
| kode | .kode/skills | — | #20 |
| mcpjam | .mcpjam/skills | — | #21 |
| mistral_vibe | .vibe/skills | U | #22 |
| mux | .xum/skills | O | #23, I7; key retained, label Xum |
| openclaude | .openclaude/skills | — | #24 |
| openhands | .agents/skills | A | #25, I8 |
| pi | .pi/agent/skills | U | #26; order only LIKELY, correctly not promoted |
| qoder | .qoder/skills | — | #27 |
| qoderwork | .qoderwork/skills | — | #28 |
| qwen_code | .qwen/skills | O | #29 |
| trae | .trae/skills | — | #30 |
| trae_cn | .trae-cn/skills | — | #31 |
| zencoder | .zencoder/skills | — | #32 |
| neovate | .neovate/skills | — | #33 |
| pochi | .pochi/skills | O | #34 |
| adal | .adal/skills | — | #35 |
| kilo_code | .kilo/skills | U | #36, I6 |
| roo_code | .roo/skills | O | #37 |
| goose | .agents/skills | U | #38, I8 |
| gemini_cli | .gemini/skills | A | #39 |
| github_copilot | .copilot/skills | U | #40 |
| droid | .factory/skills | U | #42 |
| windsurf | .codeium/windsurf/skills | U | #43, I9 hold; candidate .config/devin/skills noted |
| hermes-agent | .hermes/skills | — | #45, opt-in not promoted |
| deepseek_harness | .dsh/skills | O | §4, I13 |
| zcode | .zcode/skills | O | §4, I13 |
| codewhale | .codewhale/skills | O | §4, I13 |
| workbuddy | .workbuddy/skills | — | §4, I13, LIKELY global-only |

**Important correction to the audit prompt:** Amp is OwnWins; **Crush is Unknown**, as §1 row 16 and summary item 6 explicitly say. Its `.config/agents/skills` cross-read does not verify an own-wins collision rule. Changing Crush to OwnWins to satisfy that shorthand would contradict the source of truth.

`since` exactly preserves the four verified identifiers: Codex `rust-v0.95.0` (= requested 0.95.0), Gemini `PR #18151`, Roo `PR #11181`, Pochi `2a9ff73 (#1360)`. Others null. Reader count 27 = original 24 including OpenClaw + three new readers. WorkBuddy and the LIKELY/UNKNOWN/opt-in-only cases have no read fact.

**Every changed detector/former-dir fact checked:** Codex `.codex`, former `.codex/skills`; Antigravity detects only `.gemini/antigravity`, former both `global_skills` and `skills` under it; Kimi detects `.kimi-code` + `.kimi`, former `.config/agents/skills`; Kilo detects `.kilo` + `.kilocode`, former `.kilocode/skills`; Xum detects `.xum` + `.mux`, former `.mux/skills`; Goose `.config/goose`, former `.config/goose/skills`; OpenHands `.openhands`, former `.openhands/skills`; OpenClaw `.openclaw`, former `.clawdbot/skills` + `.moltbot/skills`, former keys `clawdbot` + `moltbot`. New detectors `.dsh`, `.zcode`, `.codewhale`, `.workbuddy`. Codewhale never detects `.deepseek`; Antigravity never detects `.gemini/config`. Unchanged rows keep their baseline detector and empty former list except Augment's pre-existing `.augment/rules`.

Kimi **does** list `.config/agents/skills` as former, but not blindly: `superseded_row` first resolves the row's own Tool, and cleanup retains another owned global row at the old path. The Kimi/Amp test verifies the required ordinary hazard; F2 records the incompleteness of the broader check.

`also_reads` matches the ticket's other-harness-dir scope: Cursor Claude+deprecated Codex; OpenCode/Amp/Augment/Continue/Goose/Codewhale Claude; Crush `.config/agents/skills` + Claude; Kimi legacy Claude+Codex+`.config/agents/skills`; Antigravity Gemini. Kilo's opt-in Claude read is correctly omitted. Codewhale's own-brand former `.deepseek` and Antigravity's own CLI alternate dir are not other registered harness dirs; their omission from this particular field is not a mismatch. Kimi's legacy-only cross-reads are encoded without a per-install version distinction, as the ticket requested, so they can conservatively over-warn a current Kimi Code installation.

`group` remains **project-scope**, with the same nine members: Cursor, Codex, OpenCode, Antigravity, Amp, Kimi, Cline, Gemini, Copilot. Global Codex/Goose/OpenHands sharing is implemented through equal global dirs, not by expanding this project group; that is consistent with I2's directory-sharing alternative and the recorded decision. All 47 `supports_symlink` values are true; their exhaustive test passed. No new tool-name branch was added to sync code.

**WorkBuddy:** `project_relative_skills_dir == None`. `project_tool_entries` filters it out; gitignore skips it; `configure_project_tools_unlocked` refuses it before writes with the allowed UI-unreachable prose bail; `resolve_project_sync_target` returns None and `resolve_assignment_artifact` propagates None. The project matrix receives no WorkBuddy column because it uses that catalog. Existing project roots remain explicit. Both global-only tests passed.

### 2. README and retired entries

Independent counts are **47/47**, not the superseded Result's 46/46. The global group sentence names Codex, Goose and OpenHands. Clawdbot/MoltBot are gone as rows; OpenClaw remains. Four new rows are present, with WorkBuddy's global-only caveat. The claimed Chinese README is absent, not merely untouched. F8 covers the English prose overclaims.

### 3. Relocation hazards and locking

- **Kimi + Amp:** ordinary same-skill owned rows at `.config/agents/skills/x` → only Kimi moves under own dirs; Amp artifact remains. Executed test passes. **Kimi alone:** old artifact removed; executed test passes.
- **Codex + existing agents_skills row:** different Tool keys, so no re-key is necessary. If destination bytes match central, Codex's same row is settled at `.agents/skills/x`, agents_skills retains its own row, old Codex artifact removed. UNIQUE(skill,tool) remains intact; two different tool rows on one shared artifact are intentional. If destination differs, Codex remains at old path with `left_in_place`, log only; F3 concerns later mutations.
- **Retired key:** re-key preserves row id, then settles the new path. Both keys × both layouts plus second invocation are covered by passing tests. When OpenClaw already has a row, former key stays, `left_in_place += 1`, preview excludes it. “Not counted” means not counted as a proposed/finished move, not absent from the summary.
- **Other owner:** uninstalled global owner is retained; never-synced/disabled marker owns nothing and does not protect it; project/other-skill owners are missing from the check (F2).
- **Collision:** identical destination may be replaced; different bytes/missing central/missing original leave the row, log and count `left_in_place`. No per-row typed error is returned from this relocation API; the summary is counts and the detail is logs, consistent with “leave + log.” A cleanup failure after row settlement only logs and counts as relocated; that pre-existing best-effort rule can leave an untracked old artifact and remains a known limitation, not a newly introduced claim of transactional rollback.
- **Guard:** launch has one call at `lib.rs:183–189`; entry wraps one `serialized` at `target_relocation.rs:79`. Save has its own one section at `:97`. Both call `relocate_unlocked`, which calls the unlocked artifact-removal seam and the low-level sync engine, not another guarded entry. No nested guard entry found. Core receives `home`; no new ambient root resolution.

### 4. #83 default, reads and wire

The saved/unset/failure branches at `settings.rs:276–323` implement the fix-pass decision: saved bool wins; absent/garbage + no global target rows writes true; absent/garbage + any global target rows returns own dirs + pending without a preference write; storage failures fall back to own dirs. Existing disabled/never-owned target rows still count as an existing install, as specified. Successful Settings load on a fresh store latches true, pending false.

**Fresh first-action import:** deterministic in the app because the synchronous launch relocation resolves/latches layout before `app.manage` and UI commands. Even calling the core import without launch, finalize creates a skill, not a global target; the first sync resolves layout at `global_sync.rs:622` before `Recorded` writes target rows. Thus the latch happens before the first imported deployment. Auto-sync-off imports create no target rows and cannot flip the default to existing. There is no dedicated fresh-first-import test in this diff; this is a code trace, while the fresh-latch unit test passed.

**Decline:** saves false, clears pending and does not move a normal Cursor own-dir target (passing test). Registry-truth relocations remain eligible independently; “nothing moved” is not a universal promise for an obsolete registry path. **Accept:** `saving_the_preference_on_an_existing_install_relocates_and_ends_the_offer` proves preview 1 and save `{relocated:1,left_in_place:0}` on the **same** store, then next launch 0. Preview is an attempt count, not a guaranteed success count: existing divergent destinations, missing central copies and I/O failures can legitimately yield fewer successful moves. Disabled-skill rows and blocked re-keys have different summary accounting; do not label the preview as guaranteed completion.

**Effective set:** all Some(read) cases (OwnWins/AgentsWins/BothShown/Unknown) target AGENTS when on, their own registry root when off; None never changes. Already structural AGENTS sharers stay there either way. Tests exercise both layouts. “Precedence” is warning metadata, not a different path algorithm.

**Read frequency:** one layout snapshot per `sync_skills_to_tools_unlocked` batch and per global Propagation pass; catalog once; relocation once. Not literally once for every outer operation: `onboarding_import.rs:586` re-reads after sync and does so per group; `skill_disable.rs:330`'s callers can re-read for each skill/pair, group resolution at `:370` then first-sync reads again. A given guarded section cannot interleave with the guarded preference setter, so these duplicate reads normally agree; multi-group imports release the guard between groups. If operation-wide immutability is a requirement, pass a resolved layout through composites and define how it coordinates with a simultaneous preference-save, rather than just caching a value before the guard. No test establishes this whole-operation contract.

**Shadow flag:** `catalog.rs:119–145`: only OwnWins + effective root differs from own root; takes recorded owned target names under deployed root, probes `own_dir/name` with `symlink_metadata`, and flags an unrecorded own copy. Uses explicit `home`, not environment. It does not inspect every foreign skill, compare manifest name versus folder name, or discover Crush's precedence in its cross-read dir. The intended ordinary foreign-own-copy test passes. A managed own copy left after failed relocation is intentionally excluded by the “foreign” condition even though it may still shadow; do not present the flag as a complete collision detector.

**Wire:** both commands have `#[tauri::command]` + `#[specta::specta]`, are in `collect_commands!`, execute synchronous work via `spawn_blocking`, and map failures to `CommandError`. Read/write settings result and relocation summary derive specta. `agents_standard_read` is `AgentsStandardReadDto | null`; `since` is `string | null`; new bool fields are required; no accidental `?` or `number | null` on the counters. `SettingUpdate` has only the hide sub-setting, not prefer; serde's tagged enum rejects an unsupported preference key rather than granting a general-update bypass. Shim re-exports are present.

### 5. Wider shared-dir semantics and the strip

`plan_batch_tool_targets` snapshots every effective sharer and filters `record_tools` by installedness, **not selection** (`global_sync.rs:317–326`). Success writes one owned Synced row per installed reader (`:139–150`). For example, selecting only Cursor while Pi and Codex are installed under preference-on still creates Pi and Codex target rows at the same path, with `synced_at=now` and the outcome's mode. If the virtual AGENTS entry was not detected at planning time, it need not get a row on that first operation merely because the write later creates `.agents`.

The current strip's pure `syncedStrip` (`skillPresentation.ts:1011–1062`) appends every target row outside the effective selected set. Thus an **unselected installed Pi** appears as an additional detected, filled/synced Pi mark after that deploy, contributes to N/M, and can be disabled from its deployment controls. Disable scopes the effective group, removal plans all its owned rows by stored path, dedupes the physical removal, and settles all rows disabled; a marker alone owns no artifact. This is exactly the existing D34/ADR-0002 rule applied to the wider group, not unauthorized auto-selection. The selection itself remains unchanged. UI copy must disclose this broader effect, as the handoff says.

### 6. Onboarding supporting changes and near misses

Scan uses **own registry dirs**, not effective layout, and reads no preference. Dedupe is therefore independent of #83 (the four structural AGENTS entries already share). Attribution/managed-copy shortcomings are F4.

`settle_superseded_originals` returns unless preference is on; requires a variant directly in a moved adapter's own dir, no same-skill row still at that path, a Synced same-tool row elsewhere, not already handled, path present and byte-identical. `settle_original` repeats same-content and Tool-dir fencing before removing and reports `Removed`/`KeptDivergent`/typed `Failed`.

Near misses, **code traces**:

- Preference off, renamed import: early return, no extra removal (the fix-pass restriction is effective).
- Preference on, import folder `old-name` with explicit installed name `new-name`: if finalization leaves the bytes identical, all predicates pass and old-name is removed. There is **no basename-equality test**. The test suite does not pin non-removal here.
- Preference on, original is a symlink to identical external bytes: discovery admits it; same-content follows it; cleanup removes the link itself (not its referent) and reports Removed. There is **no symlink-exclusion predicate**. A link into central is excluded earlier by the plan.
- Divergent original: fails same-content, stays; `sync_imported_unlocked` reports it KeptDivergent unless it is already represented by a target collision. The new test's comment mentions divergence, but the new test body creates only the identical case; existing divergence tests run with the own-dir fixture preference.
- Nested/out-of-own-dir variant: fails the direct-parent check. Inside another project's artifact or another skill's row is not checked by this helper; the ownership-inventory fix should be considered here too.

Consequently I cannot confirm the prompt's blanket “nothing is removed” for the first two on-preference near misses. These behaviors are consistent with removing identical imported originals, but the intended policy needs an explicit decision/test before the UI represents a narrower promise. If non-removal is desired, add `!variant.is_link` and an explicit artifact-name match (and report a kept outcome) before settlement; do not silently infer those safeguards already exist.

### 7. Retired key tolerance

Blocked global re-key leaves a visible undetected target (raw key label, F5). Propagation answers `Skipped { UnknownTool { tool } }` unless disabled, in which case Disabled takes precedence. The passing `a_retired_tools_stored_row_is_tolerated_and_still_deletable` test covers both retired keys, blocked re-key, skip and Delete. `RemovalScope::Skill` trusts recorded ownership and is not Tool-dir fenced. Unknown-key SkillTool planning uses its own rows. Even if fenced, `.clawdbot/skills` / `.moltbot/skills` **are** admitted because OpenClaw carries them as former dirs and `ensure_path_within_tool_dirs` includes every adapter's former dirs (`tool_adapters/mod.rs:1276–1290`). The proposed PATH_OUTSIDE_TOOL_DIRS failure for those ordinary paths does not occur. Selection pruning is covered and unchanged policy. Retired **project** assignments are not migrated by this global-key pass; that broader compatibility case is not established by its global-row tests.

### 8–9. Fresh guards and flakiness

| Command (worktree root unless stated) | Fresh outcome |
| --- | --- |
| `cd src-tauri && cargo test --all` run 1 | **PASS**, 878 passed, 0 failed, 16.89 s test execution |
| Same, run 2 | **PASS**, 878 passed, 0 failed, 15.70 s |
| `cd src-tauri && cargo fmt --check` | **PASS**, exit 0 |
| `cd src-tauri && cargo clippy --all-targets -- -D warnings` | **PASS**, exit 0 |
| `cd src-tauri && cargo test export_bindings` | **PASS**, 1 passed, 877 filtered; before/after binding hash identical |
| `git diff --check` | **FAIL**, generated whitespace, F7 |
| `npx vitest run src`, twice | **FAIL exit 1**, 1147 assertions pass but 3 / 2 unhandled teardown errors, F6 |
| `npx vitest run src/components/ui/sonner.test.tsx` | **PASS**, 3 tests |
| `npx tsc --noEmit -p tsconfig.app.json` | **PASS**, exit 0 |

No live-data/dev app, browser, source mutation, stash or commit used. The old Result's 870 Rust tests and 46 entries are superseded historical figures; current measured figures are 878 / 47. `CONTEXT.md` correctly separates project membership, global sharing, verified read, layout and cross-read, but its safety language must be made true by F1–F3 rather than merely retained as documentation.

## Final verdict

**FAIL.** The registry facts, 47-entry README parity, restored OpenClaw, Kimi/Amp ordinary relocation, fresh-install latch and existing-install pending offer are substantially correct and backed by passing Rust tests. The backend is not yet safe to commit: destination Disable is bypassed by relocation, cleanup can delete a project-owned artifact, and partial relocation can lose ownership during later Update/deploy. Fix those three sequences and add regression proofs before the UI half; then address scan ownership/attribution and obtain an exit-0 frontend gate. Keep the documented UI/fixture/release-smoke deferrals explicit, and do not convert the absent Chinese README or Crush's undocumented precedence into invented findings.
