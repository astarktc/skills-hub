# Round-4 review — brief (one child per model; both axes in one brief)

Repo: ~/Projects/skills-hub (main checkout — READ ONLY; do not edit, commit, run npm/cargo mutating commands, or launch the app; `cargo test`/`npm test`/`npm run build` are fine if you want to confirm something). Do not search outside this directory (cloud-synced folders). Do not run `npm run tauri:dev` (it mutates the operator's real skill library).

## What to review
- Fixed point: `f057190` (v1.2.3). HEAD = `7fcec94`.
- Diff: `git diff f057190...HEAD` (44 commits: `git log f057190..HEAD --oneline`). 5,904 insertions / 590 deletions, 72 files across `src-tauri/src/**`, `src/**`, `CONTEXT.md`, `docs/adr/0003-*.md`.
- Spec: `.scratch/round4/spec.md` (settled decisions Q0–Q12 — with amendments recorded inline at tickets 06 and 09 — plus user stories, implementation/testing decisions, verified facts, ticket map). Tickets: `.scratch/round4/issues/01..09-*.md` — each has a `## Comments` section written by the implementer stating what shipped and any deviation. Source ticket: `.scratch/round4/source-12-followups.md`.
- Standards sources: `AGENTS.md` (invariants, ambiguity resolution, "Do not" list — this is the primary standard), `CONTEXT.md` (domain glossary — names in code/tests/commits must use these terms; this round added **Provenance** and **Unlocatable skill**), `docs/adr/0001-*.md` (tagged error contract), `docs/adr/0002-*.md` (keep row with error on failed removal), `docs/adr/0003-*.md` (imported skill has no external source — new this round), `docs/agents/*.md`.

## Axis 1 — Standards
Report, per file/hunk where relevant: (a) every place the diff violates a documented standard — cite the standard (file + rule); (b) any baseline smell you spot — name it and quote the hunk. Distinguish hard violations from judgement calls: documented-standard breaches can be hard, baseline smells are always judgement calls, and a documented repo standard overrides the baseline. Skip anything tooling enforces (eslint, clippy, rustfmt, tsc, specta binding drift). Under 400 words.

Smell baseline (Fowler, Refactoring ch.3 — each is *what it is → how to fix*):
- Mysterious Name: a name that doesn't reveal what it does or holds → rename; if no honest name comes, the design's murky.
- Duplicated Code: the same logic shape in more than one hunk/file → extract the shared shape.
- Feature Envy: a method reaching into another object's data more than its own → move it onto the data.
- Data Clumps: the same few fields/params travelling together → bundle into one type.
- Primitive Obsession: a primitive/string standing in for a domain concept → give it a small type.
- Repeated Switches: the same switch/if-cascade on the same type recurring → polymorphism or one shared map.
- Shotgun Surgery: one logical change forcing scattered edits across many files → gather into one module.
- Divergent Change: one module edited for several unrelated reasons → split.
- Speculative Generality: abstraction/params/hooks for needs the spec doesn't have → delete, inline back.
- Message Chains: long a.b().c().d() navigation → hide behind one method.
- Middle Man: something that mostly delegates onward → cut it.
- Refused Bequest: an implementer ignoring most of what it inherits → composition.

## Axis 2 — Spec
Report: (a) requirements the spec/tickets asked for that are missing or partial; (b) behaviour in the diff that wasn't asked for (scope creep); (c) requirements that look implemented but where the implementation looks wrong. Quote the spec/ticket line for each finding. Treat each ticket's `## Comments` deviations as claims to verify, not as accepted. Under 400 words.

Areas worth particular scrutiny this round (not exhaustive):
- Data safety of the two startup passes: `ensure_schema` V9 (and the `user_version` fix) and `legacy_reclassification` — can either misclassify a genuine `local` skill or fail a second launch?
- `refresh_eligibility` vs `is_refreshable` — do Refresh (all), single Update/Restore, and the listing DTO agree, and can auto-sync reassert ever mint a target for an unlocatable or imported skill?
- Symlink resolution in `repo_subpath::LinkChain` / `git_fetcher::symlink_on_path` / `github_download` — can a target escape the repo root, and is the cache entry only ever widened?
- Import take-over (`onboarding_import::sync_imported_unlocked`) — can a divergent sibling be overwritten?
- The error contract — every new `SignalError` has a `CommandError` arm, a `describeCommandError` branch and EN + ZH copy; no prose carries a path.

## Output
Write your report to `$TMPDIR/r4-review-<model>.md` (where `<model>` is `fable`, `opus` or `sol` — you will be told which) AND return it as your final message. Structure: `## Standards`, `## Spec`, `## Summary` — the summary splits findings into **Blocking** (must fix before v1.2.4 ships: a documented-standard breach, a spec requirement missing/wrong, or a data-safety bug) vs **Follow-up** (everything else). Every finding names file(s) and quotes the hunk or the spec line. Be concrete; no generic advice.
