# 07: Rows III

Status: done — d954846

**Blocked by:** 06. There's no content dependency, but the rows share the registry file.

**Spec:** `.scratch/round20/spec.md` § Registry structure; the brief § Part 1 (table A, "Not in either list"), § Part 2;
the addendum § Summary and § Candidates for Muse Code, Poolside, Dirac and Stakpak.

**What to build:** the remaining twelve recommended harnesses become rows.

## Rows

- Code Studio (`.codestudio/skills`; detect-dir flag)
- CodeArts Agent (`.codeartsdoer/skills`; detect-dir flag)
- Qoder CN (the IDE, ex-Lingma: `.lingma/skills`)
- Qoder CN CLI (`.qoder-cn/skills`)
- Kimchi (`.config/kimchi/harness/skills`)
- AstrBot (`.astrbot/data/skills`; desktop runtime only, which the comment says)
- oh-my-pi (`.omp/agent/skills`, project `.omp/skills`)
- Letta Code (`.letta/skills`; project dir = the AGENTS group)
- Muse Code (`.config/muse/skills`)
- Poolside (`.config/poolside/skills`)
- Dirac (`.dirac/skills`)
- Stakpak (`.stakpak/skills`)

Muse Code, Poolside and Dirac carry `group: AgentsStandard` (project `.agents/skills`, addendum § Summary 2). Dirac is
**not** an alias of Cline.

## Acceptance criteria

- [ ] Every row is re-verified against its source before it's added (as in ticket 05; deviations go in Comments).
- [ ] Each row has everything ticket 05 lists.
- [ ] Dirac's precedence is `Shared` (the shared copy wins).
- [ ] The AGENTS group count and the README-count test pass.
- [ ] `npm run version:check && npm run check` passes.

## Comments

- 2026-11-06 (implementer, branch `t3/rows-iii`, draft PR #15). All twelve rows are built, and the gate passes. Each row
  was re-verified on 2026-11-06.
  - Keys: `code_studio`, `codearts_agent`, `qoder_cn`, `qoder_cn_cli`, `kimchi`, `astrbot`, `oh_my_pi`, `letta_code`,
    `muse_code`, `poolside`, `dirac`, `stakpak`.
  - Letta Code joins the AGENTS group with Muse, Poolside and Dirac, so the roster is 13.
  - Deviations and notes:
    - **AstrBot has no project dir.** The brief's Part 1 cell says `data/skills` relative to the AstrBot root. The
      only detectable runtime (desktop) fixes that root at `~/.astrbot`, and its workspace skills are per chat
      session. No repository ever holds them.
    - **oh-my-pi's precedence is `Own`.** It was LIKELY; omp.sh/docs/skills now says it, and the brief row and its T41
      source note were edited in the same commit. Its Pi user-dir read (`enablePiUser`) is `Unknown`, because the
      path is not among the listed locations.
    - **Kimchi's "if installed" cross-reads are encoded `Default`.** A pair needs the other harness installed anyway.
      `kimchi_reads_an_installed_agents_folder_but_not_claudes_by_default` pins it. Kimchi checks each agent's
      project dir first and falls back to home. That fallback is noted, but the brief's `Y` is kept.
    - **Code Studio's help URL** (help.syncfusion.com/code-studio/customization/skills) now redirects to
      docs.boldcreate.ai, Syncfusion's new product. The Code Studio pages still read as before through the search
      index (skills page plus the user-settings "Skill File Locations" default).
    - Detect-dir flags ship as specified for Code Studio `.codestudio` and CodeArts `.codeartsdoer`.
    - The Qoder CN CLI's project dir `.qoder/skills` is shared with the Qoder row (the Trae / Trae CN pattern).
- 2026-11-06 (orchestrator): merged as `d954846` (PR #15) after CI went green. The orchestrator merged `main` (05) into it, keeping both appended blocks (`d9beb43`), with the gate green. The combined adversarial row review for 05–07 follows once 06 lands.
