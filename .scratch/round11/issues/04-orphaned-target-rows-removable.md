# D5 — An orphaned target row must be removable through the UI

Status: done — 525e970

## Problem

D4 (shipped, `86cfd63`) now renders a chip for every `skill_targets` row, including rows whose Tool is
no longer detected — so the operator can finally *see* an orphan. Clicking it does nothing useful, and
lies about it.

**Planning refuses the row.** `core/artifact_removal.rs`, `global_group_keys`:

```rust
fn global_group_keys(home: &Path, tool_key: &str) -> Option<Vec<String>> {
    let Some(adapter) = adapter_by_key(tool_key) else {
        return Some(vec![tool_key.to_string()]);      // unknown key stands for itself
    };
    let group = adapters_sharing_skills_dir(adapter);
    if !group.iter().any(|a| is_installed_in(home, a)) {
        return None;                                  // ⇒ zero targets planned ⇒ row KEPT
    }
    Some(group.into_iter().map(|a| a.key().to_string()).collect())
}
```

A *known* Tool that is merely undetected (Cursor after uninstalling Cursor) yields `None`: nothing is
planned, nothing is removed, the row survives. Only an *unknown* key is removable today.

**And the empty report reads as success.** `src/lib/reportOutcome.ts:166`:

```ts
const failed = report.failed > 0 || out.errors.length > 0;   // empty report ⇒ failed = false
```

Zero targets ⇒ `failed === false` ⇒ success toast + reload, while the row is still there.

Real-world cost: 24 orphaned `cursor` rows on the operator's machine had no supported removal path.
They were deleted with raw SQL against the live DB — the exact operation this app exists to make
unnecessary. The only in-app path that reached them was `unsync_all_skills`
(`RemovalScope::EveryGlobalTarget`), which would also have deleted all 96 live Claude Code / Codex / Pi
targets.

## Decision (do not re-decide)

**A row's own `target_path` is authoritative when the registry can no longer locate the Tool.**

The existing guard's doc says *"an uninstalled tool's directory is left alone"*, and that intent is
kept for **derived** paths: we must never compute a path from the registry for a Tool that is not
there and delete whatever we find. But a `skill_targets` row carries the path of an artifact **we
created and recorded**. Removing our own recorded artifact is not "touching an uninstalled tool's
directory" — it is cleaning up after ourselves.

So: when no member of the shared-dir group is installed, `RemovalScope::SkillTool` planning falls back
to planning **the row itself, by its stored `target_path`** (no group fan-out — there is no group to
fan out to). The existing machinery then applies unchanged:

- `ensure_path_within_tool_dirs` still fences the path (a row pointing outside every Tool skills dir is
  still refused — do not weaken this).
- The one presence rule (`symlink_metadata`) still decides: artifact present ⇒ remove it, then delete
  the row; artifact absent ⇒ a successful removal that deletes the row (ADR-0002 unchanged).
- A removal that genuinely fails still **keeps** the row with sync status `error` (ADR-0002 unchanged).

**Rewrite, don't delete, the test that pins today's behaviour.**
`unsync_skill_from_tool_with_an_uninstalled_group_touches_nothing`
(`core/tests/artifact_removal.rs:857`) currently asserts "nothing installed ⇒ nothing touched, row
still present". It must now assert the new intent: the row's artifact is removed by its stored path and
the row is deleted. Rename it to say what it now proves. Check for sibling tests encoding the old rule
and update them the same way; if you find one whose intent is genuinely different (e.g. a *derived*
path case), leave it and say so in your report.

**Second, independent fix — an empty report must not read as success.** In
`src/lib/reportOutcome.ts`, a removal report with **zero targets** is not a success: it means nothing
was planned. Give `removalOutcome` a third outcome for that case (a warning toast, not a success one,
and no misleading "unsynced" copy). Do not paper over it by changing `failed`'s meaning — `failed`
counts failures, and zero-planned is a distinct condition. New i18n keys go in **both** `en` and `zh`.
This is a guard against a class of bug, not just this one path: after your backend fix the zero-target
case should no longer occur for `SkillTool`, and the fold must still be honest if it ever does.

## Files

- `src-tauri/src/core/artifact_removal.rs` — `global_group_keys` and/or the `RemovalScope::SkillTool`
  planning arm. Keep the module's single-presence-rule / single-settlement-rule structure; do not add a
  second place that probes or deletes paths.
- `src-tauri/src/core/tests/artifact_removal.rs` — rewrite the test above, add the new cases.
- `src/lib/reportOutcome.ts` + `src/lib/reportOutcome.test.ts` — the zero-target outcome.
- `src/i18n/resources.ts` — new keys in **both** `en` and `zh`.
- `docs/adr/0002-keep-row-with-error-on-failed-artifact-removal.md` — append a short note that a row
  whose Tool is undetected is planned by its stored path (the ADR's rules are otherwise unchanged).

**Do not touch**: `core/refresh.rs`, `core/settings.rs`, `core/tool_adapters/`, `core/global_sync.rs`,
`src/hooks/`, `src/components/` — all just landed from three parallel lanes.

## Tests

Rust:
- Undetected known Tool, artifact **present** at the row's `target_path` ⇒ artifact removed, row deleted.
- Undetected known Tool, artifact **absent** ⇒ row deleted (successful removal per ADR-0002).
- Undetected Tool whose row path lies outside every Tool skills dir ⇒ still refused
  (`PathOutsideToolDirs`), row kept.
- Detected Tool ⇒ unchanged group fan-out behaviour (regression guard for the normal path).

Vitest:
- `removalOutcome` with a zero-target report ⇒ not a success toast; with a normal report ⇒ unchanged.

**Prove the tests bite**: after writing each new Rust test, revert your source change, confirm it fails,
restore, confirm it passes. Report that you did this.

## Gate

`npm run version:check && npm run check`, plus `cd src-tauri && cargo test --all`.

## Comments

- 2026-09-15 — Status reconciliation: NO STATUS → done — 525e970. Evidence: Verified 525e970 then 5e98143 (plus merge b564738), release 9de83e9; src-tauri/src/core/artifact_removal.rs:298–299 fences/plans the undetected Tool row by stored path and src/lib/reportOutcome.ts:190 detects zero planned targets.
