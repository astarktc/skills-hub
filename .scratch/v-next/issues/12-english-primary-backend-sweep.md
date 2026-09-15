# 12: English-primary backend — finish the no-prose sweep

Status: resolved

Type: task
Blocked by: 10

## What to build

Standing policy (decided at the epic review): **English-primary — the backend composes no user-facing prose in any language; all locales live in the frontend catalog.** The epic's ADR states this as fact, but four verified sites still violate it (locations as of `be9a74c`):

1. `src-tauri/src/commands/mod.rs` `delete_managed_skill` (~:1064): Chinese `bail!("已删除托管记录，但清理部分工具目录失败：…")` — composed inside a command body. An EN user whose deletion hits a locked tool dir gets a Chinese toast.
2. `src-tauri/src/core/git_fetcher.rs` (~:46): Chinese "git 命令执行失败…" exec-failure bail.
3. `src-tauri/src/core/git_fetcher.rs` (~:372): Chinese "git 操作超时…" timeout bail.
4. `src-tauri/src/core/installer.rs` `fetch_skill_files` (~:1690): sniffs `"404"`/`"403"` and composes *English* user prose backend-side ("Skill not found on GitHub…", "GitHub API access denied…") — same defect class, different language.

Each becomes a typed condition rendered by `describeCommandError` with EN+ZH keys. Prefer existing machinery: the git sites likely map onto `GitCloneFailed` kinds (a `timeout` kind already exists in `GIT_CLONE_HINT_KEYS`); 403 is plausibly `RATE_LIMITED`; the rest may need new `SignalError`/`CommandError` variants (ticket 10 has made the frontend whitelist compiler-checked, so a new variant can't be silently dropped). Diagnostic detail (stderr, paths, env-var hints) may ride in a `detail` field the frontend renders verbatim under the localized headline.

Also:
- Delete the dead ZH sniff branch in `describeOther` (`src/commandError.ts` ~:65, `未在该仓库中发现可导入的 Skills`) — verified zero backend producers; its only producer is the frontend's own catalog, so it silently rewrites ZH users' messages and never fires for EN.
- AGENTS.md: state the English-primary policy in the error-contract section (backend composes no user-facing prose; new backend messages/comments are English).

New variants = derive `TS` + `cargo test` regenerates bindings (commit them) + `describeCommandError` branch + EN & ZH keys, per AGENTS.md.

## Acceptance criteria

- [x] All four sites raise typed conditions; no user-facing prose composed in the backend (grep `src-tauri` for CJK in `bail!`/`format!` user paths → only comments remain).
- [x] Dead ZH sniff branch deleted; EN and ZH users get equivalent localized copy for every touched failure path.
- [x] Bindings regenerated & committed; `describeCommandError` handles every new code; EN+ZH keys added.
- [x] AGENTS.md states the English-primary policy.
- [x] `npm run version:check && npm run check` green (`> /tmp/gate.log 2>&1; echo $?`).

## Answer

Landed green in `cf2b1cd` (Fable 5 low subagent; orchestrator-verified, rebased, combined-tree gate green). Site→variant mapping: delete-cleanup → new `DELETE_CLEANUP_FAILED { failures }`; git exec-failure → `GIT_CLONE_FAILED` with new kind `execFailed`; git timeout → existing kind `timeout`; 404 → new `GITHUB_SKILL_NOT_FOUND { url }`; 403 → existing `RATE_LIMITED` (the pre-existing `resetMinutes > 0` guard renders the no-ETA case). Env-var hints + stderr ride in `detail` (diagnostics, not copy). Dead ZH sniff branch deleted; ticket 10's compiler-derived map forced registration of both new codes, as designed. EN+ZH keys added; AGENTS.md now states English-primary. Remaining CJK in `src-tauri` is comments + test assertion messages — ticket 18's scope. Leftover: `noSkillsFoundInRepo` i18n keys now unused (flagged to ticket 16/18 cleanup).

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
