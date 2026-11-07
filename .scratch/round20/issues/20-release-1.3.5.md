# 20: Release 1.3.5

Status: ready-for-agent

**Blocked by:** 09 (done), 18 (done), 19, 10.

**Spec:** `.scratch/round20/spec.md` § Wave B → "Release split". Release procedure: AGENTS.md § Release.

**What to build:** 1.3.5 published from GitHub. It contains:
- one Deployment row per folder (09);
- ownership checked at the moment of acting (18);
- skill names that stay inside the Library (19);
- the redeploy, with its journal, its launch offer and the folder corrections to a harness's own folder: Antigravity,
  Kimi Code, Kilo, Xum and Devin (10, minus the `.agents` moves held for 1.4.0).

## Acceptance criteria

- [ ] **Final release review** of the whole diff since `v1.3.4`. Astra and Sol review in parallel, each briefed with
      the principle only:
  - the app removes or replaces a folder only when it can show, at the moment it acts, that the folder is what it
    wrote, and only inside a skills folder it manages;
  - deploy first, then remove;
  - an answer applies to exactly the path that was asked about.

  The findings are disposed of and recorded under `.scratch/reviews/`.
- [ ] **Copy pass** over every new string (`impeccable`, by the orchestrator).
- [ ] **CHANGELOG `[1.3.5]`**, in plain words:
  - **Ownership:** folders the operator replaced by hand are now asked about, never overwritten or deleted
    silently. Delete reports "left in place".
  - **Names:** a repo can't place files outside the Library.
  - **Corrected harness folders,** and the launch offer that moves them.
  - **Shared folders:** Amp and Kimi Code share one switch while they share a folder. Kimi's correction separates
    them.
  - **Old copy-mode Deployments** (Deep Agents Code, and Windows without links) are left in place or asked about,
    once.
- [ ] **Smoke list** for the operator:
  - **Nothing moves:** install over 1.3.4 and confirm no launch offer appears. Nothing on this machine is in a
    corrected folder.
  - **My Skills is unchanged:** confirm with `ls -la ~/.claude/skills ~/.codex/skills ~/.pi/agent/skills
    ~/.agents/skills` before and after.
  - **Delete leaves foreign folders:** Delete a throwaway skill after replacing one of its links by hand. The folder
    is left in place, and the report says so.
- [ ] `npm run version:set 1.3.5`, then `npm run version:check && npm run check`, then `git fetch && git rebase
      origin/main`, then push `main`. The push is the release.
- [ ] **After publishing:** the BACKLOG order line, the spec's Status, and a handoff. The effort stays open for 1.4.0.
