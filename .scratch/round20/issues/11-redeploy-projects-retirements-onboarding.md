# 11: Redeploy's project leg, the OpenClaw retirements, and onboarding's former-dir scan

Status: ready-for-agent

**Blocked by:** 10.

**Spec:** `.scratch/round20/spec.md` § Wave B spec → § Redeploy (project leg, retired keys, onboarding); grill Q6, Q7,
Q10, Q12; user stories 13, 14, 16, 17. Absorbs BACKLOG **#87**.

**What to build:** project Deployments in corrected folders move in the same redeploy action as global ones.
Clawdbot and MoltBot become OpenClaw everywhere. Import finds skills the operator put in a harness's former folder.

## Acceptance criteria

- [ ] **Project leg.** The redeploy compares every project assignment's place with placement at project scope, and
      moves each difference under 10's rules: deploy first, adopt, ask, journal, provable removal, disabled
      re-record, Q24 collapse.
  - Each project is gated by `project_ops::project_folder_exists`: `skipped { folder_missing }`, row left as recorded.
  - Project target writes keep `TargetParents::WithinRoot`.
  - One `RedeployReport` spans both scopes. The project world refetches the selected project's view after it.
- [ ] **Project-dir corrections (Q12).**
  - Goose, OpenHands and Zencoder go to project `.agents/skills`, joining the AGENTS group (`group` literal; the
    group/fact test holds).
  - Kilo goes to `.kilo/skills` and Xum to `.xum/skills`.
  - Devin goes to `.devin/skills`.
  - `project_tools` rows for a harness absorbed into the group are re-keyed to the group entry by the redeploy.
- [ ] **Retirements (Q6).**
  - The Clawdbot and MoltBot `ToolId`s are deleted; their global and project dirs become OpenClaw's former dirs.
  - The redeploy re-keys rows, the saved selection, `project_tools` and assignments to `openclaw`, merging into an
    existing row by the Q24 rule.
  - A stored unknown key stays tolerated (the existing tolerance tests stay).
  - The README table and the registry tests are updated. Their i18n labels may stay for the readable-name fallback.
- [ ] **Onboarding (Q10).** The scan includes each harness's former dirs as that harness's, excluding our own links by
      physical identity. The plan and the import re-derive under the same scope. Import's reader roster for a shared
      dir comes from placement.
- [ ] **Tests:**
  - project moves with `folder_missing`;
  - group re-key of `project_tools`;
  - retired-key re-key and merge, global and project;
  - an onboarding scan of `~/.codex/skills` finding a foreign skill and ignoring our link;
  - prior art: `project_sync` `folder_missing` tests and `onboarding_import` tests.
- BACKLOG #87 left the queue when this ticket opened.
- [ ] `npm run version:check && npm run check` is green.

## Notes

- Review: Astra and Sol in parallel. The project leg is a removal path (Q28).
