# Round 20, Wave A — registry structure, the wider roster, a safe Library folder move (opened 2026-11-06)

Status: ready-for-agent

Ships as **1.3.4** (operator 2026-11-06). Absorbs BACKLOG **#92** (registry structure; it had already absorbed #45 and
#40's detection half), **#110** (new harness rows) and **#91** (Library folder move + Clean Now). Wave B (#108 redeploy,
#109 shared-folder preference, #84 link vs copy) is grilled **after Wave A lands**, against the registry facts as built,
in this same effort (operator 2026-11-06, grill Q1). Decisions below cite the grill question that settled them (Q1–Q30,
this session, 2026-11-06 — every recommendation accepted; version changed to 1.3.4).

Evidence: `docs/design-inputs/2026-11-05-harness-candidates-and-shared-dir-reads.md` (the **brief**, Part 1 rows, Part 2
read matrix, I1–I9) and `docs/design-inputs/2026-11-05-harness-candidates-acp-addendum.md` (the **addendum**). Reference
only, never cherry-picked: branch `t3/registry-truth-pass` (`9d282d4`) and `archive/round19/issues/34-registry-truth-and-agents-dir-preference.md`.

## Problem Statement

The operator keeps one skill Library and deploys it to many harnesses. Three things stop that from growing safely:

1. **The Harness catalog can't describe the harnesses that exist.** About 40 recommended harnesses are missing, and
   CHANGELOG 1.3.1 promised four of them. The registry can't express what many of them need: a Windows dir under
   `%APPDATA%`, the absence of a project dir, or "reads `~/.agents/skills` and has no folder of its own". It also records
   nothing about which shared folders a harness reads. So the app can't warn that a skill deployed to Cursor, Claude
   Code and Codex shows up three times in Cursor, and it has no facts for the coming shared-folder preference (#109).
   Some labels are stale too: "Mux" is Xum, "Windsurf" is Devin, "Copaw" is QwenPaw.
2. **Changing the Library folder can break every harness.** Settings → General → Browse… moves skills one at a time
   with no lock. The first failure stops the loop: the earlier skills have already moved and been re-recorded, but the
   new folder is never saved. Nothing re-points a Deployment afterwards, so every link into the old folder dangles. No
   bytes are lost, but the Library is split and every harness loses its skills. One Unlocatable skill blocks the move
   forever.
3. **Clean Now runs outside the action lifecycle** and can delete a cache entry while an acquisition is reading it.

## Solution

- **Registry structure.**
  - Every registry path carries a typed root (home, roaming app data, local app data) and may have a per-OS override.
  - The project dir is optional.
  - A new **reader-only harness** kind: detected and listed as a reader of the `.agents/skills` folder, with no folder
    or switch of its own.
  - Every harness records which shared folders it reads, at each scope, plus precedence, duplicate behaviour and the
    other harnesses' dirs it also reads. Each fact cites its row in the brief.
- **The cross-read warning.** When two selected, installed harnesses would make one harness see a skill twice, the
  Harnesses pane row and the "Deploy your N skills now?" ask say so. Nothing is deselected.
- **Harnesses pane hygiene.** Undetected harnesses collapse into one closed group. Labels are fixed (Xum, "Devin
  (Windsurf)", QwenPaw, Zencoder / Zenflow). The AGENTS group's "(N harnesses)" count is derived. A test keeps the
  README table in step with the registry.
- **The wider roster.** Every row the briefs recommend lands, each re-verified against its source when added:
  - 28 own rows;
  - nine reader-only AGENTS members.
- **Move Library folder** becomes one action with a confirmation. It runs under the lifecycle and the Mutation guard:
  1. copy the Library to the new folder;
  2. switch every record in one transaction;
  3. re-point every owned linked Deployment;
  4. remove the old copies.

  A failure before the switch changes nothing. A crash after it leaves duplicates, never a missing skill. The report
  says exactly what moved, what was re-linked, what was skipped and what was left behind.
- **Clean Now** runs under the lifecycle. It skips a cache entry that is in use and says so.

## User Stories

### Harness catalog and registry

1. As the operator, I want every harness the research recommends to appear in Settings → Harnesses, so that I can deploy
   to the tools I and my clients actually use.
2. As the operator, I want the four harnesses CHANGELOG 1.3.1 promised (DeepSeek Harness, ZCode, Codewhale, WorkBuddy)
   to exist, so that the changelog stops over-promising.
3. As the operator, I want harnesses that only read `~/.agents/skills` (Zed, Warp, Dexto, fx, Sarvam Code, VT Code,
   DimCode, crow-cli, Corust Agent) listed as readers of the `.agents/skills` folder, so that I can see they're covered
   without a switch that does nothing different.
4. As the operator, I want a reader-only harness to show as detected or not, so that I know whether it's on this
   machine.
5. As the operator, I want a harness with no project skills dir (e.g. WorkBuddy, Hermes) to be absent from a project's
   harness choices, so that I can't configure a project Deployment nothing reads.
6. As a Windows user, I want a harness that keeps its skills under `%APPDATA%` (Reasonix) to deploy there, so that the
   harness finds them.
7. As a Windows user, I want a harness whose config root is under `%APPDATA%` to be detected there.
8. As the operator, I want Xum labelled "Xum", Windsurf "Devin (Windsurf)", Copaw "QwenPaw", and Zencoder to name
   Zenflow too, so that the names match the products I have installed.
9. As the operator with existing Deployments, I want those relabels to move nothing and keep every stored key, so that
   my targets, selections and project assignments survive the upgrade untouched.
10. As the operator, I want no harness detected into a folder it doesn't read, so that a "Deployed" dot never lies.
    QwenPaw's `.qwenpaw`, Antigravity CLI's and Devin's new detect dirs wait for their folder corrections in #108.
11. As the operator, I want the `.agents/skills (N harnesses)` label to count the harnesses that actually read it, so that
    it never drifts as rows are added.
12. As the operator, I want Deep Agents Code deployed as a copy, so that it loads the skill; it rejects symlinks that
    resolve outside its skill roots.

### Cross-read warning

13. As the operator turning on Claude Code while Cursor is on, I want the Harnesses pane to say "Cursor also reads Claude
    Code's folder — a skill deployed to both shows twice in Cursor", so that I can choose knowingly.
14. As the operator, I want that warning repeated as one line in the "Deploy your N skills to {harness} now?" ask, so
    that I see it at the moment I commit.
15. As the operator, I want the warning to name only installed, selected harnesses that read by default, so that a
    legacy-only, opt-in or conditional read (Kimi's legacy `kimi-cli`) never cries wolf.
16. As the operator, I want the app never to deselect a harness because of a warning, so that I stay in control:
    precedence is per harness and often unknown.
17. As the operator, I want a harness whose duplicates break invocation (fx: an ambiguous `$name` loads none) to say
    that consequence, not just "shows twice".

### Harnesses pane

18. As the operator with ~80 rows in the catalog, I want undetected harnesses collapsed under one closed "Not detected
    (N)" group by default, so that the pane shows what's on my machine first.
19. As the operator, I want to open that group and turn on a harness before installing it, as today.
20. As the operator, I want the collapse state to reset each time Settings opens, so that the default stays predictable.

### Library folder move

21. As the operator, I want Browse… to ask "Move N skills to X? Every harness link is re-pointed" before anything
    happens, so that a folder pick is never an accidental move.
22. As the operator, I want the move to refuse a folder that isn't empty, with "choose an empty folder or create one",
    so that my Documents never become my Library.
23. As the operator, I want the move refused (with a reason) when the destination is inside the current Library or
    contains it, is inside any harness or project skills dir or the git cache, is relative, or isn't writable.
24. As the operator, I want a warning (not a refusal) when the destination is in a cloud-synced folder (iCloud Drive,
    `~/Library/CloudStorage`, OneDrive), so that I know online-only eviction can break harness links.
25. As the operator, I want progress while the move runs, and every other mutation control refused with the reason while
    it does.
26. As the operator, I want a failed copy to leave my Library exactly as it was, so that a failure costs nothing.
27. As the operator, I want every owned linked Deployment, global and project, re-pointed into the new folder in the same
    action, so that every harness keeps working.
28. As the operator, I want copy-mode Deployments left alone, so that nothing is rewritten needlessly.
29. As the operator, I want an owned link that no longer points at the old copy (I changed it) reported, not touched.
30. As the operator, I want a project whose folder is gone reported as `folder_missing` and re-linked by the next resync
    once it's back.
31. As the operator with an Unlocatable skill, I want the move to skip it and say "N not moved, missing" instead of
    refusing the whole move; the skill stays truthfully Unlocatable.
32. As the operator, I want only recorded skill folders removed from the old Library, through the removal fence, and
    the old folder plus anything unrecorded left in place and named in the report.
33. As the operator, I want `.explore-cache` and leftover staging dirs not copied, so that scratch doesn't travel.
34. As the operator after the move, I want My Skills to show the moved catalog with no stale paths, from the command's
    own response.
35. As the operator, I want a crash mid-move to leave at most duplicates, never a missing skill, with a rerun or the
    report making it whole.

### Clean Now

36. As the operator, I want Clean Now refused with a reason while another action runs, like every other mutation.
37. As the operator, I want Clean Now to skip a cached repo an acquisition is using and report "Cleared N cached repos; M
    in use, skipped", so that a running install never loses its checkout.
38. As the operator, I want the startup age-out to follow the same in-use rule.

### Release

39. As the operator, I want 1.3.4's CHANGELOG to name every new harness, every relabel (moving nothing), the Library
    move's new behaviour and the cloud-folder warning, so that the smoke list writes itself.

## Implementation Decisions

### Registry structure (the Harness catalog module)

- **Typed roots (Q2).**
  - Every home-relative registry path becomes a `{ root, rel }` pair, with root one of home, roaming app data or local
    app data.
  - A path may carry a per-OS override, used only where a vendor documents one.
  - `environment.rs` resolves a `HostRoots { home, roaming, local }` once. Core takes it explicitly, like `home` today,
    and the command seam builds it.
  - On non-Windows the app-data roots resolve to the platform's equivalent or are unused. A row's override is the only
    way to target them.
  - The rule "core never resolves roots from the environment" is unchanged.
  - The first production consumer is Reasonix (Windows `%APPDATA%\reasonix`, ticket 06). Until then unit tests cover it
    (Q16).
- **Env-var homes are not honoured (Q3).** `GROK_HOME`, `DSH_HOME`, `KIMI_CODE_HOME`, `REASONIX_HOME`,
  `DEEPAGENTS_HOME`, `AUTOHAND_HOME` and `AIDER_DESK_HOME_DIR` are named in their row comments and in a README note. A
  Finder-launched app doesn't see shell env anyway. #56's custom rows are the future override.
- **Optional project dir (Q4).**
  - The project skills dir becomes optional.
  - `None` means the harness is absent from `project_tool_entries`, and the README shows "—".
  - Project sync and removal keep reading only the registry's mapping.
- **Reader-only harness (Q28): a new row kind.**
  - It has a detect dir and no global skills dir of its own.
  - It's never a selectable global target, never in the selection or target tables, and mints no rows.
  - At project scope it's the existing AGENTS group absorb.
  - `global_tool_entries` exposes it as a reader of the `.agents/skills` entry. The Harnesses pane lists it under that
    entry ("read by Zed, Warp") with its detected state and no switch.
  - New CONTEXT.md term: **Reader-only harness**.
  - This is #109's "a Deployment is a folder; harnesses are its readers" in its first, minimal form.
- **Shared-folder read facts (Q5).**
  - Per harness and per scope (global, project), each of `~/.agents/skills` and `~/.claude/skills` is one of
    `Yes | Conditional(reason) | OptIn | No | Unknown`.
  - Beside them sit:
    - precedence: `Own | Shared | Both | Unknown`;
    - duplicate behaviour: `ShowsAll | Shadows | LoadsNone | FlagsConflict | Unknown`;
    - `also_reads`: other dirs read by default, as root-relative paths. Legacy, opt-in and conditional cross-reads are
      marked and never warn.
  - LIKELY and UNKNOWN evidence is encoded as `Unknown`, with the brief row named in a comment.
  - Facts are registry literals and backend-only in Wave A, except the resolved warning pairs (below).
- **Facts vs `group` (Q17).** The `group` literal stays the deployment decision. A test forbids an AGENTS-group member
  whose project `.agents` fact is `No`.
- **The brief is the record (Q20).** Each fact cites its brief or addendum row. Changing a fact means editing that row
  and its source in the same commit. The briefs have no Quartermaster source.
- **Derived counts (Q18).**
  - The AGENTS group label's count is computed from the registry: constituents plus reader-only members, via an i18n
    string with a count.
  - A Rust test counts the README supported-tools table rows against the registry.
  - AGENTS.md's "New AI tool adapter" invariant is updated: the README table becomes test-enforced, and reader-only rows
    are named.
- **Relabels only (Q7, Q16).**
  - The changes:
    - Mux → "Xum";
    - Windsurf → "Devin (Windsurf)";
    - Copaw → "QwenPaw";
    - Zencoder's label names Zenflow too (it's an alias, not a row).
  - Every `as_key` is unchanged (round 19's `mux` decision). The relabels go in i18n `tools.<key>` as well.
  - These detect dirs are **deferred to #108**, each landing with its folder correction: QwenPaw `.qwenpaw`, Antigravity
    CLI `.gemini/antigravity-cli`, Devin `.config/devin`. Adding one now would detect a harness into a folder it doesn't
    read.
- **Deep Agents Code (Q29, brief I8).** `supports_symlink: false`, so it's a copy. #84 later makes the choice the
  operator's.
- **Unverifiable detect dirs ship (Q29).** Code Studio `.codestudio`, CodeArts `.codeartsdoer`, MiniMax `.minimax`, and
  Warp outside macOS ship with the brief's flag in the row comment. `is_installed_in`'s skills-only-footprint rule makes
  the worst case "not detected".
- **Detect-dir hazards from brief I4 are binding.**
  - Cortex Code is detected on `.snowflake/cortex`, never `.snowflake`.
  - Posit on `.posit/assistant` + `.positai`, never `.posit`.
  - ForgeCode on `.forge` + legacy `forge`.
  - Codewhale is never detected on `.deepseek`.
  - AstrBot is detected only through `.astrbot` (packaged desktop runtime).

### Cross-read warning (Q6, Q19)

- The backend resolves, per installed harness entry on `ToolInfoDto`, the installed harnesses that would see a duplicate
  if both were deployed (`duplicate_readers`, named by key), plus the duplicate behaviour that makes it matter. The
  frontend never re-derives it from dirs, the same rule as `shared_with`.
- Rule: harness R reads, **by default** (`Yes`, not legacy, opt-in or conditional), the deploy dir of a second selected,
  installed harness, *and* R's own deploy dir. Shared skills dir siblings are the existing dedupe, not a warning.
- Surfaces:
  - one line on the Harnesses pane row of each affected harness;
  - one line in `DeployHarnessAsk` when the harness being turned on completes such a pair.
  - No auto-deselect.
  - Copy comes from the duplicate behaviour: `LoadsNone` says invocation breaks, `FlagsConflict` says the harness flags a
    conflict, the rest say "shows twice".
- Copy goes through `en` in i18n, and every new string through the copy pass.

### Harnesses pane (#92's UI share)

- Undetected harnesses (reader-only included) collapse into one closed "Not detected (N)" group. The state is not
  persisted (Q8) and resets when Settings opens. Detected harnesses keep today's order.
- The collapsible uses existing primitives. A new variant, if needed, is documented in DESIGN.md in the same change.

### Library folder move (#91, Q9–Q13, Q21–Q24, Q30)

- **A dedicated command (Q13).**
  - `move_library(dest)` answers `{ report, skills }`. The skills world replaces its catalog from it.
  - `CentralRepoPath` leaves the generic `update_setting`. The setting is written only by the move's switch step.
  - The frontend runs it under `runAction` with streamed progress (a `Channel`) and no cancel.
  - Every other mutation control is refused with the reason while it runs. Browse… keeps the folder pick outside the
    gate (the AGENTS.md rule for picks) and opens the confirmation; the mutation after it is gated.
- **Admission, before anything is written (Q12, Q23).** Each refusal is a typed `CommandError` variant, with a binding,
  a `describeCommandError` branch and an i18n key. The move is refused when the destination:
  - is relative;
  - is inside or contains the current Library;
  - is inside any harness global or project skills dir, or inside the git cache;
  - is not empty;
  - is not writable.
- **Cloud-synced destination (Q30).** The confirmation warns when the destination is under a known cloud-sync root
  (`~/Library/Mobile Documents`, `~/Library/CloudStorage/*`, OneDrive). It warns and doesn't refuse. The backend answers
  the classification, so the frontend reads no paths itself.
- **Four phases (Q9),** one `mutation_guard::serialized` section:
  1. **Copy.** Copy every recorded, locatable skill into the destination and verify each by Content identity. Failure →
     remove what was staged, no record changes, typed failure. There's no rename fast path: a rename breaks links
     immediately, and Libraries are small.
  2. **Switch.** One DB transaction flips every moved skill's `central_path` and the setting.
  3. **Re-point.** For every *owned* (`synced_at` set) global target row and project assignment in link mode (symlink or
     junction) whose link resolves to the skill's old copy, re-make it into the new copy through `sync_engine`'s
     overwrite path (Q10).
     - Copy-mode Deployments are untouched.
     - An owned link that no longer points at the old copy is reported and untouched.
     - A project whose root is gone is reported `folder_missing` and its row left as is (Q24).
     - Per-target failures are report data.
     - Disabled rows are skipped as always.
  4. **Clean.** Remove each moved skill's old copy through `artifact_removal` and the removal fence, under a claim on
     that recorded path (Q11).
     - Never remove the old root or anything unrecorded; the report names what's left (Q22).
     - `.explore-cache` and `.skills-hub-staging-*` are neither copied nor reported.
- **Unlocatable skills (Q21)** are skipped and reported. Their record keeps its old path and stays Unlocatable.
- **Crash-safety.** A crash before the switch leaves orphan copies in the destination; a crash after it leaves old
  copies behind. Never a missing skill (#108's rule). A crash between Switch and Re-point leaves links into the old
  copies, which still exist. A rerun of the move is admitted only to an empty destination, so recovery is: the
  re-point and clean steps are idempotent, and the report and Skill health say which links still point at the old root.
- **Report shape.** Per skill: `moved | skipped { missing }`. Per target: `relinked | unchanged { copy } |
  skipped { not_ours | folder_missing | disabled } | failed { error }`. Plus `left_behind` paths. The frontend fold
  derives the counts once, in `reportOutcome`.

### Clean Now (Q14, Q25)

- The frontend runs it under `runAction`.
- The backend takes each entry's `git_cache` per-key lock with a **try-lock** before removing it. A held lock means "in
  use": skipped and counted. It doesn't take the Mutation guard, because acquisitions run outside it.
- The startup age-out in `cache_cleanup` follows the same try-lock rule.
- It answers `{ cleared, in_use }`, so the toast reads "Cleared N cached repos; M in use, skipped".

### Delivery (Q26, Q27)

- Lanes run in parallel worktrees:
  - **A** — tickets 01 → 02 (registry);
  - **B** — 03 → 04 (Library folder, Clean Now);
  - **C** — 05 → 06 → 07 (rows), after 02 merges.
- Opus 5.5 (`high`) implements. GPT-6 Astra runs the adversarial review per lane.
- The orchestrator writes CHANGELOG, BACKLOG and the version at merge.
- **Version 1.3.4** (operator 2026-11-06).

## Testing Decisions

- **Good tests pin behaviour through the highest existing seam** and never restate a private helper.
- **Seams (existing, highest first):**
  - Rust core integration tests under `core/tests/` with temp roots (`InstallerPaths`, the new `HostRoots`, temp
    central dir, temp app cache dir; Tool and Project dirs outside the central dir per `removal_fence::central_repo_of`);
  - the registry table tests in `core/tests/tool_adapters.rs`;
  - hook tests (`renderHook`, `src/lib/tauri.ts` mocked) for the settings world and the door;
  - pure-fold tests for `reportOutcome`;
  - the fixture backend for driving the UI.
- **Registry.**
  - Extend `project_relative_skills_dir_for_every_tool` (now optional).
  - A per-row **brief-snapshot table** restates each row's read facts, precedence, duplicate behaviour and `also_reads`
    with its brief row id.
  - The group/fact consistency test (Q17).
  - The README-row-count test (Q18).
  - Typed-root resolution on each OS root with a substituted `HostRoots`.
  - Reader-only rows never appear as selectable targets and never mint rows.
  - Relabels keep `as_key`; an existing stored key round-trips.
  - Deferred detect dirs are absent, pinned so #108 removes the pin deliberately.
- **Warning.** `duplicate_readers` resolution: default reads warn; legacy, opt-in and conditional don't (Kimi);
  uninstalled or unselected harnesses don't; shared-dir siblings don't.
- **Library move.**
  - Admission refusals, one test per typed variant.
  - A copy failure leaves records and the setting unchanged and the destination clean.
  - After a successful move, every owned link resolves into the new root, copy Deployments are unchanged, a not-ours
    link is untouched and reported, a missing project is `folder_missing`, and an Unlocatable skill is skipped.
  - Old copies are removed only through the fence; unrecorded content remains.
  - The move is serialized: a concurrent mutation is refused or queued by the guard, per its contract.
  - The prior art is the `central_repo` tests, `artifact_removal`'s fence tests and `project_sync`'s `folder_missing`
    tests.
- **Clean Now.** A held key lock makes the entry skipped and counted, never removed; the startup age-out obeys it too.
  The prior art is the `git_cache` lock tests and the `cache_cleanup` tests.
- **Frontend.**
  - `useSettingsState`: Browse… opens the confirmation and runs nothing on cancel. Move and Clean Now run under
    `runAction` and are refused while busy.
  - `reportOutcome` folds the move report and the Clean Now counts.
  - i18n key coverage via `resources.test.ts`.
  - No component render tests (AGENTS.md); primitive changes keep their `ui/*.test.tsx` contract.
- **Fixture.**
  - A `registry` scenario: detected and undetected harnesses with the group collapsed, at least one warning pair, a
    reader-only member, and a row without a project dir.
  - A Library-move flow in the fixture backend (success, plus one refusal).
- **Gate.** `npm run version:check && npm run check` per lane. Windows-only Rust (the typed roots' `%APPDATA%` arms)
  is proven by `gh pr checks` on a draft PR, the only Windows compile available from this Mac.

## Out of Scope

- Moving any Deployment between folders, folder corrections, retiring Clawdbot and MoltBot, and the deferred detect dirs
  (all #108).
- The shared-folder preference and its UI (#109), link vs copy as an operator choice (#84), and project-scope relocation
  (#87).
- Honouring env-var homes (Q3), and custom harness rows (#56).
- Surfacing read facts in the UI beyond the warning and the collapsed group (#109).
- Making the Library move cancellable (Q13), and a "reset to default location" control.
- ZeroClaw and nanobot (per-workspace skills dirs; their own look), and every row the briefs skip.
- Global Deployment presence checks (#94).

## Further Notes

- **Re-verify before adding.** Each #110 row is re-checked against its brief or addendum source when it lands. A row
  whose source no longer holds is reported back with evidence and not forced (`docs/agents/issue-tracker.md`).
- **A worktree child has no `.pi/` skills** (they're gitignored). Brief UI-copy work accordingly; the orchestrator runs the
  copy pass with `impeccable` at merge.
- **Removal-safety reviews converge slowly** (1.3.3 lesson). Ticket 03's Clean phase touches the removal fence, so budget
  more than one adversarial round for it.
- Wave B's grill starts from this spec's registry facts as merged: #108 → #109 + #84 (study T3 Code's project
  defaults/overrides) → #70, #85/#86, #93, #87.

## Status

- 2026-11-06: lanes A (01) and B (03) dispatched (Opus 5.5, high).
- 01 merged as `8401dfa` (PR #13). Astra review: fix-then-ship, one P2 fixed by the orchestrator
  (`reviews/round20-t01-registry-review.md`).
- 03 is blocked in round-1 review (3 P1, 2 P2; `reviews/round20-t03-move-library-review.md`). The round-2 fix is
  dispatched.
- 02 dispatched after 01's merge.
- 02 merged `efaf49a` (PR #14); Astra fix-then-ship, 3 findings fixed by the orchestrator. 05 dispatched.
- 05 merged as `9c66a8e` (PR #17) and 07 as `d954846` (PR #15); they ran in parallel, and the orchestrator merged them
  keeping both appended blocks. 06 is still running. The combined row review follows. 03 is in round 4 (both round-3
  reviews were fix-then-ship).
- 06 merged as `01e4ec3` (PR #16). Posit's `.positai` detect dir was dropped (Q7). All 33 new harness rows are on `main`; the combined row review is dispatched.
- 03 merged as `26ce2b6` (PR #12) after five review rounds. 04 dispatched.
- 04 merged as `6a8c235` (PR #19). All of Wave A is on `main`. Final release review dispatched.
- Final review blocked (OpenAI usage limit); release held. The CHANGELOG is committed. Handoff `handoffs/2026-11-06-1.3.4-ready-awaiting-final-review.md`.
- **Wave A released as v1.3.4** (`ed8e442`, 2026-11-06). Next: the Wave B grill in this effort.
- 2026-11-06: Wave B grilled (below, Q1–Q29); prototype and T3 Code study dispatched.

# Wave B — redeploy, shared folders, link vs copy (grilled 2026-11-06)

Status: needs-triage (spec body written 2026-11-06; tickets await the operator's approval of the breakdown)

Ships as **1.4.0** (operator 2026-11-06, Q1 + Q29). Absorbs BACKLOG **#108** (with **#87** as its project leg), **#109**,
**#84**, **#116** and **#70**. #85/#86 and #93 go to the release after (Q23). Grilled against the registry facts as built
in Wave A. Every recommendation was accepted except where noted.

## Grill log (2026-11-06)

Facts found first (code and a read-only copy of the operator's DB):

- None of #108's corrections is on `main`. `core/target_relocation.rs` is unwired (`lib.rs:201`); only Augment has a
  `former_skills_dirs` entry.
- The operator's global Deployments are Claude Code 33, Codex 33, Pi 33 and `.agents/skills` 9 (all nine also on Codex), all
  symlinks; 4 projects. Only Codex's correction touches this machine. Pi reads `~/.agents/skills` as well as its own
  folder, so moving Codex there without #109 would show about 33 duplicates in Pi. The nine `.agents` skills already
  show twice in Pi today.

### Redeploy (#108)

- **Q1 — One release.** The operator rejected splitting #108 from #109/#84. Everything ships together, which also
  removes the hazard above: Codex moves into `.agents` together with the rule that serves Pi from it alone.
- **Q2 — One mechanism.** It compares each Deployment's place as it should be (path and link/copy, worked out from the
  registry and settings) with its place as recorded. It acts on every difference. Corrections, retired keys, the
  shared-folder preference and the link/copy choice are its inputs; no pass is written for one cause.
  - `core/redeploy.rs` replaces `target_relocation.rs`, borrowing `library_move`'s physical identity and verification.
  - `former_skills_dirs` keeps only the deletion rule and the onboarding scan.
  - An old artifact is removed only when no remaining live row records it, which handles the Kimi/Amp
    `.config/agents/skills` hazard.
- **Q3 — Always an operator action.** A setting change redeploys inside the action that changes it.
  - Registry corrections: a read-only check at launch finds the work and offers one dialog ("N Deployments are in
    folders {harnesses} no longer read: move them now?"). It then runs under `runAction` with progress, the one
    overwrite ask and a report.
  - Declined: Settings → Harnesses keeps a "Move N Deployments" line.
- **Q4 — Deploy first.** For each Deployment:
  1. Deploy to the new place. Ours or identical content there is adopted; a different folder gets the overwrite ask
     (D12), and Keep leaves that Deployment unmoved.
  2. Re-record the row.
  3. Remove the old artifact only when provably ours: an owned link resolving to its central copy (physical identity),
     or a copy identical to the Library, with no other live row recording it.
  - A copy with local edits gets one question: delete your edited copy, or leave it (it becomes an unmanaged folder;
    the report names the harnesses that will show it twice). Leaving it never blocks the move.
- **Q5 — Journal.** The row switch and a "pending removal of X for row R" journal entry (with physical identity) are one
  transaction. The removal clears it, and every redeploy run (including the launch check) finishes it. Closes audit
  F18's class.
- **Q6 — Retirements.** The Clawdbot and MoltBot `ToolId`s are deleted; their dirs become OpenClaw's former dirs.
  Stored rows, the saved selection and project assignments under those keys are re-keyed to `openclaw` (merging into an
  existing row) and redeployed. Unknown keys stay tolerated.
- **Q7 — Project leg (#87 absorbed).** Projects run in the same action, under the same rules: `folder_missing` for a
  gone project folder, and one report spanning both scopes.
- **Q8 — Report.**
  - Per Deployment: `moved | adopted | unchanged | left_old { edited } | kept { declined } | skipped { folder_missing,
    disabled } | failed { error }`.
  - An old copy that could not be removed is reported as the duplicate it leaves, naming its readers from the read
    facts.
  - `reportOutcome` folds it and surfaces pending/incomplete moves; today the fold skips every non-failed status.
- **Q9 — Disabled and never-synced rows** are re-recorded at the new place with nothing written, so Enable deploys to the
  right folder.
- **Q10 — Onboarding scans former dirs** as their harness's, excluding our own links by physical identity.
- **Q11 — No research re-check.** The 2026-11-02 and 2026-11-05 briefs count as current.
- **Q12 — Correction table:**

  | Harness | Global (old → new) | Project (old → new) | Detect |
  |---|---|---|---|
  | Codex | `.codex/skills` → `.agents/skills` | unchanged (`.agents/skills`) | — |
  | Goose | `.config/goose/skills` → `.agents/skills` | `.goose/skills` → `.agents/skills` (joins the group) | — |
  | OpenHands | `.openhands/skills` → `.agents/skills` | `.openhands/skills` → `.agents/skills` (group) | — |
  | Zencoder / Zenflow | `.zencoder/skills` → `.agents/skills` | `.zencoder/skills` → `.agents/skills` (group) | — |
  | Antigravity | `.gemini/antigravity/global_skills` → `.gemini/config/skills` | unchanged | + `.gemini/antigravity-cli` |
  | Kimi Code | `.config/agents/skills` → `.kimi-code/skills` | unchanged | — |
  | Kilo | `.kilocode/skills` → `.kilo/skills` | `.kilocode/skills` → `.kilo/skills` | + `.kilo` |
  | Xum (key `mux`) | `.mux/skills` → `.xum/skills` | `.mux/skills` → `.xum/skills` | + `.xum` |
  | Devin (Windsurf) | `.codeium/windsurf/skills` → `.config/devin/skills` (Windows `%APPDATA%\devin\skills`) | `.windsurf/skills` → `.devin/skills` | + `.config/devin` |

  Devin caveat: whether Devin's CLI reads `.config/devin/skills` is UNKNOWN; with the preference on (default) Devin is
  served by `.agents` anyway. Sources: `docs/design-inputs/2026-11-02-harness-global-skills-dirs.md` § 5 and I4–I9;
  `docs/design-inputs/2026-11-05-harness-candidates-and-shared-dir-reads.md` § Part 2, I5–I6.
- **Q13 — QwenPaw deferred** (BACKLOG #118). The row keeps `.copaw/skill_pool` and its label.

### Shared folders (#109, #116)

- **Q14 — Folder cards** (reopened at the operator's prompt). A harness sees every folder it reads whatever is
  selected, so a per-harness switch cannot promise "keep it away from this harness". The Harnesses pane shows **one
  card per folder, one switch per thing written**:
  - Harness names come first. An own-folder card looks like today's row.
  - A shared card names every installed harness it serves and every other installed reader ("comes along for the
    ride"), before anything is flipped.
  - Reader-only harnesses appear in the shared card, with no switch.
  - "Codex yes, Pi no" is not expressible, because no design can make it true.
  - Chosen over A (per-harness switches with explanation lines) and B (a flat folder list). Confirm A vs C on screen
    with a fixture prototype first (Q27).
- **Q25 — One model on every surface.** The Add dialog's checkboxes, the inspector's Targets section, `DeployHarnessAsk`
  and the project tools modal all use the folder cards. No bare harness switch hides a shared folder.
- **Q15 — Which folder serves a harness, preference on.**
  - Reads `.agents` by default and not `.claude` → `.agents`.
  - Reads `.claude` and not `.agents` → `.claude` (Claude Code, Cortex Code, Neovate).
  - Reads both → the dropdown.
  - `Conditional | OptIn | No | Unknown` → its own folder.
  - Preference off → its own folder (`.agents` for Codex, Goose, OpenHands and Zencoder, per Q12).
- **Q16 — The dropdown is one global setting** ("When a harness reads both shared folders, use: `.agents` / `.claude`",
  default `.agents`). There is no per-harness dropdown.
- **Q17 — Default on for every install.** Settings define where each Deployment belongs. The first 1.4.0 launch's offer
  (Q3) covers the corrections and the preference together. Declining never turns the preference off; the "Move N" line
  stays.
- **Q18 — A Deployment is a folder; harnesses are its readers.**
  - My Skills rows show one dot per folder.
  - The inspector groups detected harnesses under the folder serving them.
  - Unavoidable duplicates are marked where they hurt (fx `LoadsNone`, Cortex Code `FlagsConflict`).
- **Q19 — Disable on a shared folder is per folder.** Its copy names every reader. One reader cannot be excluded while
  another selected harness needs the folder, and the UI says so.
- **Q24 — Existing per-harness disables on a newly shared folder.** The folder keeps the skill when any harness it
  serves had it live, and the report names each disable that no longer holds. If every harness it serves had it
  disabled, the folder records it disabled.
- **Q26 — Selection migration.** A shared card is on when any harness it serves was selected. An own-folder card keeps
  its harness's state.
- **Q20 — #116.** The warning counts installed readers, selected or not, with one line per affected harness naming the
  folders it reads twice. Harnesses that read many folders (Firebender, Warp, Kimchi, fx) also get one line.

### Link vs copy (#84)

- **Q21 — Levels, most specific wins:** folder › project › scope (global / all projects) › default (link).
  - A harness's own folder is the per-harness level. A shared folder has one mode, set on its card.
  - A copy-only harness (registry `supports_symlink: false`: Deep Agents Code) that reads `.agents` is served by its own
    folder (copy) unless `.agents` is set to copy. A group's capability is still never the AND of its readers.
  - Changing a level redeploys through Q2–Q5.
- **Q22 — Projects inherit and override.** Each project inherits the preference, the dropdown and the link/copy mode,
  and may override any of them. The project matrix's columns become the folders serving that project's harnesses. The
  defaults/overrides UI is designed after the T3 Code study.

### Overwrite ask (#70)

- **Q23 — #70 ships in 1.4.0.** The redeploy leans on the ask. It gets:
  - who put the folder there;
  - a content or diff view, and Reveal in Finder per row;
  - each button's consequence;
  - overwritten rows recorded in the run report;
  - an ask after Cancel saying the skill was already being added.

### Process

- **Q27 — Order:**
  1. A vs C prototype in the fixture (`registry` scenario; throwaway; the operator picks on screen).
  2. T3 Code study (project defaults and per-project overrides) as a dated note in `docs/design-inputs/`.
  3. This part's spec body.
  4. Tickets from `09`.
- **Q28 — Routing.**
  - Opus 5.5 `high` implements.
  - GPT-6 Astra reviews every lane adversarially.
  - The redeploy and removal lanes also get GPT-6.1 Sol in parallel. Reviewers get the principle: delete only what is
    provably a duplicate, at the moment of deletion, only inside the folder you own.
  - The prototype child runs on Opus 5.5 `medium`.
- **Q29 — Version 1.4.0.**

Carried into the spec body (from #108/#109's BACKLOG text, not re-grilled):
- audit 3's F14–F16 as acceptance tests (aliased roots, a destination aliased onto the central repo, Keep over a copy
  with local-only edits);
- the release-smoke line (a real machine's first-launch move checked with `ls -la` over old and new folders);
- #88's residue: preview copy ("would move N"), the shadowed-own-copy flag, Import near-misses under the preference,
  the layout read once per operation, and onboarding's reader roster for shared or aliased dirs.

### After the prototype (2026-11-06)

The A/B/C prototype is on the throwaway branch `proto/waveb-harness-cards` (`7e0380f`); its run notes and trade-offs are
in `.scratch/round20/prototype-harnesses.md` on that branch. The T3 Code study is
`docs/design-inputs/2026-11-06-t3code-project-defaults-and-overrides.md` (`6855499`).

- **Q30 — Turning the preference off asks which harnesses get their own folder.** Its confirmation lists every
  installed harness that is currently served by an on folder, each one ticked. You untick the ones you don't want, and
  the own-folder cards switch on to match. No per-harness record is kept underneath the cards. The prototype showed
  why this is needed: C remembers folders, not harnesses, so on the operator's machine an automatic rule wrote 6
  folders instead of 3.
- **Q31 — Calmer cards.**
  - A card's title is its folder ("Shared folder — `~/.agents/skills`", "Claude Code — `~/.claude/skills`"), with
    "Serves …" underneath, so a title never changes when harnesses regroup.
  - "Also seen by" lists names only. Every duplicate consequence lives in the pane's single "Seen twice (N)" section.
  - A card turns amber only for a harness whose invocation breaks (fx `LoadsNone`).

## Wave B spec (2026-11-06; prototype verdict: C)

The operator chose **C, folder cards** on screen (2026-11-06). Everything below follows from the grill log (Q1–Q31) and
uses the vocabulary of `CONTEXT.md`. New terms are defined at the end of § Implementation decisions.

### Problem Statement

The operator deploys one skill Library to several harnesses, and the app can't show where a skill really lands or
who sees it.

1. **Several harnesses are deployed to folders they no longer read, or read only as legacy:**
   - Antigravity, Kimi Code and Kilo point at folders their current releases don't document.
   - Codex, Goose, OpenHands, Zencoder, Xum and Devin are deployed to deprecated or legacy folders.
   - Clawdbot and MoltBot are former names of OpenClaw.

   Fixing a folder would orphan every existing Deployment there, and the only mechanism that moved Deployments (the
   1.3.2 launch pass) was disabled because it could delete Library bytes.
2. **One skill shows up two, three or four times in one harness.** A harness reads the shared folders
   (`~/.agents/skills`, `~/.claude/skills`) as well as its own. The operator's Pi already lists nine skills twice,
   and Cursor can list one skill four times. The app deploys per harness and can't express "one folder serves all
   these harnesses".
3. **A harness switch can't say what it means.** Turning a harness off doesn't stop it reading a shared folder that
   another harness's skills are in.
4. **Link vs copy belongs to the registry alone.** The operator can't choose copy for a harness that misbehaves with
   links, or per project.
5. **The overwrite ask can't be answered knowingly.** It doesn't say who put the folder there, shows no diff, and
   leaves no record of what was overwritten. The redeploy will raise it more often.

### Solution

- **Redeploy.** One mechanism moves a Deployment whenever where or how it should live changes:
  - It deploys to the new place first, then removes the old copy only when it is provably ours.
  - It is journaled, so a crash is always finished by the next run.
  - It is always an operator action with a confirmation and a report.
  - Registry corrections are found at launch and offered.
- **Deployment folders.** A Deployment is a folder; harnesses are its readers.
  - With the shared-folder preference on (the default), each harness is served by exactly one folder: `.agents`,
    `.claude` or its own.
  - The Harnesses pane, the Add dialog, the inspector, the deploy ask and the project tools modal all show
    **folder cards**: one switch per folder written, harness names first, and every installed harness that sees the
    folder listed before anything is flipped.
- **Link or copy** is the operator's choice at four levels: folder › project › scope › default. Projects inherit the
  global settings and can override them, in the T3 Code pattern.
- **The overwrite ask** names who put the folder there, shows its contents or a diff, states each button's
  consequence, and records overwritten rows in the run.

### User Stories

#### Redeploy and registry corrections

1. As the operator upgrading to 1.4.0, I want one offer at launch ("N Deployments are in folders {harnesses} no longer
   read: move them now?"), so that corrected folders take effect without my hunting for them.
2. As the operator, I want that offer to run as a normal action with progress and a report, so that I see exactly
   what moved.
3. As the operator who declines the offer, I want Settings → Harnesses to keep a "Move N Deployments" line, so that I
   can finish it later and nothing is forgotten.
4. As the operator, I want every Deployment deployed to its new folder before the old one is touched, so that a
   failure or crash leaves at most a duplicate, never a missing skill.
5. As the operator, I want an old linked Deployment removed only when it is provably ours (a link into its own
   Library copy), so that the app never deletes something it didn't make.
6. As the operator, I want an old copied Deployment removed only when it is identical to the Library, so that local
   edits are never deleted silently.
7. As the operator with a locally edited copy, I want to be asked "delete your edited copy at {path}" or "leave it",
   and told which harnesses will show it twice if I leave it, so that I decide knowingly.
8. As the operator, I want leaving an edited copy never to block the move, so that one answer doesn't strand the rest.
9. As the operator, I want a folder already at the new place to be adopted when it is ours or identical, and the
   overwrite ask raised when it differs, so that nothing foreign is overwritten unasked.
10. As the operator answering Keep for a folder in the way, I want that Deployment to stay where it was, still working,
    so that Keep never loses a skill.
11. As the operator, I want an old copy that couldn't be removed reported as the duplicate it leaves ("shows twice in
    Cursor"), so that I know the consequence.
12. As the operator, I want a crash mid-move finished by the next run or launch, so that an old copy is never forgotten.
13. As the operator with a project, I want project Deployments in corrected folders moved in the same action, so that
    projects don't lag behind global.
14. As the operator with a project whose folder is gone, I want it reported `folder_missing` and left as recorded, so
    that it moves once the folder is back.
15. As the operator with a Disabled skill or pair, I want its record pointed at the new folder without anything
    written, so that Enable later deploys to the right place.
16. As the operator who once used Clawdbot or MoltBot, I want those Deployments, my selection and project assignments
    carried into OpenClaw, so that the rename costs nothing.
17. As the operator, I want onboarding's Import to scan former folders (e.g. `~/.codex/skills`) as their harness's, so
    that skills I put there myself can still be imported.
18. As the operator, I want Antigravity, Kimi Code, Kilo, Xum, Devin, Codex, Goose, OpenHands and Zencoder deployed
    where their current releases read, so that every Deployed dot is true.
19. As a Windows user of Devin, I want its folder under `%APPDATA%\devin\skills`, so that Devin finds my skills.

#### Shared-folder preference and folder cards

20. As the operator, I want a Settings switch "Use shared folders when a harness can read them" (on by default), so
    that each harness gets each skill from exactly one folder.
21. As the operator, I want a dropdown "When a harness reads both shared folders, use .agents / .claude", so that
    harnesses that read both get the folder I prefer.
22. As the operator, I want a harness whose shared reads are conditional, opt-in, absent or unknown to keep its own
    folder, so that the app never relies on a read it can't vouch for.
23. As the operator, I want the Harnesses pane to show one card per folder skills are written to, with one switch per
    card, so that every switch means exactly what happens on disk.
24. As the operator, I want each card titled by its folder ("Shared folder — `~/.agents/skills`", "Claude Code —
    `~/.claude/skills`") with "Serves Codex, Pi, Gemini CLI" underneath, so that titles stay put when harnesses
    regroup.
25. As the operator, I want each card to list the other installed harnesses that also see it, so that I know who
    comes along for the ride before I flip anything.
26. As the operator, I want reader-only harnesses (Zed, Warp, …) shown inside the shared card that serves them, with no
    switch, so that I see they are covered.
27. As the operator, I want undetected harnesses collapsed inside each card, and own-folder cards for undetected
    harnesses under one closed "Not detected (N)" group, so that the pane leads with what is on my machine.
28. As the operator, I want every duplicate consequence gathered in one "Seen twice (N)" section, so that the cards
    stay calm and the warnings stay findable.
29. As the operator, I want a card shown in amber only when it holds a harness whose invocation breaks on duplicates
    (fx), so that amber means something.
30. As the operator, I want the warning to count installed harnesses that read a folder twice whether or not they are
    selected, so that Cursor's duplicates show even though I never selected Cursor.
31. As the operator turning the preference off, I want to be asked which harnesses get their own folder, with every
    harness currently getting skills ticked, so that no harness silently loses or gains folders.
32. As the operator changing the preference, the dropdown or a card switch, I want the confirmation to say how many
    Deployments move ("would move N"), and the change to run as one action with a report, so that a settings click
    never moves files unannounced.
33. As the operator, I want the Add dialog's "Deploy after adding" picker to show the same folder cards, so that
    choosing where a new skill goes means the same thing everywhere.
34. As the operator, I want the inspector's Targets section to group detected harnesses under the folder serving them,
    so that a skill's Deployments read as folders.
35. As the operator, I want My Skills rows to show one dot per folder, so that the row matches what is on disk.
36. As the operator, I want the "Deploy your N skills to … now?" ask to name the folder and the harnesses it serves,
    so that I commit knowingly.
37. As the operator disabling a skill on a shared folder, I want the confirmation to name every harness that loses it,
    and to say that one reader can't be excluded while another needs the folder, so that Disable never pretends.
38. As the operator upgrading with per-harness disables on a folder that becomes shared, I want the folder to keep the
    skill when any harness it serves had it on, and the report to name each disable that no longer holds, so that no
    harness loses a skill it relied on.
39. As the operator upgrading, I want my saved harness selection carried into cards (a shared card is on when any
    harness it serves was selected), so that the upgrade keeps what I had.
40. As the operator, I want a harness whose own copy shadows ours (own-folder precedence) flagged, so that I know why
    an update doesn't show.
41. As the operator, I want a skill sent to both shared folders marked where duplicates hurt (fx can't invoke it,
    Cortex Code flags a conflict), so that I see the cost of the combination.

#### Link vs copy

42. As the operator, I want link to remain the default everywhere, so that updates stay instant.
43. As the operator, I want to set link or copy for global scope and for all projects, so that I can change the
    default once.
44. As the operator, I want to set link or copy per folder card (a harness's own folder, or a shared folder for all
    its readers), so that I can fix one misbehaving harness.
45. As the operator, I want a copy-only harness (Deep Agents Code) shown with its link choice disabled and its reason,
    and served from its own folder unless `.agents` is set to copy, so that it always loads skills.
46. As the operator changing a level, I want the existing Deployments it governs redeployed through the same move
    (identical-or-ask), so that the choice takes effect.

#### Projects

47. As the operator, I want each project to inherit the global preference, dropdown and link/copy mode, so that a new
    project needs no setup.
48. As the operator, I want to override any of them for one project, and see each setting's effective value and where
    it comes from (project, all projects, built-in), so that I know why a project behaves as it does.
49. As the operator, I want an override to stay an override even when it equals the default, and "Use project
    defaults" to remove it, so that a later change to the default behaves predictably.
50. As the operator, I want each global setting to list the projects that override it, with links and a reset, so
    that exceptions are findable.
51. As the operator, I want the project matrix's columns to be the folders serving that project's harnesses, so that
    the matrix matches disk.
52. As the operator, I want the project tools modal to use the same folder cards, so that projects and global read
    alike.

#### Overwrite ask (#70)

53. As the operator facing the overwrite ask, I want each row to say who put the folder there (Skills Hub, or not ours),
    so that I know what I'd replace.
54. As the operator, I want a content view or a diff against the skill, and Reveal in Finder, per row, so that I can
    judge.
55. As the operator, I want each button to state its consequence ("Keep theirs — {skill} stays off {harness}; Enable
    later to replace it"), so that I can predict the result.
56. As the operator, I want overwritten rows recorded in the run report, so that I can find what I replaced.
57. As the operator, I want an ask that arrives after Cancel to say the skill was already being added, so that it isn't
    confusing.

#### Release

58. As the operator, I want 1.4.0's CHANGELOG to explain the folder model, the default-on preference, every corrected
    folder, the link/copy choice and the launch offer, with a smoke list that includes a real first-launch move
    checked with `ls -la` over the old and new folders.

### Implementation Decisions

#### Placement: one pure answer to "where and how should this Deployment live"

- A new **placement** module in core, pure over the registry, the read facts, the settings, and the installed set
  for one scope. It answers:
  - per scope and per harness, the **folder** that serves it (Q15/Q16 with the preference on; its own folder with it
    off);
  - per folder, its **deploy mode** (Q21's levels, with the registry capability as a floor);
  - per folder, its served harnesses, other installed readers and reader-only harnesses.
- **The serving rule (Q15):**
  - reads `.agents` by default and not `.claude` → `.agents`;
  - reads `.claude` by default and not `.agents` → `.claude`;
  - reads both → the dropdown;
  - `Conditional | OptIn | No | Unknown` → its own folder.
  - A copy-only harness (registry `supports_symlink: false`) that would be served by a link-mode shared folder is
    served by its own folder instead (Q21).
  - At project scope the same rule reads the project facts.
- **Folder identity.** A folder is a distinct physical skills dir at its scope. Its **folder key** is a registry
  entry key:
  - `agents_skills` for `.agents`;
  - `claude_code` for `.claude`;
  - the harness's own key otherwise;
  - for a dir several entries share, the virtual group entry if one owns it, else the first entry in registry order.

  Folder keys are what the selection, Deployment rows, project tools and the UI carry. The frontend never derives a
  folder from a path, the same rule as `shared_with`.
- Placement is backend-only. The catalog commands expose its results; every write path asks placement and never
  re-derives a folder.

#### Deployment rows are per folder

- A global Deployment row is keyed by (skill, folder key); a project assignment by (project, skill, folder key). The
  per-sharer record fan-out (one row per installed sibling of a shared dir, D34 amendment) is retired: one folder,
  one row. Readers are derived from placement, never stored.
- **Migrating existing rows** happens through the redeploy (below), never by a silent schema rewrite. When a redeploy
  collapses several per-harness rows into one folder row (Q24):
  - the folder row is live if any harness it serves had the skill live;
  - it is `disabled` only when every one had it disabled;
  - each disable that no longer holds is report data naming the harness.
- **The saved selection** becomes a set of folder keys (Q26). On first read under 1.4.0, a stored harness selection
  maps to the folder keys serving those harnesses: a shared card is on if any harness it serves was selected. Its
  stored spelling gets a new versioned key; the old key is read once for the migration.
- **Disable per folder (Q19).** `DisableScope`'s pair grain becomes (skill, folder). Its confirmation names every
  harness the folder serves and every other installed reader.

#### Redeploy (#108, absorbing #87)

- A new **redeploy** module in core replaces `target_relocation` (deleted, with its tests ported where they still apply).
- Each recorded Deployment's place (path, mode, folder key) is compared with placement's answer. Every difference is a
  **move**, at both scopes. The causes are the registry corrections, the retired keys, the preference, the dropdown,
  link/copy and the selection migration; the module names none of them.
- **Two entry points:**
  - a read-only **preview** (counts per cause and scope; feeds the launch offer and every confirmation's "would move N");
  - the **mutation**, under `mutation_guard::serialized`, streaming progress over a Channel.

  Composites (a setting change that redeploys) call the unlocked seam inside their own guarded section. The layout is
  read once per operation (#88 residue).
- **Per move, in order (Q4):**
  1. Deploy into the new place through `sync_engine`'s capability-aware overwrite path. Ours or identical content at
     the destination is adopted. Different content there is a `TARGET_EXISTS` row for the one overwrite ask; a Keep
     answer leaves this move undone, and the move stays pending.
  2. In one DB transaction, switch the row (path, mode, folder key) and write a **journal entry**: the old artifact,
     its physical identity, and the row that owned it (Q5).
  3. Remove the old artifact through `artifact_removal` and the removal fence, only when provably ours:
     - an owned link resolving to its skill's central copy (physical identity), or a copy whose Content identity equals
       the Library's;
     - and no other live row records it (this covers the Kimi/Amp case).

     A copy that differs gets the **edited-copy question** (delete / leave). Leaving it disowns it, the journal entry
     clears, and the report names the harnesses that will show it twice. Removal clears the journal entry.
- **The journal** is finished first by every redeploy run and by the launch check: remove-or-report each entry, acting
  only while its row and both identities still match, otherwise reporting it left behind (`library_move`'s resume rule).
- **Disabled rows and never-synced `error` rows** (Q9) are re-recorded at the new place with nothing written.
- **Retired keys (Q6).** The Clawdbot and MoltBot `ToolId`s are deleted; their dirs become OpenClaw's former dirs.
  Rows, the selection and project tools/assignments under those keys are re-keyed to `openclaw` by the redeploy
  (merging into an existing row by the Q24 rule). Unknown keys stay tolerated.
- **Project leg.** Projects are handled per project, under the same rules. `project_ops::project_folder_exists`
  gates each project, giving `folder_missing`. Project target writes keep `TargetParents::WithinRoot`.
- **Report.** One `RedeployReport` crosses the wire as itself:
  - per move: `moved | adopted | unchanged | left_old { edited } | kept { declined } | skipped { folder_missing |
    disabled } | failed { error }`;
  - the duplicates left behind, each naming its readers from the read facts;
  - the disables that no longer hold (Q24);
  - the journal entries finished or left behind.

  `reportOutcome` folds it once. The fold stops skipping non-failed statuses: pending and left-behind entries surface.
- **The launch check (Q3).** Startup runs the journal finish, then the read-only preview. When the preview has work
  from a registry correction or the 1.4.0 migration, the app offers the move once through `offerDialog`. Declining
  leaves a "Move N Deployments" line in Settings → Harnesses, backed by the same preview. Nothing moves at launch
  except the journal finish (Library move's resume precedent).
- **Registry corrections (Q12).** Each correction is its row's new `skills_dir` / `project_skills_dir` / `detect_dirs`,
  with the old dir appended to `former_skills_dirs` (deletion rule and onboarding scan only). The deferred detect dirs
  pinned absent in Wave A (`.gemini/antigravity-cli`, `.config/devin`) land with their corrections, and that pin is
  removed deliberately. Devin's Windows dir is a roaming-root `OsOverride`. Goose, OpenHands and Zencoder join the
  AGENTS group at project scope (`group` literal); the group/fact consistency test still holds.
- **Onboarding (Q10).** The scan includes each harness's former dirs as that harness's, excluding our own links by
  physical identity. Import's reader roster for shared dirs comes from placement.
- **Acceptance tests carried:** audit 3's F14 (aliased roots), F15 (a destination aliased onto the central repo) and
  F16 (Keep over a copy with local-only edits).

#### Settings and the Harnesses pane

- New settings, as typed `AppSettings` fields, each written only through a command that previews and redeploys (no
  generic `update_setting` key for them, the precedent being `CentralRepoPath` since 1.3.4):
  - the shared-folder preference (bool, default on);
  - the both-folders choice (`agents | claude`, default `agents`);
  - global-scope deploy mode;
  - all-projects deploy mode;
  - per-folder deploy mode overrides.
- **The folder catalog.** The global and project catalogs answer **folder cards**. Per card:
  - folder key, display path, scope, on/off;
  - served harnesses (with detected state), other installed readers, reader-only harnesses, undetected members;
  - effective deploy mode and its origin level, and whether link is allowed (capability);
  - the duplicate lines it contributes.

  The pane-level "Seen twice" list is resolved per installed harness (Q20, #116). `ToolInfoDto` remains for the
  harness rows the cards contain. `useToolCatalog` exposes cards; no surface rebuilds them.
- **Card copy (Q31):**
  - The title is the folder; "Serves …" sits underneath; "Also seen by" lists names only.
  - Every consequence sits in "Seen twice (N)".
  - A card is amber only for `LoadsNone`. Cortex Code's `FlagsConflict` and the shadowed-own-copy flag are notes.
- **Turning the preference off (Q30).** The confirmation lists every installed harness currently served by an on
  folder, all ticked. Unticking leaves that harness's own-folder card off. The answer becomes the card selection, and
  the redeploy runs in the same action.
- **The same cards on every surface (Q25):** the Add dialog's picker, the inspector's Targets section (harnesses
  grouped under folders), `DeployHarnessAsk`, the project tools modal, and My Skills' per-folder dots. A new or
  changed primitive variant is documented in DESIGN.md in the same change, per ADR-0006.

#### Link vs copy (#84)

- **Resolution, most specific wins (Q21):** folder override › project override › scope setting (global, or all
  projects) › built-in link. The registry capability is a floor: a copy-only harness's own folder resolves to copy
  with link disabled and its reason. A shared folder's mode applies to all its readers.
- Sync code stays name-blind: `sync_engine` receives the resolved mode as a capability, never a harness name. The
  "Do not" rule (a group's capability is its own fact, never the AND of its members) holds, because placement moves
  a copy-only reader out of a link-mode shared folder instead of degrading the folder.
- Changing any level previews and redeploys the Deployments it governs.

#### Project defaults and overrides (Q22; T3 Code pattern, `docs/design-inputs/2026-11-06-t3code-project-defaults-and-overrides.md`)

- **Stored overrides are sparse.** A project stores only the keys it overrides: preference, both-folders choice,
  deploy mode, per-folder mode. **Inherit means the key is absent**; a present key is an override even when it
  equals the default. "Use project defaults" deletes the keys.
- **One settings surface with a scope selector** ("All projects / {project}"). Each row shows its effective value, an
  origin icon opening the chain (folder › project › all projects › built-in), and Reset on an override. A
  default row lists the projects overriding it, with links, "Reset it" and "Reset all".
- **Consequential edits** go through Skills Hub's confirmation and one redeploy action with a report: an edit, a reset
  and a bulk reset alike. T3's plain save is not copied, and neither is its multi-machine "all environments".
- Resolution and the overrides' storage are backend-owned (a project-settings table or columns, migrated through
  `migrate_legacy_db_if_needed` if needed). The frontend renders the chain the backend answers.

#### Overwrite ask (#70)

- **The wire row gains provenance:** whether a recorded Deployment owns the occupant (`synced_at` set, physical
  identity) or it is foreign; plus the existing `path`, `modifiedAt`, `sameContent`.
- **A content view or diff** of the occupant against the skill. It reuses the file browser (`useSkillDocument` over
  `skillFiles`) with a backend read of the occupant, fenced like `reveal_path`. Reveal in Finder per row goes through
  `admit_reveal_path` (extended to admit a reported occupant path).
- **Button copy** states each consequence. Overwritten pairs are recorded in the run (`recordRun`) as an outcome kind
  beside `declined`. An ask arriving after Cancel says the skill was already being added.
- **The edited-copy question** is a second row kind in the same ask (delete / leave), not a second dialog, so one
  operator action still raises one ask.

#### Errors and wire

- New typed `CommandError` variants follow ADR-0001, each with a binding, a `describeCommandError` branch and i18n keys:
  - a redeploy refused while a journal entry can't be finished;
  - a setting write refused because the preview changed under the confirmation.
- Everything crossing the wire derives `specta::Type`; bindings are regenerated by `cargo test`.

#### Glossary (CONTEXT.md, in the tickets that introduce them)

- **Deployment folder** (code: folder key): a skills folder skills are written to, at one scope; it serves harnesses,
  and others may read it.
- **Served harness** / **reader**: a harness the placement assigns to a folder; any harness that reads it.
- **Shared-folder preference** (code: `prefer_shared_folders`) and **both-folders choice**.
- **Placement**: where and how a Deployment should live, answered from registry, settings and installed set.
- **Redeploy** (code: `core/redeploy.rs`), its **journal** and its **launch check**. It replaces "Target relocation".
- **Deploy mode** gains its levels and origin.

### Testing Decisions

- **Good tests pin behaviour at the highest seam** and never restate a private helper (as in Wave A).
- **Placement:** table tests over registry rows with a substituted installed set, at both scopes:
  - every Q15 branch;
  - the dropdown;
  - preference off;
  - copy-only demotion;
  - folder-key identity for every shared dir.

  A snapshot of the operator's machine: Claude Code, Codex and Pi installed and selected → two cards.
- **Redeploy (core integration tests, temp roots, Tool and Project dirs outside the central dir):**
  - every report status;
  - deploy-first ordering (the old copy is present until the new one is recorded);
  - adopt-identical;
  - Keep over a differing occupant;
  - edited copy deleted / left;
  - Kimi/Amp (Amp's artifact survives);
  - disabled rows re-recorded with nothing written;
  - retired-key re-key and merge;
  - Q24 row collapse and lost-disable reporting;
  - project leg with `folder_missing`;
  - journal finish after a simulated crash at each step (seeded intermediate states);
  - F14–F16;
  - idempotence (a second run is a no-op).

  Prior art: `library_move` tests, `artifact_removal` fence tests, `project_sync` `folder_missing` tests.
- **Registry:**
  - correction rows in the brief-snapshot table;
  - the deferred-detect-dir pin flipped;
  - the README table test;
  - `former_skills_dirs` within the deletion rule;
  - the retired `ToolId`s are gone and tolerated as unknown keys.
- **Settings and overrides:**
  - sparse storage (absent = inherit, an equal value stays an override);
  - resolution chain and origin;
  - reset and bulk reset;
  - migration of the old selection key.
- **Frontend (hooks and pure folds; no component render tests):**
  - the folder-card presentation fold;
  - the "Seen twice" fold;
  - `reportOutcome` for `RedeployReport` and overwritten rows;
  - the settings world's confirm → preview → run flow;
  - Q30's confirmation default (all ticked);
  - the door's Disable-per-folder confirmation counts (`bulkCounts`);
  - i18n coverage.
- **Primitives:** any new variant keeps its `ui/*.test.tsx` role/ARIA/focus contract.
- **Fixture:** the `registry` scenario gains folder cards (C), a pending launch offer, an edited-copy ask row and a
  project with overrides. The prototype's data set (the operator's machine plus Cursor, Gemini CLI, Cortex Code,
  fx, Deep Agents Code, Antigravity, Zed and Warp) becomes a scenario.
- **Gate:**
  - `npm run version:check && npm run check` per lane;
  - Windows `%APPDATA%` arms proven by `gh pr checks` on a draft PR;
  - the release smoke includes a real first-launch move checked with `ls -la` over the old and new folders.

### Out of Scope

- QwenPaw's folder correction (#118).
- Honouring env-var homes (Wave A Q3); custom harness rows (#56).
- A per-harness both-folders dropdown (Q16); excluding one reader from a shared folder (Q14/Q19: impossible on disk).
- Global Deployment presence checks (#94); the matrix keyboard model and selection (#85/#86); filter-hidden
  selections (#93) — after 1.4.0.
- Version gates on harness reads ("since"): brief I9, not observable.
- A cancellable redeploy (the Library move precedent); moves that run at launch without being offered.

### Further Notes

- Prototype: branch `proto/waveb-harness-cards` (`7e0380f`), throwaway, never merged. Its folder-serving fold is a
  reference for the presentation fold, not code to lift.
- Removal-safety reviews converge slowly (1.3.3 and Wave A). The redeploy lane gets Astra and Sol in parallel, each
  given the principle: delete only what is provably a duplicate, at the moment of deletion, only inside the folder
  you own.
- A worktree child has no `.pi/` skills. The orchestrator runs the copy pass (`impeccable`) at merge.
