# Round 5 — ticket-12 triage, git Re-point, and the discovery fix

**Status:** ready-for-agent
**Base:** `main` @ `6cd91cf` (v1.2.4 + the discovery fix). **Ships as v1.2.5.**
Source: `.scratch/round4/issues/12-followups.md` (13 items, triaged 2026-09-08 by grilling, decisions Q1–Q17 below) plus the ego-lite preview bug found in the operator's 1.2.4 smoke test.

## Problem Statement

Round 4 left thirteen non-blocking review findings untriaged, and the operator's first real use of 1.2.4 surfaced two things: previewing `citrolabs/ego-lite`'s only skill from Explore failed with "this repository contains multiple skills" (a bug — fixed on main in `6cd91cf`), and seven Managed git skills failed Refresh because their upstream repos **moved** them (`get-convex/agent-skills` renamed every skill; `vercel-labs/agent-browser` moved `skills/<x>` to `skill-data/<x>`). The app's only answer to a moved skill is "check this URL" + Remove + re-Add, which throws away the record, its Sync targets and its project assignments — even though round 4 built exactly this repair (Re-point) for local sources.

## Solution

A git skill whose upstream moved is repaired in place: the operator pastes the skill's new GitHub URL, the app acquires from it, and only on success rewrites the source and Updates every target. The review residue is settled once each: `formatError` stops travelling as a prop, the error contract gains a rule for when prose is acceptable, i18n keys stop mirroring wire enums, a panic on a command path becomes a typed error, an import override that the same-content policy already covers is dropped, and the glossary/naming residue is cleaned.

## User Stories

1. As an operator whose upstream repo moved a skill, I want to Re-point the Managed skill at its new GitHub URL from its card, so that the record, its targets and project assignments survive.
2. As an operator, I want Re-point to refuse a URL it cannot acquire from without touching my record, so that a typo never strands a working skill.
3. As an operator, I want Re-point to accept a URL whose repo *and* subpath differ from the recorded ones, so that a repo move is repaired the same way as a folder rename.
4. As an operator, I want the Refresh failure for a skill not found on GitHub to point me at Re-point, so that I learn the repair where I meet the problem.
5. As an operator, I want to trigger Re-point from that failure notification itself, so that the fix is one click from the report.
6. As an operator, I want the Re-point form to reject non-GitHub / malformed URLs with a clear message before any network call.
7. As a maintainer, I want components to obtain error copy by calling the pure `describeCommandError` with the `t` they already hold, so that no prop is an import with an argument pre-bound.
8. As a maintainer, I want AGENTS.md to state precisely which functions are binder-passed (stateful: `notify`, `runAction`, data, actions) and which are imported (pure), so that the round-3 direction and the round-2 rule stop contradicting.
9. As a maintainer, I want one rule for prose-encoded errors — a guard the UI cannot reach may bail with prose; a condition the operator or upstream can cause is typed — recorded in AGENTS.md.
10. As an operator, I want an upstream symlink chain that exceeds the depth bound reported as a typed error with its own copy (EN + ZH), not as an internal error.
11. As a maintainer, I want the `unlocatable` state/repair → i18n key mapping to live in `skillPresentation.ts` as a record, so that the i18n catalog no longer mirrors wire-enum spelling.
12. As an operator, I want a project assignment whose skill row vanished mid-sync to surface as a typed not-found error, never a panic.
13. As a maintainer, I want the global-rows Propagation path to resolve each row's adapter once.
14. As a maintainer, I want the onboarding import to rely on `overwrite_if_same_content` alone, with a test proving a divergent original is still never overwritten.
15. As a maintainer, I want the glossary to state the API acquisition path's leaf-link-only limitation and the Re-point validation rule.
16. As a maintainer, I want `legacy_reclassification`'s two path helpers to be one registry call and one clearly named shape check.
17. As a maintainer, I want code comments to state the rule they enforce instead of citing a gitignored `spec Qn`.
18. As an operator, I want the changes released as v1.2.5 with a CHANGELOG entry that names the ego-lite fix and git Re-point.

## Implementation Decisions

- **Q1 → (c)** `formatError` leaves the component-facing surface (`ProjectsPage`, `SkillDetailView`, `SettingsPage`); components call `describeCommandError(err, t)`. Hooks that lack `t` keep it as an internal seam of the reporter world. AGENTS.md sentence sharpened, not softened.
- **Q2 → (a)** API path stays leaf-link-only; CONTEXT.md **Acquisition** records the limitation.
- **Q3 → (a)** Drop the per-tool `overwrite: true` overrides in import iff `is_identical_to_chosen` (fingerprint equality) and `target_has_same_content` (dir hash) agree on identity — verify first; the ticket makes them agree if they do not.
- **Q4** #4 closed (CHANGELOG already records it). **Q4/#5** CONTEXT.md **Unlocatable skill** gains the Re-point validation rule (must be a skill folder; must not sit inside a Tool's skills dir — that is Import's door).
- **Q5** Rule adopted (story 9). `LinkChain` depth bound → typed `SignalError`/`CommandError` variant; `unlocatable::require_local` stays prose.
- **Q6** `{{target}}` interpolation stays in both `symlinkEscapesRepo` and `pathOutsideToolDirs`.
- **Q7** camelCase i18n keys + `Record<UnlocatableState, key>` / `Record<Repair, key>` maps in `skillPresentation.ts`.
- **Q8** `project_sync::sync_assignment_target` returns typed `NotFound` instead of `expect`.
- **Q9** `tool_owning_path` deleted if it wraps `tool_holding_path`; `tool_shaping_path` → `tool_by_home_shaped_prefix`.
- **Q10** Five `spec Qn` comments rewritten inline; the two `v-next ticket 38` citations stay (AGENTS.md anchors them).
- **Q11** Single adapter lookup on global rows, folded into the Q8 ticket.
- **Q12/Q13 git Re-point**: input is a full GitHub URL parsed by the existing GitHub URL parser (repo and subpath may both change). Surface: card/detail action on every `git` skill (deepens the existing Re-point — one action, two provenances), plus an action on the "not found on GitHub" refresh-failure Notification. Semantics: **acquire first**; only on success rewrite `source_ref` + `source_subpath` and finalize + Propagate through the normal Update path. A failed acquisition leaves the record byte-identical. Refused for non-git provenance (typed). This is Update with a source override, not a new pipeline.
- **Q14** The operator waits for Re-point rather than re-Adding the seven rows; they are its first real test.
- **Q16** Version **1.2.5**.
- **Q17** Children: Pi harness, `openai-codex/gpt-6-astra`, thinking `medium` (experiment); panel unchanged (Fable 5.1 / Opus 5 / GPT-5.6 Sol, round-4 brief).

## Testing Decisions

Tests assert observable behaviour through existing seams: `acquire` / the Refresh entry points (`core/tests/refresh.rs`, `git_acquisition.rs`), `unlocatable` tests, `onboarding_import` tests, `project_sync` tests, the commands wire tests (`commands/tests/commands.rs`) for new variants, hook-level vitest with the `invokeTauri` mock (`useSkillLibrary.test.ts`, `useStatusReporter.test.ts`). Prior art: round-4 ticket 09's Re-point tests (success / cancelled / refused / Update failure) are the template for git Re-point. No JSX tests.

## Out of Scope

Prefix probing on the API path (Q2 b); `withDetail` for path prose (Q6); a Refresh that auto-detects moves; the AGENTS.md "Ambiguity resolution" cleanup pass beyond the two sentences named.

## Ticket map

```
01 git Re-point (card) ─► 07 Re-point from the failure Notification
02 formatError retirement      (independent)
03 error contract + i18n maps  (independent)
04 project_sync / propagation  (independent)
05 import override drop        (independent)
06 docs / naming residue       (independent)
08 panel + version 1.2.5       (blocked by all)
```

## Closure — 2026-09-15

Shipped: v1.2.5 / a3fe9d1. Tickets: 4 terminal (1 superseded, 3 done), 0 still open (none).
Residue → BACKLOG: #25. Dropped by name: none.
