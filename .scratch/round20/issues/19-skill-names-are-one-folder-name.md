# 19: A skill's name is always one folder name, from install onward

Status: done — fe804958

**Blocked by:** 18 (it introduces the name rule, `placement::deployment_path` / `is_folder_name`).

**Source:** ticket 18, fix pass 2 report (2026-11-06), verified by the orchestrator: `install_finalize.rs:188`
(`ensure_name_available`) and `:211` join the requested name, or the SKILL.md `name:`, onto `central_dir` without
checking it. This predates round 20.

**What to build:** a skill's name becomes a folder in the Library and in every harness. Today only deployment refuses
a name that isn't one plain folder name (ticket 18). Install doesn't: a repo whose SKILL.md says `name: ../../x`, or a
name typed in the Add flow, can put the skill's Library copy outside the Library folder. Writes never overwrite an
existing folder (`SkillExists`), but bytes from an untrusted source should never land outside the Library.

## Acceptance criteria

- [x] **Every path that names a skill applies the one name rule** (ticket 18's `is_folder_name`): no empty name, no
      `.` or `..`, nothing absolute or drive-relative, no separator, no NUL. That covers:
  - `finalize_install` (the requested name and the SKILL.md name);
  - Explore's install;
  - onboarding import;
  - Re-point if it renames;
  - Edit.
- [x] **A SKILL.md name that fails the rule** falls back to the derived name, the way an existing name already falls
      back today. A **requested name that fails** is a typed refusal (ADR-0001), with a `describeCommandError` branch
      and an i18n key, raised before any acquisition.
- [x] **The Library copy is admitted physically inside the central repo** at finalize (the removal fence's claim),
      not only lexically.
- [x] **Existing records whose `central_path` is outside the central repo** are reported by Skill health as
      Unlocatable-style, never deployed from. Decide whether one exists in practice; a migration is not needed unless
      one does.
- [x] **Tests:** a SKILL.md with `name: ../../x` installs under the derived name inside the Library; a requested
      `../x` is refused before download; Windows separators.
- [x] `npm run version:check && npm run check` is green.

## Comments

- 2026-11-06 (orchestrator): **done, merged as `fe804958` (PR #23)**, after two Astra review rounds
  (`.scratch/reviews/round20-t19-names-review.md`).
  - **What shipped:**
    - One name rule (`placement::is_folder_name`) applied at every install and rename path.
    - `SKILL_NAME_INVALID`, raised before any download.
    - Physical admission of the destination's parent.
    - Exclusive publication through `core/no_replace.rs`: Linux `renameat2(RENAME_NOREPLACE)`, macOS
      `renameatx_np(RENAME_EXCL)`, Windows `MoveFileExW` without replace or copy.
    - Records with a bad name quarantined as Unlocatable, with Remove as the repair, Edit included.
  - **Gate on the merged main:** 1,435 frontend tests; 1,150 Rust tests, 1 ignored.
  - **Left out:** concurrent same-user-process hardening is out of the stated scope, now BACKLOG #119.
  - The children's per-pass entries stayed uncommitted in their worktrees; this entry stands in for them.
