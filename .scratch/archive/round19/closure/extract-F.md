# Round 19 closure — extract F

Slice **F** · walked: `issues/26-release-1.3.x.md`, `issues/30-colour-and-identity-tokens.md`,
`issues/31-my-skills-row-and-group-anatomy.md`, `issues/32-vocabulary-harness-deploy-update.md`,
`issues/33-auto-deploy-arrival-only.md`, `issues/34-registry-truth-and-agents-dir-preference.md`,
`issues/35-dark-ground-t3-two-tone.md`; `review/ticket-30-audit.md`, `ticket-31-audit.md`, `ticket-31-standards.md`,
`ticket-33-audit.md`, `ticket-35-audit.md`, `ticket-34-backend-audit.md`, `-2.md`, `-3.md`. Code checked on `main` @ `0f5c2d3`;
ticket 34's code lives only on `t3/registry-truth-pass` (`9d282d4`, unmerged) — for 34 only the non-relocation findings are
traced to an owner, per the task.

Paths below: `issues/…` and `review/…` are under `.scratch/round19/`.

## 1. Carry — real, still open, not in BACKLOG

| id | title | source | current evidence | proposed home | why it matters |
|---|---|---|---|---|---|
| F1 | Reverse i18n "unread key" scan is lexical, not a read detector | `review/ticket-31-audit.md:119` (F3), `review/ticket-31-standards.md:15` (S2), `issues/31-…md:253` (noted, allow-list empty) | `src/i18n/resources.test.ts:98–116`: any quoted occurrence of a key counts as a read (`quoted()`), every `` `x.${`` template prefix exempts its whole family (`DYNAMIC_PREFIX`); self-test `:129–140` covers only the 4 accepted shapes — no comment / unrelated-string / concatenation / dead-sibling-under-live-prefix case; doc comment `:90–98` still claims "Anything else is dead copy" | later | Dead copy can hide behind an unrelated string or a template prefix; `t(prefix + leaf)` reads are invisible — the guard over-promises. Fix: add the four counterexample self-tests, state the limitation, narrow to `t(…)` calls + key maps, explicit allow-list for dynamic families. |
| F2 | Selecting a project reconciles rows but never refreshes My Skills | `review/ticket-31-audit.md:34` F1 fix text ("reconcile reads that change assignment status must converge the library too") | F1 itself closed (see F26), but `useProjectState.ts:295–318` `applyView` (used by the selection read) calls no `converge()`; only `settleView` `:321–327`, `refreshView`'s `finally` `:346–349` and remove `:444` do. The read writes statuses: `get_project_view` → `project_sync::list_assignments_with_staleness` → `reconcile_assignment` (`project_sync.rs:1081–1113`, `:1134–1147`) | later | A copy found missing/stale on selecting a project leaves My Skills' row mark, Issues count and inspector health stale until some later catalog reload. |
| F3 | DESIGN.md cites gitignored evidence the closure prune deletes | ticket 30 Result / ticket 35 Result (`issues/35-…md` "The script was lost with ticket 30's worktree and rebuilt") | `DESIGN.md:30` and `:266` tell readers to run `node .scratch/round19/evidence/30-contrast.mjs` (re-measure rule); `.gitignore:65` ignores it (`git check-ignore` confirms); also `DESIGN.md:604` (31 toast png), `:894`, `:1158`, `:1337`, `:1420` cite `evidence/` captures | **now** (before closure step 2 prunes evidence) | The only contrast-measuring tool — already lost and rebuilt once — is deleted by the prune; DESIGN.md's "re-measure after any token change" rule then points at nothing. Promote it to `scripts/` (tracked, with its `--old` flag) and re-point DESIGN.md; reword image citations as dated measurements. Alternatively fold into #69 (same class: dev tool living in gitignored evidence). |
| F4 | CONTEXT says Auto-deploy governs Enable; Enable ignores it | `review/ticket-33-audit.md` § 8 ("Precision: Enable … is not newly gated on the auto-sync setting and does not automatically deploy to every selected Tool") | `CONTEXT.md:145` "on, an Add, an Onboarding import or an Enable deploys the skill to the selected harnesses …; off, it is only added to the Library"; `rg auto_sync src-tauri/src/core/skill_disable.rs` → nothing — Enable re-deploys its disabled rows regardless of the setting | CONTEXT | The glossary misstates D34's arrival set; a reader would expect Enable to be a no-op with Auto-deploy off. |
| F5 | New-harnesses "Deploy all skills" deploys nothing with Auto-deploy off | found verifying 33's arrival surfaces (audit § 8) | `App.tsx:562–581`: the button (`NewToolsModal.tsx:50`, label `syncAll: "Deploy all skills"`, `resources.ts:175`) only selects the new harnesses unless `autoSyncEnabled`; comment `:578` "With auto-sync off the new Tools are only selected, never written to" | later (beside #90, same dialog) — see Operator decision 3 | An explicit verb whose label says Deploy silently does not deploy; also an Auto-deploy use after arrival that D34 says does not exist. |
| F6 | Two guard-polling Rust tests flake under parallel load | `issues/34-…md` fix pass 1 "Unproven" + fix pass 3 Proof run 2; `review/ticket-34-backend-audit.md` F9; `-3.md` § 8 | On `main`: `core/tests/mutation_guard.rs:466–480` polls `try_serialized` 100 × 50 ms while the same file's HOLD tests occupy the process-global guard; `core/tests/project_sync.rs:745` `missing_status_when_source_absent` named alongside. Not branch-specific — the test bodies are identical on `main` | later | An intermittent red `cargo test --all` in CI/the gate. Audits' fix: isolate the no-contention case or a deterministic test-only sync seam — never make the production reader block. |
| F7 | Global deployments are never reconciled against disk | `review/ticket-34-backend-audit-3.md` F14 code note ("Propagation … can continue answering `LinkFollowsSource` without checking that the link exists") | `propagation.rs:273–286` (global) and `:531–535` (project) skip an owned link without a presence probe; no global path writes `SyncStatus::Missing` (only `sync_status.rs`'s project reconcile; `CONTEXT.md:120` "The reconcile pass (run by the project listing)"); `rg next_status src-tauri/src/core/skill_catalog.rs` → nothing | later — see Operator decision 2 | A harness skills link the user deleted stays "Deployed" globally forever and Update reports it healthy; "Copy missing" can only ever appear on project rows. |
| F8 | Registry facts, four new harnesses and the cross-read warning are orphaned | ticket 34 Work `issues/34-…md:30–64` (facts, `:54` four new rows, `:58` cross-read dedupe), `:43` Windsurf hold, `:116` Mux→Xum; audit 1 F9 / audit 2 F9 / audit 3 § 9 (stale registry-header sentence, branch `tool_adapters/mod.rs:341`); BACKLOG `:31` (#45, #40 detection half absorbed by 34) | On `main`: `rg -c 'also_reads\|reads_agents_standard' core/tool_adapters/mod.rs` → 0; no `deepseek_harness`/`zcode`/`codewhale`/`workbuddy` variant; Mux `display_name: "Mux"` `:478`; Windsurf still `.codeium/windsurf/skills` `:720`. #45's and #67/#83's BACKLOG lines were deleted on absorption; #88's "Owns" list (`BACKLOG.md:18–19`) names only dir moves, the group joins, the preference's accept path and the OpenClaw re-key. `CHANGELOG.md:95–97` publicly says "four new harnesses … held for 1.3.2" | own BACKLOG item (later), or fold into #88 — Operator decision 1 | None of these moves an artifact (the one structural change is WorkBuddy's `project_relative_skills_dir: Option`), so parking them behind #88's redesign holds back a published 1.3.2 promise and #45/#40-detection have no live owner. Scope: `reads_agents_standard_globally` / `also_reads` facts + brief-snapshot test (fix the header to "without a snapshot edit"), DeepSeek Harness / ZCode / Codewhale / WorkBuddy rows + README, Xum display label, Windsurf `.config/devin/skills` candidate, cross-read warning in the Harnesses pane and deploy confirmation (no auto-deselect; Kimi's legacy-only cross-reads over-warn), a fixture `registry` scenario. |

## 2. Covered — already a BACKLOG item

| id | title | source | # | amend? |
|---|---|---|---|---|
| F9 | Relocation ownership/alias/crash findings (audit 1 F1–F3, audit 2 F10–F11/F13, audit 3 F14–F16) | `review/ticket-34-backend-audit*.md`; `issues/34-…md:478` | #88 | no — #88 already cites F14–F16 as acceptance tests |
| F10 | Preference UI half: offer dialog, "would attempt to move N" preview copy, hide-harness-specific sub-setting, shadowed-own-copy flag, wider shared-dir Disable copy | `issues/34-…md:445–466` (UI handoff); audit 1 § 4–5, audit 2 "Divergent destination" | #88 | **yes** — #88 names only "the #83 preference's accept path"; add the UI half (offer, sub-setting, preview wording = attempted not guaranteed, Disable-of-one-reader-disables-all copy) |
| F11 | `RELOCATION_PENDING` skip and `relocation.incomplete` swallowed by the fold | `issues/34-…md:440`; `review/ticket-34-backend-audit-3.md` § 4 | #88 | **yes** — add "surface pending/incomplete in `reportOutcome` (fold skips every non-failed status today)" |
| F12 | Import near-misses under the preference (renamed / symlinked identical original removed) | `review/ticket-34-backend-audit.md:254` (§ 6); `issues/34-…md:334` | #88 | **yes** — add as a decision+test item |
| F13 | Whole-operation layout immutability (composites re-read the preference) | `review/ticket-34-backend-audit.md:236` (§ 4 read frequency); `issues/34-…md:334` | #88 | **yes** — one line |
| F14 | Onboarding: physical-path exclusion and reader roster for shared/aliased dirs; former dirs not onboarding-scanned | audit 1 F4, audit 2 F12; `issues/34-…md:206` | #88 | no (follows the group joins #88 owns); optionally name "scan former dirs (`~/.codex/skills`) for import" |
| F15 | Retired-key label fallback / OpenClaw former-name re-key, incl. retired *project* assignments | audit 1 F5, § 7 (`:266`) | #88 (global re-key) / #87 (project rows) | **yes, #87** — add "retired-key project assignments (clawdbot/moltbot) are not migrated by the global re-key" |
| F16 | Kilo / Xum project dirs held on old values | `review/ticket-34-backend-audit.md` F8.1; `issues/34-…md` fix pass 2 F8 | #87 | **yes** — also name Goose / OpenHands project `.agents/skills` (34 Result `:204–206` lists them; audit calls them "documented compatible, less concerning") |
| F17 | Real-machine first-launch move check (`ls -la ~/.codex/skills ~/.agents/skills …`) | `issues/34-…md` Proof + `:202` | #88 | **yes** — carry as #88's release-smoke line (34's ticket-26 smoke item died with the park) |
| F18 | Lexical launch relocation on `main` also forgets a failed former-artifact removal | `review/ticket-34-backend-audit-2.md` F10 (reproduced on the Augment `.augment/rules` row); audit 1 § 3 ("cleanup failure … only logs") | #89 | **yes** — on `main`, `target_relocation.rs:126–157` settles the row `SyncCompleted` and counts `relocated` *before* `remove_superseded_artifact_unlocked`, whose failure is only `log::warn!` → an unowned old artifact; #89's guard should cover it (or the disable-the-pass option does) |
| F19 | Sync engine overwrite can delete its own source through an aliased parent | `review/ticket-34-backend-audit-3.md` F15 ("not new in pass 3") | #88 / #89 | no — #88 names "`sync_engine`'s overwrite" explicitly |
| F20 | #40's detection half pointer is stale | `BACKLOG.md:163` "(detection half absorbed by `round19/issues/34` 2026-11-02 …)" | #40 | **yes** — 34 is superseded; re-point the detection half to F8's new item (or #88) |

## 3. Closed — fixed since

| id | title | source | closed by | code evidence |
|---|---|---|---|---|
| F21 | Contrast script quantised OKLab/`/90` intermediates | `review/ticket-30-audit.md:7` F1 | 9ca0bc8 (ticket 30 comment 11-04); re-verified by 35 audit | `review/ticket-35-audit.md` § 2: 95 dark cells agree within 0.01 with an independent float calculation |
| F22 | Amber-on-`raised` rule untrue of ProjectList | `review/ticket-30-audit.md:34` F2 | 9ca0bc8 | `DESIGN.md:125–129` names the `ProjectList.tsx` exception at 4.58 |
| F23 | How dark should dark be (30-dark-ground prototype) | `issues/30-…md` Comments 11-04 | ticket 35, 03065db | `src/styles/theme.css` dark grounds `#000000`/`#040404`/`#0e0e0e`/`#1e1e1e` |
| F24 | Light amber `#985700` / light `row` = background need operator eyes | `issues/30-…md` Result decision 3 | operator 2026-11-04 (30 Comments) | — (decision) |
| F25 | Legacy `--bg-element-hover` alias 4.41 light; App.css fallbacks; dormant `Layout.tsx` / legacy `.btn-secondary` hovers | `issues/30-…md` Result; `issues/35-…md` Result; 35 audit § 3 | ticket 25 (App.css deleted, 1.3.1) | `src/App.css` absent; `DESIGN.md:266–268`; no `Layout.tsx` under `src/` |
| F26 | Project mutations left My Skills' project scope/health stale | `review/ticket-31-audit.md:34` F1 | a2eed57 | `App.tsx:108` `onDeploymentsChanged: library.loadManagedSkills`; `useProjectState.ts:246–253,321–327,346–349,444`; test `useProjectState.test.ts:1300–1316` (residual: F2) |
| F27 | Five expanded toasts covered the header | `review/ticket-31-audit.md:78` F2 | a2eed57 | `App.tsx:632` `visibleToasts={3}`; `DESIGN.md:606–608` |
| F28 | Visible-order fold lived in the hook | `review/ticket-31-standards.md:7` S1 | a2eed57 | `skillPresentation.ts:91` `onScreenOrder` |
| F29 | Inline group header wanted a component | `review/ticket-31-standards.md:22` S3 (carried to 24) | 6b2e343 (ticket 24) | `src/components/skills/SourceGroupHeader.tsx` |
| F30 | Neutral (ownerless) chip invisible on dark `muted` | `issues/31-…md:238` | a2eed57 | `SourceGroupHeader.tsx:62` `variant="solid" tone="neutral"` |
| F31 | Gemini CLI → `GC` initials, operator to confirm | `issues/31-…md` Result | a2eed57 comment ("stands"); audit: 45 registry labels distinct | — |
| F32 | DESIGN strip table's stale light `faint`/`raised` figures | `issues/31-…md` Result; `issues/35-…md:145–147` | a2eed57 | `DESIGN.md:903` reads `6.24 · 5.53 · 5.04` light |
| F33 | Update's overwrite retry minted an absent sharer row (P0); D35 boundary tests; `RefreshSkipReason` shim; CONTEXT first-deploy wording | `review/ticket-33-audit.md:125,174,180` F1–F3, Re-check F4 | 23938db (+ closing commit) | `src/components/skills/types.ts:30`; `CONTEXT.md:77,81,145,150` carry the sibling-record qualification; `core/tests/refresh.rs` `refresh_all_reports_delete_incomplete_before_unlocatable_…` |
| F34 | New 33 strings used retired words | `issues/33-…md:190` (5) | 60dc80e (ticket 32) | `resources.test.ts` "uses no retired vocabulary word in any en value (D45)" |
| F35 | Inline code drew `muted` after the split | `review/ticket-35-audit.md:9` F1 | 03065db | `src/styles/prose.css:144–150` `background: var(--raised)` |
| F36 | Neutral `outline` on a `raised` parent has no hover delta | `review/ticket-35-audit.md:34` F2 | 03065db | `DESIGN.md:342` |
| F37 | Row hover `card` invisible on black; `accent` hover inside popover drew nothing | `issues/35-…md` Result consumer sweep | 03065db (35 Comments) | rows `hover:bg-accent`, controls hover `raised` (35 audit § 3 table, browser-verified) |
| F38 | Tooltip has no border, contrary to "every floating layer has a border" | 35 audit § 4 | 03065db | `DESIGN.md:549` states the inverted-Tooltip exception |
| F39 | Frontend suite teardown error (`sonner.test.tsx`, "window is not defined") | `review/ticket-34-backend-audit.md:85` F6 | 75aabc3 | `sonner.test.tsx` unmounts every Toaster before teardown; CI green at 718509e (26 Result) |
| F40 | Generated bindings carried trailing whitespace | `review/ticket-34-backend-audit.md:113` F7 | branch only; not present on `main` | `rg -c '[[:space:]]$' src/bindings/index.ts` → 0 |
| F41 | Round-19 closure walk | `issues/26-…md` Result | this extract walk | — |

## 4. Drop candidates — not worth carrying

| id | title | source | reason to drop |
|---|---|---|---|
| F42 | 1.3.1 operator smoke (VoiceOver, reduced motion, 960×640, `impeccable` exec bit) and 31's "VoiceOver speech unproven" | `issues/26-…md:26`; `issues/31-…md:235` | BACKLOG "Dropped by name": *Operator smoke of a release — not a queue item*. |
| F43 | Description column 141 px at 960 (47 px at the 22 rem cap) | `issues/31-…md:241`; 31 audit § 5 | Conforms to D39 (audit: "cramped but conforms"); no operator evidence asks for an earlier drop. |
| F44 | A 4th+ toast queues behind three | DESIGN.md § Toaster | Sonner's default; a run is one toast and every run is recorded in Notifications. |
| F45 | `delete_incomplete` run item offers Remove…, not Reveal | `issues/33-…md:190` (1) | Reveal lives in the Delete dialog's report that Remove… opens (per kept path); a second Reveal row action would duplicate it. |
| F46 | AGENTS.md has no D35 sentence | `issues/33-…md:190` (3) | `CONTEXT.md:79` owns the rule; AGENTS.md's Update bullet is architecture, and the brief said reword only. |
| F47 | Code names (`SyncedStrip`, `useSyncLight`, `AssignProjectModal`, AGENTS prose) keep old words | `issues/32-…md` "Deliberately left" | D45 table "Code keeps" — deliberate. |
| F48 | `solid ok` / semantic `ok` hover steps 1.016 on `raised` | `review/ticket-35-audit.md:52` F3 | No production `Button tone="ok"` (`rg 'Button[^>]*tone="ok"' src` → none outside the specimen); revisit only when one is introduced (could be one DESIGN.md line beside `:342`). |
| F49 | Light 75 % ring floor printed 3.09, documented 3.08 | `issues/35-…md:147`; 30 audit § 8 | < 0.01 difference; passes 3.0 before rounding. |
| F50 | No Chinese README to update | `review/ticket-34-backend-audit.md` F8.3 | Not a finding — the file does not exist. |

## 5. Operator decisions

1. Split ticket 34's registry facts, four new harnesses, Xum label and cross-read warning (F8) out of #88 so they can
   ship in 1.3.2 ahead of the relocation redesign? — **Yes, as their own BACKLOG item, cut fresh from `main` (not by
   cherry-picking the 44-file branch).** — None of them moves an artifact; CHANGELOG 1.3.1 already promises "four new
   harnesses" for 1.3.2, and #45/#40-detection otherwise have no owner while #88 is a design effort.
2. Should global deployments get a disk reconcile so "Copy missing" can apply to them (F7)? — **Yes, later: probe
   presence where the catalog is listed (read-only status, `try_serialized` write like the project listing).** — Today a
   deleted global link reads Deployed indefinitely and Update vouches for it; the vocabulary already has the state.
3. New-harnesses dialog's "Deploy all skills" with Auto-deploy off (F5): deploy anyway or relabel? — **Deploy anyway:
   it is an explicit verb, and D34 confines Auto-deploy to a new skill's arrival.** — The current behaviour makes the
   button's label false and keeps an after-arrival use of the setting that D34 removed elsewhere.

---
Counts: Carry 8 · Covered 12 (9 rows need an amend: #88 ×5 lines, #87 ×2, #89, #40) · Closed 21 · Drop 9 · Operator decisions 3.
Dangerous: no P0/P1 left open on `main` from this slice beyond #89 — but F18 widens #89: `main`'s launch relocation also
leaves an unowned artifact when the former-path removal fails (logged only). F3 is time-critical: closure step 2 would delete the only contrast tool DESIGN.md depends on.
