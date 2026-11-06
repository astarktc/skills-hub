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
