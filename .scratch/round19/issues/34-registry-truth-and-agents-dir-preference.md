# 34 — Registry truth pass: harness rows corrected to vendor docs, global AGENTS-standard membership, cross-read dedupe, and the "prefer the AGENTS standard dir" preference

Status: parked — held out of 1.3.1 (operator rule 11: a P0 needing a design decision parks the ticket); branch `t3/registry-truth-pass` holds the work; BACKLOG #88 is the redesign
Blocked by: 20 (the Harnesses pane is this ticket's UI home); 32 (vocabulary — done)
Blocks: 25
Source: `docs/design-inputs/2026-11-02-harness-global-skills-dirs.md` (the research brief — every claim below cites
its §; a wrong claim is fixed there first). Absorbs BACKLOG **#67** (Codex), **#45** (three path claims + four rows),
**#83** (the preference), and the detection half of **#40**. Operator approval 2026-11-02 ("registry truth approved").
Spec: D47 (per-harness rows in the Harnesses pane), D45 (vocabulary), AGENTS.md "New AI tool adapter" invariant
(registry literal + README row + `project_relative_skills_dir_for_every_tool` table test, all in one change).

## Problem

The registry (`core/tool_adapters/mod.rs`) was written from upstream and a 2026-09 survey. Checked against vendor
docs and harness source on 2026-11-02 (brief § 1, § 5): **Codex** reads `~/.agents/skills` since 0.95.0 and marks
`~/.codex/skills` deprecated — we deploy to the deprecated one, and a same-name skill in both shows twice in Codex
(§ 3); **Antigravity** documents `~/.gemini/config/skills` (ours: `.gemini/antigravity/global_skills`); **Kimi Code
CLI** reads `~/.kimi-code/skills` (ours: Amp's shared `.config/agents/skills`); **Kilo** moved to `~/.kilo/skills`;
**Mux** is now **Xum** (`~/.xum/skills`); **Goose** and **OpenHands** recommend `~/.agents/skills`; **Windsurf**'s
docs moved to `~/.config/devin/skills` (unverified build); **Clawdbot / MoltBot** are former names of OpenClaw.
**24 of 44** harnesses read `~/.agents/skills` globally (§ 1, I11). And harnesses **cross-read** each other's dirs:
Cursor reads `~/.claude/skills` and `~/.codex/skills`; OpenCode, Amp, Augment, Crush, Goose, Continue, Codewhale and
legacy Kimi read `~/.claude/skills` (§ 1, I1) — so deploying one skill to Claude Code *and* Cursor already shows it
twice inside Cursor. The four upstream rows (DeepSeek Harness, ZCode, Codewhale, WorkBuddy) are real products (§ 4).

## Work

### Registry facts (backend, `core/tool_adapters/`)

- **New per-entry facts**, registry literals only, each compiler- or test-enforced: `reads_agents_standard_globally:
  Option<AgentsStandardRead { since: Option<&'static str>, precedence: OwnWins | AgentsWins | BothShown | Unknown }>`
  (brief § 1 column "Reads ~/.agents/skills? / Since / Precedence" — VERIFIED rows only; LIKELY/UNKNOWN stay `None`
  with a comment naming the brief row), and `also_reads: &[&'static str]` (relative dirs of *other* harnesses this one
  reads — Cursor: `.claude/skills`, `.codex/skills`; the `~/.claude/skills` readers listed in I1). Add a table test
  that every entry's facts match a checked-in snapshot derived from the brief, so a registry edit without a brief
  edit fails.
- **Rows corrected** (brief § 5, I2–I10), each with its former dir in `former_relative_skills_dirs` so **Target
  relocation** moves existing deployments on next launch: Antigravity → `.gemini/config/skills` (former
  `.gemini/antigravity/global_skills`, `.gemini/antigravity/skills`; detect stays `.gemini/antigravity`, never
  `.gemini/config` — a skills-only footprint); Kilo → `.kilo/skills` (former `.kilocode/skills`; detect both); Mux →
  Xum (display name, key stays `mux` unless a key migration is cheap — decide and record; `.xum/skills`, former
  `.mux/skills`, detect both); Clawdbot and MoltBot: retire as rows **only after** checking the operator's machine
  has no targets there (brief I10) — else keep detect-only; Windsurf: hold, `.config/devin/skills` noted as a
  candidate (I9).
- **Kimi Code CLI → `.kimi-code/skills`** with the I4 hazard handled: `.config/agents/skills` is Amp's (and Crush's)
  live shared dir, so it must **not** go into Kimi's `former_relative_skills_dirs` blindly — relocation would move
  Amp's targets. Model it so relocation only moves a row whose Tool is Kimi and whose path is inside the former dir,
  and never removes the artifact when another installed sharer's row points at it (extend
  `core/target_relocation.rs`'s rule; test it with Amp installed beside Kimi).
- **Codex joins the AGENTS-standard group globally** (I2 option (a)): `.agents/skills` becomes Codex's global
  deployment dir through group membership (shared-dir dedupe, `shared_with`, the README group sentence), with
  `.codex/skills` as a former dir so existing targets relocate. Goose and OpenHands likewise (I8). The virtual group's
  own capability rule (AGENTS.md "Do not") is unchanged.
- **Four new rows** (I13): DeepSeek Harness (`.dsh/skills`), ZCode (`.zcode/skills`), Codewhale (`.codewhale/skills`,
  never detect on `.deepseek`), WorkBuddy (`.workbuddy/skills`, project dir unresolved — global-only, say so in the
  row comment and README). README table rows + the table test, per the invariant.

### Cross-read dedupe (backend + Harnesses pane)

- The Tool catalog (`global_tool_entries`) gains, per entry, the harnesses whose dirs it **also reads**
  (`also_reads` resolved against installedness), so the frontend can warn: "Cursor also reads Claude Code's and
  Codex's skills — a skill deployed to all three shows three times in Cursor." Shown in the Harnesses pane row and
  as a one-line note on the deploy confirmation when the chosen set contains such a pair. **No automatic
  de-selection** — a warning, since precedence is per-harness and often unknown.

### The preference (#83; D47 — UI in ticket 20's Harnesses pane)

- Setting **"Prefer the AGENTS standard folder (`~/.agents/skills`) for harnesses that read it"** (off by default
  for existing installs; decide the default for a fresh install and record it). When on: for every selected harness
  with `reads_agents_standard_globally` VERIFIED, the effective global target becomes the AGENTS-standard group
  entry instead of the harness's own dir — **and the harness-specific deployment stops** (brief I1: otherwise the
  skill appears twice), with existing harness-dir targets relocated into `~/.agents/skills` by the same relocation
  rule (byte-identical replace, else leave + log) on save, under one confirmation that counts the moves. Sub-setting
  (visible only when on): "Hide harness-specific folders in this list" — collapses those rows to a single
  "AGENTS standard — Cursor, Codex, OpenCode +5" row. A harness with precedence `OwnWins` whose own dir still holds a
  foreign copy is flagged (the own copy shadows ours). Settings keys + `AppSettings` fields + `specta` export.
- Tests: effective-target-set derivation with the preference on/off across `OwnWins` / `AgentsWins` / `BothShown` /
  `None`; the relocation on save; the warning rows.

## Proof

- `cargo test --all` (registry snapshot test, relocation tests incl. the Kimi/Amp hazard, effective-set tests);
  README table matches the `ToolId` count; `npm run test`.
- Fixture: a `registry` scenario (or `rich`) showing the Harnesses pane with the cross-read warning, the preference
  on/off, and the collapsed group row; screenshots both themes under `evidence/34/`.
- Operator check on the real machine before release: `ls -la ~/.codex/skills ~/.agents/skills` before/after the
  relocation (release smoke item on ticket 26).
- D31 audit (Astra) on the Harnesses pane changes; `npm run version:check && npm run check`.

## Result

### Backend half (2026-11-04, delegated child; uncommitted — the orchestrator reviews and commits)

**Registry facts** (`core/tool_adapters/mod.rs`). Every row carries `reads_agents_standard_globally:
Option<AgentsStandardRead { since, precedence: Precedence::{OwnWins, AgentsWins, BothShown, Unknown} }>` (VERIFIED § 1
rows only; every LIKELY / UNKNOWN / VERIFIED-absent row is `None` with a comment naming its brief row) and
`also_reads: &[&str]` (I1). 26 readers: the brief's 24 minus OpenClaw (retired) plus DeepSeek Harness, ZCode, Codewhale;
`since` only on Codex (`rust-v0.95.0`), Pochi (`2a9ff73 (#1360)`), Roo (`PR #11181`), Gemini CLI (`PR #18151`) — I11.
Cross-reads: Cursor `.claude/skills` + `.codex/skills`; OpenCode, Amp, Augment, Continue, Goose, Codewhale
`.claude/skills`; Crush `.config/agents/skills` + `.claude/skills`; Kimi (legacy kimi-cli, S14) `.claude/skills`,
`.codex/skills`, `.config/agents/skills`; Antigravity `.gemini/skills` (its CLI, S9). Kilo's `.claude/skills` read is
opt-in, so not recorded. `project_relative_skills_dir` is now `Option<&str>`: `None` = global-only (WorkBuddy) — absent
from the project catalog and the gitignore patterns, refused by `configure_project_tools` (prose bail: the UI cannot
produce it), `resolve_project_sync_target` answers `None`.

**Rows changed** (each cites the brief in its literal):

| Row | Change | Brief |
| --- | --- | --- |
| Codex | global dir `.agents/skills`, former `.codex/skills` (global sharer of `agents_skills`) | § 3, I2 (a) |
| Goose | global dir `.agents/skills`, former `.config/goose/skills`; project dir unchanged | § 1 #38, I8 |
| OpenHands | global dir `.agents/skills`, former `.openhands/skills`; project dir unchanged | § 1 #25, I8 |
| Antigravity | `.gemini/config/skills`, former `.gemini/antigravity/global_skills` + `.gemini/antigravity/skills`; detect stays `.gemini/antigravity` | § 2, I3 |
| Kimi Code CLI | `.kimi-code/skills`, former `.config/agents/skills` (Amp's live dir — the relocation clause below); detect `.kimi-code`, `.kimi` | § 2, I4 |
| Kilo Code | `.kilo/skills`, former `.kilocode/skills`; detect `.kilo`, `.kilocode`; project dir unchanged | § 5, I6 |
| Mux → Xum | display "Xum", `.xum/skills`, former `.mux/skills`; detect `.xum`, `.mux`; project dir unchanged | § 5, I7 |
| Windsurf | held; `.config/devin/skills` named as the candidate in the comment | I9 |
| Clawdbot, MoltBot, OpenClaw | retired (variants, `as_key` arms, literals, README rows) | I10; operator 2026-11-04 |
| DeepSeek Harness (`deepseek_harness`), ZCode (`zcode`), Codewhale (`codewhale`), WorkBuddy (`workbuddy`) | new rows; Codewhale never detected on `.deepseek`; WorkBuddy global-only | § 4, I13 |

**Decisions.**

- **Mux / Xum key stays `mux`** — it is stored in target rows, project tool rows, assignments, `global_selected_tools_v1`
  and `installed_tools_v1`; a rename buys nothing the display name does not. i18n `tools.mux` now reads "Xum".
- **Retired rows, stored data:** no migration and no tombstone branch in production code — a stored `clawdbot` /
  `moltbot` / `openclaw` row is an unknown key, and every path already tolerates one: the selection read prunes it,
  Propagation skips it (`PropagationSkip::UnknownTool`), relocation ignores it, Delete removes its artifact by the row's
  stored path (Skill scope is unfenced; a SkillTool scope on an unknown key plans its own rows). Pinned by
  `a_retired_tools_stored_row_is_tolerated_and_still_deletable` and `a_selection_naming_a_retired_tool_is_pruned`
  (iterating the test-only `tool_adapters::RETIRED_TOOL_KEYS`); CHANGELOG states the removal; the i18n labels stay so a
  stored row still shows a readable name. **OpenClaw was retired too**, per the 2026-11-04 Comment's literal wording —
  it is a real product (VERIFIED, S33, AGENTS-wins); restoring it is one literal + README row + table rows if that was
  not the intent.
- **Codex "joins the group globally"** is modelled as Codex's own global dir being `.agents/skills`, so the existing
  shared-dir machinery (dedupe, `shared_with`, record fan-out, Disabled rule) covers it with no new concept; the
  project-scope group is unchanged (still 9).
- **Default ON applies at launch too.** The preference defaults on for fresh and existing installs, so the once-per-launch
  relocation pass reads it: on first launch an existing install's deployments in every AGENTS-standard reader's own dir
  move into `~/.agents/skills` (byte-identical replace, else leave + log). Not doing so would leave rows in the own
  dirs while the derivation deploys elsewhere (a re-sync would orphan the old artifact — a duplicate in that harness).
  This is the one-line lever if the orchestrator wants the move confirmation-only: `relocate_former_global_targets`
  reads `settings::global_layout`.
- **Shared-dir semantics under the preference** (existing D34 rule, now wider): a first deploy into `~/.agents/skills`
  records a row for every *installed* sharer — with the preference on that is every installed reader, selected or not —
  and disabling any one of them removes the one shared artifact for all (the Disabled rule). Truthful (they all read
  that dir), but the UI half must present it.

**Relocation rule, new clauses** (`core/target_relocation.rs`, `tool_adapters::superseded_skills_dirs_in`). A row
relocates when its artifact sits directly in one of its Tool's *superseded* dirs under the `GlobalLayout` — former
dirs, plus the own dir when the preference moved the Tool to `~/.agents/skills`, plus `~/.agents/skills` when a reader
is back on its own dir — into `global_skills_dir_in(home, adapter, layout)`; moves are keyed by (old path, destination).
**The old artifact is removed only when no remaining row (not Disabled, owning its artifact) still records it** — Kimi's
row leaves `~/.config/agents/skills`, Amp's artifact stays. `remove_superseded_artifact_unlocked` is fenced to the
superseded dirs under the layout. The skills-only-footprint rule also treats former dirs as skills paths (`~/.codex`
holding only `skills/` is not Codex).

**The preference (#83).** `AppSettings.prefer_agents_standard_dir` (key `prefer_agents_standard_dir`, **default
true**) and `hide_harness_specific_dirs` (default false; `SettingUpdate::HideHarnessSpecificDirs`). The preference is
deliberately **not** a `SettingUpdate`: its write and relocation run in one `mutation_guard::serialized` section.
`settings::global_layout(store) -> GlobalLayout` is read by every global-dir derivation: the batch planning
(`plan_batch_tool_targets(home, layout, keys)` — the effective target set; `PlannedToolTarget.sharers` now carries the
plan's sharers for the Disabled rule and override matching), artifact removal's group keys, Propagation's sharers,
Disable's marker path and Enable's group, the catalog and relocation. Onboarding import (auto-sync on) removes a
byte-identical original a moved Tool left in its own dir once that Tool's row is synced elsewhere
(`settle_superseded_originals`); the onboarding scan reads a shared dir once (first in-scope sharer gets the variant).

**Wire.** Commands (both `#[specta::specta]`, in `collect_commands!`): `preview_prefer_agents_standard_dir(enabled) ->
number` (rows the save would move) and `set_prefer_agents_standard_dir(enabled) -> AgentsStandardPreferenceResult {
settings, relocation: RelocationSummary { relocated, left_in_place } }`. `ToolInfoDto` gains `also_reads: string[]`
(installed keys beside `shared_with` whose dir this harness also reads, incl. the AGENTS-standard sharers when the
preference is off), `agents_standard_read: AgentsStandardReadDto { since, precedence } | null`,
`shadowed_by_own_dir: boolean` (OwnWins harness moved by the preference whose own dir holds an unrecorded copy of a
skill deployed to `~/.agents/skills`). Shim re-exports: `AgentsStandardReadDto`, `Precedence`,
`AgentsStandardPreferenceResult`, `RelocationSummary` (`src/components/skills/types.ts`).

**Tests** (new): `registry_matches_the_brief_snapshot` (the checked-in `BRIEF_SNAPSHOT`, one row per entry with its
citation), `the_agents_standard_readers_are_the_briefs_verified_rows`, `every_cross_read_is_another_harness_dir`,
`a_former_skills_dir_is_a_live_dir_only_where_pinned` (replaces `no_former_skills_dir_is_a_current_skills_dir`; pins
Kimi→Amp), `a_former_dir_alone_is_a_skills_only_footprint`, `detect_dirs_avoid_the_briefs_footprint_hazards`,
`the_layout_moves_every_verified_reader_and_only_them` (OwnWins / AgentsWins / BothShown / Unknown / None × on/off),
`superseded_dirs_follow_the_layout`, `retired_rows_are_not_registered`; catalog:
`the_preference_groups_every_agents_standard_reader_with_the_entry`, `cross_reads_resolve_to_installed_harness_keys`,
`an_own_wins_harness_with_a_foreign_copy_is_flagged_shadowed`, `a_global_only_tool_is_absent_from_the_project_catalog`;
batch: `the_preference_resolves_readers_to_the_agents_standard_dir`,
`the_batch_stops_the_harness_specific_deployment_under_the_preference`; relocation:
`kimi_relocates_out_of_amps_shared_dir_and_amps_artifact_stays` (mutation-checked: fails without the keep clause),
`kimi_alone_relocates_and_its_former_artifact_is_removed`,
`codex_targets_relocate_from_the_deprecated_dir_to_the_agents_standard_dir`,
`corrected_rows_relocate_from_their_former_dirs`, `the_preference_relocates_a_readers_targets_both_ways_on_save`
(preview count + save + back), `turning_the_preference_off_keeps_the_artifact_a_sharer_still_records`,
`under_the_preference_every_reader_on_a_shared_dir_moves_together`, `the_preference_leaves_a_non_reader_alone`,
`a_retired_tools_stored_row_is_tolerated_and_still_deletable`; settings: defaults, `the_global_layout_reads_the_preference_with_its_default`,
`apply_hide_harness_specific_dirs_round_trips`, `a_selection_naming_a_retired_tool_is_pruned`; onboarding:
`a_shared_skills_dir_is_scanned_once`, `auto_sync_on_under_the_preference_removes_the_superseded_identical_original`;
project: `configure_tools_refuses_a_global_only_tool_before_writing_anything`. Shared-dir tests that used Amp + Kimi now
use Codex + Goose; fixtures that pin own-dir semantics set the preference off explicitly.

**Gate.** `cargo test --all` 870 passed (bindings regenerated by `export_bindings`); `npm run test` 1147 passed;
`npm run version:check && npm run check` green (lint, styles, test, build, fmt, clippy `-D warnings`, cargo test).
README table rows **46** = `ToolId` variants **46**.

**Unproven / follow-ups.** No run against the operator's real library (dev runs mutate it) — the first-launch moves
(Codex's `~/.codex/skills`, every reader's own dir under the default-on preference) need the release smoke check on
ticket 26 (`ls -la ~/.codex/skills ~/.agents/skills ~/.cursor/skills ~/.pi/agent/skills` before/after). Project-scope
dirs the brief also flags (Goose/OpenHands `.agents/skills`, Kilo `.kilo/skills`, Xum `.xum/skills`) are unchanged:
no project-assignment relocation exists. Former dirs are not onboarding-scanned, so unmanaged skills another installer
put in `~/.codex/skills` are not offered for import. The fixture registry keeps Kimi / Codex / Goose / OpenHands on
their former dirs (its shared-dir tests are built on Amp + Kimi) and does not simulate the layout.

### Fix pass (2026-11-04, orchestrator decisions; delegated child; uncommitted)

Supersedes the backend half's "OpenClaw was retired too" and "Default ON applies at launch too" decisions.

- **OpenClaw restored** (brief row 10, S33: VERIFIED, `AgentsWins`): `ToolId::OpenClaw` / `openclaw`, global
  `.openclaw/skills`, detect `.openclaw`, project dir `skills` (as before ticket 34), `also_reads: &[]` (the brief lists
  no other harness's dir). Clawdbot and MoltBot are its **former names** (rows 41 / 44, I10):
  `former_relative_skills_dirs: [".clawdbot/skills", ".moltbot/skills"]` and a new registry fact
  `ToolAdapter::former_keys: ["clawdbot", "moltbot"]` (`tool_adapters::adapter_succeeding_key`). The relocation pass
  **re-keys** a stored `clawdbot` / `moltbot` global row to `openclaw` as it moves it out of a former dir
  (`SkillStore::rekey_skill_target`, same row id) — cheap, and without it a moved row would record OpenClaw's dir under
  an unknown key. When the skill already has an `openclaw` row the former row cannot be re-keyed
  (`UNIQUE(skill_id, tool)`): it stays, counted `left_in_place` (and not in the preview), still deletable.
  `RETIRED_TOOL_KEYS` = `clawdbot`, `moltbot`. Readers: **27**. README table rows **47** = `ToolId` variants **47**.
  Tests: `the_retired_keys_are_openclaws_former_names`, `a_former_names_row_is_rekeyed_to_openclaw_and_relocates`
  (both keys × both layouts, idempotent), `a_retired_tools_stored_row_is_tolerated_and_still_deletable` (now the
  blocked-re-key case), `corrected_rows_relocate_from_their_former_dirs` (+ an `openclaw` row in `.moltbot/skills`,
  its pre-rename dir), snapshot / `project_relative_skills_dir_for_every_tool` rows.
- **#83 default — recorded decision: on for a fresh install; an existing install keeps its own dirs with a pending
  offer; the launch pass never moves on the default.** `settings::global_layout(store)` distinguishes unset from set:
  saved → that value; unset (absent or malformed) with **no** global target row → writes `true` and returns
  `PREFER_AGENTS_STANDARD` (latched, so the first deployment happens under it and later reads see it saved); unset
  with any target row → `OWN_DIRS`, nothing written, `AppSettings.agents_standard_offer_pending = true` (specta). A
  storage failure reads `OWN_DIRS`. `FRESH_INSTALL_PREFER_AGENTS_STANDARD_DIR` replaces the old default constant. So
  `relocate_former_global_targets` on an unsaved existing install moves only registry-truth relocations (Codex,
  Goose, OpenHands, Antigravity, Kilo, Xum, Kimi with the Amp guard, OpenClaw's former names). Answering the offer
  through `set_prefer_agents_standard_dir` (either way) writes the row and ends it. Tests:
  `a_fresh_install_latches_the_preference_on`, `an_existing_install_keeps_its_own_dirs_and_is_offered_the_preference`,
  `a_saved_preference_wins_and_garbage_reads_as_unset` (settings);
  `the_launch_pass_never_moves_a_readers_target_on_the_unset_default` (Cursor untouched, Codex moved, nothing
  written, offer pending), `saving_the_preference_on_an_existing_install_relocates_and_ends_the_offer` (preview 1,
  summary 1, next launch pass 0), `declining_the_offer_saves_own_dirs_and_ends_it` (relocation).
- **The two out-of-brief consumers, re-checked.** The onboarding scan's shared-dir dedupe keys on each Tool's
  **own** dir (`skills_dir_in`), never the layout — it reads no preference (the dirs it dedupes, `~/.agents/skills`
  for the entry / Codex / Goose / OpenHands, are shared by registry truth). The import's
  `settle_superseded_originals` now returns early unless `global_layout` is on (the effective value — saved, or a
  fresh install's latch), and settles only an original sitting directly in the own dir of a Tool the layout moved
  (before, any Tool row synced at another path qualified — a name-mismatched original under own dirs would have been
  removed). Test: `auto_sync_on_with_the_preference_unset_on_an_existing_install_keeps_own_dirs` (taken over in place,
  nothing in `~/.agents/skills`, nothing removed, no preference written); the existing
  `auto_sync_on_under_the_preference_removes_the_superseded_identical_original` covers the saved-on case.
- **UI half** additionally reads `AppSettings.agents_standard_offer_pending` and, when true, `offerDialog`s the
  one-time choice ("Prefer the AGENTS standard folder? N deployments would move": `previewPreferAgentsStandardDir(true)`
  → `setPreferAgentsStandardDir(answer)`). The fixture's `setPreferAgentsStandardDir` clears the flag.
- **Unproven:** no run against the operator's real library. A pre-existing timing flake recurs under machine load
  in the full suite (`mutation_guard::listing_reconciles_when_no_mutation_is_in_flight`,
  `project_sync::missing_status_when_source_absent`: a `try_serialized` reader polls 100 × 50 ms and can lose the
  guard to the parallel suite); both pass alone and on re-run.

### Fix pass 2 (audit) (2026-11-04, orchestrator dispositions of `.scratch/round19/review/ticket-34-backend-audit.md`; delegated child; uncommitted)

Every P0 got a failing test reproducing the audit's trace first (all six red for the traced reason), then the fix.

- **F1 (P0) — destination Disabled admission.** `global_sync::DisabledGate` is now `pub(crate)` with
  `blocks_group(adapter, sharers)`; `target_relocation::admission` is the one rule for the pass **and**
  `preview_relocation`: a candidate row stays (`left_in_place`, logged) when the skill is disabled at skill grain or any
  Tool of the **destination** group (`adapters_sharing_skills_dir(adapter, layout)`) has a `disabled` row, owning or
  not; a blocked re-key stays as before. No marker is touched. Tests (red before):
  `the_preference_never_moves_a_target_into_a_dir_a_disabled_sharer_reads` (Cursor live in `.cursor/skills`, Codex
  marker on `.agents/skills`, preference on → preview 0, `{0, 1}`, Cursor stays, marker intact) and
  `codexs_registry_move_waits_while_a_destination_sharer_is_disabled` (disabled `agents_skills` marker, both layouts).
- **F2 (P0) — ownership across every row.** `artifact_removal::artifact_is_owned(store, path)` over the existing
  `recorded_target_paths` inventory (global rows of every skill + resolved project assignments; no second ownership
  rule), comparing with `same_artifact_path`: equal as written, or same leaf name in parents that canonicalise to one
  dir (a project root is stored canonicalised — macOS `/var` → `/private/var` in the tests; the leaf is never
  canonicalised, it is usually a link). Relocation's cleanup uses it instead of "this skill's global rows". Tests (red
  before): `relocation_never_removes_an_artifact_a_project_assignment_owns` (home as a project + global Augment row,
  both directions: the global row moves, the link stays, the assignment stays owned) and
  `kimis_relocation_keeps_an_artifact_a_config_agents_project_owns` (`~/.config/agents` project, OpenClaw assignment).
- **F3 (P0) — stored-path ownership survives an incomplete migration.**
  - Propagation partitions a shared-dir group's installed rows by **write target**: an owned row's stored path (rows at
    different paths are different artifacts, each updated in place, no row settled at another's path, no path
    changed); a row that owns nothing has no artifact to keep and first-syncs where the group deploys now (same
    artifact name). Siblings are minted only by a first sync into the group's dir. The per-artifact write moved into
    `ArtifactWrite::settle` unchanged otherwise; the Disabled and not-installed rules stay group-wide. Test (red
    before): `an_update_after_a_partial_relocation_updates_each_owned_path_in_place` (Cursor copy in own dir, divergent
    Codex copy in `.agents/skills`, preference on → both Synced at their own paths, `v2` in both, Delete leaves
    nothing).
  - A global first sync never re-points an owned row out of a superseded dir: `sync_skill_into_root` skips `Recorded`
    for a record Tool whose row owns a present artifact directly in one of the planned group's superseded dirs
    (`PlannedToolTarget.superseded_dirs`, each sharer's `superseded_skills_dirs_in`); `sync_skills_to_tools_unlocked`
    then runs `target_relocation::relocate_skill_unlocked` for each synced skill, which moves that row by its own
    rules (Disabled admission, byte-identical replace — the batch just wrote it — ownership-aware cleanup). A rename
    within the same dir is not held (pre-existing behaviour). Test (red before):
    `a_first_sync_never_replaces_a_different_owned_target_path` (refused relocation → confirmed overwrite deploy → no
    artifact without an owner, Cursor's row moved, old copy gone; Delete leaves nothing).
- **F4 (P1) — onboarding.** The managed-target exclusion is keyed by physical path alone (owned rows,
  `synced_at` set), so an owned copy is excluded whatever Tool the shared dir is attributed to.
  `OnboardingVariant.readers: Vec<String>` (specta; bindings regenerated): every in-scope **installed** harness that
  loads the copy — own dir, VERIFIED AGENTS-standard read, or cross-read (new registry rule
  `ToolAdapter::reads_global_dir` / `other_global_reads`, which the catalog's cross-read resolution now uses too) —
  plus the attributed Tool itself; registry order. Including AGENTS readers and cross-readers is this pass's reading
  of "harnesses that read that dir" (Cursor and Goose read `~/.claude/skills`, so a Claude copy lists them). The
  import report is unchanged (`OriginalOutcome` has the path; the UI joins it to the variant's roster — no duplicated
  removal). Fixture: `model.ts::onboardingReaders` mirrors the rule. Tests:
  `an_owned_copy_in_a_shared_dir_is_never_offered_whatever_its_attribution` (red before; Installed and
  Selected(codex)), `a_shared_copy_names_every_in_scope_reader`.
- **F5 (P2).** `skillPresentation.catalogToolLabel(t, labelById, key)` = catalog label, else `toolLabel(t, key)`;
  App's strip `labelOfTool` and its unknown-strip-tool labels use it; `skillToolChips` takes `t` and falls back the
  same way. Tests: `catalogToolLabel` describe (retired `clawdbot` → "Clawdbot", strip mark undetected) and
  `labels a retired Tool's row by its retained translation, undetected` (red before).
- **F6.** Not reproduced anywhere: `npx vitest run src` on `main` (read-only) exit 0 twice (53 files / 1147 tests at
  `8b42ce5`), and exit 0 once more under concurrent `cargo test` load (main had moved to `3bbab9b`: 54 / 1199); in this
  worktree exit 0 twice quiet, once under load, once in the gate (53 / 1152). The diff touches no test infrastructure
  or `sonner`; the teardown error is timing-dependent and, given the audit's two failing runs, pre-existing or
  environmental — the orchestrator's to handle.
- **F7.** The `export_bindings` test strips trailing spaces/tabs per line after `export` (deterministic; SHA identical
  across runs); `git diff --check` exits 0.
- **F8.** Kilo / Xum **project** dirs stay `.kilocode/skills` / `.mux/skills` **by recorded deferral**: no
  project-assignment relocation exists, so correcting them would strand existing assignments; BACKLOG #87 (opened by the
  orchestrator on `main`) carries it, and the registry comments say so. README: evidence wording distinguishes
  VERIFIED / LIKELY / UNKNOWN rows (WorkBuddy secondary-only); "no harness sees a skill twice" qualified to own-dir vs
  `~/.agents/skills`, with the cross-read case named; "nothing moves until it answers" corrected (registry-truth moves
  still run). CHANGELOG: **27** verified readers, **24** of them move (Codex, Goose, OpenHands already deploy there) —
  the old "10 named + 14 more" was the 24 movers but never said so; same duplicate qualification; registry-truth moves
  stated as independent of the setting. CONTEXT.md's Deployment relocation entry states the F1–F3 rules.
- **F9 — kept the field, asserted.** `BriefRow.brief` has no `allow(dead_code)`:
  `every_snapshot_citation_exists_in_the_brief` reads the brief and requires each `§N` heading, `#N` §1 row and `IN`
  implication to exist, and a `(LIKELY)` / `(UNKNOWN)` tag to appear on the cited row (sensitivity checked: an `I14`
  and a false `LIKELY` both fail). The snapshot doc now says what it is: hand-transcribed, bound to the brief by
  citations, not by a digest.
- **Proof (fresh):** `cargo test --all` ×2 → 887 passed each, bindings SHA identical; `cargo clippy --all-targets -D
  warnings` 0; `cargo fmt --check` 0; `npx vitest run src` exit 0 (1152); `npx tsc --noEmit -p tsconfig.app.json` 0;
  `git diff --check` 0; `npm run version:check && npm run check` exit 0.
- **Unproven / left:** the audit's §6 import near-misses (a renamed or symlinked identical original under the
  preference is removed) were not in the dispositions and are unchanged. The fixture's Propagation still writes at the
  first member's path (no layout simulation yet — the UI half's fixture work). Whole-operation layout immutability
  (audit §4) unchanged. No run against the real library. `main` has moved (`3bbab9b`, ticket 21's Import review touches
  onboarding UI) — expect a rebase touch on `OnboardingVariant` literals.

### Fix pass 3 (audit 2) (2026-11-04, orchestrator dispositions of `.scratch/round19/review/ticket-34-backend-audit-2.md`; delegated child; uncommitted)

The auditor's failing reproductions (`/tmp/t34-audit2-scratch/.../audit2*.rs`) are ported into
`core/tests/target_relocation.rs` under their finding names (`f10_*` … `f13_*`), with the extra cases each disposition
called for. **Red first:** run against the pre-pass production code (the scratch copy, whose `src/` differed from this
worktree only in the files touched here; new-API assertions stripped so it compiled) → **9 of 10 failed**; the 10th,
`f10_a_crash_after_materialising_is_completed_next_launch`, already passed (identical-destination replace was existing
behaviour) and stays as a crash-boundary guard. The audit's own run had the four original reproductions red. All green now.

- **F10 (P0) — a failed source removal is rolled back, never forgotten.** `target_relocation::relocate_skill_unlocked`
  now groups admitted rows into moves (keeping each row's own adapter for the re-key) and `settle_move` orders the writes
  **materialise → remove the source → settle the rows**. Before removing the source, ownership comes from
  `artifact_removal::artifact_is_owned_except(store, path, moving_row_ids)`, the same inventory as before, not counting
  the moving rows. If the removal fails, the move is **rolled back** (ADR-0002):
  - the destination is removed through the new fenced seam
    `artifact_removal::remove_relocation_destination_unlocked` (fenced to the adapter's current dir), unless a row owns
    it. `materialise` always leaves our bytes there, so whatever was at the destination before is irrelevant; a failed
    rollback is logged.
  - every moving row stays at its source as `SyncFailed` (`error`, chain in `last_error`, `synced_at` kept, so it still
    owns the source).
  - each row is counted `left_in_place`, and the reason is in the summary: new
    `RelocationSummary.incomplete: Vec<RelocationIncomplete { skill_id, tool, path, error: CommandError }>`. That
    needed `PartialEq, Eq` on `CommandError`. Bindings regenerated; the type is re-exported from the skills shim; the
    fixture returns `incomplete: []`.
  - the next launch's admission sees the `error` row (still owning) and retries it.

  Crash states:
  - between materialise and removal: the old row and artifact are intact, and our identical bytes at the destination
    are replaced on retry.
  - between removal and settlement: the row sits at an absent source while our bytes wait at the destination.
    `materialise` now **adopts** these: an absent source no longer refuses when the destination already holds identical
    managed content.

  So no artifact stays unowned past the next launch. Through a first sync, `global_sync::relocate_synced_skills` turns
  the attempted pair's `Synced` into `Failed` with the rollback's error; the batch-written copy is removed unless a
  sharer records it.

  Tests:
  - `f10_removal_failure_is_retried_or_reported_next_launch` (the audit's real 0500 permission failure). The first
    launch gives `{0, 1}` with one `incomplete`; the row is at the source as `error`, still owning; the destination
    was rolled back. The second launch gives `{1, 0, []}`. One assertion changed from the audit's: the audit asserted
    the row moved on the first launch; the disposition says it stays.
  - `f10_a_crash_after_source_removal_is_completed_next_launch` and
    `f10_a_crash_after_materialising_is_completed_next_launch` (crash boundaries).
  - `f10_rollback_keeps_a_destination_another_row_owns` (Codex's artifact survives Cursor's rollback).
  - `f10_a_first_sync_whose_old_copy_cannot_be_removed_fails_and_rolls_back`, with and without the fixture's Augment
    sharer.
- **F11 (P0) — Keep never orphans a superseded owned path.** In `global_sync::record_kept`, when the attempted pair's
  row owns a present artifact in one of the planned group's superseded dirs (`owns_a_superseded_artifact`), that
  artifact is first settled through the new `artifact_removal::settle_superseded_owner_unlocked`:
  - fenced to `superseded_dirs`. Not removed when another row owns it (`artifact_is_owned_except`), or when it is
    already absent.
  - on success the row is deleted, so the Keep's `DisabledRecorded` mints a fresh **non-owning** marker at the
    destination. The strip then shows `disabled` with no live copy behind it, and Enable starts clean.
  - on failure the row is kept at its old path, still owning, as `error` with the chain, and the pair reports `Failed`
    with the classified error, not `Declined`.
  - the kept foreign folder is never touched.

  Tests:
  - `f11_keep_then_enable_never_orphans_the_owned_former_copy`: refused move → Keep → Enable → Delete. Asserts
    `Declined`, the old copy gone, a non-owning disabled marker, the foreign bytes intact; after Enable no orphan;
    Delete with 0 failed rows.
  - `f11_keep_whose_former_copy_cannot_be_removed_fails_and_keeps_the_row`.

  Enable itself was not changed: after this fix nothing produces a `disabled` row that still owns a present artifact
  (the audit's optional Enable hardening was not in the disposition).
- **F12 (P1) — onboarding identity is physical.** I extracted `artifact_removal::artifact_identity(path)`: canonical
  parent + leaf as written (never the link's referent), with a lexical fallback when the parent cannot be resolved. It
  is the one rule:
  - `artifact_is_owned_except` compares with it (replacing `same_artifact_path`).
  - onboarding's `managed_target_key` is it. The exclusion set is now `HashSet<PathBuf>`, the same key for the
    invalid-entry exclusion; the Windows-only lowercasing went with the old key.
  - scan-root dedupe canonicalises each existing skills dir (lexical fallback). Copies keep their as-written,
    first-in-scope attribution and path. Every registry dir reaching one physical root is collected (`ScannedRoot`),
    and `readers_of` unions the in-scope installed readers of all of them, plus the attributed Tool.

  Tests (Selected and Installed scope each): `f12_owned_copy_is_excluded_through_a_symlinked_tool_dir` and
  `f12_unmanaged_copy_is_deduped_through_a_symlinked_tool_dir` (one variant, readers include Cursor and Claude Code).
- **F13 (P1) — no silent second copy.** In `propagation::propagate_global_rows`, the group's `pending_owner` is an
  installed member whose row owns a present artifact directly in one of its Tool's superseded dirs under the layout.
  While there is one, a **non-owner** partition's first sync into the group's root is skipped, never written, and no
  siblings are recorded. The skip is new: `PropagationSkip::RelocationPending { error }`, with
  `error = CommandError::RelocationPending { tool, path }` (`RELOCATION_PENDING`, naming the Tool and the path still to
  move). Owned partitions still update in place. Also added: the TS mirror regenerated, a `describeCommandError` branch,
  `en` key `errors.relocationPending` ("Waiting for {{harness}}'s folder to move — relaunch or re-deploy to finish"),
  and a test in `commandError.test.ts`.

  Test: `f13_propagation_nonowner_does_not_add_second_cursor_copy` (Cursor `v2` in place and Synced; Codex skipped
  with `RELOCATION_PENDING { cursor, .cursor/skills/skill }`; no `.agents` copy; no Augment sibling).
- **Docs.** CONTEXT.md's Deployment relocation entry covers the write order, rollback, adoption, Keep and
  `RELOCATION_PENDING`; CHANGELOG [Unreleased] adds three plain-language lines.
- **Proof (fresh):** `cargo test --all` 897 passed (run 1), bindings SHA-256 `5f0dc0cc…bf2` identical before and after
  each run. Run 2 had one failure, `mutation_guard::tests::listing_reconciles_when_no_mutation_is_in_flight`: its 5 s
  `try_serialized` retry budget ran out under guard contention (the `assert_serialized` HOLD tests). It is in no code
  path this pass touched, and 6 more full runs were 897/897. `cargo clippy --all-targets -- -D warnings` 0;
  `cargo fmt --check` 0; `npx vitest run src` exit 0 (53 files / 1153); `npx tsc --noEmit -p tsconfig.app.json` 0;
  `git diff --check` 0; `npm run version:check && npm run check` exit 0.
- **Unproven / left:**
  - F9's stale registry-header sentence (`tool_adapters/mod.rs` "a registry edit without a brief edit fails") was not
    in these dispositions and is unchanged.
  - The frontend Propagation fold (`reportOutcome.propagationEntries`) still treats every skip as silent, so
    `RELOCATION_PENDING` reaches no toast yet. That is the UI half's, along with showing `relocation.incomplete`.
  - Not covered by tests: a hard crash mid-move (simulated only by seeding the intermediate states), and Windows.
  - Not run against the real library.

### UI half — handoff (for the child after ticket 20)

- **Read**: `AppSettings.prefer_agents_standard_dir`, `AppSettings.hide_harness_specific_dirs`; per global
  `ToolInfoDto`: `shared_with` (under the preference the AGENTS-standard entry's `shared_with` is the collapsed row's
  roster — "AGENTS standard — Cursor, Codex, OpenCode +N"), `also_reads` (the cross-read warning, row + deploy
  confirmation note when the chosen set contains such a pair; no auto de-selection), `agents_standard_read`
  (`precedence`, `since`), `shadowed_by_own_dir` (the own-copy-shadows-ours flag). Project `ToolInfoDto`s carry empty
  / null values.
- **Write**: toggle → `commands.previewPreferAgentsStandardDir(enabled)` for the confirmation's count ("N deployments
  move") → `commands.setPreferAgentsStandardDir(enabled)` → adopt `result.settings`, report `result.relocation`
  (`relocated`, `left_in_place`, and each `incomplete` row's typed error — a move rolled back, retried next launch),
  then re-read `getToolStatus` and the catalog (`loadManagedSkills`) — the move changes
  rows and sharers. The sub-setting is `updateSetting({ key: "hide_harness_specific_dirs", value })`.
- **Present**: under the preference a first deploy records every installed reader and a pair Disable on one reader
  disables the shared artifact for all of them (shared-dir rule) — the Harnesses pane and Disable copy should say so.
  WorkBuddy is global-only. Xum is `mux` on the wire. A Propagation skip `relocation_pending` (`RELOCATION_PENDING
  { tool, path }`, `describeCommandError` has the copy) means a harness's copy is waiting to move; surface it (the fold
  is silent today).
- **Fixture**: `src/fixtures/registry.ts` carries `reads` / `alsoReads` / `globalOnly` and `model.ts::crossReads`;
  `previewPreferAgentsStandardDir` returns 0 and `setPreferAgentsStandardDir` only stores the flag. A `registry`
  scenario needs: the fixture's Kimi / Codex / Goose / OpenHands dirs brought to the backend's (re-pointing the
  Amp + Kimi shared-dir fixture tests at Codex + Goose), a layout-aware `globalRoot` / `globalSharers`, and a
  non-zero preview to show the confirmation. i18n keys for the pane copy are the UI child's.

## Comments

- 2026-11-02 — opened at the operator's approval, after the research child's brief landed (`1cb3d36`). Blocked on
  ticket 20 for its UI home; the backend registry half could start earlier in a worktree if 20 is slow.
- 2026-11-04 — operator grill (autonomous run through 31 → 20 ∥ 21 → 34 → 24 → 25 → 26; results reviewed at the end): Codex (and Goose, OpenHands) join the AGENTS-standard group globally as the brief decided — the operator's own `~/.codex/skills` targets relocate on first launch, accepted. **#83 preference "prefer the AGENTS standard dir" defaults ON.** Clawdbot / MoltBot / OpenClaw: the operator never installed them — retire the rows (no machine check needed).
- 2026-11-04 — backend half implemented by delegated child; UI half awaits ticket 20.
- 2026-11-04 — orchestrator fix pass: OpenClaw restored (Clawdbot/MoltBot its former dirs); #83 = on for fresh installs, pending offer for existing installs — the launch pass never moves on the default
- 2026-11-04 — fix pass 2 after audit: F1–F9 as the orchestrator disposed; re-check pending
- 2026-11-04 — fix pass 3 after audit 2: F10–F13 as the orchestrator disposed; re-check pending
- 2026-11-04 — **orchestrator: parked.** Three full Astra audits (`round19/review/ticket-34-backend-audit{,-2,-3}.md`), three
  fix passes (897 cargo tests, every prior finding closed), and each audit still found a new P0 in the relocation /
  ownership path — audit 3: **F15** directory aliases let relocation delete the destination, including the central
  skill itself, while reporting success (lexical identities in move / cleanup / rollback / Keep; the sync write seam
  can delete its own source through an aliased parent); F14 a refused crash adoption leaves a missing source `synced`;
  F16 Keep deletes the only copy of target-local operator edits. These are not fixable as a fourth pass: relocation
  needs an **alias-aware artifact identity used by every seam** (relocation, cleanup, rollback, Keep, the sync
  engine's overwrite) and a resumable operation with truthful status — a design, not a patch. Decision under the
  operator's rule 11 (a P0 that needs a design decision parks the ticket): **34 ships nothing in 1.3.1**. The branch
  `t3/registry-truth-pass` keeps all three passes (commit on the branch, unmerged) as the starting point; BACKLOG
  **#88** is the relocation redesign, **#89** the pre-existing F15 hazard on `main`'s Augment former dir. 26's
  CHANGELOG drops the registry lines. Open for the operator's review: whether the registry **facts and rows** (no
  dir corrections, no group moves, no preference) are worth splitting out for 1.3.1 — the orchestrator's read is no:
  the diff is 44 files and three fix passes deep; a clean cut is its own risk.
