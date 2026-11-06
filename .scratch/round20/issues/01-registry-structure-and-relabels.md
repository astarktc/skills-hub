# 01: Registry structure and relabels

Status: done — 8401dfa

**Blocked by:** None (can start immediately).

**Spec:** `.scratch/round20/spec.md` § Implementation Decisions → Registry structure (typed roots, env-var homes,
optional project dir, reader-only harness, derived counts, relabels only, Deep Agents Code note is ticket 06's).

**What to build:** the Harness catalog can describe the wider roster, without moving any existing Deployment.

- A registry path carries a typed root, with per-OS overrides.
- A harness may have no project dir.
- A harness may be **reader-only**: detected, listed as a reader of the `.agents/skills` entry, never a target.
- The stale labels read right.
- The AGENTS group's "(N harnesses)" count derives from the registry.
- The README table is test-enforced.

## Acceptance criteria

- [x] **Typed roots.**
  - Every registry path (global dir, former dirs, detect dirs) is a `{ root, rel }` pair, with root one of home,
    roaming app data or local app data, plus an optional per-OS override.
  - `core/environment.rs` resolves a `HostRoots { home, roaming, local }` once. Core functions that resolve registry
    paths take it explicitly (or `home` plus the resolved roots); no new `dirs::*` caller outside `environment.rs`.
  - The command seam builds it. Every existing row resolves to exactly the paths it does today, pinned by a test over
    all rows.
- [x] Unit tests resolve a Windows-roaming override and a local-app-data override with a substituted `HostRoots`. No
      production row uses an override yet; Reasonix in ticket 06 is the first.
- [x] **Optional project dir.**
  - The project skills dir is optional, and a `None` row is absent from `project_tool_entries`.
  - Every current row keeps its dir.
  - `project_relative_skills_dir_for_every_tool` covers the optional case.
- [x] **Reader-only row kind.**
  - It has detect dirs and no global skills dir. `global_tool_entries` / `ToolInfoDto` expose reader-only harnesses
    as readers of the `.agents/skills` entry, with their detected state (a specta-derived field, bindings regenerated,
    re-exported from the shim).
  - It's never a selectable global target, never in the saved selection or `installed_tools_v1` diffing that raises
    the new-harness ask, and never mints a target row.
  - It's absorbed at project scope by the AGENTS group.
  - Tests cover each of these. One test-only reader-only row, or the first real one if trivial, proves the shape.
- [x] The Harnesses pane lists the reader-only harnesses under the `.agents/skills` entry ("Read by …") with detected
      state and no switch. This is invisible until ticket 05 adds rows; the fixture can show it with a fixture-only row.
- [x] **Relabels.**
  - Mux → "Xum", Windsurf → "Devin (Windsurf)", Copaw → "QwenPaw", and Zencoder → "Zencoder / Zenflow" (or the copy
    pass's equivalent).
  - Registry `display_name` and i18n `tools.<key>` both change; every `as_key` is unchanged.
  - A test round-trips the stored keys `mux`, `windsurf`, `copaw`, `zencoder`.
- [x] **Deferred detect dirs are absent** and pinned by a test naming #108: `.qwenpaw`, `.gemini/antigravity-cli`,
      `.config/devin`.
- [x] **Derived count.**
  - The AGENTS group's project label count is computed (constituents + reader-only members) through an i18n string
    with `count`; no literal "(9 harnesses)" remains.
  - A Rust test counts README supported-tools table rows against the registry (reader-only rows included) and fails
    on drift.
- [x] Env-var homes are not honoured. Row comments name them where they exist (e.g. Kimi's), and the README carries a
      one-line note.
- [x] AGENTS.md's "New AI tool adapter" invariant names typed roots, the optional project dir, reader-only rows, and the
      README-count test. CONTEXT.md gains **Reader-only harness**.
- [x] `npm run version:check && npm run check` passes. A draft PR's `gh pr checks` proves the Windows build if any
      `cfg(windows)` code was added.

## Comments

- 2026-11-06 (implementer, branch `t3/registry-structure-relabels`, draft PR #13): all criteria built and the
  gate passes. Shape notes for 02/05/06: reader-only rows live in their own table `READER_ONLY_HARNESSES`
  (`ReaderOnlyHarness { key, display_name, detect_dirs, reads: VirtualGroup }`), never a `ToolId` — so no target path
  can reach one; the global catalog lists them on the `.agents/skills` entry (`ToolInfoDto.readers`) and the project
  catalog absorbs them into the group's `constituents`. Registry literals use `home(".x/skills")`; an override is
  `home(".reasonix/skills").overridden(&[OsOverride { os: HostOs::Windows, root: HostRoot::RoamingAppData, rel:
  "reasonix/skills" }])`. `group_label` is gone: the count is the frontend's `projectToolLabel` (i18n
  `projectsSurface.groupLabel_one/_other`). The README table gained a project-dir column (`—` for none; reader-only
  rows read `—` in both dir columns) and is parsed by `the_readme_table_lists_every_registry_row` (key, label, default
  dirs). Placement paths reach the UI pre-spelled (`~/…`, absolute under a non-home root).
- 2026-11-06 (orchestrator): review round 1 (GPT-6 Astra, high) fix-then-ship, no P1; every existing row's paths
  independently confirmed unchanged. One P2 fixed by the orchestrator in `7a02003` (a detected reader-only harness makes
  its group installed, so the group's first global deployment does not wait for `~/.agents`; regression
  `a_detected_reader_makes_its_group_deployable_before_the_group_folder_exists`). CI green (rust, rust-windows, web).
  Merged as `8401dfa` (PR #13). Review record: `.scratch/reviews/round20-t01-registry-review.md`.

