# 19: A skill's name is always one folder name, from install onward

Status: ready-for-agent

**Blocked by:** 18 (it introduces the name rule, `placement::deployment_path` / `is_folder_name`).

**Source:** ticket 18, fix pass 2 report (2026-11-06), verified by the orchestrator: `install_finalize.rs:188`
(`ensure_name_available`) and `:211` join the requested name, or the SKILL.md `name:`, onto `central_dir` without
checking it. This predates round 20.

**What to build:** a skill's name becomes a folder in the Library and in every harness. Today only deployment refuses
a name that isn't one plain folder name (ticket 18). Install doesn't: a repo whose SKILL.md says `name: ../../x`, or a
name typed in the Add flow, can put the skill's Library copy outside the Library folder. Writes never overwrite an
existing folder (`SkillExists`), but bytes from an untrusted source should never land outside the Library.

## Acceptance criteria

- [ ] **Every path that names a skill applies the one name rule** (ticket 18's `is_folder_name`): no empty name, no
      `.` or `..`, nothing absolute or drive-relative, no separator, no NUL. That covers:
  - `finalize_install` (the requested name and the SKILL.md name);
  - Explore's install;
  - onboarding import;
  - Re-point if it renames;
  - Edit.
- [ ] **A SKILL.md name that fails the rule** falls back to the derived name, the way an existing name already falls
      back today. A **requested name that fails** is a typed refusal (ADR-0001), with a `describeCommandError` branch
      and an i18n key, raised before any acquisition.
- [ ] **The Library copy is admitted physically inside the central repo** at finalize (the removal fence's claim),
      not only lexically.
- [ ] **Existing records whose `central_path` is outside the central repo** are reported by Skill health as
      Unlocatable-style, never deployed from. Decide whether one exists in practice; a migration is not needed unless
      one does.
- [ ] **Tests:** a SKILL.md with `name: ../../x` installs under the derived name inside the Library; a requested
      `../x` is refused before download; Windows separators.
- [ ] `npm run version:check && npm run check` is green.
