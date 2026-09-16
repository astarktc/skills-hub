# Round-5 review — brief (one child per model; both axes in one brief)

Repo: ~/Projects/skills-hub (main checkout — READ ONLY; do not edit, commit, run npm/cargo mutating commands, or launch the app; `cargo test`/`npm test`/`npm run build` are fine if you want to confirm something). Do not search outside this directory (cloud-synced folders). Do not run `npm run tauri:dev` (it mutates the operator's real skill library). Do not `git add` anything; `.scratch/` is gitignored on purpose.

## What to review
- Fixed point: `6cd91cf` (v1.2.4 + the ego-lite discovery fix, which is *also* in scope — review it too: `git show 6cd91cf`). HEAD = `378a473`.
- Diff: `git diff ed630c0...HEAD` (v1.2.4 → HEAD, ~20 commits: `git log ed630c0..HEAD --oneline`). Files across `src-tauri/src/**`, `src/**`, `AGENTS.md`, `CONTEXT.md`.
- Spec: `.scratch/round5/spec.md` (problem, stories, decisions Q1–Q17, ticket map). Tickets: `.scratch/round5/issues/01..07-*.md` — each has a `## Comments` section written by the implementer stating what shipped and any deviation; treat those as claims to verify. Source of the round: `.scratch/round4/issues/12-followups.md`.
- Standards sources: `AGENTS.md` (invariants, ambiguity resolution, "Do not" list — primary standard; this round added two sentences: the prose-vs-typed error rule and the "components import pure functions / props carry state" rule), `CONTEXT.md` (glossary — this round widened **Unlocatable skill**/Re-point and **Git acquisition**), `docs/adr/0001..0003-*.md`, `docs/agents/*.md`.

## Axis 1 — Standards
Report, per file/hunk where relevant: (a) every place the diff violates a documented standard — cite the standard (file + rule); (b) any baseline smell you spot — name it and quote the hunk. Distinguish hard violations from judgement calls: documented-standard breaches can be hard, baseline smells are always judgement calls, and a documented repo standard overrides the baseline. Skip anything tooling enforces (eslint, clippy, rustfmt, tsc, specta binding drift). Under 400 words.

Smell baseline (Fowler, Refactoring ch.3 — *what it is → how to fix*): Mysterious Name → rename; Duplicated Code → extract; Feature Envy → move onto the data; Data Clumps → one type; Primitive Obsession → small type; Repeated Switches → one map/polymorphism; Shotgun Surgery → gather; Divergent Change → split; Speculative Generality → delete; Message Chains → hide; Middle Man → cut; Refused Bequest → composition.

## Axis 2 — Spec
Report: (a) requirements the spec/tickets asked for that are missing or partial; (b) behaviour in the diff that wasn't asked for (scope creep); (c) requirements that look implemented but where the implementation looks wrong. Quote the spec/ticket line for each finding. Under 400 words.

Areas worth particular scrutiny this round (not exhaustive):
- **Git Re-point is acquire-first**: can any failure path (bad URL, 404, multi-skill ambiguity, non-installable download, cancel) leave `source_ref`/`source_subpath` rewritten or the central copy replaced? Read `installer::acquire_managed_skill_update_from` and `refresh::repoint_git_skill_with` + `finalize`. Is the override honoured for both adapters (API fast path and clone)? Is `parse_repoint_source`'s URL validation stricter than `parse_github_url` in a way that refuses a legitimate URL shape the Add flow accepts (e.g. a `/blob/` URL, a branch with a slash, `.git` suffix)?
- **Discovery fix (`6cd91cf`)**: `discover_skills` now collapses symlink aliases by canonical directory; `resolve_subpath` takes a lone nested skill as *the* skill. Can the canonicalisation misfire (root behind a symlink, Windows junctions, a link whose target is outside the repo), and does the "lone nested skill" rule change any Add/Update behaviour the tests don't pin?
- **Import** (`onboarding_import::sync_imported_unlocked`): identity now uses the live `target_has_same_content` against the finalized central copy; the chosen variant's own path is no longer accepted unconditionally. Can the chosen original itself now be misreported as divergent (e.g. hash differs because of files `hash_dir` ignores differently from the fingerprint, or a symlinked original)?
- **Notification action**: a closure now lives on a `Notification`; confirm nothing serializes it and that `skillFailureEntries` closing over `managedSkills` cannot pick a stale/wrong skill.
- **Error contract**: every new `SignalError` (`SymlinkChainTooDeep`, `InvalidGithubUrl`, `GitRepointRequiresGit`) has a `CommandError` arm, a `describeCommandError` branch and EN + ZH copy; the new AGENTS.md prose-vs-typed rule is applied consistently to the guards this round touched.
- `project_sync` `expect` → typed `NotFound`: is `NotFound { kind: "skill" }` the right kind for "artifact path could not be resolved"?

## Output
Write your report to `~/Projects/skills-hub/.scratch/round5/r5-review-<model>.md` (where `<model>` is `fable`, `opus` or `sol` — you will be told which) AND return it as your final message. Structure: `## Standards`, `## Spec`, `## Summary` — the summary splits findings into **Blocking** (must fix before v1.2.5 ships: a documented-standard breach, a spec requirement missing/wrong, or a data-safety bug) vs **Follow-up** (everything else). Every finding names file(s) and quotes the hunk or the spec line. Be concrete; no generic advice.
