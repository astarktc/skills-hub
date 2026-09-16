# Wave B review brief — v1.2.12 (template; pin candidate HEAD before dispatch)

One independent model seat, Pi `anthropic/claude-opus-5` high, substituting exhausted Fable under Q11. Two axes reported separately; do not merge findings across axes. Fresh context. Parent fills exact candidate SHA and commit list at launch. Standards and Spec both required.

## Authority / target

Review integrated main checkout `~/Projects/skills-hub`, read-only source/git authority. Do not edit, commit, merge, push, version, release, or launch children. Report output under `.scratch/round10/` allowed. Read AGENTS.md first; review approach pre-approved, begin without confirmation. Search only this project (cloud-sync); ignore `.scratch/round10/worktrees/` when searching integrated source. No app/dev runs, live DB/library/Tool-directory access or network acquisition. Deterministic test/build commands allowed only by arrangement with parent's gate so generated files are not concurrent writers. Do not `git add` anything, `.scratch/` ignored. Use code-review and codebase-design skills; use vercel-react-best-practices for hook changes, pi-lens-lsp-navigation for code intelligence. Reviewer evidence is not release authority.

## Fixed point / spec

Baseline `a824caf45fffb0bd5c38318ba10b2ffa0ca11c62`. Diff `git diff a824caf45fffb0bd5c38318ba10b2ffa0ca11c62...HEAD`, commits `git log a824caf45fffb0bd5c38318ba10b2ffa0ca11c62..HEAD --oneline`. Confirm baseline resolves and candidate SHA matches parent brief before analysis.

Spec: `.scratch/round10/issues/03-git-source-resolution.md`, `04-manifest-module.md`, `05-mutations-return-report-and-skills.md`; settled `.scratch/round10/decisions.md`; `.scratch/round10/wave-b-ticketing-evidence.md` records factual corrections and deliberate deferrals. Read whole tickets. Standards: AGENTS.md, CONTEXT.md, docs/adr/0001 through 0004. Parent supplies actual worker evidence and scope adaptations; verify rather than trusting claims.

## Standards axis

Report documented violations with file/hunk and exact rule. Also consider these Fowler heuristics; label smells as judgement calls, never hard rules, and suppress when repo conventions explicitly endorse the pattern. Skip tooling-enforced issues:

- Mysterious Name: meaning/role unclear; rename or clarify design.
- Duplicated Code: repeated logic; centralize one rule.
- Feature Envy: knowledge belongs to another module; move it there.
- Data Clumps: same fields travel together; use one real concept.
- Primitive Obsession: primitive hides a domain concept; give it a meaningful type.
- Repeated Switches: duplicated dispatch; one owning rule.
- Shotgun Surgery: one policy change scattered across callers; gather ownership.
- Divergent Change: one module changes for unrelated reasons; separate responsibilities.
- Speculative Generality: unused extension points/abstractions; remove hypothetical needs.
- Message Chains: caller navigates internals; hide the walk.
- Middle Man: pass-through with no leverage; call the owner directly.
- Refused Bequest: implementation rejects inherited contract; repair model/composition.

Do not mistake mandated compatibility re-exports, DTO wiring or a narrow fixture injection for forbidden indirection. Judge interface depth, ownership and actual usage. Watch broad generic closures/new factories created just to make a test pass; the seam should represent real variation.

## Spec axis / risk checklist

B1:
- Characterization preceded production edits; literal outputs and intentional Q5 deltas recorded. Listing and acquisition now agree without unintentionally losing root/nested/folder/malformed/installable exception/alias/name handling. Same resolved source reused for selected install, explicit null default branch preserved, old callers supported.
- Stored record hint repair only on SHA 404, one retry; content 404 not repaired, explicit selection never silently retargeted, actual SHA preserved. Branch assumed-main exception still differs from explicit main. No persisted source update before successful finalize, symlink alias/escape/cancel/cache widening retained.
- No leaked caller protocol for TreeSplit/acquire_resolved/branch_assumed. Listing intent shares resolution/admission, discovery scan ladder remains one implementation.
- Preview private sibling -> completed rename under short lock; no partially visible cache, loser cleans only own scratch/reuses winner, failure/cancel no false cache hit, no lock across acquire, no new deadlock. Concurrency tests use deterministic barriers/channels; red proves behavior not compilation.

B2:
- Exactly one backend manifest grammar/writer; scan/provenance/finalize/Edit orchestration still with their owners. Byte-preserving, not YAML rewrite; persisted InvocationLines field compatibility; same-mode no-op; duplicate keys/clear/unrelated bytes/body/permissions/temp cleanup.
- Complete column-zero fences, trailing whitespace/CRLF, scalar indented fence, prefix rejection and later-body fence behavior agree in Rust and TS. Frontend extraction remains pure presentation and avoids visual/syntax feature creep.
- Required I/O typed, optional metadata/mode/JSON lock failures degrade as before. JSON schema/provenance still skill_lock, no generic file service masquerading as Manifest depth. Replay/rollback failure atomicity and typed invalid UTF8 preserved.

B3:
- All five actions return full post-op catalog including reassert; returned failures/partial/skips still replace entire catalog (row removal/other-row changes), no success-tail IPC. Catalog errors not swallowed; no full catalog under new global mutation lock.
- Update/Restore remain batch-of-one core policy. Delete and Refresh-all still refetch with old wire contracts; Add/import/Detach/unsync/project unchanged. No untagged value-dependent IPC union.
- Edit global/project target failures surfaced once through pure fold, expected skips silent, central save still applied and modal settled; no unconditional success or hook nested report interpretation.
- Completion precedence and modal behavior preserved: central settled closes even target failures/conflict, acquire failure/skips/empty keep repair open; thrown singles reload/rethrow, thrown Edit unchanged. Action IDs resolve current state, no stale record/ref workaround, no double toast.
- Existing SkillGone/StaleAcquisition consumed through new shape; reason-neutral EN/ZH summary. New command annotations/registration/generation/shim complete, bindings not hand-mirrored.

Across lanes:
- Verify no unexpected source reversions to round11 target-selection/removal/chips/zero-target reporting or wave A acquire-first/Edit replay/Propagation/content identity. Shared files must retain integrated changes.
- Malformed-selection/C3/C4/content-identity+UpdateRequest backlog/wave C/#9 excluded. Raise unrelated real issues as separately deferred, not requirements to expand this release.
- Tests should exercise public/owned interfaces with literal expectations; baseline-preserving tests green on old source is expected, sensitivity proven via deliberate behavior mutation. New behavior must have genuine behavioral red, not missing import/type failure.

## Output

Write `.scratch/round10/r10b-review-opus.md` and summarize final response. `## Standards` and `## Spec` separately (prefer <=400 words each; detailed evidence appendix if necessary), `## Summary` with counts per axis, concrete Blocking vs Follow-up and `Merge verdict: BLOCK / OK / OK with notes`. Every finding cites current file/line, reproducer or source-proven contract contradiction; quote spec/rule. No generic advice or severity inflation. Explicitly identify unverified test evidence/coverage limitations. Parent owns classification, fixes, rechecks and release.
