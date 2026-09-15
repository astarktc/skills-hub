# 26: One skill-discovery module

Status: resolved

Type: task
Blocked by: 22

## What to build

Review #2 (Fable, verified — and the orchestrator found a *third* copy): "discover skill candidates in a directory tree" is one concept implemented three times in `core/installer.rs` at `943f85c` — `list_git_skills` (`:996-1106`), `list_local_skills` (`:1108-1297`), and a third scan tail ending at `:1037-1038` — each hand-rolling the same multi-strategy ladder (root SKILL.md → known scan bases → `marketplace.json` → recursive depth-5) and the same `out.sort_by(...); out.dedup_by(|a,b| a.subpath == b.subpath)` (`:1037`, `:1102`, `:1293`), with drifted details: the local side re-parses SKILL.md for marketplace hits `scan_marketplace_skills` already parsed (`:1225-1250`) and tracks `valid`/`reason` per candidate; the git side does neither. The ~20 private scan helpers (`:393-723`) are a de-facto module with no interface. Two real adapters exist, so the seam is real.

Deepen (lands on top of ticket 22's reshaped installer):

- One entry, e.g. `discover_skills(root) -> Vec<DiscoveredSkill { subpath, name, description, validity }>`; git listing, local listing and `update_managed_skill_from_source`'s `scan_skill_candidates_in_dir` (`:688-700`) become thin adapters mapping to their DTOs. Validity becomes an explicit adapter decision, not an accident of which copy ran.
- Whether this is a submodule of `installer` or its own `core/skill_discovery.rs` is your call (declare it in `core/mod.rs` if new).
- Tests: fixture-tree → candidate-list, independent of git and of install flows (root SKILL.md, scan base, marketplace, deep recursion, dedup).

## Acceptance criteria

- [ ] Exactly one implementation of the scan ladder and one sort/dedup; three callers use it.
- [ ] Fixture-tree tests cover each strategy and the dedup; existing git/local listing behaviour unchanged for the app (including validity info on the local path).
- [ ] `npm run version:check && npm run check` green.

## Answer

Landed green in `6f06e32` (Fable 5.1 child, medium thinking; rebase over 24/25 had one trivial `core/mod.rs` module-list conflict; 330 cargo + 67 vitest, full gate green on main).

**Interface** (new `core/skill_discovery.rs`):
```rust
pub fn discover_skills(root: &Path) -> Vec<DiscoveredSkill>
pub struct DiscoveredSkill { subpath, name, description: Option<String>, validity: Validity }
pub enum Validity { Valid, InvalidSkillMd(&'static str /* read_failed|invalid_frontmatter|missing_name */), MissingSkillMd }
// Validity::{is_valid, is_installable, reason}
```
Ladder: root `SKILL.md` → known scan bases → root-level skills / `*skill*` containers → `marketplace.json` → depth-5 walk; one `HashSet` first-seen dedup by subpath, one sort by (name, subpath). Validity is reported, never enforced — adapters decide: `installable_skills_in_repo` (update backfill + `fetch_skill_files`; installable ∧ not root), `git_candidates_in` (installable; folder-URL scoping kept), `list_local_skills` (everything, `valid`/`reason` carried). `is_installable` is the same predicate `ensure_installable_skill_dir` enforces at install time, so listing and install agree by construction. Installer −841 lines; ~20 private scan helpers + the SKILL.md parser moved; `install_finalize` imports `find_skill_md`/`parse_skill_md` from the module. No wire DTO change.

**Drift found and resolved** (git's rule won where they differed): git listing ran marketplace + recursion twice (dropped); `dedup_by` after name-sort only removed *adjacent* duplicates (proper dedup); local side re-parsed SKILL.md for marketplace hits (one inspect per candidate); validity only on local side (computed for all, policies preserved per adapter); broken-SKILL.md fallback desc (folder + `plugin.json` desc everywhere); root fallback name on parse failure (`"root-skill"` everywhere — the one deliberate local-side change, cosmetic: the entry is invalid and shows its reason); marketplace hits `canonicalize()`d so under a symlinked root (macOS `/var` → `/private/var`) **absolute paths leaked as subpaths** — latent bug in both listings, fixed; `skills/.curated`/`.experimental`/`.system` surfaced as invalid local candidates (scan-base children that are scan bases are now skipped); path-sort vs name-sort unified.

**Drift disclosed later (review #3, verified, kept as-is)**: (a) the folder-URL git branch (`github.com/o/r/tree/main/<dir>`) now runs the full discovery ladder inside the folder (marketplace + depth walk via `git_candidates_in` → `discover_skills(scan_root)`); base ran only `collect_skill_dirs(&dir)` — a widening, judged desirable. (b) the local listing admits the root when `is_claude_skill_dir(root)` even with no `SKILL.md`; base required root `SKILL.md`. Such a root lists as valid and is then rejected at install with `SKILL_INVALID` (`install_local_skill_from_selection` requires the manifest) — cosmetic (error either way); aligning the local `valid` rule with the install path's manifest requirement is a candidate for a later polish pass.

Tests: 22 fixture-tree tests (per strategy, recursion depth, dedup/order, parser) + 4 adapter-policy tests; helper-level installer tests moved. CONTEXT.md gained **Skill discovery** and **Skill candidate** (installable vs valid).

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
