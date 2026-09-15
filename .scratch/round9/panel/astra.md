# Round-9 architecture review — Astra

Baseline: `main` at `96cd33c` (v1.2.8). Read canonical context, all three ADRs, deep-module guidance, the 150-commit history, and round-7 exclusions. Static review; no app launch or runtime verification. Only this report is written. Paths below are project-relative; Rust paths abbreviate `src-tauri/src/core/` as `core/`.

## 1. Deepen central replacement through Edit replay

**Files:** `core/install_finalize.rs`, `core/installer.rs`, `core/skill_edits.rs`, `core/tests/skill_edits.rs`.

**Problem:** Finalize's failure-atomic implementation finishes and deletes the old bytes at `install_finalize.rs:306–325`; its caller only then replays Edits (`installer.rs:387–391`). Replay can fail on the edit-row write, manifest write, or hash/skill-row write (`skill_edits.rs:149–177`). Thus a reported failed Update can already have replaced the central copy and discarded recovery bytes. Existing failure tests exercise `replay_unlocked` directly, after manually writing upstream bytes, rather than this enclosing Update sequence.

**Solution:** Give one deep module ownership of installing upstream bytes, replaying Edits, and settling the central record before releasing recovery material. Keep Propagation after central settlement; preserve the deliberate durable-Edit intent on direct Edit failures rather than casually imposing all-or-nothing semantics everywhere.

**Benefits:** Failure policy gains locality; tests through Update can verify bytes, Edit base, record and retry together using temporary directories and SQLite failure triggers.

**Evidence:** Deletion test: deleting `finalize_and_propagate_unlocked` moves a small ordering recipe into Refresh, not substantial hidden policy. Deleting finalize itself would duplicate real recovery complexity—deepen its reach, do not discard it. Call chain: Refresh → installer → finalize → replay → Propagation. `faf109a` bought failure atomicity; `a261e89`/`fb71b1b` added a fallible step after its protection ends.

**Strength:** Strong. **ADR conflict:** None.

## 2. Deepen Refresh's acquired-result settlement

**Files:** `core/installer.rs`, `core/refresh.rs`, `core/install_finalize.rs`.

**Problem:** `AcquiredUpdate` exports a whole mutable record (`installer.rs:215–219`) captured outside the Mutation guard (`installer.rs:282–288`). The pool completes before apply acquires the guard (`refresh.rs:275–341`). Finalize later upserts that snapshot, retaining its other fields (`install_finalize.rs:292–307`). Serialization prevents simultaneous writes, but does not prove that an acquired result still belongs to the current Managed skill/source. Delete, Detach or another Re-point during acquisition can invalidate that assumption.

**Solution:** Make the apply module own whether acquired bytes remain admissible against current identity and Provenance, and which acquisition facts may update the current record. Keep slow acquisition outside the Mutation guard; hide snapshot reconciliation inside settlement.

**Benefits:** One locality for stale-work rules rather than caller vigilance. Existing injected acquisition can exercise interleavings deterministically; assert that removed skills stay removed and newer source decisions are not silently overwritten.

**Evidence:** Deletion test: deleting the `AcquiredUpdate` carrier alone removes no behavior—its callers still need the undocumented validity protocol. Depth requires owning that protocol, not merely renaming the carrier. Both ordinary Refresh and git Re-point feed this same apply path. `8666f8e`, `95b7893`, and `f48c001` expanded what the acquired record carries and preserves; this is static risk evidence, not a reproduced race.

**Strength:** Strong. **ADR conflict:** None; preserves the per-skill Mutation guard.

## 3. Deepen mutation outcome settlement, not just error formatting

**Files:** `core/skill_edits.rs`, `src/hooks/useSkillLibrary.ts`, `src/hooks/useStatusReporter.ts`.

**Problem:** Direct Edit performs Propagation but logs and discards failed targets before returning a catalog entry (`skill_edits.rs:124–142`). The hook consequently announces “saved” without those outcomes (`useSkillLibrary.ts:88–94`). Refresh preserves target failures, but single-skill settlement only emits them (`useSkillLibrary.ts:555–568`); its completion selection checks Edit conflicts, not target failures (`useSkillLibrary.ts:572–598`). Local Re-point uses a separate action recipe with unconditional success copy (`useSkillLibrary.ts:679–701`). One operator outcome is distributed across command result shape, report helpers and action completion policy.

**Solution:** Preserve central-change and per-target outcomes through the Edit command seam. Within the skill-library world, let one settlement module own report interpretation, Notification severity, refresh-on-failure and completion eligibility for Update/Restore/Re-point. Keep the reporter responsible for Notification delivery, not skill-domain policy.

**Benefits:** Leverage across callers without a universal action framework. Test successful central changes with failed targets through the Edit interface, and test each action's observable Notifications through the hook. Pure report helpers alone cannot prove their callers choose the right completion.

**Evidence:** Deletion test: deleting `settleSingleReport` merely redistributes a short failure scan and two reporter calls; callers still own the difficult completion rules. Conversely, deleting Propagation would redistribute substantial target policy—keep it. `12f5d91` fixed misleading batch success; `cf47020` fixed completion-dependent modal closing; `a261e89` introduced another outcome path that cannot reuse the complete policy.

**Strength:** Strong. **ADR conflict:** None; preserve ADR-0001's typed errors and report-data distinction.

## 4. Deepen Explore preview publication

**Files:** `core/installer.rs`, `core/git_acquisition.rs`, `src/hooks/useExploreState.ts`.

**Problem:** Preview's cache probe calls any non-`.git` entry a hit (`installer.rs:667–678`). Its own guard ends before acquisition writes directly into the published directory (`installer.rs:681–702`). A second request can therefore observe an in-progress directory; clone-copy failure can also leave partial content (`git_acquisition.rs:544–545`). The frontend immediately treats the returned path as readable preview content (`useExploreState.ts:121–149`). This lifecycle knowledge is not hidden by Git acquisition: that module correctly accepts a caller-owned destination.

**Solution:** Give the preview module ownership of private acquisition, complete publication, same-preview coordination and failed-attempt disposal. Keep the clone cache and Git acquisition as separate deep modules; this is a preview-lifecycle change, not another general cache framework.

**Benefits:** A small preview interface gains actual depth: a returned path means completed content. Temporary directories plus an injected acquisition adapter can test interrupted copies and overlapping requests without the app or network.

**Evidence:** Deletion test: removing the probe/prepare guard loses little correctness because it does not cover publication; removing preview ownership entirely pushes cache lifetime and destination rules into its caller. Deepen that ownership rather than extracting the current block unchanged. `6b0ec7d` and `4bbedc0` revisited this adapter for acquisition changes while leaving its lifecycle separate; neither proves an observed preview failure. Both overlap and partial-copy cases are static risks.

**Strength:** Worth exploring. **ADR conflict:** None.

## Top recommendation

Start with **central replacement through Edit replay**. It joins two recently hardened implementations whose composition remains fallible after recovery material is discarded. First demonstrate an Update with a persistent Edit and an injected edit-row failure, then verify central bytes, record, Edit base and retry through that same interface. Address stale acquired-result settlement next; it is a distinct validity problem, not solved by rollback.

**What I would not deepen:** Keep Git acquisition's real download/clone adapter seam and the clone cache's per-key coordination; deleting them would spread substantial policy across install, Refresh and Explore. Keep Propagation and Artifact removal distinct: their outcome semantics differ, and ADR-0002 earns its settlement rule. Keep the project world's returned Project view and the explicit cross-world hook dependencies. I would not replace these with a generic workflow module, or revisit the excluded GET-helper, repoint alias, fence-parser and conflict-convergence items.
