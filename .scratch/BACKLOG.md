# BACKLOG — the only cross-effort live queue

Read this first every session, then the newest note in `.scratch/handoffs/`. An item leaves this file in the
commit that closes it (or that opens the effort/ticket which absorbs it — say which). Every item keeps its
source pointer; re-verify it against the code before scheduling. Procedure and status vocabulary:
`docs/agents/issue-tracker.md` § Lifecycle.

Numbers are stable: never renumber; retire by deleting the line (history keeps it). Next free number: **#112**.

Round 19 closed 2026-11-05 (`archive/round19/spec.md` § Closure — 2026-11-05); every line it carried was re-derived from
the code that day (`archive/round19/closure/extract-{A..G}.md`).

**Order (operator 2026-11-05):**

1. **1.3.3, a hygiene patch** — a solid base before roadmap features. #107 first, then tier 1 (#106 #105 #103 #95 #68
   #63 #100 #102, #74's pointer) and tier 2 (#96 #97 #98 #90 #99 #101 #104 #73 #72 #76 #75).
2. **Round 20**: #92 then #110, beside #91 (they touch different files); #108; #109 and #84 (both need the
   project defaults/overrides pattern); then #70, #85/#86, #93, #87.

Harness research behind #92 and #108–#110: `docs/design-inputs/2026-11-05-harness-candidates-and-shared-dir-reads.md` and
`docs/design-inputs/2026-11-05-harness-candidates-acp-addendum.md`.

## Now — evidence of unfinished work is strong

- **#107 Removal can delete the central library through an aliased parent (P0-class, pre-existing; first in 1.3.3).**
  `sync_engine::remove_path_any` (`:421`) deletes a *real* leaf at its resolved physical location when a parent is a
  symlink/junction. With a parent aliased onto the central repo, the 1.3.2 review's differential probe deleted the
  central skill on 1.3.1 and 1.3.2 alike (`.scratch/reviews/1.3.2-patch-review.md` :115–151, :226). A leaf *link* is safe.
  The same flaw sits in the git cache's error-retry `remove_dir_all` (`git_cache.rs:281`; review R1, :257–271) and in
  every destructive path fenced only by the lexical `tool_adapters::ensure_path_within_tool_dirs`: Import's original
  removal (`onboarding_import.rs:720`), `sync_engine`'s overwrite replacement, artifact removal. Fix shape (review :136):
  admit by physical entry identity (canonical parent + leaf, never resolving a leaf link), an explicit protected-central
  boundary, and source/destination identity checks. A blanket `remove_dir` or canonicalise-and-delete is not a fix.
  Extracted from #88 (operator 2026-11-05: urgent). Source: as cited; `archive/round19/review/ticket-34-backend-audit-3.md`
  F14–F15.
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
exclusion included → #107; resumable moves, dir corrections, the launch pass, F14–F16 and the smoke line → #108; the
#83 preference and its UI → #109. #89 — the lexical launch relocation pass is no longer called (1.3.2, PR #5,
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
- **#95 A Disable that only records intent toasts "Nothing was removed".** Disabling a pair with nothing on disk (never
  deployed, or its first deploy failed — D9) records the `disabled` marker (`skill_disable.rs:205–224`,
  `artifact_removal.rs:645–655`) but adds nothing to the `RemovalReport`, so the fold reads it as a skipped no-op
  (`reportOutcome.ts:677–683` → `unsyncNothingPlanned`). It worked; say so ("{skill} stays off {harness}"), with a
  report field for flipped markers. Operator 2026-11-05: yes. Source: `archive/round19/issues/01-disabled-target-state.md:263`.
- **#96 Selecting a project can rewrite Deployment statuses without refreshing My Skills.** `get_project_view` reconciles
  (`project_sync.rs:1081–1147`), but the selection read's `applyView` (`useProjectState.ts:295–318`) does not
  `converge()` the library as mutations do (`:321–349`, `:444`), so a copy found missing/stale leaves My Skills' mark,
  Issues count and inspector stale until a later reload. Source: `archive/round19/review/ticket-31-audit.md:34`;
  `archive/round19/closure/extract-F.md` F2.
- **#97 Layer/focus-contract gaps.** (1) The row switch's inline Enable confirmation is a triggerless anchored
  Popover (`SkillRow.tsx:299–305`; `popover.tsx` passes `anchor` to the Positioner only): it leaves an unnamed
  `aria-hidden` focus-guard Tab stop and stays open — still on the layer stack — after a Tab-out (ticket 16 audit R1,
  passed on by 16 → 17 → 23). (2) Popover focus return depends on Base UI's internal `data-tabindex` protocol
  (`popover.tsx:80–93`), pinned only in jsdom, under a caret range (`@base-ui/react ^1.8.0`): add a browser regression
  (`scripts/browser/cdp-keys.ts` drives it) or pin the version exactly. (3) `ProjectToolsModal` is conditionally
  mounted (`ProjectToolsModal.tsx:181–184`), against "keep a dialog's shell mounted" — its focus return races the
  unmount. Source: `archive/round19/review/ticket-16-audit.md:205, :226`; `ticket-20-audit.md:162`;
  `archive/round19/closure/extract-D.md` D1/D3, `extract-E.md` E3/E7.
- **#98 Import review gaps.** (1) Unreachable when only invalid manifests remain: the banner counts
  `plan.group_count` only (`SkillsList.tsx:98`), so invalid copies in harness dirs become invisible — the operator asked
  that unmanaged skills "always" be visible. (2) A divergent copy is deleted *before* the deploy
  (`onboarding_import.rs:719–720`); if the deploy then fails, that harness has neither; the copy
  (`resources.ts:1485`) should say so. Source: `archive/round19/review/ticket-21-audit.md:13, :156, :178`.
- **#90 Fixture parity.** (1) The New harnesses dialog is unreachable under StrictMode: `src/fixtures/backend.ts`
  reports newly detected harnesses once (`seenTools`, :1572–1575) and the dev startup effect runs twice
  (`backend.test.ts:449–453` pins once-only on purpose) — give it a StrictMode-safe trigger (scenario flag, or report
  until acknowledged). (2) The fixture's onboarding plan dedupes a shared skills dir (`backend.ts:1309–1316`,
  `seenPaths`); the Rust plan reports every harness on it. (3) The fixture's `resolveProjectCollisions` throws on a
  missing project (`backend.ts:1708–1709`) where Rust answers a per-decision `NOT_FOUND` row. Source:
  `archive/round19/review/ticket-25-audit.md` § F3, `ticket-21-audit.md:114`, `ticket-04-review.md:92`.
- **#99 Tests and guards that promise more than they check.** (1) Install batch: the git test passes `resolution: None`
  (`core/tests/install_batch.rs:427–439`); no same-content first-pass or extraneous-override test; `resolve_collisions`'
  owned-row / gone-row / disabled refusal arms are untested (`project_sync.rs` ~:907–947); no unknown-mtime test.
  (2) The door's verb table asserts only that a call happened (`useSkillActions.test.ts:259–260`) and compares
  reload-on-throw against the code's own `RELOAD_ON_THROW` (`:667`) — use a literal table and pin retry → catalog read
  → `refreshView` → fold order. (3) The reverse i18n "unread key" scan is lexical (`resources.test.ts:98–116`): any
  quoted occurrence counts as a read, a template prefix exempts a whole family, `t(prefix + leaf)` is invisible; add
  the four counterexample self-tests and narrow it. (4) `styles:check`'s `importsAppCss` is a regex
  (`scripts/check-styles.mjs:72–84`) a string can fool. Source: `archive/round19/review/ticket-04-review.md:89–93`,
  `ticket-09-review.md:115`, `ticket-31-audit.md:119`, `ticket-25-audit.md` § F1.
- **#100 Two guard-polling Rust tests flake under parallel load.** `core/tests/mutation_guard.rs:466–480` polls
  `try_serialized` 100 × 50 ms while the same file's HOLD tests occupy the process-global guard;
  `core/tests/project_sync.rs:745` (`missing_status_when_source_absent`) flakes alongside. Isolate the no-contention case
  or add a deterministic test-only seam — never make the production reader block. Source:
  `archive/round19/review/ticket-34-backend-audit.md` F9, `-3.md` § 8.
- **#101 A run toast can sit over a dialog that is its own report.** Only Add's and Import's Results set
  `resultShownIn` (`useAddSkillFlow.ts:557`, `useImportReview.ts:264`); e.g. the Delete dialog's report also raises the
  run toast (`ticket-29-audit.md` check 2), and ticket 21's F5 showed a toast can intercept a dialog's Close. Measure
  occlusion first. Source: `archive/round19/review/ticket-21-audit.md` § F5.
- **#102 README screenshots show the pre-1.3 UI.** `docs/assets/*.png` (`README.md:20–29`) date from March 2026;
  recapture from the fixture (`npm run dev:fixture`, `?scenario=rich`, both themes). The feature text was corrected at
  round 19's closure. Source: `archive/round19/issues/19-docs-and-release-1.3.0.md:203`.
- **#103 Windows Deployment paths mix separators.** The operator's Windows 11 install (1.3.1, 2026-11-05) recorded
  `C:\Users\…\.claude/skills\grill-me` — registry relative dirs carry `/` and are joined onto a `\` home. Harmless to
  `Path` comparisons (components accept both), ugly wherever a path is shown or copied. Normalise at the registry join.
  Source: round 19 closure session (`ssh gpc`, DB read).
- **#104 SQLite write contention fails on a slow Windows disk.** `skill_store::concurrent_readers_and_writers_all_succeed`
  (8 writers × 50 rounds, default rollback journal, 5 s busy timeout, `core/skill_store.rs:1885`) fails even when run
  alone on GitHub's `windows-latest` ("database is locked", run 42101089457) and is excluded from the Windows CI job
  (1.3.2); it passes on the operator's Windows 11 PC. Refresh's pool is 4 workers, so first add a representative
  four-worker probe; if it fails, decide the connection / write-admission policy (WAL, one writer, longer timeout).
  Source: `.scratch/reviews/1.3.2-patch-review.md` § F3.
- **#105 `derive_name_from_repo_url` splits only on `/`.** A Windows-style `file://C:\…` URL makes the derived skill name an
  absolute path (seen only through a test fixture in 1.3.2's Windows work). Guard it: split on both separators, or
  refuse a name that is not a plain leaf. Source: 1.3.2 implementer report, 2026-11-05.
- **#111 Registry drift check.** A scheduled workflow (like `update-featured-skills.yml`) diffs `vercel-labs/skills`'
  `src/agents.ts`, the ACP registry (`agentclientprotocol/registry`) and the upstream fork's registry against
  `TOOL_ADAPTERS`, and opens an issue on a new or changed row — replacing manual audits like this session's. Operator
  2026-11-05: worth designing. Source: this session.
- **#106 An update-check failure shows the updater plugin's raw English.** Settings → Updates printed "None of the
  fallback platforms `["windows-x86_64-nsis", "windows-x86_64"]` were found in the response `platforms` object" (operator,
  Windows, 2026-11-05 — the feed lacked Windows; fixed in the release workflow). `useAppUpdate`'s check shows the
  rejection text as-is; give it one copy line ("Couldn't check for updates — download the latest release from GitHub",
  with the link) and keep the raw text as detail. Source: operator screenshot, 2026-11-05.
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
- **#76 A removal's permission failure has no next step.** "Permission denied (os error 13)" comes with Retry and
  Reveal but no explanation of what to check (ownership, read-only, a process holding it) or that Retry will fail the
  same way. Removal failures classify as `OTHER` with the raw io text (`core/artifact_removal.rs:621`); the shape is a
  typed removal-permission variant (Rust + binding + `describeCommandError` branch), as global sync has
  (`TOOL_NOT_WRITABLE`). (The "Removed from … lacks the project" bullet was struck at round 19's closure: both sides
  carry "· project" since `56da8d9`, `reportOutcome.ts:1475–1503`, pinned by `reportOutcome.test.ts:2196`.) Source:
  `archive/round19/issues/18-operator-think-aloud.md` § Findings (P2·Q10).
- **#75 Undo on a project matrix cell's toggle-off.** One accidental click on a matrix cell un-assigns the skill (the
  project's copy is removed), with no confirmation and no Undo (`useProjectActions.ts:173–178`). The inspector's targets
  list has a 5 s Undo (D21, `TargetsSection.tsx:115–142`); the matrix should match. Check whether ticket 17's larger
  cell hit area made misclicks likelier. Source: `archive/round19/issues/18-operator-think-aloud.md` § Findings (P6·Q1).
- **#74 Notifications after a relaunch — the empty-panel pointer (1.3.3).** Asked "tomorrow morning, how would you find
  today's failures?", the operator said he'd use the Notifications history. That history is session memory only
  (`useNotificationHistory.ts:8–14`), so the panel is empty after a relaunch. Now: the empty panel points at "View N
  skills with issues" (health is stored). Persisting runs moved to #53's unattended-runs effort (operator 2026-11-05). (The vanishing run toast was explained
  at round 19's closure — a failure-tone run toast was a 5 s `warning`, `reportOutcome.ts:97–98` — and 1.3.2 makes it
  persist, operator 2026-11-05.) Source: `archive/round19/issues/18-operator-think-aloud.md` § Findings (P2·Q7).
- **#73 Bulk confirmation copy.** Ticket 32 changed the words, not the shapes:
  - The Enable sentence ("Enable {{skills}} — {{targets}} on {{tools}} and {{assignments}} are deployed again, a first
    deploy each.", `resources.ts:902–903`) was unparseable on first read: *"I have no clue what the hell that thing is
    saying."*
  - The Disable sentence's zero case, "no project deployment changes" (`:894`), reads as "a skill-grain Disable leaves
    project assignments live". It means none exist. Say "None of these skills is assigned to a project".
  - 2026-10-29 smoke: the select-all Disable reads "… are removed and kept disabled …" (`:893–894`) — accurate, but
    "removed and kept disabled" reads as a contradiction.
  - Source: `archive/round19/issues/18-operator-think-aloud.md` § Findings (P2·Q8).
- **#72 My Skills reading fixes.**
  - The summary "7 failed deployments · View 11 skills with issues" (`LibraryHeader.tsx:80–93`) was read as 7 skills:
    two units side by side, and the first number taken.
  - The list is grouped by source, so 4 of 11 issue rows sat below the fold and were missed. "View N skills" already
    applies the Issues lens (`App.tsx:541`); the ask is problems-first ordering (sort modes are name / updated / added
    only, `skillPresentation.ts:56`).
  - The inspector offers Deploy on a harness it labels "(not detected)" and fails (`TOOL_NOT_INSTALLED`):
    `targetActions` returns `["sync"]` for `not_synced` regardless of `mark.detected` (`TargetsSection.tsx:46–47`,
    label :173). Refuse it in place with its reason.
  - A run headline's "1 Edit to review" item sits last in a run collapsed by default (`reportOutcome.ts:490–496`,
    `useStatusReporter.ts:600–603`); order skill-level items first, or link the headline to them.
  - Imported skills group under "Local" while their source line says "Managed here · Imported from {Tool}"
    (`skillPresentation.ts:277–281` vs `:155–168`): give them a "Managed here" group; folder sources keep "Local"
    (operator 2026-11-05: yes; ticket 11 :126–127).
  - The bulk rail at the window foot (`App.tsx:686–695`) was found only after three other paths (low; think-aloud P2·Q8
    follow-up, `issues/18` :402–415).
  - Source: `archive/round19/issues/18-operator-think-aloud.md` § Findings (P1·Q1, P2·Q5, P2·Q6, P6·Q1).
- **#68 Retire the reporter's legacy `setError` dialect** — `notifyError` is meant to be the one failure dialect, but `useStatusReporter` still exposes `setError`, with six call sites (re-verified 2026-11-05): `useSyncOrchestration.ts:675`, `useAddSkillFlow.ts:337`, `:373`, `:674`, and `useImportReview.ts:211`, `:259` (added by ticket 21). `showActionErrors` / `showActionWarnings` have no production caller (`useStatusReporter.ts:418–420`, `:581–590`). Migrate those call sites to `notifyError` / `notify("error", …)`, delete the three reporter members, and drop AGENTS.md's "`setError` remains only in the sync, Add and Import-review worlds" clause in the same change. Source: `archive/round19/issues/09-action-door-lifecycle-health-fold.md` ~:250 ("Legacy reporter members").
- **#66 One multi-skill delete command (round 19 ticket 13)** — `delete_managed_skill` takes one skill; the bulk rail's Remove loops it inside one `runAction` (failures collected, one `RemovalReport` fold — the rail's report dialog counts "N of M not deleted"). Add `delete_managed_skills(ids)` under one `serialized` section answering one report so a whole-command failure mid-batch still carries the settled rows (same class as #64 (1)). Source: ticket 13 Result "Delete batch shape".
- **#65 Inspector metadata the wire does not carry (round 19 ticket 12, audit I9)** — three D15/D18/D22 clauses shipped honestly degraded because `ManagedSkillDto` / `SkillTargetDto` lack the fields: (1) a **typed settled error per target row** (`last_error` classified as `CommandError` where it settles, exported through specta, folded once by `skillHealth`) so a target row can read "permission denied" / "Overwrite and retry" instead of state words — today the inspector's Retry raises the ask from its fresh report and the row shows product-state copy only; (2) the **recorded git ref/SHA** on the skill (the store has it; the DTO does not) for the Upstream section's revision line (also #62's input); (3) a **detached-origin path** kept on Detach so the provenance line can read "Detached from ~/…" (today Detach clears source metadata and the line reads "No upstream — Refresh skips it"). Never hand-mirror these in TS; each is derive + `cargo test` + shim re-export. Source: `archive/round19/review/ticket-12-audit.md` § I9; ticket 12 Result "BACKLOG follow-ups".
- **#64 Four accepted command-API compromises from round 19 ticket 09** — each ships in 1.3.0 under one outer lifecycle action with one ask/fold/run, but falls short of one-command-per-operator-action: (1) project assign/unassign over several skills is one `bulk_assign_skill` / `bulk_unassign_skill` per skill → a multi-skill command that still carries settled partial reports when a later skill fails whole-command; (2) Deploy all is Enable then first sync, two commands over disjoint partitions (no atomic cross-partition promise) → a combined backend surface; (3) a successfully answered project collision row is dropped from the Update/Edit report (`resolve_project_collisions` returns no sync mode; folds count only failures, views are re-read) → a lossless typed settled-target representation before any consumer treats the array as a complete audit; (4) the Enable retry re-runs the whole scope with overrides (it reselects only still-disabled rows) → a pair-exact answered-pair Enable API, keeping the tests that unrelated completed pairs stay untouched. Source: `archive/round19/review/ticket-09-review.md` § Re-review ("Four command-API compromises").
- **#63 Tighten the secure-deletion zeroing test** — `core/tests/settings.rs` (~:768) asserts live bytes are zeroed but also passes without `secure_delete` (VACUUM alone hides it); use a page-spanning sentinel and a sensitivity check so `secure_delete` is pinned on its own. Source: Astra final check of `4f6e312`, `.scratch/archive/round18/review/ticket-02-review.md` § Final check.
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
