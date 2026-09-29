# 06 — Re-point preview command; local skills expose their recorded source on the wire

Status: ready-for-agent
Blocked by: —
Spec: `.scratch/round19/spec.md` — D15 (grill Q15). Source: `flows.md` Flow 5 gaps 1–3.

## Work

- **`preview_repoint_source(skill_id, target)`** (`core/repoint.rs`, beside `repoint_skill_source`): validates and
  resolves the `RepointTarget::{Git, Local}` exactly as the real re-point would (same acquisition intent, listing
  clone reused through the cache), **writes nothing**, and answers `{ resolved_name, ref, subpath, name_matches:
  bool, candidates: Vec<…> | null }` — `candidates` non-null when the target is a multi-skill repo so the frontend can
  open the picker (reuse listing's candidate admission). Typed refusals (`GITHUB_SKILL_NOT_FOUND`, …) are thrown as
  today's command errors.
- **`source_ref` on the wire for local skills.** `skillSourceLabel` returns `central_path` for every non-git skill, so a
  local skill never shows its folder — the one fact the source-missing repair needs. Confirm `ManagedSkill` carries
  `source_ref`; if it does, this half is frontend-only (`skillPresentation.ts` + test + `SkillCard`/detail source
  line show the folder, copyable). The Re-point modal pre-fills the current source ("was: ~/Downloads/scratchpad").
- Bindings; shims; fixture handler for the preview (name mismatch and multi-skill producible in `failures`);
  tests: preview writes nothing, mismatch flagged, multi-skill answers candidates, local source shown.
- The confirm-on-mismatch UI and inline errors are ticket 12.

## Result

(child fills in)
