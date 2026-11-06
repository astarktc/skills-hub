# 10: Redeploy (global scope) and the registry corrections

Status: ready-for-agent

**Blocked by:** 09, 18 (the occupant check).

**Spec:** `.scratch/round20/spec.md` § Wave B spec → Implementation decisions § Redeploy, § Registry corrections; grill
Q2–Q5, Q8, Q9, Q12; user stories 1–12, 15, 18, 19.

**What to build:** one mechanism moves global Deployments whenever placement says they belong somewhere else. The
first users are the Q12 registry corrections. At first launch of the new build the app offers to move them, runs the
move as one action with progress, and reports exactly what happened. It never deletes anything that isn't provably a
duplicate of ours.

## Acceptance criteria

- [ ] **`core/redeploy.rs` replaces `core/target_relocation.rs`.** The old module is deleted; its tests are ported where
      they still apply. Each recorded global Deployment's (path, mode, folder key) is compared with placement's answer,
      and every difference is a move. No cause is named in the module.
- [ ] **Preview:** a read-only entry point answers counts per scope and per cause (correction / migration).
- [ ] **Mutation:** an entry point under `mutation_guard::serialized`, with progress over a Channel and no cancel. The
      unlocked seam is for composites (12 and 14 call it inside their own guarded section). The layout is read once
      per operation.
- [ ] **Per move, in order:**
  1. **Deploy** to the new place through `sync_engine`'s capability-aware overwrite path.
     - Ours or identical content there is **adopted**.
     - Different content is a `TARGET_EXISTS` row for the one overwrite ask. A Keep answer leaves the move undone and
       still pending.
  2. **One transaction** switches the row and writes a **journal entry**: the old artifact, its physical identity and
     the row that owned it.
  3. **Remove the old artifact** through `artifact_removal` and the removal fence, only when it is an owned link that
     resolves (by physical identity) to its skill's central copy, or a copy whose Content identity equals the
     Library's, **and** no other live row records it.
     - A differing copy raises the **edited-copy question** as a row of the same ask: delete / leave. Leaving disowns
       it and the report names its readers.
     - A successful removal clears the journal entry.
- [ ] **Journal finish.** Every run and the launch check finish journal entries first: remove-or-report, acting only
      while the row and both identities still match, otherwise reporting the entry left behind (`library_move`'s resume
      rule).
- [ ] **Disabled and never-synced `error` rows** are re-recorded at the new place with nothing written.
- [ ] **Row collapse (Q24).** When several rows land in one folder, the folder row is live if any had the skill live,
      and `disabled` only if all did. Each disable that no longer holds is report data naming the harness.
- [ ] **`RedeployReport` on the wire as itself.**
  - Per move: `moved | adopted | unchanged | left_old { edited } | kept { declined } | skipped { disabled } |
    failed { error }`.
  - It also carries the duplicates left behind (with readers), the disables that no longer hold, and the journal
    entries finished or left behind.
  - `reportOutcome` folds it, and pending or left-behind entries surface; the fold no longer skips them.
- [ ] **Launch check.** At startup: journal finish, then preview. If there is work, the app offers it once through
      `offerDialog`, and accepting runs the mutation under `runAction`. If declined, Settings → Harnesses shows a
      "Move N Deployments" line backed by the preview. Nothing else moves at launch.
- [ ] **Registry corrections (Q12 table in the grill log).**
  - Codex, Goose, OpenHands and Zencoder go to `.agents/skills`.
  - Antigravity goes to `.gemini/config/skills`, with detect `+ .gemini/antigravity-cli`.
  - Kimi goes to `.kimi-code/skills`.
  - Kilo goes to `.kilo/skills`, with detect `+ .kilo`.
  - Xum goes to `.xum/skills`, with detect `+ .xum`.
  - Devin goes to `.config/devin/skills`, with a roaming `OsOverride` to `%APPDATA%\devin\skills` and detect
    `+ .config/devin`.
  - Each old dir goes into `former_skills_dirs`. The README table and the brief-snapshot / table tests are updated,
    and the Wave A pin that kept the deferred detect dirs absent is removed deliberately.
  - Project-dir corrections are 11's.
- [ ] **Typed errors (ADR-0001)** for every new refusal, each with a binding, a `describeCommandError` branch and an
      i18n key.
- [ ] **Tests** (core integration, temp roots, Tool dirs outside the central dir):
  - every report status;
  - deploy-first ordering;
  - adopt-identical;
  - Keep over a differing occupant;
  - edited copy deleted / left;
  - Kimi/Amp: Amp's artifact survives Kimi's move;
  - disabled re-record;
  - the Q24 collapse;
  - a crash seeded at each step, then finished;
  - idempotence;
  - **audit 3 F14 (aliased roots), F15 (destination aliased onto the central repo), F16 (Keep over a copy with
    local-only edits)**.
- [ ] **Frontend:** hook tests for the offer, run and decline paths and the "Move N" line; `reportOutcome` fold tests.
- [ ] **CONTEXT.md:** **Redeploy**, its **journal** and **launch check** replace "Target relocation". "Former skills dir"
      is updated.
- [ ] `npm run version:check && npm run check` is green. Windows arms are proven by `gh pr checks` on a draft PR.

## Notes

- Review: GPT-6 Astra **and** GPT-6.1 Sol in parallel (Q28), each briefed with the principle: delete only what is
  provably a duplicate, at the moment of deletion, only inside the folder you own. Budget more than one round.
- Prior art: `core/library_move.rs` (physical identity, verification, journal, resume) and the `artifact_removal` fence
  tests.
- The edited-copy row here is functional. 16 gives the ask its provenance, diff and consequence copy.
