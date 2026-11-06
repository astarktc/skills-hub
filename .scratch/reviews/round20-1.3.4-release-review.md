# 1.3.4 final release review — `v1.3.3..956c931`

## GPT-6 Astra (high), 2026-11-06: **fix-then-ship**

The first dispatch stopped on a false-positive ChatGPT "Subscription Sharing usage limit" error. The operator resumed
it.

1. **P2: Move Library admitted a reader-only harness's own skills folder** (fx's `~/.fx/skills`) as the destination.
   Both admission passes listed only the adapters' dirs.
2. **P3: "reader-only" was described as having no skills folder.** Fx, Warp, Dexto, VT Code, DimCode and Corust do
   have one; the registry records them.
3. **P3: CONTEXT said no entry is copy-only**, but Deep Agents Code is.

**Sound:**

- Ticket 04's leases, try-lock and callers; no deadlock found. #117 stays open, pre-existing.
- The latest Move fixes.
- All 45 existing rows keep their paths, keys, grouping and capability.
- Startup resume is a no-op without a journal.
- Bindings, i18n, fixtures and Windows code; CI green.

## Disposition

All three were fixed by the orchestrator in PR #20:

1. A `harness_skills_dirs` helper (adapter dirs plus reader-only `skills_dirs`) is used by both passes. Regression
   `refuses_a_reader_only_harness_skills_dir`, red first.
2. and 3. README, CONTEXT, AGENTS and the CHANGELOG were reworded.

Gate green (Rust 1,022); CI green.
