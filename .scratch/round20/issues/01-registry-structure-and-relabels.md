# 01: Registry structure and relabels

Status: ready-for-agent

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

- [ ] **Typed roots.**
  - Every registry path (global dir, former dirs, detect dirs) is a `{ root, rel }` pair, with root one of home,
    roaming app data or local app data, plus an optional per-OS override.
  - `core/environment.rs` resolves a `HostRoots { home, roaming, local }` once. Core functions that resolve registry
    paths take it explicitly (or `home` plus the resolved roots); no new `dirs::*` caller outside `environment.rs`.
  - The command seam builds it. Every existing row resolves to exactly the paths it does today, pinned by a test over
    all rows.
- [ ] Unit tests resolve a Windows-roaming override and a local-app-data override with a substituted `HostRoots`. No
      production row uses an override yet; Reasonix in ticket 06 is the first.
- [ ] **Optional project dir.**
  - The project skills dir is optional, and a `None` row is absent from `project_tool_entries`.
  - Every current row keeps its dir.
  - `project_relative_skills_dir_for_every_tool` covers the optional case.
- [ ] **Reader-only row kind.**
  - It has detect dirs and no global skills dir. `global_tool_entries` / `ToolInfoDto` expose reader-only harnesses
    as readers of the `.agents/skills` entry, with their detected state (a specta-derived field, bindings regenerated,
    re-exported from the shim).
  - It's never a selectable global target, never in the saved selection or `installed_tools_v1` diffing that raises
    the new-harness ask, and never mints a target row.
  - It's absorbed at project scope by the AGENTS group.
  - Tests cover each of these. One test-only reader-only row, or the first real one if trivial, proves the shape.
- [ ] The Harnesses pane lists the reader-only harnesses under the `.agents/skills` entry ("Read by …") with detected
      state and no switch. This is invisible until ticket 05 adds rows; the fixture can show it with a fixture-only row.
- [ ] **Relabels.**
  - Mux → "Xum", Windsurf → "Devin (Windsurf)", Copaw → "QwenPaw", and Zencoder → "Zencoder / Zenflow" (or the copy
    pass's equivalent).
  - Registry `display_name` and i18n `tools.<key>` both change; every `as_key` is unchanged.
  - A test round-trips the stored keys `mux`, `windsurf`, `copaw`, `zencoder`.
- [ ] **Deferred detect dirs are absent** and pinned by a test naming #108: `.qwenpaw`, `.gemini/antigravity-cli`,
      `.config/devin`.
- [ ] **Derived count.**
  - The AGENTS group's project label count is computed (constituents + reader-only members) through an i18n string
    with `count`; no literal "(9 harnesses)" remains.
  - A Rust test counts README supported-tools table rows against the registry (reader-only rows included) and fails
    on drift.
- [ ] Env-var homes are not honoured. Row comments name them where they exist (e.g. Kimi's), and the README carries a
      one-line note.
- [ ] AGENTS.md's "New AI tool adapter" invariant names typed roots, the optional project dir, reader-only rows, and the
      README-count test. CONTEXT.md gains **Reader-only harness**.
- [ ] `npm run version:check && npm run check` passes. A draft PR's `gh pr checks` proves the Windows build if any
      `cfg(windows)` code was added.

## Comments
