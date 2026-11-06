# 07: Rows III

Status: ready-for-agent

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
