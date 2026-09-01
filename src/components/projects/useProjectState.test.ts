// Tests at the ProjectState seam for the add-project → configure-tools →
// gitignore sequence. The backend owns the persist → derive → write ordering
// (`configure_project_tools` carries the ignore intent); the hook's job is
// to capture the intent at registration and hand it to that single command
// when the tool set is confirmed. The backend is mocked at the invokeTauri
// module seam.

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  GitignoreStatusDto,
  IgnoreUpdateOptions,
  ProjectDto,
  ProjectToolDto,
} from "./types";

vi.mock("../../lib/tauri", () => ({
  isTauri: true,
  invokeTauri: vi.fn(),
}));

import { invokeTauri } from "../../lib/tauri";
import { useProjectState } from "./useProjectState";

const mockInvoke = vi.mocked(invokeTauri);

function project(id: string, toolCount = 0): ProjectDto {
  return {
    id,
    path: `/work/${id}`,
    name: id,
    created_at: 1,
    updated_at: 1,
    tool_count: toolCount,
    skill_count: 0,
    assignment_count: 0,
    sync_status: "synced",
    path_exists: true,
  };
}

function toolRecords(projectId: string, tools: string[]): ProjectToolDto[] {
  return tools.map((tool) => ({ id: `${projectId}:${tool}`, project_id: projectId, tool }));
}

/** Minimal echo backend: registration appends a project; tool config persists the set. */
function stubBackend() {
  const projects: ProjectDto[] = [];
  const toolsByProject = new Map<string, string[]>();
  let nextId = 1;
  mockInvoke.mockImplementation((command: string, args?: unknown) => {
    switch (command) {
      case "list_projects":
        return Promise.resolve([...projects]);
      case "get_managed_skills":
        return Promise.resolve([]);
      case "register_project": {
        const created = project(`p${nextId++}`);
        projects.push(created);
        return Promise.resolve(created);
      }
      case "list_project_tools": {
        const { projectId } = args as { projectId: string };
        return Promise.resolve(
          toolRecords(projectId, toolsByProject.get(projectId) ?? []),
        );
      }
      case "list_project_skill_assignments":
        return Promise.resolve([]);
      case "configure_project_tools": {
        const { projectId, tools } = args as {
          projectId: string;
          tools: string[];
        };
        toolsByProject.set(projectId, tools);
        return Promise.resolve(toolRecords(projectId, tools));
      }
      case "get_project_gitignore_status":
        return Promise.resolve({
          in_gitignore: true,
          in_exclude: false,
        } satisfies GitignoreStatusDto);
      case "update_project_gitignore":
        return Promise.resolve(undefined);
      default:
        return Promise.resolve(undefined);
    }
  });
}

const callsTo = (command: string) =>
  mockInvoke.mock.calls
    .filter(([name]) => name === command)
    .map(([, args]) => args as Record<string, unknown>);

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
    expect(callsTo("update_project_gitignore")).toEqual([]);
    expect(callsTo("configure_project_tools")).toEqual([]);

    await act(async () => {
      await result.current.selectProject("p1");
    });
    await act(async () => {
      await result.current.configureTools(["claude_code", "windsurf"]);
    });

    expect(callsTo("configure_project_tools")).toEqual([
      {
        projectId: "p1",
        tools: ["claude_code", "windsurf"],
        gitignore: intent,
      },
    ]);
    // One command owns the whole sequence — no separate gitignore replay.
    expect(callsTo("update_project_gitignore")).toEqual([]);
    expect(commandOrder().indexOf("register_project")).toBeLessThan(
      commandOrder().indexOf("configure_project_tools"),
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

    expect(callsTo("configure_project_tools").map((a) => a.gitignore)).toEqual([
      { add_to_gitignore: true, add_to_exclude: true },
      null,
    ]);
  });

  it("registers without an intent when neither toggle is set", async () => {
    const { result } = await renderReady();
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
      await result.current.configureTools(["claude_code"]);
    });

    expect(callsTo("configure_project_tools")[0]?.gitignore).toBeNull();
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
    expect(callsTo("configure_project_tools")).toEqual([
      { projectId: "p1", tools: ["claude_code"], gitignore: null },
    ]);
  });

  it("re-fetches tools when the batch command fails so state converges on the backend", async () => {
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
    // backend now lists the new tool set.
    const base = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command, args) => {
      if (command === "configure_project_tools") {
        return base("configure_project_tools", args).then(() =>
          Promise.reject({ code: "INTERNAL", message: "disk full" }),
        );
      }
      return base(command, args);
    });

    await act(async () => {
      await expect(
        result.current.configureTools(["claude_code"]),
      ).rejects.toMatchObject({ code: "INTERNAL" });
    });

    expect(result.current.tools.map((t) => t.tool)).toEqual(["claude_code"]);
  });

  it("refreshes tools, assignments and the project list after configuring", async () => {
    const { result } = await renderReady();
    await act(async () => {
      await result.current.registerProject("/work/p1", {
        add_to_gitignore: false,
        add_to_exclude: false,
      });
    });
    await act(async () => {
      await result.current.selectProject("p1");
    });
    mockInvoke.mockClear();
    await act(async () => {
      await result.current.configureTools(["pi"]);
    });

    const order = commandOrder();
    expect(order[0]).toBe("configure_project_tools");
    expect(order).toContain("list_project_skill_assignments");
    expect(order).toContain("list_projects");
    expect(result.current.tools.map((t) => t.tool)).toEqual(["pi"]);
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
    expect(callsTo("get_project_gitignore_status")).toEqual([
      { projectId: "p9" },
    ]);

    await act(async () => {
      await result.current.updateGitignore("p9", {
        add_to_gitignore: false,
        add_to_exclude: true,
      });
    });
    expect(callsTo("update_project_gitignore")).toEqual([
      { projectId: "p9", addToGitignore: false, addToExclude: true },
    ]);
  });
});
