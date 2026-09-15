# 08: Add → local folder refuses a folder inside a Tool's skills dir and steers to Import

Status: done — 2892049

**What to build:** When the operator adds a skill from a local folder and that folder lives inside any Tool's skills directory, the add is refused with a message that says the skill already lives in a Tool and that Import is how to take it over — so no skill is ever again created with a source the app will later overwrite or delete. The check is the registry's own (the inverse of the rule that refuses deleting outside every Tool dir), never a hand-rolled path test. Ordinary folders anywhere else are unaffected.

Source: `../spec.md` Q6; ADR-0001.

**Blocked by:** 01 (typed-error pattern for this round), 06 (`imported` provenance — the copy names Import as the alternative)

- [x] Core test: `install_local_skill` on a path under a Tool skills dir in the temp home raises the typed condition; nothing is copied into central and no record is written
- [x] Core test: a path outside every Tool dir installs as before
- [x] The command seam maps it to its own code; `describeCommandError` branch + EN and ZH copy that name Import
- [x] Hook test: the Add flow surfaces the refusal as an error Notification with that copy
- [x] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Status reconciliation: ready-for-agent → done — 2892049. Evidence: Cited 2892049 refuses Tool-directory sources; src-tauri/src/core/installer.rs:137 constructs LocalSourceInsideToolDir. Registry/EN-ZH/hook slices 240c1ee,1170eae,bde8aca shipped v1.2.4.

### 2026-09-05 — implementation (branch r4/08-add-refuses-tool-dir-paths)

**Shipped** (4 commits on top of main `16d4617`):
- `240c1ee` feat(registry): `tool_adapters::tool_holding_path(home, path) -> Option<&'static ToolAdapter>` — which Tool's *global* skills dir holds a path; the inverse of `ensure_path_within_tool_dirs`. Answers by the path as spelled (lexical `starts_with`) **or** by what it resolves to (`canonicalize` of both sides when both exist — `/private/var` aliases, a link from elsewhere into a Tool dir). Tests (`core/tests/tool_adapters.rs`): inside → names that Tool (claude_code/pi/cursor), outside → `None` (Documents, the tool root `.claude`, `/`), and `#[cfg(unix)]` a symlink from `~/Documents/alias` into `~/.claude/skills/x` → `claude_code`.
- `2892049` feat(add): `installer::install_local_skill` asks the registry first — `Some(holder)` → `bail!(SignalError::LocalSourceInsideToolDir { path, tool })` (`tool` = registry key), via a private `local_source_inside_tool_dir(path, holder)` helper mirroring `source_path_missing`. `CommandError::LocalSourceInsideToolDir { path, tool }` + `From` arm → wire `LOCAL_SOURCE_INSIDE_TOOL_DIR`; `src/bindings/index.ts` regenerated and committed. Tests: `installer::adding_a_folder_inside_a_tool_skills_dir_is_refused_before_anything_is_written` (typed value; central dir never created; `list_skills` empty; the Tool's copy untouched), `installer::selecting_a_candidate_inside_a_tool_skills_dir_is_refused_the_same_way` (the selection flow reaches `install_local_skill`, so it is refused too), `installer::adding_a_folder_outside_every_tool_skills_dir_installs_as_before` (a folder under the temp home but outside every Tool dir → `local` provenance, `source_ref` = the folder), `commands::local_source_inside_tool_dir_serializes_the_path_and_the_tool_key` (wire JSON through `from_anyhow` with a context layer).
- `1170eae` feat(errors): `describeCommandError` branch — Tool label via `t(\`tools.${e.tool}\`, { defaultValue: e.tool })` (the same catalog the cards use for `imported_from_tool`), refused path as the detail line through `withDetail`; `COMMAND_ERROR_CODE_MAP` entry; EN `errors.localSourceInsideToolDir` = "This folder is already inside {{tool}}'s skills directory. Use Import to take over a skill that lives in a tool:" + ZH "该文件夹已位于 {{tool}} 的 Skills 目录内。要接管已在工具中的 Skill，请使用“导入”：". Test `commandError.test.ts` ×1.
- `bde8aca` test(add): `useAddSkillFlow.test.ts` — single valid local candidate, `installLocalSelection` rejects with the structured payload, `formatError` wired to the real `describeCommandError`; asserts `setError` (the reporter's one-shot error channel, which its effect records as an error Notification) received the new key's output with the Tool label and the path, and that nothing was deployed / no success toast.

**Gate**: `npm run version:check` OK (1.2.3); `npm run check` green — vitest 13 files / **196** tests (was 193), build OK, rustfmt clean, clippy `-D warnings` clean, `cargo test` **491/491**; `cargo test --all` 491/491. Worktree clean.

**Deviations**
1. `ensure_path_within_tool_dirs` was **left alone** rather than rewritten over `tool_holding_path`. The two are not the same predicate once resolution is in play: the deletion rule judges the artifact path *as spelled* (a link in a Tool dir resolving to central must still be deletable — lexical says yes — but a link outside that resolves *into* a Tool dir must not be), so reusing the resolving predicate would widen what may be deleted. The doc comment on `tool_holding_path` names it the inverse; the commit message records why deletion stays lexical.
2. The refusal fires at **install** time (`install_local_skill`), not at listing (`list_local_skills`). The ticket asks for the add to be refused; with a multi-candidate base path under a Tool dir the operator will see the picker first and then one collected refusal per picked candidate. Refusing at listing would be a UX improvement but a second predicate call site — not asked for.
3. Wire field `tool` carries the **registry key** (like `ToolNotInstalled`/`UnknownTool`, and ticket 06's `imported_from_tool`), not a display name; the frontend localizes it. No lookup existed in `describeCommandError`, so the branch does the `tools.*` lookup inline (one expression, same shape as `SkillCard`).

**Notes for the orchestrator**
- Merge touchpoints: `core/tool_adapters/mod.rs` (one new `pub fn` directly above `ensure_path_within_tool_dirs`), `core/installer.rs` (one `use`, the guard in `install_local_skill`, one private fn beside `source_path_missing`), `core/errors.rs` (variant + Display arm, appended), `commands/error.rs` (variant + `From` arm, appended), `src/bindings/index.ts` (one union member), `src/commandError.ts`, `src/i18n/resources.ts`, `src/commandError.test.ts`, `src/hooks/useAddSkillFlow.test.ts`, 3 test files under `src-tauri`. Not touched: `provenance.rs`, `skill_store.rs`, `lib.rs`, card/detail UI, `install_imported_skill`.
- Ticket 07 overlap: if its private "which Tool's dir shape does this path match" helper is about *global* skills dirs, `tool_holding_path` is the public replacement; if it needs project-relative dirs it is a different question.
- `install_local_selection` (`commands/mod.rs`) still builds the base path with `PathBuf::from(basePath)` — no `~` expansion — pre-existing, unchanged.
- Operator smoke test: Add → Local folder → pick `~/.claude/skills/<any>` → expect the refusal toast naming Claude Code with the path on its own line; Import of the same skill still works.
