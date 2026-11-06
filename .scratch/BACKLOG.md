# BACKLOG — the only cross-effort live queue

Read this first every session, then the newest note in `.scratch/handoffs/`. An item leaves this file in the
commit that closes it (or that opens the effort/ticket which absorbs it — say which). Every item keeps its
source pointer; re-verify it against the code before scheduling. Procedure and status vocabulary:
`docs/agents/issue-tracker.md` § Lifecycle.

Numbers are stable: never renumber; retire by deleting the line (history keeps it). Next free number: **#116**.

Round 19 closed 2026-11-05 (`archive/round19/spec.md` § Closure — 2026-11-05); every line it carried was re-derived from
the code that day (`archive/round19/closure/extract-{A..G}.md`).

**Order (operator 2026-11-05):**

1. **1.3.3, a hygiene patch** — a solid base before roadmap features (`.scratch/release-1.3.3/spec.md`). Merged and
   removed from this file: #63 #68 #72 #73 #74 (its pointer; persistence is #53's) #75 #90 #95 #96 #97 #98 #99 #100
   #101 #102 (dark theme only — the README embeds one image per screen) #103 #104 #106 #107 #76 #105.
2. **Round 20**: #92 then #110, beside #91 (they touch different files); #108; #109 and #84 (both need the
   project defaults/overrides pattern); then #70, #85/#86, #93, #87.

Harness research behind #92 and #108–#110: `docs/design-inputs/2026-11-05-harness-candidates-and-shared-dir-reads.md` and
`docs/design-inputs/2026-11-05-harness-candidates-acp-addendum.md`.

## Now — evidence of unfinished work is strong

- **#92 Registry structure for the wider roster (registry truth without relocation), rebuilt from `main`.** What every new
  row (#110) needs, and the facts #109 reads. Cut it fresh from `main`; branch `t3/registry-truth-pass` (`9d282d4`) is
  reference only — never cherry-pick its 44 files. Scope:
  - **Per-OS dirs.** Devin, Reasonix and Zed resolve under `%APPDATA%` on Windows, Crush also reads `%LOCALAPPDATA%`; the
    registry knows only home-relative paths. Decide whether documented home overrides (`KIMI_CODE_HOME`, `DSH_HOME`,
    `GROK_HOME`, …) are honoured — AGENTS.md's "core never resolves roots from the environment" rule applies.
  - **An optional project dir** (WorkBuddy, Hermes and others have none).
  - **Shared-folder facts per harness** from the 2026-11-05 brief's Part 2: reads `~/.agents/skills` / `~/.claude/skills`
    (default / opt-in / no / unknown), precedence, duplicate hazard (fx loads neither copy; Cortex Code flags a conflict),
    plus `also_reads` cross-reads — pinned by a brief-snapshot test.
  - **The cross-read warning** in the Harnesses pane and the deploy confirmation (no auto-deselect; Kimi's legacy-only
    cross-reads must not over-warn).
  - **Undetected harnesses collapse by default** in the Harnesses pane (operator 2026-11-05).
  - **Label and detect fixes that move no deployment:** Xum's label (`tool_adapters/mod.rs:478` still "Mux"); Windsurf
    relabelled Devin (Devin CLI reads `~/.codeium/windsurf/skills`); Antigravity CLI's `.gemini/antigravity-cli` detect
    dir; Zenflow as a Zencoder alias; QwenPaw's label and `.qwenpaw` detect dir (its dir move is #108).
  - A fixture `registry` scenario; README table; the table test.
  Absorbs #45 and #40's detection half. Source: `docs/design-inputs/2026-11-05-harness-candidates-and-shared-dir-reads.md`
  § Part 2, § Implications; `archive/round19/issues/34-registry-truth-and-agents-dir-preference.md:30–64, :116`;
  `archive/round19/closure/extract-F.md` F8.
- **#110 New harness rows (operator 2026-11-05: every recommended row, the optional ones included).** Blocked by #92.
  Each row re-verified against its brief source when added; README rows; the table test; the AGENTS group label's count.
  - **Own rows:** DeepSeek Harness, ZCode, Codewhale, WorkBuddy (promised by CHANGELOG 1.3.1); Grok Build, Deep Agents
    Code (copy by default — it rejects symlinks resolving outside its skill dirs; see #84), Reasonix, Cortex Code, Rovo
    Dev, IBM Bob, ForgeCode, Firebender, MiniMax Code, Posit Assistant, AiderDesk, Autohand Code CLI, Code Studio,
    CodeArts Agent, Qoder CN (IDE, ex-Lingma), Qoder CN CLI, Kimchi, AstrBot, oh-my-pi, Letta Code; Muse Code, Poolside,
    Dirac, Stakpak (ACP addendum; Muse Code, Poolside and Dirac also join the AGENTS group at project scope).
  - **AGENTS-group members only** (no own deploy target): Warp, Zed, Dexto, fx, Sarvam Code, VT Code, DimCode,
    crow-cli, Corust Agent.
  - Skipped by the briefs (each with its reason there): Codemaker, inference.sh, Jazz, Loaf, Moxby, Ona, Terramind,
    Tinycloud, Tabnine CLI, Eve, PromptScript, Replit, fast-agent, siGit Code, Minion Code, Harn, Nova, GLM Agent,
    Agoragentic. ZeroClaw and nanobot (per-workspace skills dirs) need their own look.
  Source: both 2026-11-05 briefs.
- **#108 Redeploy: deploy to the new place, then undeploy the old (operator 2026-11-05).** One mechanism for every change
  of where or how a Deployment lives: the shared-folder preference (#109), registry dir corrections, retired keys and
  copy ↔ link switches (#84). Rules:
  - **Deploy first, then remove** — a crash leaves at most a duplicate, never a missing skill, and a rerun completes it.
  - **Remove the old copy only when byte-identical** to the library; a differing copy (local edits) goes to the one
    overwrite ask (D12).
  - Removal goes through `artifact_removal`, so **#107 lands first**.
  - An old copy that fails to remove is reported as the duplicate it leaves ("shows twice in Cursor");
    `RELOCATION_PENDING` / `relocation.incomplete` surface through `reportOutcome` (the fold skips every non-failed
    status today).
  Corrections it carries: Codex, Goose, OpenHands → `~/.agents/skills` globally; Antigravity → `~/.gemini/config/skills`;
  Kimi → `~/.kimi-code/skills` (never blindly: `.config/agents` is Amp's and Crush's too); Kilo → `~/.kilo/skills`; Xum →
  `~/.xum/skills`; Zencoder → `~/.agents/skills` (`.zencoder/skills` is deprecated); Copaw → QwenPaw (`~/.qwenpaw`);
  **Clawdbot and MoltBot retired into OpenClaw**, OpenClaw kept (a stored retired-key row is tolerated as an unknown key,
  `archive/round19/issues/34-…md:125–131`). Also: onboarding scans former dirs (`~/.codex/skills`) for import; the launch
  pass returns as this mechanism (disabled in 1.3.2, `lib.rs` ~:172); acceptance tests are audit 3's F14–F16 (aliased
  roots, a destination aliased onto the central repo, Keep over a copy with local-only edits); a release-smoke line — a
  real machine's first-launch move checked with `ls -la` over the old and new dirs. Source: this session;
  `archive/round19/review/ticket-34-backend-audit-3.md` § F15; `archive/round19/closure/extract-F.md` F9–F19,
  `extract-E.md` E18.
- **#109 Shared-folder preference (operator 2026-11-05; supersedes the former #83).** A Settings toggle — use shared
  skills folders when a harness can read them — and a dropdown for a harness that reads both: `.agents` (default) or
  `.claude`. Each harness is served by exactly one folder, chosen from #92's facts; a harness reading neither keeps its
  own. Off = every harness's own folder, which for Codex is `~/.agents/skills` and for Claude Code `~/.claude/skills`.
  Applied through #108. Downstream:
  - **A Deployment is a folder; harnesses are its readers.** My Skills rows show one dot per folder (`.agents`,
    `.claude`, plus own-folder dots); the inspector groups detected harnesses under the folder serving them.
  - **Disable on a shared folder is per folder**, and its copy names every reader it affects. One reader cannot be
    excluded while another harness needs the folder — say so rather than pretend.
  - **Unavoidable duplicates**: a skill sent to a `.claude`-only harness (Claude Code) and an `.agents`-only one (Codex)
    sits in both, and every harness reading both sees two (still fewer than today: Cursor can see four). Mark it where
    it hurts (fx, Cortex Code).
  - **Project scope** gets the same model with a per-project override; the defaults/overrides UI is designed once with
    #84 — study T3 Code's project defaults and per-project overrides in the spec phase (operator 2026-11-05).
  - Carries #88's preference residue: the offer dialog, preview copy ("would move N"), the shadowed-own-copy flag,
    Import near-misses under the preference (a renamed / symlinked identical original), the layout read once per
    operation (composites re-read it today), onboarding's reader roster for shared / aliased dirs.
  Blocked by #92 and #108. Source: this session; `docs/design-inputs/2026-11-05-harness-candidates-and-shared-dir-reads.md`
  § Part 2, § Implications; `archive/round19/issues/34-…md:440–466`.
- **#91 Changing the library folder is unguarded, non-atomic and strands every link.** Settings → General → Browse…
  writes `central_repo_path` outside `runAction` (`src/hooks/useSettingsState.ts:214–228`); the backend
  (`commands/mod.rs` `update_setting` → `core/settings.rs:490–505` → `core/central_repo.rs:19–55` `move_central_repo`)
  takes no `mutation_guard`, moves one skill at a time and `?`-bails mid-loop (earlier skills already moved and
  re-recorded, the new root never saved), and nothing re-points or re-materialises a Deployment afterwards — every
  symlink/junction into the old root dangles. No bytes are lost; the library splits and every harness breaks. Clean
  Now (`clearGitCacheNow`, `useSettingsState.ts:309–320`, `commands/mod.rs:245–252`) also runs outside the lifecycle and
  the guard while an acquisition may read the cache. Design both as mutations under the lifecycle + guard; the move
  all-or-nothing (or resumable with truthful status) and followed by re-pointing every owned Deployment. Pre-existing
  (not a round-19 regression); designed in round 20 (operator 2026-11-05). Source: `archive/round19/review/ticket-20-audit.md:161`;
  `archive/round19/closure/extract-E.md` E1–E2.
(Retired: #88 — split 2026-11-05 (operator): physical identity in every destructive seam, onboarding's physical-path
exclusion included → #107; resumable moves, dir corrections, the launch pass, F14–F16 and the smoke line → #108;
the #83 preference and its UI → #109. #89 — the lexical launch relocation pass is no longer called (1.3.2, PR #5,
`fc277aa`); re-enabling it is #108's.
Absorbed: #79 → round 19 ticket 29, #71 → 33, #78 → 30, #80 #81 #82 → 31 — all shipped; #67 #45 #83 and #40's detection
half → ticket 34, superseded 2026-11-04 → #88 (relocation, the preference) and #92 (registry facts, rows, warning).)

- **#70 The overwrite ask can't be answered knowingly.** Twice in the think-aloud the operator chose Overwrite without
  being able to judge: *"it gives me no way to see them whatsoever, and I can't even see the diff."*
  - The ask should name **who put the folder there**. An Add or Enable collision has no target row, so it is never
    Skills Hub's (`OverwriteRow`, `src/lib/overwriteAsk.ts:30–42`, carries no provenance; the wire row is `path`,
    `modifiedAt`, `sameContent`).
  - It should offer a **content view or diff** against the skill, plus Reveal in Finder per row (`OverwriteModal.tsx:93–103`).
  - Each button should state its **consequence** ("Keep theirs — {skill} stays off {Tool}; Enable later to
    replace it"). He couldn't predict Keep (`resources.ts:439–447`, `:575–583`).
  - The **overwritten** rows should be recorded in the run report: he had forgotten which folders he overwrote. Kept
    rows already are (a quiet "declined" note, `reportOutcome.ts:303–304`); an overwritten pair settles as plain
    `synced` and vanishes (`overwriteAsk.ts:317–324`; same family as #64 (3)).
  - An ask that arrives after Cancel should say the skill was already being added.
  - Source: `archive/round19/issues/18-operator-think-aloud.md` § Findings (P2·Q4, P5·Q3, P5·Q4).

(Absorbed earlier: #39 #46 #47 #49 #52 and the §E borrows were absorbed by `archive/round19/spec.md` on 2026-09-27: #52 → tickets 01 + 13, #49 → 11, #39 → 11, #47 → 16, #46 → 05 + 21. Earlier: #43 #44 → `archive/round18/issues/01`, `02`.)

(Earlier dispositions: #43 #44 → `archive/round18/issues/01`, `02`. #37 opened as the round 18 → 19 UI effort (`archive/round18/spec.md`, `archive/round19/spec.md`). every pre-round-16 `Later`/`Parked` line was absorbed by `archive/round16/spec.md` § Dispositions — items #12 #15 #22
#28 #35 #36 → tickets; #18 → ticket 08; #16 #17 #21 #23 #24 #25 #27 #29 dropped by name there; #30 superseded by #37–#41; #26 moved
to Future efforts. #42 (overwrite ask on `TARGET_EXISTS`) was absorbed by `archive/round17/spec.md` D3 / `archive/round17/issues/03`. Items #43–#60
were seeded 2026-09-24 from the upstream/field survey in `docs/design-inputs/` — each carries its § pointer; re-verify against the
code before scheduling.)

- **#84 Link vs copy, chosen by the operator at every level (operator 2026-11-05: important; unparked).** Today
  copy-vs-link is the registry's `supports_symlink` alone (every entry `true`; Cursor's history shows a harness can
  regress). Default link everywhere; overrides at global scope, project scope, per project and per harness — most
  specific wins (harness › project › scope › default). A registry `supports_symlink: false` always copies, and the link
  choice shows disabled with its reason (Deep Agents Code, #110). Sync code stays name-blind: the setting is read as a
  capability. Changing a level re-deploys existing Deployments through #108 (identical-or-ask). The per-project UI
  shares #109's defaults/overrides pattern. Source: grill 2026-11-02 (Q18); this session.

## Later — real, not urgent

- **#93 A filter can hide selected skills, and every bulk verb still acts on them.** `BulkRail.tsx:51` shows only the
  total; each verb runs over the whole selection (`:60–73`), Remove included (its confirmation names the first 6,
  `DeleteModal.tsx:19–20`). **Decided 2026-11-05 (operator):** keep the selection across filter changes, and say
  "N hidden by the filter" in the rail's count and in every count confirmation. Source:
  `archive/round19/issues/13-selection-bulk-rail-verbs.md:169–170`; `archive/round19/closure/extract-C.md` C2.
- **#94 Global Deployments are never checked against disk.** A harness link the user deletes by hand stays "Deployed"
  forever and Update vouches for it: Propagation skips an owned link without a presence probe
  (`core/propagation.rs:273–286`), and only the project listing reconciles (`CONTEXT.md:120`), so "Copy missing" exists
  only on project rows. **Decided (operator 2026-11-05): yes** — a read-only presence probe where the library is listed,
  status written through `try_serialized` like the project reconcile. Source: `archive/round19/review/ticket-34-backend-audit-3.md`
  F14 note; `archive/round19/closure/extract-F.md` F7.
- **#112 The matrix's whole-cell hit area makes un-assign misclicks likelier.** Since 1.3.0 (ticket 17) the whole cell
  (36 px × ≥144 px) toggles, and it is the same area hovered to read the state word; in 1.2.x only the 18 px checkbox
  did. 1.3.3's 5 s Undo (#75) mitigates; shrinking or splitting the hit area is a design call. Source: 1.3.3 L5 report.
- **#113 Import deletes a divergent original before deploying over it.** The deploy cannot go first (the copy occupies
  the target path); a safe order is move-aside → deploy → delete on success / restore on failure
  (`onboarding_import.rs` original removal). 1.3.3 only made the copy say a failed deploy leaves the harness without the
  skill (#98). Source: 1.3.3 L5 report.
- **#114 My Skills' sort mode is not remembered.** `useListLens` keeps sort in `useState` (`src/hooks/useListLens.ts`
  ~:126), so "Problems first" (1.3.3) and the others reset on relaunch; persist it through `preferences.ts` (a new key;
  keep `name` / `updated` / `added` / `issues` values). Source: 1.3.3 L4 review.
- **#115 The bulk Enable confirmation shows a stale project count for a moment** after a Disable, until project
  placements reload (pre-existing; seen in 1.3.3 L4's fixture checks). Source: 1.3.3 L4 report.
- **#111 Registry drift check.** A scheduled workflow (like `update-featured-skills.yml`) diffs `vercel-labs/skills`'
  `src/agents.ts`, the ACP registry (`agentclientprotocol/registry`) and the upstream fork's registry against
  `TOOL_ADAPTERS`, and opens an issue on a new or changed row — replacing manual audits like this session's. Operator
  2026-11-05: worth designing. Source: this session.
- **#87 Project-scope relocation for corrected harness dirs.** Blocked by #108: `main` still has Kilo global *and*
  project `.kilocode/skills` (`tool_adapters/mod.rs:636, :639`) and Xum both `.mux/skills` (`:480, :483`) — ticket 34
  never merged. When #108 corrects the global dirs, the project dirs still need a project relocation pass (per project,
  under the guard, same byte-identical-replace rule, reported in the project view) before they flip; the same pass
  covers Goose / OpenHands project `.agents/skills` and project assignments under a retired key (clawdbot / moltbot —
  #108 retires them). Likely becomes #108's project-scope leg. Source: `archive/round19/review/ticket-34-backend-audit.md` § F8,
  `docs/design-inputs/2026-11-02-harness-global-skills-dirs.md` I6/I7.
- **#85 Projects matrix keyboard model (ticket 17 audit N1, carried by ticket 23).** The matrix is a plain Tab walk:
  127 Tab presses from the first to the last cell of the 25-skill fixture matrix (`archive/round19/review/ticket-17-audit.md`
  § N1). Ticket 23 decided the ⌘K palette's keys define no grid model, so this is its own ticket: spatial/roving
  navigation (arrow keys across cells, one Tab stop per row or per matrix), named cell actions, a clear Tab exit —
  designed and tested as a complete interaction model, not `role=grid` on the current table. CHANGELOG 1.3.0 (:200) and
  1.3.1 (:102) list it as a known limitation. Source: ticket 23 Comments 2026-10-02 and 2026-11-02.
- **#86 The matrix publishes no selection to the surface module.** `useSurfaceState` has a selection slot per
  surface (D4), but `AssignmentMatrix`'s checkboxes are mutation controls, not a producer of `selection.matrix` — so
  the ⌘K palette's selection verbs are My Skills-only, and the bulk rail cannot exist on Projects. Decide whether the
  matrix should have a selection (checked rows → bulk assign / unassign / resync through the door) and, if so, build
  it with #85's keymap. Source: ticket 23 Result "Deliberately left", 2026-11-02.
- **#66 One multi-skill delete command (round 19 ticket 13)** — `delete_managed_skill` takes one skill; the bulk rail's Remove loops it inside one `runAction` (failures collected, one `RemovalReport` fold — the rail's report dialog counts "N of M not deleted"). Add `delete_managed_skills(ids)` under one `serialized` section answering one report so a whole-command failure mid-batch still carries the settled rows (same class as #64 (1)). Source: ticket 13 Result "Delete batch shape".
- **#65 Inspector metadata the wire does not carry (round 19 ticket 12, audit I9)** — three D15/D18/D22 clauses shipped honestly degraded because `ManagedSkillDto` / `SkillTargetDto` lack the fields: (1) a **typed settled error per target row** (`last_error` classified as `CommandError` where it settles, exported through specta, folded once by `skillHealth`) so a target row can read "permission denied" / "Overwrite and retry" instead of state words — today the inspector's Retry raises the ask from its fresh report and the row shows product-state copy only; (2) the **recorded git ref/SHA** on the skill (the store has it; the DTO does not) for the Upstream section's revision line (also #62's input); (3) a **detached-origin path** kept on Detach so the provenance line can read "Detached from ~/…" (today Detach clears source metadata and the line reads "No upstream — Refresh skips it"). Never hand-mirror these in TS; each is derive + `cargo test` + shim re-export. Source: `archive/round19/review/ticket-12-audit.md` § I9; ticket 12 Result "BACKLOG follow-ups".
- **#64 Four accepted command-API compromises from round 19 ticket 09** — each ships in 1.3.0 under one outer lifecycle action with one ask/fold/run, but falls short of one-command-per-operator-action: (1) project assign/unassign over several skills is one `bulk_assign_skill` / `bulk_unassign_skill` per skill → a multi-skill command that still carries settled partial reports when a later skill fails whole-command; (2) Deploy all is Enable then first sync, two commands over disjoint partitions (no atomic cross-partition promise) → a combined backend surface; (3) a successfully answered project collision row is dropped from the Update/Edit report (`resolve_project_collisions` returns no sync mode; folds count only failures, views are re-read) → a lossless typed settled-target representation before any consumer treats the array as a complete audit; (4) the Enable retry re-runs the whole scope with overrides (it reselects only still-disabled rows) → a pair-exact answered-pair Enable API, keeping the tests that unrelated completed pairs stay untouched. Source: `archive/round19/review/ticket-09-review.md` § Re-review ("Four command-API compromises").
- **#48 Network-boundary lint** — a build-failing script that finds every `reqwest::Client` / `git2::FetchOptions` / `Repository::clone`
  outside one module (we construct clients in several). The proxy feature it guards upstream is not wanted. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §B4.
- **#62 Passive upstream update check** — a read-only "check upstream" that compares the recorded SHA with the remote (Contents API /
  `ls-remote`) and surfaces `Update available` on the row and in the inspector without acquiring or applying; Refresh (all) stays the
  apply step. Decide the anchor with #59 (branch commit vs subtree SHA) before building. Source: round 18 bake-off, both prototypes
  rendered a state the backend cannot produce (`.scratch/archive/round18/spec.md` § Bake-off).
- **#50 `PreparedDirReplacement` mechanics** behind `finalize_update`'s backup and Propagation's copy re-materialisation — port the
  staging/backup/rollback, drop its read-time compare-and-swap hash (ADR-0005). Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §B6.

## Parked — needs a product decision before it is work

- **#77 Re-point renames to the upstream name by default.** Change source to a folder with a different skill name keeps
  the managed name and takes the new bytes. The operator wants *"it rename by default or give me a choice … the
  renaming should be default for sure"* (an upstream rename). A rename moves every target folder (global and project)
  and the central dir, and touches Edits and health keys. Decide the default and the override. Source: `archive/round19/issues/18-operator-think-aloud.md` § Findings (P4·Q2).
- **#51 Tags** — clean backend port (2 tables / 9 store methods / 7 commands) but answers "how do I find it", which the repo grouping
  (1.2.2) and wildcard search (1.1.9) already answer at ~80 skills; #52 answers "what is active". Decide at ~150 skills. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §B5.
- **#59 Subtree-SHA update anchor** (record the skill subdirectory's tree SHA, not the branch commit, so monorepo commits elsewhere
  never trigger a refresh or an Edit-conflict check) — changes what "upstream changed" means for every git row; decide with #26.
  Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §C9.
- **#61 Broaden localisation** to the top ~5 languages via machine translation, once the copy pass (round 19 `clarify`) has settled EN.
  ZH was removed in round 18 (unverifiable upstream inheritance; `git show <sha>:src/i18n/resources.ts` recovers it). Source: operator,
  2026-09-24, `archive/round18/issues/03`.
- **#60 Smaller peer-derived ideas, one line so they stay findable**: provenance backfill for imported skills from `npx skills`
  lockfiles (§C10); `metadata.targets` frontmatter as per-harness routing (§C11); per-tool "hold at previous central copy" pin (§C12);
  web mode from the CLI binary after #54 (§E10). Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md`.

## Future efforts — decided as "yes, someday", each needs its own grill/spec before it is work

- **#26 Skill Edit V2 / Fork** (keep upstream, layer operator edits, replay on Update, flag conflicts). Edit V1 is
  the foundation; Manifest module is the write door. Source: `archive/round7/issues/02:19`, `archive/round10/issues/04:35`.
  Design references (2026-09-24): Skillfile's `pin`/`diff`/`resolve` patch overlay — still the only structured one in the field;
  SkillDock's per-hunk diff / revert / push-back as the owned-skill upstream-PR path; upstream's `device_sync/merge.rs` +
  `text_merge.rs` as a pure three-way planner. `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §C13–C14.
- **#38 Add skills via `npx skill add …`-style commands**, not only GitHub repo URLs. Source: operator, 2026-09-22.
- **#40 Harness-specific detection/deployment audit** (detection half absorbed by ticket 34 2026-11-02, now #92; the deployment half — e.g. `openai.yaml` for Codex — stays here) — what does and doesn't exist per Tool (e.g. `openai.yaml`
  deployment for Codex). Source: operator, 2026-09-22. Detection half is #56; adapter rows are #45. Upstream still has the
  skills-only-footprint bug we fixed in 1.2.17 — keep `is_installed_in` when copying rows (`docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §A4).
- **#41 Repo scan finds both a Codex and a Claude Code variant of one skill** — keep both, collapse, or
  something else; needs analysis, likely absorbs part of #40. Source: operator, 2026-09-22.
- **#53 Scheduled `Refresh (all)`** via the OS scheduler — port upstream's `system_scheduler.rs` builders; needs a headless entry and
  a cross-process lock (Mutation guard is process-wide only). Not upstream's `auto_update.rs`. Operator evidence (round 19 think-aloud, P7): he assumes his morning starts after an automatic schedule has refreshed (since D34 an Update deploys nothing new, so a schedule adds no Deployments). **Unattended runs (operator 2026-11-05):** designed with #74's persistence half — a backend-written run log (each run's raw report plus action metadata, retention-bounded, folded by the frontend on load), the only record a run with no window open can leave, and what the Notifications panel reads after a relaunch. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §B3, §E6.
- **#54 `skillshub` CLI (or MCP) on the Rust core, sharing the DB** — sessions, the session-end audit and a Mac over SSH act without
  the GUI. Grill first: changes what the app is. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §C3.
- **#55 Declarative library manifest — sync the roster, not the bytes** (sources, tool selection, project assignments, Edits;
  versioned in `~/Projects/agent-skills`; the doctor view's diff target). The two-Mac gap upstream's device sync does not fill.
  Grill first. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §C4.
- **#56 Custom tool rows + generic `~/.*/skills` sweep + vendored `vercel-labs/skills` `agents.ts` behind a verified tier** — one
  detection effort (`ToolKey = Builtin \| Custom`, the Tools page showing the matched detect dir); absorbs the detection half of #40.
  Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §C5–C7, §D3.
- **#57 Presets / kits as desired state with drift** ("BDA kit" applied to a new project or Mac). Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §C8.
- **#58 Doctor / usage view** — resident-vs-body token cost per skill/harness with a residency ladder (asm's design), effective skill
  set incl. plugin-provided skills (kitter's `effective_skills.rs`), drskill's fired-in-30-days data. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §D1–D2.

## Dropped by name (recorded so nobody requeues them)

- A mutation queue — D19's "one mutation at a time" is permanent: a second action is refused with its reason, never queued
  (operator 2026-11-05). `archive/round19/spec.md` § Closure — 2026-11-05.
- Suppressing the switch-thumb and bulk-rail motion for keyboard input (Emil's literal rule) — both are 120 ms,
  contract-named and fire once (operator 2026-11-05). Same closure block.
- Reporting bugs to the upstream `qufei1993/skills-hub` — the repo detached from that fork network; no upstream issues or
  PRs (operator 2026-11-05; e.g. ticket 28's exec-bit loss).
- VoiceOver / reduced-motion checks on a release smoke — never (operator 2026-11-05; AGENTS.md § Release).
- Upstream v0.10 multi-device sync as built — syncs bytes + tags, not targets/projects/Edits; would shadow `agent-skills`; bypasses
  finalize; open data-loss PRs #157/#158. #55 is the answer. `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §F.
- Upstream `tool_distribution.rs` (a copy-refresh helper, not project sync — Propagation covers it), `skill_issues.rs` (substring
  classification of error prose), `auto_update.rs` run loop (`AppHandle` + settings-KV progress), recycle bin as built (deletes the
  row inside the archive — ADR-0002), GitHub proxy, discovery-scan settings as a second list, install-scope choice at Add, CC plugin
  discovery in onboarding, tool-hiding, avatars, tray, Korean, the v0.8 CSS. `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §F.
- Upstream *patterns*: `CODE|…` string errors in any form, settings table as per-row state, read-time re-hashing, `AppHandle` in
  core, a second tool catalog in commands, feature modules importing other features' internals. `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §F.

- Modal/route-state ownership module (round-9 panel #9) — dropped, round-10 decisions Q10 ("backlog note only").
- Virtual-group capability as AND of constituents — rejected, v-next ticket 36 (capability is the group's own registry fact).
- `NameIntent::FolderDerived` variant — rejected, v-next ticket 34:66 (variants express naming policy).
- Backend defaults over the wire — rejected, v-next ticket 34:72 (pre-load placeholders still needed).
- ts-rs type generation — superseded by tauri-specta, v-next map:54.
- Double single-action toast, fence rules, override convergence — fixed (round 9/10), not open.
- Operator smoke of a release — not a queue item: the operator installs each GitHub release build and smokes that.
- Pin "unknown status + copy + matching hash → Synced" as an integration test — declined, v-next 35:41; stored-string tests exist.
- Old nonempty Explore preview cache validation (#16) — startup wipes `.explore-cache` (`lib.rs:179`); round16 spec.
- Permissioned Cursor symlink smoke (#17) and Cursor-through-junction on Windows (#18 facet b) — no Cursor on any
  operator host; vendor docs; `supports_symlink` is the revert lever; round16 spec.
- Notification history persistence (#21) — per-row `last_error` is the durable record; round16 spec. **Reopened as #74** (operator evidence, 2026-10-27).
- Bulk local Re-point (#23) — never hit; its own effort if demand shows; round16 spec.
- WSL ↔ Windows path translation (#24), GitHub API ancestor-symlink (#25) — explicit exclusions, no new evidence; round16 spec.
- Collapse the three unsync commands (#27) — uniform since round 15; distinct toasts remain the reason; round16 spec.
  Moot since 1.3.0: round 19 ticket 01 replaced all three with Disable / Enable.
- Decouple `update_managed_skill` from the batch signature (#28 half) — "a batch of one" is the documented design; round16 spec.
- Document `content_identity::record`'s upsert (#29) — doc + 2-line body already say it; round16 spec.
