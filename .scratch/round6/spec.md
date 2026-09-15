# Round 6 — closing the round-5 backlog

Base: `main` = `95b7893` (v1.2.5 + toast fix `f961ce8` + re-point policy `95b7893`). Source: `.scratch/round5/issues/10-followups.md`,
grilled on 2026-09-07; every decision below is the operator's ruling.

## Decisions

- **D1 (#2 finalize atomicity)**: rename-aside. `central` → `central.old` (sibling), `move_into(central)`, `upsert`, then remove
  `.old`. Move failure → rename `.old` back. Upsert failure → remove new `central`, rename `.old` back. If a rollback step
  itself fails, leave `.old` in place and return an error naming it. No journal, no trait.
- **D2 (#2 tests)**: real faults first — `#[cfg(unix)]` chmod on the central parent for the move failure; drop/rename the
  `skills` table (or otherwise break the store) for the upsert failure. A `#[cfg(test)]` fault hook is allowed only for the
  upsert if the store cannot be made to fail cleanly.
- **D3 (#3 internal symlinks)**: *not content*. `hash_dir` skips symlink entries entirely (neither name nor target hashed);
  `copy_dir_recursive` keeps dropping them. Symmetric; no symlink privilege needed; no path escapes the skill dir.
  Re-verify `onboarding_import`'s "the chosen variant's own Tool always among them" claim with a symlink-bearing fixture.
- **D4 (#4 branch names with `/`)**: resolve in `git_acquisition`, not the parser. When a URL has `/tree/<seg1>/<rest>` with
  ≥2 segments after `tree/`, call GitHub `git/matching-refs/heads/<seg1>` once, pick the longest ref that is a prefix of
  `<seg1>/<rest>`, split there; on API failure or no match keep the first-segment split. Preferred cache: a Managed skill
  already persists `source_subpath` — when a stored subpath is a suffix of the tree path, the branch is the remainder and no
  API call is needed (Refresh/Update/Re-point pay nothing after the first resolution). Applies uniformly to Add, Refresh, Re-point.
- **D5 (#5 stale action)**: the notification closure captures the skill **id**; on click, resolve against the latest
  `managedSkills` via a ref; if missing, `notify` a warning (new i18n key, en + zh), no modal. Panel entries are not rewritten.
- **D6 (#6 head-only toast action)**: keep; document in `useStatusReporter`'s batch doc — the toast carries the head entry's
  action, every entry's action lives in the panel.
- **D7 (#7 GitRepointModal)**: keep the modal open through the action. `loading` drives spinner/disabled; close on success
  only; on failure the modal stays with the URL (error still reported by the reporter).
- **D8 (#8 detailSkill)**: `useSkillLibrary` owns `detailSkillId` + derived `detailSkill` (selected from `managedSkills`,
  auto-clears when gone); App only renders.
- **D9 (#9 one door)**: one pure predicate in `src/lib/skillPresentation.ts` (e.g. `repointDoor(skill): "git" | "local"`)
  used by both `SkillCard` (button visibility) and `handleRepointSkill` (dispatch).
- **D10 (#10)**: closed — the "cite titles not SHAs" rule is in AGENTS.md.
- **Lanes**: R1 = #2, R2 = #3, R3 = #4 (Rust, disjoint files), F1 = #5 + #6 + #7 + #8 + #9 (frontend). Merge serially
  R1 → R2 → R3 → F1 with AGENTS.md worktree-safety checks. Then panel (Fable 5.1 / Opus 5 / Astra, medium) over
  `95b7893..HEAD` reusing `.scratch/round5/review-brief.md`, then `version:set 1.2.6`.

## Ticket map

| Lane | Ticket | Model |
| --- | --- | --- |
| R1 | `issues/01-finalize-atomicity.md` | Astra medium |
| R2 | `issues/02-internal-symlinks.md` | Astra low |
| R3 | `issues/03-branch-with-slash.md` | Astra medium |
| F1 | `issues/04-frontend-shape.md` | Astra low |

## Closure — 2026-09-15

Shipped: v1.2.6 / ac8b514. Tickets: 5 terminal (5 done), 0 still open (none).
Residue → BACKLOG: none. Dropped by name: none.
