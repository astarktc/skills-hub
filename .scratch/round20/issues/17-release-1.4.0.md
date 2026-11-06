# 17: Release 1.4.0

Status: ready-for-agent

**Blocked by:** 09, 10, 11, 12, 13, 14, 15, 16.

**Spec:** `.scratch/round20/spec.md` § Wave B spec; user story 58. Release procedure: AGENTS.md § Release.

**What to build:** 1.4.0 published from GitHub, with a CHANGELOG that explains the folder model and a smoke list
the operator can follow on his own machine.

## Acceptance criteria

- [ ] **Final release review** of the whole Wave B diff since `v1.3.4`: Astra and Sol in parallel, each given the
      removal-safety principle. Its findings are disposed of and recorded under `.scratch/reviews/`.
- [ ] **Copy pass** over every new string with `impeccable` (orchestrator).
- [ ] **CHANGELOG `[1.4.0]`:**
  - the folder model in plain words;
  - the default-on preference and the launch offer;
  - every corrected folder (Q12) and the OpenClaw retirements;
  - link vs copy;
  - project overrides;
  - the overwrite ask;
  - QwenPaw deferred (#118).
- [ ] **Smoke list** in the CHANGELOG, including:
  - first launch on a real machine: accept the offer, then `ls -la ~/.codex/skills ~/.agents/skills ~/.pi/agent/skills
    ~/.claude/skills` before and after (Codex's and Pi's skills in `.agents`, the old links gone, Claude Code
    untouched);
  - Pi no longer lists duplicates;
  - turn the preference off and back on.
- [ ] `npm run version:set 1.4.0`, `npm run version:check && npm run check`, `git fetch && git rebase origin/main`,
      push `main` (the push is the release).
- [ ] After publishing: the BACKLOG order line, the effort's Status and closure per `docs/agents/issue-tracker.md` §
      Lifecycle, and a handoff.
