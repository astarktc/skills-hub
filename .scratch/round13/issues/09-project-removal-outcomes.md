# 09 Surface per-target project-removal outcomes

Status: open
Lane: E
Source: BACKLOG #14

`src-tauri/src/core/project_ops.rs:268,300` bail `SignalError::DeleteCleanupFailed { failures }` (strings) although a `RemovalReport` is built at :184. Make the remove-project / remove-project-tool paths return the per-target report as report data (per AGENTS.md: per-target failures are report data, not command errors). Wire: core result → command DTO (derive specta::Type; `cargo test` regenerates `src/bindings/index.ts` — commit nothing, but include the regenerated file in your changes) → shim re-export in `src/components/projects/types.ts` → `src/lib/reportOutcome.ts` fold (reuse the removal fold pattern) → `useProjectState` applies it → i18n keys in BOTH en and zh in `src/i18n/resources.ts` (paste raw CJK; Edit tool oldText must be verbatim). Remove `DeleteCleanupFailed` only if no other caller remains (then also its `describeCommandError` branch + i18n keys). `remove_project` still returns the projects list — extend its response to `{ projects, report }` rather than a second command. Read AGENTS.md **Artifact removal** and **Target fan-out** sections first. Gate: `npm run check`.

## Done when

Per-target outcomes reach the toast; vitest fold test added; bindings regenerated; EN+ZH keys; full gate green.

## Comments
