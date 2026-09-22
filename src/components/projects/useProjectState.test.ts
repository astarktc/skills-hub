// Tests at the ProjectState seam. Every project mutation returns the fresh
// view of the project it changed (`ProjectViewDto`), so the hook applies one
// result instead of orchestrating follow-up reads — these tests assert the
// state that got applied, not how many calls it took. The backend is mocked
// at the invokeTauri module seam.

const emptyRemoval = (): RemovalReport => ({
  targets: [],
  central_removed: false,
  record_deleted: false,
});

import { StrictMode } from "react";
import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  CommandError,
  GitignoreStatusDto,
  IgnoreUpdateOptions,
  ProjectDto,
  ProjectSkillAssignmentDto,
  ProjectToolDto,
  ProjectSyncOutcome,
  ProjectSyncReport,
  ProjectViewDto,
  RemovalReport,
} from "./types";

vi.mock("../../lib/tauri", () => ({
  isTauri: true,
  invokeTauri: vi.fn(),
}));

import {
  invokeTauri,
  type CommandName,
  type Commands,
} from "../../lib/tauri";
import { useProjectState, type ToggleResult } from "./useProjectState";

// The seam is generic over the command table; the stub switches on the
// command name, so it is typed loosely (positional args, unknown result).
const mockInvoke = vi.mocked(
  invokeTauri as unknown as (
    command: CommandName,
    ...args: unknown[]
  ) => Promise<unknown>,
);

function toolRecords(projectId: string, tools: string[]): ProjectToolDto[] {
  return tools.map((tool) => ({
    id: `${projectId}:${tool}`,
    project_id: projectId,
    tool,
  }));
}

function assignmentRecord(
  projectId: string,
  skillId: string,
  tool: string,
): ProjectSkillAssignmentDto {
  return {
    id: `${projectId}:${skillId}:${tool}`,
    project_id: projectId,
    skill_id: skillId,
    skill_name: skillId,
    tool,
    mode: "symlink",
    status: "synced",
    last_error: null,
    synced_at: 1,
    content_hash: null,
    created_at: 1,
  };
}

function syncItem(
  row: ProjectSkillAssignmentDto,
  status: ProjectSyncOutcome["status"] = { status: "synced" },
): ProjectSyncOutcome {
  return {
    assignment_id: row.id,
    project_id: row.project_id,
    skill_id: row.skill_id,
    skill_name: row.skill_name,
    tool: row.tool,
    status,
  };
}

/**
 * A tiny in-memory backend with the same contract as the real one: each
 * mutation applies its write and answers with the resulting view.
 */
function stubBackend(options: { reconciled?: boolean } = {}) {
  const reconciled = options.reconciled ?? true;
  const order: string[] = [];
  const projectIds: string[] = [];
  const toolsByProject = new Map<string, string[]>();
  const assignmentsByProject = new Map<string, ProjectSkillAssignmentDto[]>();
  let nextId = 1;

  const projectRow = (id: string): ProjectDto => {
    const tools = toolsByProject.get(id) ?? [];
    const assignments = assignmentsByProject.get(id) ?? [];
    return {
      id,
      path: `/work/${id}`,
      name: id,
      created_at: 1,
      updated_at: 1,
      tool_count: tools.length,
      skill_count: new Set(assignments.map((a) => a.skill_id)).size,
      assignment_count: assignments.length,
      sync_status: assignments.length === 0 ? "none" : "synced",
      path_exists: true,
    };
  };
  const view = (id: string): ProjectViewDto => ({
    project: projectRow(id),
    tools: toolRecords(id, toolsByProject.get(id) ?? []),
    assignments: assignmentsByProject.get(id) ?? [],
    reconciled,
  });

  mockInvoke.mockImplementation((command, ...args) => {
    order.push(command);
    switch (command) {
      case "listProjects":
        return Promise.resolve(projectIds.map(projectRow));
      case "getManagedSkills":
        return Promise.resolve([]);
      case "registerProject": {
        const id = `p${nextId++}`;
        projectIds.push(id);
        return Promise.resolve(view(id));
      }
      case "removeProject": {
        const [projectId] = args as Parameters<Commands["removeProject"]>;
        projectIds.splice(projectIds.indexOf(projectId), 1);
        return Promise.resolve({
          projects: projectIds.map(projectRow),
          report: emptyRemoval(),
        });
      }
      case "getProjectView": {
        const [projectId] = args as Parameters<Commands["getProjectView"]>;
        return Promise.resolve(view(projectId));
      }
      case "configureProjectTools": {
        const [projectId, tools] = args as Parameters<
          Commands["configureProjectTools"]
        >;
        toolsByProject.set(projectId, tools);
        // Dropping a tool cascades to its assignments — the backend's view
        // already reflects that.
        assignmentsByProject.set(
          projectId,
          (assignmentsByProject.get(projectId) ?? []).filter((a) =>
            tools.includes(a.tool),
          ),
        );
        return Promise.resolve({ view: view(projectId), report: emptyRemoval() });
      }
      case "toggleProjectSkillAssignment": {
        const [projectId, skillId, tool] = args as Parameters<
          Commands["toggleProjectSkillAssignment"]
        >;
        const rows = assignmentsByProject.get(projectId) ?? [];
        const existing = rows.findIndex(
          (a) => a.skill_id === skillId && a.tool === tool,
        );
        if (existing >= 0) {
          rows.splice(existing, 1);
          assignmentsByProject.set(projectId, rows);
          return Promise.resolve({
            kind: "unassigned",
            view: view(projectId),
            report: emptyRemoval(),
          });
        }
        const created = assignmentRecord(projectId, skillId, tool);
        rows.push(created);
        assignmentsByProject.set(projectId, rows);
        return Promise.resolve({
          kind: "assigned",
          view: view(projectId),
          report: { items: [syncItem(created)] },
        });
      }
      case "bulkAssignSkill": {
        const [projectId, skillId] = args as Parameters<
          Commands["bulkAssignSkill"]
        >;
        const rows = assignmentsByProject.get(projectId) ?? [];
        const items: ProjectSyncOutcome[] = [];
        for (const tool of toolsByProject.get(projectId) ?? []) {
          const held = rows.find((a) => a.skill_id === skillId && a.tool === tool);
          if (held) {
            items.push(syncItem(held, { status: "already_assigned" }));
          } else {
            const created = assignmentRecord(projectId, skillId, tool);
            rows.push(created);
            items.push(syncItem(created));
          }
        }
        assignmentsByProject.set(projectId, rows);
        return Promise.resolve({ view: view(projectId), report: { items } });
      }
      case "bulkUnassignSkill": {
        const [projectId, skillId] = args as Parameters<
          Commands["bulkUnassignSkill"]
        >;
        const rows = assignmentsByProject.get(projectId) ?? [];
        const removed = rows.filter((a) => a.skill_id === skillId);
        assignmentsByProject.set(
          projectId,
          rows.filter((a) => a.skill_id !== skillId),
        );
        return Promise.resolve({
          view: view(projectId),
          report: {
            targets: removed.map((a) => ({
              path: `/work/${projectId}/${a.tool}/${skillId}`,
              rows: [
                {
                  scope: "assignment",
                  id: a.id,
                  project_id: projectId,
                  skill_id: skillId,
                  tool: a.tool,
                },
              ],
              status: { status: "removed" },
            })),
            central_removed: false,
            record_deleted: false,
          } satisfies RemovalReport,
        });
      }
      case "resyncProject": {
        const [projectId] = args as Parameters<Commands["resyncProject"]>;
        return Promise.resolve({
          view: view(projectId),
          report: {
            items: (assignmentsByProject.get(projectId) ?? []).map((a) =>
              syncItem(a),
            ),
          },
        });
      }
      case "resyncAllProjects":
        return Promise.resolve({
          report: { items: [] },
          projects: projectIds.map(projectRow),
        });
      case "updateProjectPath": {
        const [projectId] = args as Parameters<Commands["updateProjectPath"]>;
        return Promise.resolve(view(projectId));
      }
      case "getProjectGitignoreStatus":
        return Promise.resolve({
          in_gitignore: true,
          in_exclude: false,
        } satisfies GitignoreStatusDto);
      case "updateProjectGitignore":
        return Promise.resolve(undefined);
      default:
        return Promise.resolve(undefined);
    }
  });
  return { order };
}

/** Positional args of every call to `command`, typed from the bindings. */
const callsTo = <K extends CommandName>(command: K) =>
  mockInvoke.mock.calls
    .filter(([name]) => name === command)
    .map(([, ...args]) => args as Parameters<Commands[K]>);

const commandOrder = () => mockInvoke.mock.calls.map(([name]) => name);

beforeEach(() => {
  vi.clearAllMocks();
  stubBackend();
});

async function renderReady() {
  const rendered = renderHook(() => useProjectState());
  await waitFor(() => {
    expect(rendered.result.current.projectsLoading).toBe(false);
  });
  return rendered;
}

/** Register p1, select it, and give it a tool set. */
async function withSelectedProject(
  result: { current: ReturnType<typeof useProjectState> },
  tools: string[] = ["claude_code"],
) {
  await act(async () => {
    await result.current.registerProject("/work/p1", {
      add_to_gitignore: false,
      add_to_exclude: false,
    });
  });
  await act(async () => {
    await result.current.selectProject("p1");
  });
  await act(async () => {
    await result.current.configureTools(tools);
  });
}

describe("useProjectState add-project → configure-tools → gitignore", () => {
  it("hands the ignore intent captured at registration to configure_project_tools", async () => {
    const { result } = await renderReady();
    const intent: IgnoreUpdateOptions = {
      add_to_gitignore: true,
      add_to_exclude: false,
    };

    let created: ProjectDto | undefined;
    await act(async () => {
      created = await result.current.registerProject("/work/p1", intent);
    });
    expect(created?.id).toBe("p1");
    // Registration alone never touches ignore files: the patterns come
    // from persisted tools, which do not exist yet.
    expect(callsTo("updateProjectGitignore")).toEqual([]);
    expect(callsTo("configureProjectTools")).toEqual([]);

    await act(async () => {
      await result.current.selectProject("p1");
    });
    await act(async () => {
      await result.current.configureTools(["claude_code", "windsurf"]);
    });

    expect(callsTo("configureProjectTools")).toEqual([
      ["p1", ["claude_code", "windsurf"], intent],
    ]);
    // One command owns the whole sequence — no separate gitignore replay.
    expect(callsTo("updateProjectGitignore")).toEqual([]);
    expect(commandOrder().indexOf("registerProject")).toBeLessThan(
      commandOrder().indexOf("configureProjectTools"),
    );
    expect(result.current.tools.map((t) => t.tool)).toEqual([
      "claude_code",
      "windsurf",
    ]);
  });

  it("consumes the intent once: a later tool config carries no gitignore", async () => {
    const { result } = await renderReady();
    await act(async () => {
      await result.current.registerProject("/work/p1", {
        add_to_gitignore: true,
        add_to_exclude: true,
      });
    });
    await act(async () => {
      await result.current.selectProject("p1");
    });
    await act(async () => {
      await result.current.configureTools(["claude_code"]);
    });
    await act(async () => {
      await result.current.configureTools(["claude_code", "pi"]);
    });

    expect(
      callsTo("configureProjectTools").map(([, , gitignore]) => gitignore),
    ).toEqual([{ add_to_gitignore: true, add_to_exclude: true }, null]);
  });

  it("registers without an intent when neither toggle is set", async () => {
    const { result } = await renderReady();
    await withSelectedProject(result);

    expect(callsTo("configureProjectTools")[0]?.[2]).toBeNull();
  });

  it("drops the intent when tools are configured for a different project", async () => {
    const { result } = await renderReady();
    await act(async () => {
      await result.current.registerProject("/work/p1", {
        add_to_gitignore: true,
        add_to_exclude: false,
      });
    });
    await act(async () => {
      await result.current.registerProject("/work/p2", {
        add_to_gitignore: false,
        add_to_exclude: false,
      });
    });
    await act(async () => {
      await result.current.selectProject("p1");
    });
    await act(async () => {
      await result.current.configureTools(["claude_code"]);
    });

    // p2's registration superseded p1's intent; nothing is written for p1.
    expect(callsTo("configureProjectTools")).toEqual([
      ["p1", ["claude_code"], null],
    ]);
  });

  it("converges on the backend's view when the batch command fails", async () => {
    const { result } = await renderReady();
    await act(async () => {
      await result.current.registerProject("/work/p1", {
        add_to_gitignore: true,
        add_to_exclude: false,
      });
    });
    await act(async () => {
      await result.current.selectProject("p1");
    });
    // Tools persisted, ignore write failed: the command errors but the
    // backend now reports the new tool set. The rejection carries a real
    // wire code (typed as `CommandError`, so a fabricated code fails the
    // build) — this is what the backend raises when the project dir went
    // missing between registration and the ignore write.
    const ignoreWriteFailure: CommandError = {
      code: "INVALID_PATH",
      path: "/work/p1",
      reason: "missing",
    };
    const base = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command, ...args) => {
      if (command === "configureProjectTools") {
        return base("configureProjectTools", ...args).then(() =>
          Promise.reject(ignoreWriteFailure),
        );
      }
      return base(command, ...args);
    });

    await act(async () => {
      await expect(
        result.current.configureTools(["claude_code"]),
      ).rejects.toMatchObject({ code: "INVALID_PATH", reason: "missing" });
    });

    expect(result.current.tools.map((t) => t.tool)).toEqual(["claude_code"]);
  });

  it("retains the intent when the command fails so a retry replays it", async () => {
    const { result } = await renderReady();
    const intent: IgnoreUpdateOptions = {
      add_to_gitignore: true,
      add_to_exclude: true,
    };
    await act(async () => {
      await result.current.registerProject("/work/p1", intent);
    });
    await act(async () => {
      await result.current.selectProject("p1");
    });

    const base = mockInvoke.getMockImplementation()!;
    let failNext = true;
    mockInvoke.mockImplementation((command, ...args) => {
      if (command === "configureProjectTools" && failNext) {
        failNext = false;
        return Promise.reject({ code: "INTERNAL", message: "disk full" });
      }
      return base(command, ...args);
    });

    await act(async () => {
      await expect(
        result.current.configureTools(["claude_code"]),
      ).rejects.toMatchObject({ code: "INTERNAL" });
    });
    // The modal stays open on failure; Confirm again must carry the intent.
    await act(async () => {
      await result.current.configureTools(["claude_code"]);
    });

    expect(
      callsTo("configureProjectTools").map(([, , gitignore]) => gitignore),
    ).toEqual([intent, intent]);
  });

  it("discards the intent when the tool-config modal is dismissed", async () => {
    const { result } = await renderReady();
    await act(async () => {
      await result.current.registerProject("/work/p1", {
        add_to_gitignore: true,
        add_to_exclude: false,
      });
    });
    await act(async () => {
      await result.current.selectProject("p1");
    });
    act(() => {
      result.current.discardPendingIgnore();
    });
    await act(async () => {
      await result.current.configureTools(["claude_code"]);
    });

    expect(callsTo("configureProjectTools")[0]?.[2]).toBeNull();
  });
});

describe("useProjectState applies the view a mutation returns", () => {
  it("applies tools, assignments and the project row from configureTools alone", async () => {
    const { result } = await renderReady();
    await withSelectedProject(result, ["pi"]);
    mockInvoke.mockClear();

    await act(async () => {
      await result.current.configureTools(["pi", "cursor"]);
    });

    expect(commandOrder()).toEqual(["configureProjectTools"]);
    expect(result.current.tools.map((t) => t.tool)).toEqual(["pi", "cursor"]);
    expect(
      result.current.projects.find((p) => p.id === "p1")?.tool_count,
    ).toBe(2);
  });

  it("shows the cascade when a tool is dropped, without re-reading", async () => {
    const { result } = await renderReady();
    await withSelectedProject(result, ["pi", "cursor"]);
    await act(async () => {
      await result.current.toggleAssignment("s1", "pi");
    });
    await act(async () => {
      await result.current.toggleAssignment("s1", "cursor");
    });
    expect(result.current.assignments).toHaveLength(2);
    mockInvoke.mockClear();

    await act(async () => {
      await result.current.configureTools(["cursor"]);
    });

    expect(commandOrder()).toEqual(["configureProjectTools"]);
    expect(result.current.assignments.map((a) => a.tool)).toEqual(["cursor"]);
    const row = result.current.projects.find((p) => p.id === "p1");
    expect(row?.tool_count).toBe(1);
    expect(row?.assignment_count).toBe(1);
  });

  it("applies the view and hands back the dropped tools' report from configureTools", async () => {
    const { result } = await renderReady();
    await withSelectedProject(result, ["pi", "cursor"]);
    await act(async () => {
      await result.current.toggleAssignment("s1", "pi");
    });
    // pi's artifact stayed on disk: the backend keeps the tool row and its
    // `error` assignment (ADR-0002), answers with that view, and the report
    // names the path. The hook applies the view and returns the report
    // unread — the fold interprets it.
    const kept: RemovalReport = {
      targets: [
        {
          rows: [{ scope: "assignment", id: "a1", project_id: "p1", skill_id: "s1", tool: "pi" }],
          path: "/work/p1/.pi/skills/s1",
          status: { status: "failed", error: { code: "OTHER", message: "busy" } },
        },
      ],
      central_removed: false,
      record_deleted: false,
    };
    const base = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command, ...args) => {
      if (command === "configureProjectTools") {
        const [projectId] = args as Parameters<Commands["configureProjectTools"]>;
        return base("getProjectView", projectId).then((view) => {
          const settled = view as ProjectViewDto;
          return {
            view: {
              ...settled,
              assignments: settled.assignments.map((a) => ({
                ...a,
                status: "error",
                last_error: "busy",
              })),
            } satisfies ProjectViewDto,
            report: kept,
          };
        });
      }
      return base(command, ...args);
    });
    mockInvoke.mockClear();

    let report: RemovalReport | undefined;
    await act(async () => {
      report = await result.current.configureTools(["cursor"]);
    });

    expect(report).toEqual(kept);
    expect(commandOrder()).toEqual(["configureProjectTools"]);
    expect(result.current.tools.map((t) => t.tool)).toEqual(["pi", "cursor"]);
    expect(result.current.assignments.map((a) => a.status)).toEqual(["error"]);
  });

  it("toggles an assignment on and off from the backend's own decision", async () => {
    const { result } = await renderReady();
    await withSelectedProject(result, ["claude_code"]);
    mockInvoke.mockClear();

    await act(async () => {
      expect(await result.current.toggleAssignment("s1", "claude_code")).toEqual({
        kind: "assigned",
        report: {
          items: [
            syncItem(assignmentRecord("p1", "s1", "claude_code")),
          ],
        },
      } satisfies ToggleResult);
    });
    expect(result.current.assignments.map((a) => a.skill_id)).toEqual(["s1"]);
    expect(
      result.current.projects.find((p) => p.id === "p1")?.assignment_count,
    ).toBe(1);

    await act(async () => {
      expect(await result.current.toggleAssignment("s1", "claude_code")).toEqual({
        kind: "unassigned",
        report: emptyRemoval(),
      } satisfies ToggleResult);
    });
    expect(result.current.assignments).toEqual([]);

    // No refetch tail: each toggle is one command, and the frontend never
    // told the backend which way to go.
    expect(commandOrder()).toEqual([
      "toggleProjectSkillAssignment",
      "toggleProjectSkillAssignment",
    ]);
    expect(
      callsTo("toggleProjectSkillAssignment"),
    ).toEqual([
      ["p1", "s1", "claude_code"],
      ["p1", "s1", "claude_code"],
    ]);
  });

  it("returns a kept-target toggle report and applies its view without a refetch", async () => {
    const { result } = await renderReady();
    await withSelectedProject(result, ["pi"]);
    await act(async () => {
      await result.current.toggleAssignment("s1", "pi");
    });
    const kept: RemovalReport = {
      targets: [{
        path: "/project/.pi/skills/s1",
        rows: [{ scope: "assignment", id: "a1", project_id: "p1", skill_id: "s1", tool: "pi" }],
        status: { status: "failed", error: { code: "OTHER", message: "busy" } },
      }],
      central_removed: false,
      record_deleted: false,
    };
    const base = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command, ...args) => {
      if (command === "toggleProjectSkillAssignment") {
        return base("getProjectView", "p1").then((view) => {
          const settled = view as ProjectViewDto;
          return {
            view: {
              ...settled,
              assignments: settled.assignments.map((a) => ({ ...a, status: "error", last_error: "busy" })),
            } satisfies ProjectViewDto,
            kind: "unassigned",
            report: kept,
          };
        });
      }
      return base(command, ...args);
    });
    mockInvoke.mockClear();
    await act(async () => {
      expect(await result.current.toggleAssignment("s1", "pi")).toEqual({
        kind: "unassigned",
        report: kept,
      } satisfies ToggleResult);
    });
    expect(commandOrder()).toEqual(["toggleProjectSkillAssignment"]);
    expect(result.current.assignments.map((a) => a.status)).toEqual(["error"]);
    expect(result.current.projects.find((p) => p.id === "p1")?.assignment_count).toBe(1);
  });

  it("applies the bulk-assign view without a follow-up read", async () => {
    const { result } = await renderReady();
    await withSelectedProject(result, ["pi", "cursor"]);
    mockInvoke.mockClear();

    let report: ProjectSyncReport | undefined;
    await act(async () => {
      report = await result.current.bulkAssign("s1");
    });

    expect(commandOrder()).toEqual(["bulkAssignSkill"]);
    expect(result.current.assignments.map((a) => a.tool)).toEqual([
      "pi",
      "cursor",
    ]);
    // The report is handed back unread — counters are the fold's business.
    expect(report?.items.map((i) => [i.tool, i.status.status])).toEqual([
      ["pi", "synced"],
      ["cursor", "synced"],
    ]);
    expect(result.current.pendingCells.size).toBe(0);
  });

  it("returns a toggle-on sync failure as report data with the kept error row applied", async () => {
    // A sync failure inside a freshly created row is not a thrown command:
    // the backend keeps the row with status `error` and reports it failed.
    const { result } = await renderReady();
    await withSelectedProject(result, ["pi"]);
    const failure: CommandError = { code: "OTHER", message: "denied" };
    const base = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command, ...args) => {
      if (command === "toggleProjectSkillAssignment") {
        return base(command, ...args).then((raw) => {
          const answered = raw as Extract<
            Awaited<ReturnType<Commands["toggleProjectSkillAssignment"]>>,
            { kind: "assigned" }
          >;
          return {
            kind: "assigned",
            view: {
              ...answered.view,
              assignments: answered.view.assignments.map((a) => ({
                ...a,
                status: "error",
                last_error: "denied",
              })),
            } satisfies ProjectViewDto,
            report: {
              items: answered.report.items.map((i) => ({
                ...i,
                status: { status: "failed", error: failure },
              })),
            },
          };
        });
      }
      return base(command, ...args);
    });
    mockInvoke.mockClear();

    let toggled: ToggleResult | null = null;
    await act(async () => {
      toggled = await result.current.toggleAssignment("s1", "pi");
    });

    expect(toggled).toMatchObject({
      kind: "assigned",
      report: { items: [{ tool: "pi", status: { status: "failed", error: failure } }] },
    });
    expect(commandOrder()).toEqual(["toggleProjectSkillAssignment"]);
    expect(result.current.assignments.map((a) => a.status)).toEqual(["error"]);
  });

  it("bulk-unassigns one skill from every tool, applying the view and returning the removal report", async () => {
    const { result } = await renderReady();
    await withSelectedProject(result, ["pi", "cursor", "claude_code"]);
    await act(async () => {
      await result.current.bulkAssign("s1");
    });
    await act(async () => {
      await result.current.toggleAssignment("s2", "pi");
    });
    // s1 comes off cursor by hand first: only its remaining cells are in flight.
    await act(async () => {
      await result.current.toggleAssignment("s1", "cursor");
    });
    const base = mockInvoke.getMockImplementation()!;
    let release!: () => void;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    mockInvoke.mockImplementation((command, ...args) =>
      command === "bulkUnassignSkill"
        ? gate.then(() => base(command, ...args))
        : base(command, ...args),
    );
    mockInvoke.mockClear();

    let pending!: Promise<RemovalReport | undefined>;
    act(() => {
      pending = result.current.bulkUnassign("s1");
    });
    expect([...result.current.pendingCells].sort()).toEqual([
      "s1:claude_code",
      "s1:pi",
    ]);
    let report: RemovalReport | undefined;
    await act(async () => {
      release();
      report = await pending;
    });

    expect(commandOrder()).toEqual(["bulkUnassignSkill"]);
    expect(callsTo("bulkUnassignSkill")).toEqual([["p1", "s1"]]);
    expect(report?.targets.flatMap((t) => t.rows.map((r) => r.tool)).sort()).toEqual([
      "claude_code",
      "pi",
    ]);
    // Only s1 left the project; s2 is untouched.
    expect(result.current.assignments.map((a) => `${a.skill_id}:${a.tool}`)).toEqual([
      "s2:pi",
    ]);
    expect(result.current.pendingCells.size).toBe(0);
  });

  it("converges on the backend's view when bulk unassign throws", async () => {
    const { result } = await renderReady();
    await withSelectedProject(result, ["pi", "cursor"]);
    await act(async () => {
      await result.current.bulkAssign("s1");
    });
    const base = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command, ...args) =>
      command === "bulkUnassignSkill"
        ? Promise.reject({ code: "OTHER", message: "database is locked" } satisfies CommandError)
        : base(command, ...args),
    );
    mockInvoke.mockClear();

    await act(async () => {
      await expect(result.current.bulkUnassign("s1")).rejects.toMatchObject({
        code: "OTHER",
      });
    });

    expect(commandOrder()).toEqual(["bulkUnassignSkill", "getProjectView"]);
    expect(result.current.assignments).toHaveLength(2);
    expect(result.current.pendingCells.size).toBe(0);
  });

  it("sends nothing for bulk actions without a selected project", async () => {
    const { result } = await renderReady();
    mockInvoke.mockClear();

    await act(async () => {
      expect(await result.current.bulkAssign("s1")).toBeUndefined();
      expect(await result.current.bulkUnassign("s1")).toBeUndefined();
      expect(await result.current.toggleAssignment("s1", "pi")).toBeNull();
    });

    expect(commandOrder()).toEqual([]);
  });

  it("applies the resync view and returns its report", async () => {
    const { result } = await renderReady();
    await withSelectedProject(result, ["pi"]);
    await act(async () => {
      await result.current.toggleAssignment("s1", "pi");
    });
    mockInvoke.mockClear();

    let summary: Awaited<
      ReturnType<typeof result.current.resyncProject>
    > | null = null;
    await act(async () => {
      summary = await result.current.resyncProject();
    });

    expect(commandOrder()).toEqual(["resyncProject"]);
    expect(summary).toMatchObject({ items: [{ tool: "pi", status: { status: "synced" } }] });
    expect(result.current.assignments).toHaveLength(1);
  });

  it("converges on the backend's view when a resync throws", async () => {
    const { result } = await renderReady();
    await withSelectedProject(result, ["pi"]);
    const base = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command, ...args) =>
      command === "resyncProject" || command === "resyncAllProjects"
        ? Promise.reject({ code: "OTHER", message: "busy" } satisfies CommandError)
        : base(command, ...args),
    );
    mockInvoke.mockClear();

    await act(async () => {
      await expect(result.current.resyncProject()).rejects.toMatchObject({ code: "OTHER" });
    });
    await act(async () => {
      await expect(result.current.resyncAll()).rejects.toMatchObject({ code: "OTHER" });
    });

    expect(commandOrder()).toEqual([
      "resyncProject",
      "getProjectView",
      "resyncAllProjects",
      "getProjectView",
    ]);
  });

  it("resync-all returns the one report spanning every project and re-reads only the shown matrix", async () => {
    const { result } = await renderReady();
    await withSelectedProject(result, ["pi"]);
    const spanning: ProjectSyncReport = {
      items: [
        syncItem(assignmentRecord("p1", "s1", "pi")),
        syncItem(assignmentRecord("p9", "s1", "pi"), {
          status: "failed",
          error: { code: "OTHER", message: "denied" },
        }),
      ],
    };
    const base = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command, ...args) =>
      command === "resyncAllProjects"
        ? base(command, ...args).then((raw) => ({
            ...(raw as Awaited<ReturnType<Commands["resyncAllProjects"]>>),
            report: spanning,
          }))
        : base(command, ...args),
    );
    mockInvoke.mockClear();

    let answer: { report: ProjectSyncReport; viewRefreshed: boolean } | undefined;
    await act(async () => {
      answer = await result.current.resyncAll();
    });

    expect(answer).toEqual({ report: spanning, viewRefreshed: true });
    expect(commandOrder()).toEqual(["resyncAllProjects", "getProjectView"]);
    expect(callsTo("getProjectView")).toEqual([["p1"]]);
  });

  it("resync-all keeps its report and marks the view stale when the matrix re-read fails", async () => {
    const { result } = await renderReady();
    await withSelectedProject(result);
    const base = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command, ...args) =>
      command === "getProjectView"
        ? Promise.reject({ code: "OTHER", message: "db locked" })
        : base(command, ...args),
    );
    mockInvoke.mockClear();

    let answer: { report: ProjectSyncReport; viewRefreshed: boolean } | undefined;
    await act(async () => {
      answer = await result.current.resyncAll();
    });

    expect(answer?.viewRefreshed).toBe(false);
    expect(answer?.report.items).toBeDefined();
    expect(commandOrder()).toEqual(["resyncAllProjects", "getProjectView"]);
  });

  it("takes the remaining project list straight from removeProject", async () => {
    const { result } = await renderReady();
    await withSelectedProject(result);
    mockInvoke.mockClear();

    let report: RemovalReport | null = null;
    await act(async () => {
      report = await result.current.removeProject("p1");
    });

    expect(commandOrder()).toEqual(["removeProject"]);
    expect(report).toEqual(emptyRemoval());
    expect(result.current.projects).toEqual([]);
    expect(result.current.selectedProjectId).toBeNull();
    expect(result.current.assignments).toEqual([]);
  });

  it("returns the per-target report and keeps the project it lists when an artifact stayed", async () => {
    const { result } = await renderReady();
    await withSelectedProject(result, ["pi"]);
    await act(async () => {
      await result.current.toggleAssignment("s1", "pi");
    });
    // Artifact removal could not take s1's link off disk: ADR-0002 keeps
    // the row with status `error` and the project stays registered; the
    // command answers with the list that still names it plus the report
    // (report data, not an error). The response carries no matrix, so the
    // hook converges on the backend's settled view — visible now, not
    // after a reselect.
    const kept: RemovalReport = {
      targets: [
        {
          rows: [{ scope: "assignment", id: "a1", project_id: "p1", skill_id: "s1", tool: "pi" }],
          path: "/work/p1/.pi/skills/s1",
          status: {
            status: "failed",
            error: { code: "OTHER", message: "permission denied" },
          },
        },
      ],
      central_removed: false,
      record_deleted: false,
    };
    const listed = result.current.projects.map((p) =>
      p.id === "p1" ? ({ ...p, sync_status: "error" } satisfies ProjectDto) : p,
    );
    const base = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command, ...args) => {
      if (command === "removeProject")
        return Promise.resolve({ projects: listed, report: kept });
      if (command === "getProjectView") {
        return base(command, ...args).then((view) => {
          const settled = view as ProjectViewDto;
          return {
            ...settled,
            project: { ...settled.project, sync_status: "error" },
            assignments: settled.assignments.map((a) => ({
              ...a,
              status: "error",
              last_error: "permission denied",
            })),
          } satisfies ProjectViewDto;
        });
      }
      return base(command, ...args);
    });
    mockInvoke.mockClear();

    let report: RemovalReport | null = null;
    await act(async () => {
      report = await result.current.removeProject("p1");
    });

    expect(report).toEqual(kept);
    expect(commandOrder()).toEqual(["removeProject", "getProjectView"]);
    expect(callsTo("getProjectView")).toEqual([["p1"]]);
    expect(result.current.selectedProjectId).toBe("p1");
    expect(result.current.assignments.map((a) => a.status)).toEqual(["error"]);
    expect(
      result.current.projects.find((p) => p.id === "p1")?.sync_status,
    ).toBe("error");
  });

  it("converges on the backend's view when the removal command itself fails", async () => {
    const { result } = await renderReady();
    await withSelectedProject(result, ["pi"]);
    await act(async () => {
      await result.current.toggleAssignment("s1", "pi");
    });
    // A thrown error is a store failure, not a per-target outcome; the
    // rows may still have settled, so the hook re-reads before rethrowing.
    const storeFailure: CommandError = {
      code: "OTHER",
      message: "database is locked",
    };
    const base = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command, ...args) => {
      if (command === "removeProject") return Promise.reject(storeFailure);
      return base(command, ...args);
    });
    mockInvoke.mockClear();

    await act(async () => {
      await expect(result.current.removeProject("p1")).rejects.toMatchObject({
        code: "OTHER",
      });
    });

    expect(commandOrder()).toEqual(["removeProject", "getProjectView"]);
    expect(result.current.selectedProjectId).toBe("p1");
    expect(result.current.assignments.map((a) => a.skill_id)).toEqual(["s1"]);
  });

  it("selects a project with one view read", async () => {
    const { result } = await renderReady();
    await withSelectedProject(result, ["pi"]);
    mockInvoke.mockClear();

    await act(async () => {
      await result.current.selectProject("p1");
    });

    expect(commandOrder()).toEqual(["getProjectView"]);
    expect(result.current.tools.map((t) => t.tool)).toEqual(["pi"]);
  });
});

describe("useProjectState selection and matrix agree", () => {
  it("applies a late mutation result for a deselected project to its row only", async () => {
    const { result } = await renderReady();
    await withSelectedProject(result, ["pi"]);
    await act(async () => {
      await result.current.registerProject("/work/p2", {
        add_to_gitignore: false,
        add_to_exclude: false,
      });
    });

    // p1's toggle is still in flight when the operator selects p2.
    const base = mockInvoke.getMockImplementation()!;
    let release!: () => void;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    mockInvoke.mockImplementation((command, ...args) => {
      if (command === "toggleProjectSkillAssignment") {
        return gate.then(() => base(command, ...args));
      }
      return base(command, ...args);
    });
    let toggle!: Promise<ToggleResult | null>;
    act(() => {
      toggle = result.current.toggleAssignment("s1", "pi");
    });
    await act(async () => {
      await result.current.selectProject("p2");
    });
    await act(async () => {
      release();
      await toggle;
    });

    // The matrix is p2's; p1's new assignment shows only on p1's row.
    expect(result.current.selectedProjectId).toBe("p2");
    expect(result.current.assignments).toEqual([]);
    expect(
      result.current.projects.find((p) => p.id === "p1")?.assignment_count,
    ).toBe(1);
  });

  it("derives the matrix once per view under StrictMode's doubled updaters", async () => {
    // StrictMode invokes state updaters twice; deriving the matrix inside
    // one would double-fire. Selection and removal must settle on the
    // single view read either way.
    const rendered = renderHook(() => useProjectState(), {
      wrapper: StrictMode,
    });
    await waitFor(() => {
      expect(rendered.result.current.projectsLoading).toBe(false);
    });
    const { result } = rendered;
    await withSelectedProject(result, ["pi"]);
    await act(async () => {
      await result.current.toggleAssignment("s1", "pi");
    });
    mockInvoke.mockClear();

    await act(async () => {
      await result.current.selectProject("p1");
    });

    expect(commandOrder()).toEqual(["getProjectView"]);
    expect(result.current.tools.map((t) => t.tool)).toEqual(["pi"]);
    expect(result.current.assignments.map((a) => a.skill_id)).toEqual(["s1"]);
    expect(result.current.assignmentsReconciled).toBe(true);

    await act(async () => {
      await result.current.removeProject("p1");
    });

    expect(result.current.selectedProjectId).toBeNull();
    expect(result.current.tools).toEqual([]);
    expect(result.current.assignments).toEqual([]);
  });
});

describe("useProjectState assignment reconciliation flag", () => {
  it("defaults to reconciled and carries a reconciled view through", async () => {
    const { result } = await renderReady();
    expect(result.current.assignmentsReconciled).toBe(true);

    await withSelectedProject(result);

    expect(result.current.assignmentsReconciled).toBe(true);
  });

  it("reports a skipped reconcile so the UI cannot treat it as healthy", async () => {
    stubBackend({ reconciled: false });
    const { result } = await renderReady();

    await withSelectedProject(result);

    expect(result.current.assignmentsReconciled).toBe(false);
  });
});

describe("useProjectState dialog", () => {
  it("opens, replaces and closes one dialog value", async () => {
    const { result } = await renderReady();
    expect(result.current.dialog).toBeNull();

    act(() => {
      result.current.openDialog({ kind: "add" });
    });
    expect(result.current.dialog).toEqual({ kind: "add" });

    act(() => {
      result.current.openDialog({ kind: "remove", projectId: "p1" });
    });
    // Opening a dialog replaces the previous one: two cannot be open, and a
    // target can never outlive the dialog it belongs to.
    expect(result.current.dialog).toEqual({
      kind: "remove",
      projectId: "p1",
    });

    act(() => {
      result.current.closeDialog();
    });
    expect(result.current.dialog).toBeNull();
  });
});

describe("useProjectState edit-project gitignore", () => {
  it("reads status and writes toggles through the gitignore commands", async () => {
    const { result } = await renderReady();

    let status: GitignoreStatusDto | undefined;
    await act(async () => {
      status = await result.current.getGitignoreStatus("p9");
    });
    expect(status).toEqual({ in_gitignore: true, in_exclude: false });
    expect(callsTo("getProjectGitignoreStatus")).toEqual([["p9"]]);

    await act(async () => {
      await result.current.updateGitignore("p9", {
        add_to_gitignore: false,
        add_to_exclude: true,
      });
    });
    // The DTO travels whole: `update_project_gitignore` takes the same
    // `IgnoreUpdateOptions` shape `configure_project_tools` does, so nothing
    // unpacks it into positional bools on the way to the backend.
    expect(callsTo("updateProjectGitignore")).toEqual([
      ["p9", { add_to_gitignore: false, add_to_exclude: true }],
    ]);
  });
});
