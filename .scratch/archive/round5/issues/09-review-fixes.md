# 09: Round-5 panel fixes

Status: done — a403359

**What to build:** The Blocking findings the four-model panel (Fable 5.1, Opus 5, GPT-5.6 Sol, GPT-6 Astra) upheld at HEAD `378a473`, plus the cheap follow-ups that share their files. Reports: `.scratch/round5/r5-review-{fable,opus,sol,astra}.md`.

**Blocked by:** 01–07 (all on main)

Blocking:
- [ ] **Re-point accepts every URL shape the Add flow's parser accepts** (Fable Spec 1, Opus Spec 1, Astra P1). Delete the second GitHub-URL grammar in `refresh.rs::parse_repoint_source`; instead parse with `git_acquisition::parse_github_url` and validate the *result*: `api.is_some()` (github.com coordinates) and, per ticket 01's "full URL" policy, refuse shorthand (`owner/repo`) — but accept `https://`, `http://`, schemeless `github.com/…`, `/tree/` and `/blob/` (incl. a trailing `/SKILL.md`, which the parser already normalises to the folder), and a `.git` suffix. If a strict predicate is needed, it lives beside `parse_github_url` in `git_acquisition` with named clauses (Opus Std 1 / Fable Std 2 / Astra S1). Tests: `/blob/main/skills/foo/SKILL.md` re-points to subpath `skills/foo`; `owner/repo` shorthand is `INVALID_GITHUB_URL`; existing repoint tests stay green.
- [ ] **One source-kind door** (Fable Std 1): replace the three raw `source_type === "git"` compares (`useSkillLibrary.ts:157,604`, `SkillDetailView.tsx:531`) with `sourceKind(skill) === "git"` from `skillPresentation.ts`.

Follow-ups folded in (same files):
- [ ] `refresh.rs:214` `record.source_type != "git"` → `Provenance::parse` like its neighbours (Fable Std 3).
- [ ] Root subpath spelling (Fable Spec 2): when the override resolves to the repo root, store `source_subpath = None`, not `Some(".")`, matching Add.
- [ ] `describeCommandError` renders `SYMLINK_CHAIN_TOO_DEEP` like its sibling `SYMLINK_ESCAPES_REPO` (interpolate `{{subpath}}`, not `withDetail`) — EN + ZH (Fable Std 6).
- [ ] `SkillIntent::NamedSkillOrWholeRepo` docstring states the post-`6cd91cf` rule (lone nested skill wins over whole repo) (Fable Std 7); pin the mismatching-name case for both `NamedSkill` and `NamedSkillOrWholeRepo` in `core/tests/git_acquisition.rs` (Opus Spec 3).
- [ ] CONTEXT.md: **Skill discovery** entry says one dedup by subpath — add alias collapse by canonical directory (Astra S2); give **Re-point** its own glossary entry, cross-referenced from **Unlocatable skill** (Opus Std 2).
- [ ] AGENTS.md: rewrap the ~240-char `skillPresentation` sentence to ~100 cols (Opus Std 5).
- [ ] `npm run version:check && npm run check` green; `cargo test --all` green.

Not in this ticket (recorded in `10-followups.md`): finalize atomicity (Astra P2 — inherited from `fe60d94`, every Update has it); internal-symlink-in-skill hash/copy asymmetry (Opus Spec 2 / Fable import residual); stale notification action after delete (Astra P3); `GitRepointModal` `loading` props unused / keep modal open during action (Fable Std 4); `detailSkill` freshness lookup in `App.tsx` (Fable Std 5 / Opus Std 3); duplicate provenance switch card↔hook (Opus Std 4).

## Comments

- 2026-09-15 — Status reconciliation: ready-for-agent → done — a403359. Evidence: git log v1.2.4..v1.2.5 identifies a403359, aligning Re-point grammar and strict resolution; src-tauri/src/core/refresh.rs:230 calls the shared parser and src-tauri/src/core/git_acquisition.rs:588–593 parses the accepted full-URL forms.
