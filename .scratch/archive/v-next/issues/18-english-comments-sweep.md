# 18: English comments sweep — translate the fork-heritage Chinese comments

Status: resolved

Type: task
Blocked by: 11, 12, 13, 14, 16

## What to build

Per the English-primary policy (decided at the epic review): translate all remaining Chinese code comments (upstream-fork heritage, mostly in `src-tauri/`) to accurate English. Comments and doc-comments only — **zero behavioral or structural code changes**. Preserve each comment's technical meaning; where a comment is stale or wrong, translate faithfully and add a `TODO:` note rather than silently "fixing" it.

Blocked by every Rust-touching ticket so the sweep lands last and can't conflict with them.

Also: AGENTS.md's "Environment gotchas" note ("Rust source carries Chinese comments … this is expected, not corruption") becomes obsolete — remove or rewrite it to record that comments are now English (policy: English-primary).

## Acceptance criteria

- [x] No CJK characters remain in `src-tauri/src/` or `src/` outside i18n catalog values and test fixtures that deliberately exercise ZH strings (`rg -n '[\p{Han}]'` to verify).
- [x] Diff is comment-only (spot-check with `git diff --word-diff`); `cargo test` and the full gate prove nothing behavioral moved.
- [x] AGENTS.md heritage note updated.
- [x] `npm run version:check && npm run check` green (`> /tmp/gate.log 2>&1; echo $?`).

## Answer

Landed green in `943f85c` (Fable 5 low subagent; orchestrator-verified — word-diff spot-checked for faithful translation, CJK grep re-run firsthand: zero matches outside the i18n catalog). 14 sites across 9 files (4 comments incl. the Cursor force-copy rationale, 10 test assertion messages) + the obsolete AGENTS.md heritage gotcha removed. No kept-ZH fixtures existed. Earlier tickets had already cleared most of the footprint — the sweep was smaller than estimated. **This closes the epic-review fix-forward effort: all 9 tickets (10–18) resolved.**

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
