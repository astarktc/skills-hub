import { describe, expect, it } from "vitest";
import type {
  BatchTargetOutcome,
  InstallBatchReport,
  ProjectSyncReport,
  PropagationOutcome,
  RedeployReport,
  RefreshReport,
} from "../bindings";
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
  redeployAnswers,
  redeployAskRows,
  mergeRedeployRetry,
  refreshCollisionRows,
} from "./overwriteAsk";

const collided = (skill_id: string, tool_key: string, sameContent = false): BatchTargetOutcome => ({
  skill_id,
  skill_name: skill_id,
  tool_key,
  folder_key: tool_key,
  status: { status: "failed", error: { code: "TARGET_EXISTS", path: `/${tool_key}/${skill_id}`, modifiedAt: 7, sameContent } },
});
const synced = (skill_id: string, tool_key: string): BatchTargetOutcome => ({
  skill_id,
  skill_name: skill_id,
  tool_key,
  folder_key: tool_key,
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
          folder_key: "claude",
          status: { status: "failed", error: { code: "TARGET_EXISTS", path: "/p/a", modifiedAt: null, sameContent: false } },
        },
      ],
    };
    const rows = [...globalCollisionRows([collided("a", "claude")], {}), ...projectCollisionRows(project, {})];
    expect(new Set(rows.map(overwriteRowKey)).size).toBe(2);
    expect(globalOverrides(rows, ["keep", "overwrite"])).toEqual([
      { skill_id: "a", tool: "claude", path: "/claude/a", decision: "keep" },
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
      folder_key: tool,
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
      folder_key: tool,
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

// A TARGET_EXISTS here is Propagation's first sync of a row that owns nothing.
describe("Propagation rows (ticket 09 B2)", () => {
  const exists = (path: string) => ({ code: "TARGET_EXISTS" as const, path, modifiedAt: 1, sameContent: false });
  const targets: PropagationOutcome[] = [
    { scope: { scope: "global", tool: "claude" }, folder_key: "claude", status: { status: "failed", error: exists("/g/claude/a"), first_deploy: true } },
    { scope: { scope: "global", tool: "cursor" }, folder_key: "cursor", status: { status: "failed", error: exists("/g/cursor/a"), first_deploy: true } },
    { scope: { scope: "project", project_id: "p1", tool: "claude" }, folder_key: "claude", status: { status: "failed", error: exists("/p1/claude/a"), first_deploy: true } },
    { scope: { scope: "global", tool: "codex" }, folder_key: "codex", status: { status: "failed", error: { code: "OTHER", message: "io" }, first_deploy: false } },
    { scope: { scope: "global", tool: "pi" }, folder_key: "pi", status: { status: "synced", mode_used: "symlink" } },
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
      skills: [{ skill_id: "a", skill_name: "alpha", status: { status: "refreshed", content_hash: null, source_revision: null, targets, edit_conflict: null } }],
    };
    expect(refreshCollisionRows(report, {})).toHaveLength(3);
  });

  it("an answered row leaves the report unless its retry failed; a kept one becomes declined", () => {
    const rows = propagationCollisionRows(skill, targets, {});
    const merged = mergePropagationRetry(skill, targets, {
      rows,
      global: [
        { skill_id: "a", skill_name: "alpha", tool_key: "claude", folder_key: "claude", status: { status: "declined" } },
        { skill_id: "a", skill_name: "alpha", tool_key: "cursor", folder_key: "cursor", status: { status: "failed", error: { code: "OTHER", message: "busy" } } },
      ],
      projects: { skipped_projects: [], items: [{ assignment_id: "x", project_id: "p1", skill_id: "a", skill_name: "alpha", tool: "claude", folder_key: "claude", status: { status: "synced" } }] },
    });
    expect(merged.declined).toEqual([{ skillName: "alpha", tool: "claude" }]);
    expect(merged.targets).toEqual([
      // The retry of a first deploy that failed is still a first deploy.
      { scope: { scope: "global", tool: "cursor" }, folder_key: "cursor", status: { status: "failed", error: { code: "OTHER", message: "busy" }, first_deploy: true } },
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
      folder_key: "claude",
      status: { status: "skipped", reason: { reason: "project_unavailable", project_id: "p1" } },
    });
  });

  it("rows the retry did not answer stay as they settled", () => {
    const rows = propagationCollisionRows(skill, targets, {});
    const report: RefreshReport = {
      skills: [{ skill_id: "a", skill_name: "alpha", status: { status: "refreshed", content_hash: null, source_revision: null, targets, edit_conflict: null } }],
    };
    const { report: merged, declined } = mergeRefreshRetry(report, { rows, global: [], projects: null });
    expect(declined).toEqual([]);
    expect(merged).toEqual(report);
  });
});

// ---------------------------------------------------------------------------
// Redeploy (round 20 #108)
// ---------------------------------------------------------------------------

describe("the redeploy's ask", () => {
  const EMPTY: RedeployReport = { moves: [], journal: [], left_behind: [], edited_copies: [], lost_disables: [] };
  const move = (
    skill: string,
    status: RedeployReport["moves"][number]["status"],
  ): RedeployReport["moves"][number] => ({
    skill_id: skill,
    skill_name: skill,
    folder_key: "agents_skills",
    path: `/h/.agents/skills/${skill}`,
    cause: "correction",
    rows: [{ id: `r-${skill}`, tool: "codex", from: `/h/.codex/skills/${skill}` }],
    status,
  });
  const first: RedeployReport = {
    ...EMPTY,
    moves: [
      move("a", {
        status: "failed",
        error: { code: "TARGET_EXISTS", path: "/h/.agents/skills/a", modifiedAt: 5, sameContent: false },
      }),
      move("b", { status: "left_old", edited: true }),
      move("c", { status: "moved" }),
    ],
    edited_copies: [
      { skill_id: "b", skill_name: "b", tool: "codex", path: "/h/.codex/skills/b", modified_at: 9, readers: ["codex"] },
    ],
  };

  it("asks the folders in a move's way and the edited copies in one ask", () => {
    const rows = redeployAskRows(first, { agents_skills: ".agents/skills", codex: "Codex" });
    expect(rows).toEqual([
      {
        skillId: "a",
        skillName: "a",
        toolKey: "agents_skills",
        toolLabel: ".agents/skills",
        path: "/h/.agents/skills/a",
        modifiedAt: 5,
        sameContent: false,
        scope: { kind: "redeploy" },
        question: "occupied",
      },
      {
        skillId: "b",
        skillName: "b",
        toolKey: "codex",
        toolLabel: "Codex",
        path: "/h/.codex/skills/b",
        modifiedAt: 9,
        sameContent: false,
        scope: { kind: "redeploy" },
        question: "editedCopy",
        readers: ["Codex"],
      },
    ]);
    expect(new Set(rows.map(overwriteRowKey)).size).toBe(2);
  });

  it("answers each row at the path it showed: Overwrite/Keep, Delete/Leave", () => {
    const rows = redeployAskRows(first, {});
    expect(redeployAnswers(rows, ["keep", "overwrite"])).toEqual([
      { question: "occupied", skill_id: "a", folder_key: "agents_skills", path: "/h/.agents/skills/a", decision: "keep" },
      { question: "edited_copy", skill_id: "b", tool: "codex", path: "/h/.codex/skills/b", decision: "delete" },
    ]);
    expect(redeployAnswers(rows, ["overwrite", "keep"])[1]).toMatchObject({ decision: "leave" });
  });

  it("merges the retry: asked moves replaced, a deleted edited copy reads moved", () => {
    const retry: RedeployReport = {
      ...EMPTY,
      moves: [move("a", { status: "moved" })],
      journal: [{ skill_id: "b", skill_name: "b", tool: "codex", path: "/h/.codex/skills/b", status: "finished" }],
    };
    const merged = mergeRedeployRetry(first, retry);
    expect(merged.moves.map((m) => [m.skill_id, m.status.status])).toEqual([
      ["a", "moved"],
      ["b", "moved"],
      ["c", "moved"],
    ]);
    expect(merged.edited_copies).toEqual([]);
    expect(merged.journal).toHaveLength(1);
  });

  it("a left edited copy keeps its move left_old and joins what stays", () => {
    const retry: RedeployReport = {
      ...EMPTY,
      journal: [{ skill_id: "b", skill_name: "b", tool: "codex", path: "/h/.codex/skills/b", status: "left_behind" }],
      left_behind: [
        { skill_id: "b", skill_name: "b", tool: "codex", path: "/h/.codex/skills/b", reason: "edited", readers: [], error: null },
      ],
    };
    const merged = mergeRedeployRetry(first, retry);
    expect(merged.moves[1].status).toEqual({ status: "left_old", edited: true });
    expect(merged.left_behind).toHaveLength(1);
    expect(merged.moves[0].status.status).toBe("failed");
  });
});
