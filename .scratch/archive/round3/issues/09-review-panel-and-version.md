# 09: End-of-round review panel; version 1.2.3; Release

Status: done — f057190

**What to build:** The round is reviewed by the 3-model panel (Fable 5.1 / Opus 5 / GPT-5.6 Sol; Standards + Spec axes; brief shape from round 2) over everything since `f400440`. Every finding is verified at HEAD; blockers are fixed on `r3/fix-*` branches; the rest becomes a new follow-ups ticket. Then `npm run version:set 1.2.3` (after 06, so both lockfiles move), the gate, push to `main` → auto-tag and Release; the operator confirms the run and smoke-tests the notification UX in `tauri:dev`.

Source: `../spec.md` Q7, Q14, Q15. Skill: **code-review**.

**Blocked by:** 01, 02, 03, 04, 05, 06, 07, 08

- [ ] Panel reports saved under `$TMPDIR`; decisions captured in Comments
- [ ] `npm run version:check && npm run check` green at the bump commit
- [ ] Release run green on all five targets; operator smoke test done

## Comments

- 2026-09-15 — Status reconciliation: panel + bump done (`f057190`, Release run 33945623532); Release confirmation **ready-for-human** → ready-for-human. Evidence: f057190 and local tag v1.2.3 verify the release commit; CHANGELOG.md:103 records the round. Ticket comments record run 33945623532 dispatched, not confirmed green. round4/spec.md records later operator smoke of built 1.2.3, but no five-target release confirmation was found; retain the explicit human release-confirmation remainder.
- 2026-09-15 — Previous status wording (historical, not a current merge/publication claim): panel + bump done (`f057190`, Release run 33945623532); Release confirmation **ready-for-human**

**2026-09-05 — orchestrator.** Panel run over `f400440..4d17493` (Fable 5.1 / Opus 5 / GPT-5.6 Sol); reports at `$TMPDIR/r3-review-{fable,opus,sol}.md`. 21 distinct findings, no true blocker: Fable's "blocking" was the CHANGELOG (written in ticket 11); Sol's Q10 blocker downgraded — the live-name fallback fires only for an empty stored name, which the V6 migration backfills (`skill_store.rs:326-330`), so stored never loses to live. Consensus fixes shipped as ticket 11 (`93641d7..cc31ef5`): never-narrow holds without a sidecar; `hash_key_parts` third slot dropped (digest pinned); completion toasts carry a message (Q1 line readable); batch summaries with failures are warnings; docs + CHANGELOG. Q6 ruling recorded in spec.md. Everything else → ticket 12. Bump `f057190` (`version:set 1.2.3` — first use of the five-location rewrite; `cargo test` after it left the tree clean). Pushed after rebasing on the nightly bot commit; auto-tag `v1.2.3` created; Release run `33945623532` dispatched — operator confirms.
- 2026-09-15: Release confirmation closed — GitHub Release run 33945623532 (workflow `Release`) completed `success` (`gh run view`); v1.2.3 tag published. Status → `done — f057190`.
