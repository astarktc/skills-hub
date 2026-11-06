# 06: Rows II

Status: ready-for-agent

**Blocked by:** 05. There's no content dependency, but the rows share the registry file, so they land in sequence.

**Spec:** `.scratch/round20/spec.md` § Registry structure (detect-dir hazards are binding; Deep Agents Code copies);
the brief § Part 1 table A, § Part 2 "Recommended Part 1 additions", I4, I7 and I8.

**What to build:** twelve harnesses with their own skills folder become rows.

## Rows

- Grok Build (`.grok/skills`)
- Deep Agents Code (`.deepagents/agent/skills`, **`supports_symlink: false`**)
- Reasonix (`.reasonix/skills`; Windows `%APPDATA%\reasonix\skills` through 01's typed-root override, its first
  production consumer)
- Cortex Code (`.snowflake/cortex/skills`, detect `.snowflake/cortex`, **never** `.snowflake`)
- Rovo Dev (`.rovodev/skills`)
- IBM Bob (`.bob/skills`)
- ForgeCode (`.forge/skills`, detect `.forge` + `forge`)
- Firebender (`.firebender/skills`)
- MiniMax Code (`.minimax/skills`, detect-dir flag in the comment)
- Posit Assistant (`.posit/assistant/skills`, detect `.posit/assistant` + `.positai`, **never** `.posit`)
- AiderDesk (`.aider-desk/skills`)
- Autohand Code CLI (`.autohand/skills`)

## Acceptance criteria

- [ ] Every row is re-verified against its brief source before it's added (as in ticket 05; deviations go in Comments).
- [ ] Each row has everything ticket 05 lists: variant, `as_key`, literal with facts, README row, table-test rows and an
      i18n label.
- [ ] Env-var homes (`GROK_HOME`, `DEEPAGENTS_HOME`, `REASONIX_HOME`, `AUTOHAND_HOME`, `AIDER_DESK_HOME_DIR`) are named
      in the row comments, not honoured.
- [ ] Deep Agents Code deploys by copy. Pin it with a test through `sync_dir_for_tool_with_overwrite`, or the existing
      capability test.
- [ ] Reasonix resolves its Windows override in a test with a substituted `HostRoots`.
- [ ] A draft PR's `gh pr checks` proves the Windows build.
- [ ] `npm run version:check && npm run check` passes.

## Comments
