import { describe, expect, it } from "vitest";
import type { BatchTargetOutcome, InstallBatchReport, ProjectSyncReport, PropagationOutcome, RefreshReport } from "../bindings";
import {
  globalCollisionRows,
  globalOverrides,
  installTargets,
  mergeInstallRetry,
  mergeProjectRetry,
  overwriteRowKey,
  projectCollisionRows,
  mergePropagationRetry,
  mergeRefreshRetry,
  propagationCollisionRows,
  refreshCollisionRows,
} from "./overwriteAsk";

const collided = (skill_id: string, tool_key: string, sameContent = false): BatchTargetOutcome => ({
  skill_id,
  skill_name: skill_id,
  tool_key,
  status: { status: "failed", error: { code: "TARGET_EXISTS", path: `/${tool_key}/${skill_id}`, modifiedAt: 7, sameContent } },
});
const synced = (skill_id: string, tool_key: string): BatchTargetOutcome => ({
  skill_id,
  skill_name: skill_id,
  tool_key,
  status: { status: "synced", outcome: { mode_used: "symlink", target_path: `/${tool_key}/${skill_id}`, replaced: true } },
});

describe("overwriteAsk", () => {
  it("only TARGET_EXISTS rows are askable, carrying the backend's facts", () => {
    const rows = globalCollisionRows(
      [
        synced("a", "claude"),
        collided("a", "cursor", true),
        { ...synced("b", "claude"), status: { status: "failed", error: { code: "OTHER", message: "x" } } },
      ],
      { cursor: "Cursor" },
    );
    expect(rows).toEqual([
      {
        skillId: "a",
        skillName: "a",
        toolKey: "cursor",
        toolLabel: "Cursor",
        path: "/cursor/a",
        modifiedAt: 7,
        sameContent: true,
        scope: { kind: "global" },
      },
    ]);
  });

  it("global and project rows of one pair have distinct keys; overrides name only global rows", () => {
    const project: ProjectSyncReport = { skipped_projects: [],
      items: [
        {
          assignment_id: "x",
          project_id: "p",
          skill_id: "a",
          skill_name: "a",
          tool: "claude",
          status: { status: "failed", error: { code: "TARGET_EXISTS", path: "/p/a", modifiedAt: null, sameContent: false } },
        },
      ],
    };
    const rows = [...globalCollisionRows([collided("a", "claude")], {}), ...projectCollisionRows(project, {})];
    expect(new Set(rows.map(overwriteRowKey)).size).toBe(2);
    expect(globalOverrides(rows, ["keep", "overwrite"])).toEqual([
      { skill_id: "a", tool: "claude", decision: "keep" },
    ]);
  });

  it("the install report keeps its shape; asked rows are replaced by the retry's", () => {
    const report: InstallBatchReport = {
      candidates: [
        {
          subpath: "s/a",
          name: "a",
          status: {
            status: "installed",
            skill: { skill_id: "a", name: "a", central_path: "/hub/a", content_hash: null },
            targets: [synced("a", "claude"), collided("a", "cursor")],
          },
        },
        { subpath: "s/b", name: "b", status: { status: "cancelled" } },
      ],
    };
    const asked = globalCollisionRows(installTargets(report), {});
    const merged = mergeInstallRetry(report, asked, [synced("a", "claude"), synced("a", "cursor")]);
    expect(installTargets(merged).map((row) => [row.tool_key, row.status.status])).toEqual([
      ["claude", "synced"],
      ["cursor", "synced"],
    ]);
    expect(merged.candidates[1]).toEqual(report.candidates[1]);
  });

  it("a project retry replaces its assignments' rows only", () => {
    const item = (tool: string, status: ProjectSyncReport["items"][number]["status"]) => ({
      assignment_id: tool,
      project_id: "p",
      skill_id: "a",
      skill_name: "a",
      tool,
      status,
    });
    const merged = mergeProjectRetry(
      { skipped_projects: [], items: [item("claude", { status: "synced" }), item("pi", { status: "failed", error: { code: "OTHER", message: "x" } })] },
      { skipped_projects: [], items: [item("pi", { status: "declined" })] },
    );
    expect(merged.items.map((i) => i.status.status)).toEqual(["synced", "declined"]);
  });

  it("a project the retry skipped whole (folder gone before the answer) replaces its asked rows with one skip", () => {
    const item = (project: string, tool: string, status: ProjectSyncReport["items"][number]["status"]) => ({
      assignment_id: `${project}-${tool}`,
      project_id: project,
      skill_id: "a",
      skill_name: "a",
      tool,
      status,
    });
    const collided = { status: "failed" as const, error: { code: "TARGET_EXISTS" as const, path: "/x", modifiedAt: null, sameContent: false } };
    const skip = { project_id: "gone", reason: { reason: "folder_missing" as const } };
    const report: ProjectSyncReport = {
      skipped_projects: [],
      items: [item("gone", "claude", { status: "synced" }), item("gone", "pi", collided), item("gone", "codex", collided), item("live", "pi", collided)],
    };
    const merged = mergeProjectRetry(report, {
      skipped_projects: [skip],
      items: [item("live", "pi", { status: "synced" })],
    });
    // Settled rows stand; the unanswered collisions leave for the one skip line.
    expect(merged.items.map((i) => `${i.project_id}:${i.tool}:${i.status.status}`)).toEqual([
      "gone:claude:synced",
      "live:pi:synced",
    ]);
    expect(merged.skipped_projects).toEqual([skip]);
    // A part that asked nothing in that project does not inherit its skip.
    expect(mergeProjectRetry({ skipped_projects: [], items: [item("live", "pi", collided)] }, { skipped_projects: [skip], items: [] }))
      .toEqual({ skipped_projects: [], items: [item("live", "pi", collided)] });
  });
});

describe("Propagation / re-assert rows (ticket 09 B2)", () => {
  const exists = (path: string) => ({ code: "TARGET_EXISTS" as const, path, modifiedAt: 1, sameContent: false });
  const targets: PropagationOutcome[] = [
    { scope: { scope: "global", tool: "claude" }, status: { status: "failed", error: exists("/g/claude/a") } },
    { scope: { scope: "global", tool: "cursor" }, status: { status: "failed", error: exists("/g/cursor/a") } },
    { scope: { scope: "project", project_id: "p1", tool: "claude" }, status: { status: "failed", error: exists("/p1/claude/a") } },
    { scope: { scope: "global", tool: "codex" }, status: { status: "failed", error: { code: "OTHER", message: "io" } } },
    { scope: { scope: "global", tool: "pi" }, status: { status: "synced", mode_used: "symlink" } },
  ];
  const skill = { skillId: "a", skillName: "alpha" };

  it("asks only the TARGET_EXISTS rows, global and project alike", () => {
    const rows = propagationCollisionRows(skill, targets, { claude: "CLAUDE" });
    expect(rows.map(overwriteRowKey)).toEqual([
      "global\na\nclaude",
      "global\na\ncursor",
      "project\np1\na\nclaude",
    ]);
    expect(rows[0]).toMatchObject({ toolLabel: "CLAUDE", path: "/g/claude/a", scope: { kind: "global" } });
    const report: RefreshReport = {
      skills: [{ skill_id: "a", skill_name: "alpha", status: { status: "refreshed", content_hash: null, source_revision: null, targets, reassert_error: null, edit_conflict: null } }],
    };
    expect(refreshCollisionRows(report, {})).toHaveLength(3);
  });

  it("an answered row leaves the report unless its retry failed; a kept one becomes declined", () => {
    const rows = propagationCollisionRows(skill, targets, {});
    const merged = mergePropagationRetry(skill, targets, {
      rows,
      global: [
        { skill_id: "a", skill_name: "alpha", tool_key: "claude", status: { status: "declined" } },
        { skill_id: "a", skill_name: "alpha", tool_key: "cursor", status: { status: "failed", error: { code: "OTHER", message: "busy" } } },
      ],
      projects: { skipped_projects: [], items: [{ assignment_id: "x", project_id: "p1", skill_id: "a", skill_name: "alpha", tool: "claude", status: { status: "synced" } }] },
    });
    expect(merged.declined).toEqual([{ skillName: "alpha", tool: "claude" }]);
    expect(merged.targets).toEqual([
      { scope: { scope: "global", tool: "cursor" }, status: { status: "failed", error: { code: "OTHER", message: "busy" } } },
      targets[3],
      targets[4],
    ]);
  });

  it("a project row whose project the retry skipped (folder gone) is project_unavailable, never a collision again", () => {
    const rows = propagationCollisionRows(skill, targets, {});
    const merged = mergePropagationRetry(skill, targets, {
      rows,
      global: [],
      projects: { skipped_projects: [{ project_id: "p1", reason: { reason: "folder_missing" } }], items: [] },
    });
    expect(merged.targets[2]).toEqual({
      scope: { scope: "project", project_id: "p1", tool: "claude" },
      status: { status: "skipped", reason: { reason: "project_unavailable", project_id: "p1" } },
    });
  });

  it("rows the retry did not answer stay as they settled", () => {
    const rows = propagationCollisionRows(skill, targets, {});
    const report: RefreshReport = {
      skills: [{ skill_id: "a", skill_name: "alpha", status: { status: "refreshed", content_hash: null, source_revision: null, targets, reassert_error: null, edit_conflict: null } }],
    };
    const { report: merged, declined } = mergeRefreshRetry(report, { rows, global: [], projects: null });
    expect(declined).toEqual([]);
    expect(merged).toEqual(report);
  });
});
