# 36: Should a virtual group inherit its constituents' symlink capability?

Status: resolved

Type: grilling
Source: review #3 S3 (Sol; verified as pre-existing, downgraded from "regression") — [verdict](../assets/review-3/verdict.md)

## The question

At project scope, Cursor is absorbed into the `agents_skills` virtual group (`tool_adapters/catalog.rs:55-78`). The group's own registry literal is `supports_symlink: true` (`tool_adapters/mod.rs:180-187`) while Cursor's is `false`. `assign_and_sync` (`project_sync.rs:40-61`) passes the **group** adapter to `sync_dir_for_tool_with_overwrite`, so a project whose only installed constituent is Cursor gets `.agents/skills/<skill>` as a **symlink** — which AGENTS.md's do-not says Cursor cannot read.

This is **not** a regression: at `943f85c` project sync was keyed by the string `"agents_skills"`, never `"cursor"`, so the old `eq_ignore_ascii_case("cursor")` check never fired at project scope either. Ticket 23 made the capability a field but did not change what the group's field means.

Decide:

1. **Is it a real defect?** Does Cursor actually read `<project>/.agents/skills/` at all, and if so does it reject a symlinked dir there the way it does under `~/.cursor/skills`? Evidence needed (Cursor docs / an operator check with explicit permission — dev runs mutate real state).
2. **If yes, what's the rule?** Options:
   - (a) A group's effective capability = AND over its *installed* constituents (Cursor installed → the whole group copies). Cost: 8 other tools lose symlinks because Cursor is present. Needs `home` at the sync call.
   - (b) Static: set the group literal to `supports_symlink: false` (always copy `.agents/skills`). Simplest; same cost for everyone regardless of Cursor.
   - (c) Leave as-is and document that project-scope `.agents/skills` is symlinked; Cursor users needing project skills rely on Cursor's own dir. Record in CONTEXT.md **Virtual group**.
3. Whichever: the CONTEXT.md **Tool capability** / **Virtual group** entries should state how a group's capability relates to its constituents, and a test should pin it.

## Acceptance criteria

- [ ] Decision recorded under `## Answer` with the evidence for (1).
- [ ] If (a) or (b): implementation + a `tests/sync_engine.rs` / `tests/project_sync` case proving a Cursor-only home copies at project scope; CONTEXT.md updated.
- [ ] If (c): CONTEXT.md + AGENTS.md do-not amended to scope the Cursor rule to global sync.

## Answer

**Decision: (c) — leave the code as-is; a virtual group's capability is its own registry fact, not the AND of its constituents.** Docs amended (CONTEXT.md **Tool capability** / **Virtual group**; AGENTS.md Cursor do-not now scopes the rule to the Cursor entry and records this decision).

### Evidence for (1)

- Cursor loads project-level skills from **both** `.cursor/skills/` and `.agents/skills/` (official docs, https://cursor.com/docs/skills — "Skills are automatically loaded from … `.agents/skills/` Project-level"). The CLI changelog adds `.claude/skills` and `.codex/skills` as fallbacks. So a project-scope `.agents/skills` symlink *is* something Cursor sees.
- The symlink defect that motivated `supports_symlink: false` was a **home-directory** discovery bug (`~/.cursor/skills`, also `~/.cursor/rules`), reported Dec 2025–Jan 2026 (forum threads 146010, 149693, 150028, 151014). In thread 149693 a user reports a **project-scope** symlink (`.cursor/skills → .codex/skills`) working on 2.4.21 and asks "does this only affect the home directory?"; Cursor staff (Colin): "sounds like that's the case". Staff then confirmed the fix shipped in **Cursor 2.5** (Feb 20, 2026: "This has been fixed in 2.5!"), and the CLI changelog carries "Skills found through symlinks. The skills menu follows symlinked directories when discovering skills." One later report (2.5.22) says symlinked skills still vanish from the Settings tab after restart — so the global-dir fix may be incomplete for the IDE's skills UI, which is why the Cursor entry's copy default is left in place for now.
- Net: project-scope `.agents/skills` symlinks were never the broken path; forcing the whole 9-tool group to copy for Cursor's sake would cost every other AGENTS-standard tool its symlink for a defect that does not apply there.

### Why not (a)/(b)
- (a) AND-over-installed-constituents makes a group's sync mode depend on which other tools happen to be installed — surprising, needs `home` threaded into the sync call, and untestable without a temp home per case.
- (b) static copy for the group penalises 8 tools permanently for a bug that was home-dir-only and is now fixed upstream.

### Follow-up fog (recorded in map.md → Not yet specified)
Is Cursor's `supports_symlink: false` itself obsolete now that 2.5 follows symlinks? Revisit with an operator check on a current Cursor build (dev runs touch real state — explicit permission needed). If yes: flip the field, delete the AGENTS.md do-not + README FAQ, keep the capability mechanism (it is the right shape regardless).

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
