# Round-3 review — brief (one child per model; both axes in one brief)

Repo: ~/Projects/skills-hub (main checkout — READ ONLY; do not edit, commit, run npm/cargo mutating commands, or launch the app; `cargo test`/`npm test` are fine if you want to confirm something). Do not search outside this directory (cloud-synced folders).

## What to review
- Fixed point: `f400440` (v1.2.2 + nightly featured-skills bump). HEAD = `4d17493`.
- Diff: `git diff f400440...HEAD` (27 commits: `git log f400440..HEAD --oneline`). 2,665 insertions / 581 deletions, 51 files across `src-tauri/src/**`, `src/**`, `scripts/version.mjs`, `AGENTS.md`, `CONTEXT.md`, `package-lock.json`.
- Spec: `.scratch/round3/spec.md` (15 settled decisions Q0–Q15 + verified facts + ticket map). Tickets: `.scratch/round3/issues/01..10-*.md` — each has a `## Comments` section written by the implementer stating what shipped and any deviation. Source tickets: `.scratch/round3/source-13-review-followups.md`, `.scratch/round3/source-14-toast-lifetime.md`.
- Standards sources: `AGENTS.md` (invariants, ambiguity resolution, "Do not" list — this is the primary standard), `CONTEXT.md` (domain glossary — names in code/tests/commits must use these terms), `docs/adr/0001-*.md` (tagged error contract), `docs/adr/0002-*.md` (keep row with error on failed removal), `docs/agents/*.md`.

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

Things the orchestrator already knows (don't spend words re-reporting them; DO say if you disagree with the disposition):
- `core::refresh::tests::acquisitions_overlap_instead_of_running_one_at_a_time` is a wall-clock test that flakes under CPU load — a follow-up, untouched this round.
- The refresh summary notification is `success` even when `failed > 0` — noted as a follow-up.
- `path not found in repo` / `central path not found` / `source path not found` are raw `OTHER` strings leaking absolute (cache-internal) paths — noted as follow-ups.
- Upstream in-repo symlinks (tanstack) break sparse/API acquisition — pre-existing, follow-up.

## Output format
```
## Standards
…
## Spec
…
## Summary
One line: total findings per axis and the worst issue within each axis. Then a list "Blocking" (must fix before release) vs "Follow-up" — with file:line for each.
```
Verify every claim against HEAD (file:line) before reporting it; a finding that doesn't reproduce at HEAD is worse than no finding.
