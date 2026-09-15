# 29: Add-flow — one candidate-pick module, backend-owned name matching

Status: resolved

Type: task
Blocked by: 21, 22

## What to build

Review #2 (all three panelists on the hook width; Fable on matching; orchestrator-verified). `src/hooks/useAddSkillFlow.ts` (841 lines, 43-member return, lint complexity 95 at `943f85c`) is a transcript of its implementation. Most of the width is a **mirrored pair**: git vs local candidate picking — `gitCandidates`/`localCandidates`, `*CandidateSelected`, `show*PickModal`, toggle/toggleAll/cancel/install-selected (`:217-290`, `:701-793`). The mirror has diverged on validation: the local path rejects duplicate names *within* the selection (`:713-724`) and falls back to `selected[0].name` for a single custom name (`:725-728`); the git path (`:758-793`) has neither, so two same-named candidates in one repo proceed to a mid-batch failure. `runBatchInstall` (`:655-699`, ticket 15) already deduped the install→deploy tail — the *selection* half wasn't.

Name-matching policy ("does candidate X match target Y": case-insensitive exact, else bidirectional containment) is implemented four times in three dialects across two tiers: `useAddSkillFlow.ts:528-535` (single-candidate check), `:564-571` (exact else *unique* containment), `installer.rs:1776-1793` `find_skill_by_name` (*first* containment hit, SKILL.md names then dir names), `installer.rs:802-816` update backfill (exact else unique). An Explore install and a later update can resolve the same skill differently.

Deepen (after tickets 21 and 22 have shrunk the hook and reshaped the installer):

- One `useCandidatePick(source)` module owning `{ candidates, selected, visible, validate, install }`, parameterised by a small source adapter (`discover`, `installOne`, reset extras) — two instances (git, local). Validation written once (the within-selection duplicate rule applies to both). Fold the `if (!loading) setShowX(false)` guards into one helper.
- Backend owns matching: one pure core `match_skill_candidate(target, candidates) -> Resolved | Ambiguous(list) | None` used by all backend sites, exposed through the candidate-listing command (e.g. an optional `target_name`) so the frontend's two dialects are deleted and `handleCreateGit` only handles "resolved / pick".
- Tests: `useCandidatePick` at `renderHook` level with a stub source (select-all default, close vs cancel, duplicate rejection, batch install error collection); core table test for matching incl. `react` vs `json-render-react` and the ambiguity case. `useAddSkillFlow.test.ts` keeps passing.
- Plain hooks, no state library; App stays the binder (its wiring should shrink, not grow).

## Acceptance criteria

- [ ] One pick implementation instantiated twice; git and local apply identical selection validation; `useAddSkillFlow`'s return shrinks by ≥ a third.
- [ ] One definition of name matching, in core, with tests; zero matching logic in TypeScript.
- [ ] Vitest and cargo suites green; `npm run version:check && npm run check` green.

## Answer

Landed green in `270c46e` + `da2f77c` (Fable 5.1 child, medium thinking; rebase over 27 had one import-line conflict in `installer.rs`; 357 cargo + 89 vitest, full gate green on main).

**Backend — one matching definition** (`core/skill_matching.rs`): `match_skill_candidate(target, &[T: MatchableSkill { name, subpath, dir_name }]) -> SkillMatch::{Resolved(&T) | Ambiguous(Vec<&T>) | None}`. Rule (case-insensitive, tiered): exact name → containment on name → exact dir name → containment on dir name; the first non-empty tier decides (1 hit = Resolved, >1 = Ambiguous). Used by `find_skill_by_name` (fetch/Explore), the legacy update backfill, and the listing: `list_git_skills(…, target_name: Option<&str>) -> GitSkillListing { candidates, target_match: Option<CandidateMatch> }` with wire enum `CandidateMatch` (`{kind:"resolved",subpath} | {kind:"ambiguous",subpaths} | {kind:"none"}`); `list_git_skills_cmd` gains `targetName`. Bindings `CandidateMatch.ts`, `GitSkillListing.ts` committed. **Behaviour unified**: `find_skill_by_name` took the *first* containment hit — an ambiguous hit is now the `MultiSkills` condition; the backfill's exact tier was case-sensitive and lacked the dir-name tier. Table test incl. `react` vs `json-render-react`, ambiguity, duplicate exact names, blank target, wire shape.

**Frontend — one pick module** (`src/hooks/useCandidatePick.ts`): `useCandidatePick<C, Ctx>(source { customName, selectable?, installOne, resetForm }, deps { t, reporter: Pick<StatusReporter,…>, isSkillNameTaken, deploy, afterBatch }) -> { candidates, selected, visible, open, close, cancel, toggle, toggleAll, install }`. Selection validation written once (empty / multi + custom name / **within-selection duplicates** / taken names) — git now gets the duplicate rule. `useAddSkillFlow` instantiates it twice (`git`, `local`); both TS matching dialects deleted — `handleCreateGit` sends `targetName` and only branches on `target_match`; `handleCreate` dispatches on the modal tab (moved out of App). **Return 43 → 28 members (−35%), 794 → 581 lines.** 14 `useCandidatePick` renderHook tests; `useAddSkillFlow.test.ts` adapted (the two Explore matching tests became "follows the backend resolution").

**Doc**: AGENTS.md hook-import rule clarified by the orchestrator — *world* hooks never import each other; a world may own building-block hooks (this is the first instance).

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
