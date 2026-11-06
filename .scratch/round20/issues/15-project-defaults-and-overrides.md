# 15: Project defaults and per-project overrides

Status: ready-for-agent

**Blocked by:** 11, 13, 14.

**Spec:** `.scratch/round20/spec.md` § Wave B spec → § Project defaults and overrides; grill Q22; user stories 47–52.
Study: `docs/design-inputs/2026-11-06-t3code-project-defaults-and-overrides.md`. Finishes BACKLOG **#109** (project
half) and **#84** (project level).

**What to build:** every project inherits the global shared-folder preference, both-folders choice and deploy mode.
Any project can override any of them, and every row shows its effective value and where it comes from. The project
matrix's columns are the folders serving that project's harnesses, and the project tools modal uses folder cards.
Every edit, reset or bulk reset previews and runs as one redeploy action with a report.

## Acceptance criteria

- [ ] **Sparse overrides, backend-owned.**
  - A project stores only the keys it overrides: preference, both-folders choice, deploy mode, per-folder mode.
  - Absent means inherit. A present key is an override even when it equals the default.
  - "Use project defaults" deletes keys.
  - Storage is a project-settings table or columns. Consider `migrate_legacy_db_if_needed` if the schema changes.
- [ ] **Resolution:** placement at project scope reads folder › project › all projects › built-in, and answers each
      value's origin.
- [ ] **One settings surface with a scope selector** ("All projects / {project}").
  - Each row shows its effective value and an origin icon that opens the chain.
  - An override is marked and has Reset.
  - A default row lists the projects that override it, with links, "Reset it" and "Reset all" (T3 Code pattern; not
    its multi-machine "all environments").
- [ ] **Consequential edits** (an edit, a reset, a bulk reset) each run through a confirmation with "would move N" and
      one redeploy action with a report under `runAction`. Projects whose folder is gone are reported `folder_missing`.
- [ ] **Project matrix.** The columns are that project's folders (the AGENTS group generalised). Cells, legend and chips
      come from `projectPresentation`.
- [ ] **Project tools modal:** folder cards, detected harnesses only.
- [ ] **Tests:**
  - sparse storage (absent = inherit, an equal value stays an override);
  - resolution and origin;
  - reset and bulk reset;
  - a project override of the preference moving that project only;
  - frontend folds and hook flows.
- BACKLOG #109 and #84 left the queue when tickets 12, 14 and 15 opened.
- [ ] `npm run version:check && npm run check` is green.
