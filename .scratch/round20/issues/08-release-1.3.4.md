# 08: Release 1.3.4

Status: done — ed8e442 (v1.3.4)

**Blocked by:** 01–07.

**What to build:** Wave A ships as **1.3.4** (operator 2026-11-06).

## Acceptance criteria

- [ ] Every lane is merged and every ticket carries `done — <sha>`.
- [ ] The `git diff` from before the merge to `HEAD` over `src-tauri/src` and `src/` shows no unintended reverts.
- [ ] A copy pass over every new UI string (warning, collapsed group, move confirmation and report, Clean Now toast).
- [ ] A final adversarial review (GPT-6 Astra, high) of `v1.3.3..main`; its findings are fixed or disposed in the
      review file.
- [ ] CHANGELOG `[1.3.4]` names:
  - every new harness and the reader-only members;
  - the relabels, which move nothing;
  - the cross-read warning;
  - the collapsed "Not detected" group;
  - the Library move's new behaviour and its cloud-folder warning;
  - Clean Now's in-use rule.

  Also a smoke list for the operator:
  1. move the Library to an empty folder and back, with `ls -la` on a harness link;
  2. the warning with Cursor + Claude Code;
  3. a reader-only harness shown under `.agents/skills`.
- [ ] The README table, the supported-tools count and the Harnesses screenshot (if the pane changed visibly) are
      recaptured from the fixture, dark theme only.
- [ ] `npm run version:set 1.3.4`, then `npm run version:check && npm run check`. Then `git fetch && git rebase
      origin/main`, and push `main`; `auto-tag.yml` releases it.
- [ ] The release run is green, and `updater.json` lists all five platforms.
- [ ] Wave A residue is recorded in BACKLOG or the spec's Status, and a handoff points at Wave B's grill.

## Comments
- 2026-11-06 (orchestrator): every lane is merged; the CHANGELOG `[1.3.4]` and the smoke list are committed. The final
  review (Astra) failed on the ChatGPT usage limit before it started, so the release is held. See
  `handoffs/2026-11-06-1.3.4-ready-awaiting-final-review.md`.
- 2026-11-06 (orchestrator): released as **v1.3.4**: tag `v1.3.4` = `ed8e442`, release run 42246044318, published
  2026-11-06 16:09 UTC, 14 assets. `updater.json` lists darwin-aarch64/x86_64, linux-x86_64 and windows-aarch64/x86_64.
  The operator resumed the final review after a false-positive usage-limit error. It was fix-then-ship and its fixes
  are in PR #20 (`reviews/round20-1.3.4-release-review.md`). The round's worktrees and branches are removed; only
  `t3/registry-truth-pass` is kept.

