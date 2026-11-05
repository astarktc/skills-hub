# 05 — Astra review fixes (M1, M2, S1, S2, S3)

Status: done — 88e8885
Spec: `.scratch/round17/spec.md` § Review. Review: `.scratch/round17/review/astra-review.md`.

## Work

- **M2** `ToolAdapter.relative_detect_dir: &str` → `relative_detect_dirs: &[&str]` (any present root counts;
  `detect_dirs_in` replaces `detect_dir_in`). Amp → `.config/amp` (ampcode.com/docs/cli/settings); Kimi →
  `[".kimi-code", ".kimi"]` — the vendor's two doc sites disagree (kimi.com/code vs kimi-cli.com), so a single
  root would knowingly miss one generation. `.config/agents` is no adapter's detect dir any more; the registry
  test `no_adapter_is_detected_by_a_shared_skills_convention_dir` keeps it so (the earlier "every adapter nests
  skills under detect" assertion was wrong for Amp/Kimi and for Gemini/Antigravity, and is gone). Fixtures that
  relied on "installing Amp installs Kimi" now install both. README rows updated.
- **S2** `sole_entry`: a per-entry `Err` while enumerating reads as "not a footprint" (installed), same as a
  failed `read_dir`.
- **M1** `useSyncOrchestration.scanScopeRevision` bumps on every saved tool configuration; `useAddSkillFlow`
  reloads the plan on it (banner/count follow the configuration, including empty → populated) and
  `handleReviewImport` always fetches afresh (the reviewed plan is the one the import acts on). Two hook tests.
- **S1** `useOverwriteConfirmation`: resolver in a ref, one-shot settlement, a newer request settles the
  displaced one as declined, `cancel` reads the ref, unmount settles pending as declined. Own test file.
- **S3** `useSkillLibrary.syncOrReload`: a thrown sync request reloads the catalog before rethrowing (the
  "thrown Update/Restore also reload" precedent); the three library call sites use it. Seam test for a thrown
  retry; library test for the reload.

## Comments
