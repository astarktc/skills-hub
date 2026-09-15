# Wave B ticketing — source evidence and ownership

## Scope / publication

Verified `main` HEAD `a824caf45fffb0bd5c38318ba10b2ffa0ca11c62` (`chore: update featured-skills.json`), clean tracked worktree. Published only:

- `issues/03-git-source-resolution.md` — B1 #3 + #8 preview rider, Astra xhigh.
- `issues/04-manifest-module.md` — B2 #5, Astra high.
- `issues/05-mutations-return-report-and-skills.md` — B3 #6, Astra high.
- This report.

All tickets are `ready-for-agent`, no logical blockers between them; parent worktree isolation/integration/review only. Q11 remains one reviewer model seat: today's explicitly requested Anthropic Opus 5 high replaces exhausted Fable. Discover the exact available ID at dispatch; no child was launched here.

Read AGENTS, CONTEXT, all four existing ADRs, issue-tracker instructions, decisions, both wave-A tickets and round7 backlog. Used to-tickets, codebase-design and pi-lens-lsp-navigation; also read TDD, React best-practices and AST-search instructions for applicable verification. No implementation, test suite, dev app, network acquisition, real library/DB read, version edit, commit or push was performed. Gates/red-green instructions in tickets are future implementer obligations, not claimed results of this ticketing task.

## Verified call paths / intended test seams

| Lane | Source bodies verified | Tests/adapters verified |
|---|---|---|
| B1 listing/install | `commands/mod.rs::list_git_skills_cmd`/`install_git_selection` wiring; `installer::list_git_skills_with` → `resolve_tree_source` → injected checkout → `resolved_git_candidates_in`/`git_candidates_in`; `install_git_selection_with` → acquire or acquire_resolved → finalize; `useAddSkillFlow` picker and direct install both forward resolution | `tests/installer.rs` candidate/folder fixtures; `CountingRefsApi` and `listing_and_install_share_one_refs_resolution_and_old_selection_still_resolves`; scripted `GithubApi`, local committed repo/temp-root fixtures |
| B1 Update/preview | `skill_update::acquire_update` chooses intent/suffix → acquisition corrected subpath proposal → `apply_unlocked`/finalize persists. Preview creates final cache path under a short lock then acquires outside it | `tests/git_acquisition.rs` stored-SHA repair/download-404/no-clone assertions; `tests/skill_update.rs::stale_split_repair_is_acquire_first_and_persisted_only_by_finalize`; preview hit and timeout-bounded deadlock tests. New deterministic paused-acquisition preview seam is required, not present today |
| B2 Manifest | discovery metadata/mode/fence parsing; finalize's `read_skill_md_meta`; `frontmatter_edit` read/capture/write/restore/atomic rename; `skill_edits` set/replay/write and persisted base; catalog's mode read; lock JSON read/enrichment; detail view's pure parser | Existing byte corpus, indented-fence, rename-failure and clear-preservation tests; lock JSON/enrichment temp fixtures; Edit SQLite-trigger rollback and failed-copy report tests. Proposed new pure frontend helper test avoids JSX |
| B3 results/fold | Refresh + both repairs converge on batch apply/reassert; command DTO mapping; catalog assembly; Edit returns entry+propagation; hook discards propagation; `runSingleRefresh` reloads returned reports and thrown requests; fold owns completion and acquisition skips | `useSkillLibrary.test.ts` mocked IPC/Channel/dialog/reporter and synthetic fold completion table; Edit's current no-refetch patch-one-row test; pure `reportOutcome.test.ts` precedence/close/skip fixtures; real temp-store mutation→catalog result-composition seam to add |

Navigation limitation: Rust LSP document-symbol probes returned empty; the review graph warned it was built at `9de83e9e`, not current HEAD. Used outlines only as navigation hints, then current `read`/`read_symbol` bodies and scoped AST searches. No claim relies on cached callers or old line numbers. A final bounded Bun check verified **41 literal source/test anchors in 15 files**; `git diff --exit-code` and `git status --short` were clean after publishing the three tickets.

## Factual corrections captured in tickets

1. Q13 skips are already consumed by `refreshOutcome`, with both reason keys and fixtures. B3 preserves/proves them rather than recreating variants. The reason-neutral skipped summary remains appropriate.
2. Edit backend already reports failed targets; current frontend reads only `entry` and emits unconditional success. B3 is not a backend fan-out rewrite.
3. Update/Restore share `refreshManagedSkills` today. A typed single-skill wrapper can preserve Refresh-all's bare report/refetch without a value-dependent union. This is an implementation interface choice, not an unresolved product decision.
4. Actual modal policy closes after central settlement even with conflict/target/reassert failure; acquisition failure/skips keep open. Existing table tests prove it. Warning severity is not a keep-open rule.
5. `skill_lock` parses `.skill-lock.json`, not SKILL.md. B2 must honor Q21's shared read route without moving provenance/JSON policy into frontmatter parsing or surfacing optional read errors.
6. Rust opening fence uses `trim()`, closing uses a complete column-zero comparison. Frontend uses prefix matching and `indexOf`, with LF-oriented offsets. Tickets distinguish these and scope the existing fourth-fence follow-up without adding UI features.
7. Both git candidate implementations already filter installable candidates. The interesting difference includes folder-root suppression/root-versus-nested handling; B1 must characterize old complete outputs first and record intentional Q5 deltas.
8. Listing resolution reuse and stored-hint repair are already implemented/tested; B1 internalizes their protocol, not introduces them. Serialize-only `resolution` remains optional in the generated TS today; two current `?? null` sites are in `useAddSkillFlow`.
9. No-clone-on-404 has existing narrowly defined exceptions (assumed-main SHA, stored-hint SHA repair); flattening that statement would be a regression.
10. Preview partial publication remains real in source: its final directory exists before acquisition completes; a hit checks only for a non-`.git` entry.

## Ownership / integration overlaps

- `installer.rs` and its tests: B1 git listing/selection/preview; B2 metadata imports only. B3 no ownership.
- `skill_update.rs`: B1 only acquire intent/result wiring; no B3 settlement/admission cleanup. B2 keeps compatibility parser/type exports to avoid churn.
- `skill_edits.rs`/tests: B2 Manifest imports/calls/byte compatibility; B3 optional response adaptation/tests only, preferably command-side so core guarded result stays unchanged.
- `commands/mod.rs`: B1 install resolution type/call; B2 invocation type imports only; B3 single-mutation DTOs/result assembly. `lib.rs` registration belongs to B3 only if a command is added.
- `core/mod.rs`: B2 Manifest declaration/removal; B1 normally needs no top-level declaration for private resolution; B3 one declaration only if focused result composition requires a module.
- `src/components/skills/types.ts`: B1 resolution export; B3 mutation DTO exports. B2 normally no shim changes (InvocationMode wire stable).
- `src/bindings/index.ts`: generator output from all lanes; parent regenerates on integrated source, never hand-merges types blindly.
- `useAddSkillFlow.ts`: B1 owns only two resolution arguments; B3 has no needed edit because Add is excluded from the new envelopes. Any surprise dependency needs an explicit symbol handoff.

Parent must rebase each isolated worktree onto current main, inspect shared hunks/unexpected deletions rather than taking whole files, regenerate bindings, run full gates, and add Source resolution / Manifest vocabulary plus updated invariants separately. Versions/changelog/release remain parent work.

## Deferred / unresolved

No genuinely unresolved **product** decision found within the approved breakdown. Engineering choices remain: exact intent/result signatures, preview private acquisition adapter, Manifest text-read route for the JSON lock adapter, and concrete single-skill command/result-test seam. Tickets give constraints without inventing additional product scope.

Deferred, not implementation permission: backlog 17–20 content-identity ambient state/trust/documentation and UpdateRequest/adapter cleanup; unrelated earlier hygiene; malformed `global_selected_tools`; round11 C3/C4; wave C #7 report wire unification/ADR amendment; #9. Backlog 21 belongs to B3 and 22 may ride only the fold touched there. Parent can separately triage the source-identity caveat; none of these blocks B1/B2/B3.

## Next implementation briefs

Dispatch three isolated Pi children using the full ticket body, pre-approve “state your approach and continue,” forbid unrelated cleanup/live runs/children and force-adding scratch, and require targeted edits plus re-reads. B1 starts with listing characterization before production edits; B2 starts with byte/fence corpus and compatibility-preserving Manifest extraction; B3 starts with a typed envelope/no-refetch tracer and Edit report fold. Each must return genuine behavioral red/green evidence (mutation sensitivity for behavior-preserving cases), exact gates and ownership deviations. Parent verifies writes itself and performs final integration plus the single-seat review.
