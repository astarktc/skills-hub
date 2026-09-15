# Round 10 — deepening round, grilled decisions (source: round-9 panel, `.scratch/round9/panel/`)

## Round 1 (2026-09-08, all as recommended)
- Q1 Waves, each a patch release: A = #1 content identity → #2 Update module (backend lane) ∥ #4 report fold (+#6) (frontend lane); B = #3 git resolution (+#8 preview rider) ∥ #5 manifest; C = #7 reports cross the wire. #9 deferred.
- Q2 #1: hash stored at finalize/edit; readers trust the row; missing hash = compute+backfill inside the module (replaces project_sync read-time backfill and global_sync both-sides re-hash).
- Q3 #2 adapters: git Update, local Re-point (becomes atomic), Edit write (returns propagation report — closes the fan-out-rule breach), Restore. installer.rs = Add flows only.
- Q4 #2 admission: acquired result vs current row mismatch → discard staged bytes, report `Skipped { StaleAcquisition }` / `Skipped { SkillGone }`; no in-batch retry.
- Q5 #3: listing adopts acquisition's candidate rule; characterization tests of today's listing outputs land FIRST.
- Q6 #4: pure fold `(report, labels) → { toast, entries, completion: { reload, closeModal, conflict } }` in `src/lib/reportOutcome.ts`; actions carry a skill id resolved at click time (kills managedSkillsRef).
- Q7 #6: Update / Restore / both Re-points / Edit return `{ report, skills }`; Delete + Refresh-all keep the refetch.
- Q8 #7: deferred to wave C; ADR-0001 amendment written then.
- Q9 #5: reads + byte-preserving writes (Edit V2 prerequisite).
- Q10 #8 rides in #3's lane (private temp dir, rename on completion); #9 dropped, backlog note only.
- Q11 Review: wave-A backend = 3-seat panel; everything else = single Fable.
- Q12 ADR "edits replay inside finalize's window; edit wins; row is source of truth" written by the parent after D2 ships; CONTEXT.md gains **Content identity** with #1.

## Round 2 (all as recommended)
- Q13 #6 moves to wave B (return-shape collision with #2). Wave A frontend = #4 only, against today's DTOs; new `Skipped` variants + Edit propagation report consumed in a wave-B follow-up.
- Q14 #1 → new `core/content_identity.rs` absorbing `content_hash.rs`; `hash_dir` is implementation, not a seam. CONTEXT.md term **Content identity**.
- Q15 Propagation / reconcile / same-content ASK content_identity (one read, backfills if absent); `propagate_unlocked` drops `content_hash: Option<&str>`.
- Q16 #2 → `core/skill_update.rs`; one entry "settle central copy from these bytes and bring targets in line" → `UpdateOutcome`; bytes via adapter enum (git staging, local folder, edit-in-place, restore-rebuild); `finalize_and_propagate_unlocked` + D2 closure become its body; refresh.rs keeps pool + guard + reassert.
- Q17 local Re-point's new `source_ref` rides in the record inside the settle window (atomic like git Re-point); unlocatable.rs = validation + adapter construction.
- Q18 `set_invocation_override` returns `{ entry, propagation }` (UpdateOutcome-derived DTO); frontend reads `entry` in wave A, fold consumes `propagation` in wave B.
- Q19 #4: fold gets table tests; hook tests that only exercised the closures are deleted; one hook test per action proves invoke-fold + execute-completion.
- Q20 #3: extend `SkillIntent` to StoredRecord / Selection(with GitSourceResolution) / ByName; resolve_tree_source, S3 repair, branch_assumed become private to a `resolution` submodule of git_acquisition; listing = Selection without subpath.
- Q21 #5 → `core/manifest.rs`; skill_lock + install_finalize::read_skill_md_meta read through it; frontmatter_edit.rs folded in as the write half.
- Q22 Wave A = 2 tickets (backend #1+#2 one lane; frontend #4) + 3-seat review brief under `.scratch/round10/`; Astra medium; start only after v1.2.9 is released.

## Closure — 2026-09-15

Shipped: v1.2.10 / 830eef0 (wave A, also included in v1.2.11); wave B integrated through a971b18 and released as v1.2.12 on 2026-09-15 (tag at f4fd7f6, Release run 35035055428). Tickets: 5 terminal (2 done, 3 resolved), 0 still open (none).
Residue → BACKLOG: #02, #06, #07, #08, #09, #15, #16, #20, #28. Dropped by name: none.
