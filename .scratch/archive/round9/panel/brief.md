# Round-9 architecture panel — fresh explore

Repo: `/Users/alexstark/Projects/skills-hub` (Tauri 2 + React 19; Rust `src-tauri/src`, TS `src`). Branch `main` @ `96cd33c` (v1.2.8). **READ ONLY**: do not edit, commit, run mutating commands, launch the app (`npm run tauri:dev` mutates the operator's real skill library), or `git add` anything. `cargo test --all`, `npm test`, `npm run build` are fine to confirm something. Do not search outside this directory (cloud-synced folders). AGENTS.md Workflow step 1 does not apply — start immediately.

Read first: `AGENTS.md` (canonical context), `CONTEXT.md` (domain glossary — use its terms for the domain), `docs/adr/*.md` (decisions; do not re-litigate unless the friction is real, and say so explicitly), and `~/.pi/agent/skills/codebase-design/SKILL.md` + its `DEEPENING.md` (architecture vocabulary — use **module, interface, implementation, depth, deep, shallow, seam, adapter, leverage, locality** exactly; never component/service/API/boundary/layer/wrapper).

## Task
Surface architectural friction and propose **deepening opportunities**: refactors that turn shallow modules into deep ones, for testability and AI-navigability. Weight recent change: walk `git log --oneline -150` for hot spots (rounds 6–8 touched `install_finalize`, `git_acquisition`, `github_download`, `installer`, `refresh`, `skill_edits`, `frontmatter_edit`, `propagation`, `artifact_removal`, `App.tsx`, `useSkillLibrary`). Explore organically; note where you feel friction:
- understanding one concept requires bouncing between many small modules
- shallow modules (interface nearly as complex as implementation)
- pure functions extracted for testability while the bugs hide in how they are called (no locality)
- tightly-coupled modules leaking across seams
- untested or hard-to-test-through-the-interface areas
Apply the **deletion test** to anything you suspect is shallow. Do NOT propose concrete interfaces/signatures — candidates only.

Known and already scheduled (do not report): backlog hygiene items in `.scratch/round7/backlog.md` (2, 6, 7, 9, 10, 11) and the round-8 cosmetics (extract the S3 repair block from `acquire_resolved`; duplicate no-bearer test).

## Output
Write to `/Users/alexstark/Projects/skills-hub/.scratch/round9/panel/<your-file>.md` incrementally AND return it as the final message. 4–8 candidates, ranked. Per candidate:
- **Title** (names the deepening), **Files**, **Problem** (why it causes friction now — cite lines), **Solution** (plain English, no signatures), **Benefits** in locality/leverage/test terms, **Evidence** (the deletion-test result; concrete call sites; recent commits that paid the cost), **Strength**: Strong / Worth exploring / Speculative, **ADR conflict** if any (only when worth reopening, with why).
End with **Top recommendation** and one paragraph on what you'd NOT deepen (real seams that earn their keep). Concrete; no generic advice; under ~1500 words.
