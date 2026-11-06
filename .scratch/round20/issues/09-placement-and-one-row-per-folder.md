# 09: Placement and one Deployment row per folder

Status: ready-for-agent

**Blocked by:** None (can start immediately).

**Spec:** `.scratch/round20/spec.md` § Wave B spec → Implementation decisions § Placement, § Deployment rows are per
folder; grill Q2, Q18, Q26.

**What to build:** the groundwork the rest of Wave B stands on.

- **Placement.** A pure backend module answers, for one scope, which **Deployment folder** serves each harness and
  in which deploy mode. In this ticket every harness is served by its own folder, as the registry has it today; the
  shared-folder rule arrives in 12.
- **Rows per folder.** Global Deployment rows and project assignments are recorded once per (skill, folder key), never
  once per shared-dir sibling.
- **Selection.** The saved selection is stored as folder keys.

Nothing a user sees changes. Amp and Kimi, the only shared pair today, record one row for `.config/agents/skills`
instead of two.

## Acceptance criteria

- [ ] **Placement module in core, pure** over registry, read facts, settings and installed set. For one scope it
      answers:
  - each harness → its folder key and folder path;
  - each folder → deploy mode (today: the registry capability) and its served harnesses, other installed readers and
    reader-only harnesses, **detected only**, plus the undetected members kept separately for the pane.
- [ ] **Folder key rule:** `agents_skills` for `.agents`, `claude_code` for `.claude`, otherwise the harness's own
      key. A dir several entries share takes the virtual group entry when one owns it, else the first entry in
      registry order. Table tests cover every dir the registry shares.
- [ ] **One row per folder.**
  - The global sync batch, Propagation, Disable/Enable, delete, Onboarding import's first sync and project sync
    record and read one row per (skill, folder key).
  - The per-sharer record fan-out (D34 amendment) is retired.
  - Existing tests that pinned per-sharer rows are rewritten to the folder rule, not deleted.
- [ ] **Existing per-sharer rows on a dir shared today** (Amp + Kimi) are read correctly. Collapsing them to one row is
      left to the redeploy (10), not done here. Until then every read path treats sibling rows as one folder.
- [ ] **Saved selection as folder keys.**
  - It moves to a new versioned settings key holding folder keys.
  - The old `global_selected_tools_v1` is read once and translated: a folder is on if any harness it serves was
    selected (Q26).
  - Tests cover the translation, including a corrupt old value.
- [ ] **Catalog on the wire.** `ToolInfoDto` carries each harness's folder key. The frontend reads it and derives
      nothing from paths. Bindings are regenerated.
- [ ] **CONTEXT.md** gains **Deployment folder** (folder key) and **Placement**. The "Shared skills dir group" and
      "Effective global harness set" entries are updated to the folder rule.
- [ ] `npm run version:check && npm run check` is green.

## Notes

- Highest-risk shared files (AGENTS.md § Worktree safety): `global_sync.rs`, `project_sync.rs`, `artifact_removal.rs`,
  `commands/mod.rs`. Keep the change mechanical where possible.
- Everything later builds on placement (10's redeploy, 12's serving rule, 14's mode levels, 15's project overrides), so
  keep its interface narrow: inputs in, answers out, no store access.
