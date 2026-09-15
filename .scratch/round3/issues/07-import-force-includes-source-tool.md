# 07: Onboarding import always overwrites the source Tool's original

Status: done — e118efb

**What to build:** With auto-sync on, importing a skill found in a Tool that is **deselected** in the global auto-sync selection no longer leaves the original behind as an untracked copy: the Tool the chosen variant was found in is force-included in the sync target set (union with the policy's Tools) and its original is overwritten in place, exactly as already happens when that Tool is selected. Nothing is deleted. The per-group report says the source Tool was included beyond the policy so the import summary can phrase why a deselected Tool received a link (add EN + ZH copy if the current summary cannot express it). The selection UI does not change.

Source: `../spec.md` Q1 and the verified-facts section; `../source-13-review-followups.md` #10; CONTEXT.md **Onboarding import**. Skill: **tdd**.

**Blocked by:** None (can start immediately)

- [x] Core test: auto-sync on, source Tool absent from the policy → the source Tool dir holds a link (not the original copy) and a target row exists for it; the report flags the forced inclusion
- [x] Core test: auto-sync on, source Tool present in the policy → behaviour and report unchanged
- [x] Any new report field crosses the wire via the generated binding, and the frontend renders it with EN + ZH copy
- [x] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Status reconciliation: ready-for-agent → done — e118efb. Evidence: Cited e118efb (integrated as 2fab542) force-includes the source Tool; widened, not removed, by round4/05. src-tauri/src/core/onboarding_import.rs:342–347 extends the selection with identical originals.

### Implementation (branch `r3/07-import-source-tool`)

**Shipped**

- `e118efb` fix(import): force-include the source Tool in the auto-sync target set — `core/onboarding_import.rs::sync_imported_unlocked` now syncs to `policy.tools ∪ {source Tool}`; the force-overwrite `BatchOverride` for the source Tool applies regardless of the policy. `ImportGroupStatus::Imported` gains `forced_source_tool: Option<String>` (the source Tool's key when it was not in the policy's list; `None` when the policy named it or auto-sync is off). DTO `ImportGroupStatusDto::Imported.forced_source_tool: string | null`, binding regenerated. Two core tests: `auto_sync_on_force_includes_a_source_tool_the_policy_deselected` (claude_code deselected, policy = cursor → both synced, claude_code target row with `mode = symlink`, the original path is a link to the central copy, report names `claude_code`) and the pre-existing shared-dir test now also asserts `forced_source_tool == None` when the policy names the source Tool.
- `296b33f` feat(import): the import success toast gains one line per forced group — `status.importSourceToolForced` (EN: "{{name}}: also synced to {{tool}}, where the original was found." / ZH: "{{name}}：已同步到发现原始副本的 {{tool}}。"). `handleImport` returns the report so `runAction`'s `successToast(value)` composes it (`importSuccessToast`). Hook test added. CONTEXT.md **Onboarding import** entry states the strengthened rule.

**Decisions / deviations**

- The forced inclusion is rendered on the **success** toast, not through `showActionErrors`: it is not a failure, so the modal still closes and no error toast fires (the `kept_divergent` precedent keeps the modal open, which would be wrong here). Sonner's title has no `white-space` rule, so the `\n`-joined lines collapse to one sentence run — same as the existing `showActionErrors` convention; it reads fine either way.
- The source Tool is appended **after** the policy's Tools so the batch's shared-dir dedupe ("first in caller order wins") is byte-for-byte unchanged for the in-policy case. In the shared-dir case (source Tool not in the policy but sharing its dir with a policy Tool) the forced flag is still reported — the Tool's key was not in the policy, and the line is true (a link is there) — the batch's record fan-out already writes both rows as before.
- `policy.tools == None` (every installed Tool) always contains the source Tool (the plan only scans installed Tools), so `forced_source_tool` is `None` there; the union is still applied defensively.

**For the orchestrator**

- Nothing left open. The binding diff is one field on `ImportGroupStatusDto`; any other round-3 branch that constructs an `ImportGroupStatusDto`/`ImportGroupStatus::Imported` literal (only `commands/mod.rs` and the two test files at HEAD) will need the field on merge.
- Gate: `npm run version:check && npm run check` green — vitest 154 (12 files), cargo 431, clippy `-D warnings`, rustfmt, typescript-7 build.
