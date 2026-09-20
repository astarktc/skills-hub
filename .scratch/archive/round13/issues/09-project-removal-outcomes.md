# 09 Surface per-target project-removal outcomes

Status: done — 6280df2
Lane: E
Source: BACKLOG #14

`src-tauri/src/core/project_ops.rs:268,300` bail `SignalError::DeleteCleanupFailed { failures }` (strings) although a `RemovalReport` is built at :184. Make the remove-project / remove-project-tool paths return the per-target report as report data (per AGENTS.md: per-target failures are report data, not command errors). Wire: core result → command DTO (derive specta::Type; `cargo test` regenerates `src/bindings/index.ts` — commit nothing, but include the regenerated file in your changes) → shim re-export in `src/components/projects/types.ts` → `src/lib/reportOutcome.ts` fold (reuse the removal fold pattern) → `useProjectState` applies it → i18n keys in BOTH en and zh in `src/i18n/resources.ts` (paste raw CJK; Edit tool oldText must be verbatim). Remove `DeleteCleanupFailed` only if no other caller remains (then also its `describeCommandError` branch + i18n keys). `remove_project` still returns the projects list — extend its response to `{ projects, report }` rather than a second command. Read AGENTS.md **Artifact removal** and **Target fan-out** sections first. Gate: `npm run check`.

## Done when

Per-target outcomes reach the toast; vitest fold test added; bindings regenerated; EN+ZH keys; full gate green.

## Comments

- 2026-09-16 (lane E child): Per-target project-removal outcomes are report data end to end.
  - Core `project_ops.rs`: `remove_project_and_artifacts` → `Result<RemovalReport>` (project row deleted only when
    every artifact went; kept + `log::warn!` otherwise — no bail); `configure_project_tools[_unlocked]` →
    `Result<Vec<RemovalReport>>` (one report per dropped tool, continue semantics unchanged). `DeleteCleanupFailed`
    **kept**: `artifact_removal::remove_skill` (whole-skill delete) and `project_sync` still raise it, so
    `describeCommandError` and its i18n keys stay.
  - Commands `projects.rs`: `remove_project` → `RemoveProjectResultDto { projects, report }`;
    `configure_project_tools` → `ConfigureProjectToolsResultDto { view, report }` (dropped tools' reports merged into
    one `RemovalReportDto` at the seam via `to_removal_report_dto`). Bindings regenerated (`cargo test export_bindings`).
  - Frontend: shim re-exports (`RemoveProjectResultDto`, `ConfigureProjectToolsResultDto`, `RemovalReportDto`); new
    fold `projectRemovalOutcome(report, { action: "removeProject" | "configureTools" })` in `reportOutcome.ts` (per-
    target error entries titled `errors.projectRemovalFailedTitle`, warning toast `projects.removeKept` /
    `projects.toolRemovalKept`, success `projects.removeComplete`; empty report is a normal outcome, never
    "nothing planned"; `closeModal = !failed`, no reload); `useProjectState.removeProject` unpacks `{ projects, report }`,
    applies the list, takes the failure-path `refreshView` when the project is still listed, returns the report;
    `configureTools` unpacks `{ view, report }`, applies the view, returns the report; `ProjectsPage` folds and shows
    via new `showActionErrors` prop (App passes `reporter.showActionErrors`). EN+ZH keys added. AGENTS.md line on
    `remove_project`'s wire shape updated (was stale).
  - Evidence: `cargo test project_ops` 28 passed (incl. `remove_project_and_artifacts_keeps_the_project_when_an_artifact_stays`,
    `configure_tools_applies_the_rest_and_reports_the_removal_failures`,
    `remove_project_and_artifacts_removes_project_scope_target_for_divergent_tool`,
    `configure_tools_diffs_against_persisted_tools_and_rewrites_the_block`); `cargo test export_bindings` ok;
    `cargo clippy --all-targets -- -D warnings` clean; vitest 357 passed (new: reportOutcome "a project removal reports
    each kept target and keeps its modal open", "a clean or empty project removal closes without a nothing-planned
    warning"; useProjectState "returns the per-target report and keeps the project it lists when an artifact stayed",
    "converges on the backend's view when the removal command itself fails", "applies the view and hands back the
    dropped tools' report from configureTools"); `npm run build` ok; `npm run lint` clean. `npm run check` / `cargo
    test --all` left to the parent.


- 2026-09-16 (parent) — closed `done — 6280df2`; review fixes in fa4ae9f; released as 1.2.13 (c927683).
