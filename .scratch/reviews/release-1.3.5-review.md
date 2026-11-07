# 1.3.5 final release review (`v1.3.4..1e9ebbb5`)

- **Astra (GPT-6 Astra, high), 2026-11-07:** **FIX-THEN-RELEASE.**
- **Sol (GPT-6.1 Sol, high), 2026-11-07:** **RELEASE**, after the copy pass, the CHANGELOG and the version bump. No new
  blocking findings.

## Astra

### 1. P1, blocking: disabling Kimi before the redeploy removes Amp's live Deployment

- **The interaction:** ticket 10's Kimi correction gives Kimi its own folder. Ticket 09's per-folder selection then
  treats the unconverted `kimi_cli` row as alone in `.kimi-code`, while it still shares the
  `~/.config/agents/skills/alpha` link with Amp's row. A pair Disable of Kimi removes that link
  (`artifact_removal.rs:497–506`, `:849–870`; `placement.rs:323–327`).
- **Reproduced on a real 1.3.4-schema database:** Kimi ends up `disabled`; Amp stays `Synced` and owned, but the link is
  gone.
- **Control:** with the pre-correction registry shape, the same Disable settles both rows and passes.
- **Evidence:** `.scratch/reviews/evidence/release-upgrade-probe.rs`.

### 2. P2: CONTEXT.md describes behaviour that has been superseded

- **Redeploy:** the entry still names the two-rename fallback; it now refuses with `cannotReplaceSafely`.
- **Journal:** the entry still says a mismatch is dropped; it is now kept as `Unfinished`.
- **Former skills dir:** the onboarding scan is described as shipped, but it is ticket 11's.
- **Ownership and V12:** dated 1.4.0; they ship in 1.3.5.

## Upgrade probe: passes

- **Setup:** a database built from v1.3.4's schema, with 33 skills and 108 symlink Deployments (Claude Code 33,
  Codex 33, Pi 33, `.agents` 9).
- **Migration:**
  - V11 → V12 runs, and the journal table is created empty;
  - the v1 selection moves to v2, and running it again changes nothing;
  - a stored 1.3.4 Library-move report decodes.
- **Catalog:** unchanged.
- **Launch preview:** 0 corrections, 0 migrations, 0 journal entries.
- **On disk:** every symlink survives.
- **Mutations:** a sync, Disable, Enable, Delete cycle works.

The release split is confirmed: the Codex, Goose, OpenHands and Zencoder literals are byte-identical to v1.3.4's.

**Gate:** 1,470 frontend tests; 1,205 Rust tests. `main` CI is green.

## Disposition (orchestrator, 2026-11-07)

Fix pass for finding 1, with one general rule inside `artifact_removal`: no removal takes an artifact that another live
row outside the operation's scope still records. A live row is one that is not disabled and owns its artifact; the
match is the same recorded path or the same physical entry.

- **What changes:** the row in scope settles as Disable or Delete would settle it, and the artifact stays, reported
  "kept: still used by {harness}".
- **Why it's consistent:** it matches the redeploy cleanup's rule, and it lends no ownership.
- **Finding 2:** the glossary is corrected in the same pass, because the docs should match what ships.

## Sol

- **101. P3 → ticket 12:** a launch offer for journal-only work asks "Move 1 Deployment…" and shows an empty harness
  list (`redeploy.rs:394–417`, `RedeployOfferDialog.tsx:25–49`). Give it its own copy: "Finish an interrupted move?"
- **102. P3:** the same as Astra's finding 2, the stale passages in CONTEXT.md. Fixed in the release fix pass.
- **Fixture walk** (`rich`, `failures`, `registry`, `redeploy`; screenshots are in `.scratch/release-review/evidence/`):
  - Disable and Enable at pair and skill grain both behave correctly.
  - Add raises the ask, and Keep is honoured.
  - A name containing a slash is refused with `SKILL_NAME_INVALID`.
  - Update on a replaced link raises the ask.
  - Delete leaves a replaced folder in place and names it.
  - The project matrix and totals are coherent.
  - The redeploy offer, "Move N", the Keep/Leave report and the retry all work.

  The fixture still has the pre-correction registry (Sol 106 → ticket 12), so the walk proves the flows, not the
  corrected folder paths. Rust tests prove those.
- **The operator's upgrade:** no launch offer, no change to any skill folder or link at first launch, and the catalog
  is unchanged. Startup still writes the database (schema, selection key); that is not a skill-folder change.
- **Copy-pass list and CHANGELOG draft:** in the child thread, and used by the orchestrator for ticket 20.

## Disposition: released (orchestrator, 2026-11-07)

- **Astra 1:** fixed in PR #25 (`773120cd`). The probe against a database with the v1.3.4 schema now passes.
- **Astra 2 = Sol 102:** fixed in the same PR (CONTEXT.md, AGENTS.md).
- **Sol 101:** moved to ticket 12.
- **Copy pass:** applied, following Sol's list.
- **Released** as `28252e75` (`v1.3.5`).
