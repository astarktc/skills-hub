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

import {
  invokeTauri,
  type CommandName,
  type Commands,
} from "../../lib/tauri";
import { useProjectState } from "./useProjectState";

// The seam is generic over the command table; the stub switches on the
// command name, so it is typed loosely (positional args, unknown result).
const mockInvoke = vi.mocked(
  invokeTauri as unknown as (
    command: CommandName,
    ...args: unknown[]
  ) => Promise<unknown>,
);

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
  mockInvoke.mockImplementation((command, ...args) => {
    switch (command) {
      case "listProjects":
        return Promise.resolve([...projects]);
      case "getManagedSkills":
        return Promise.resolve([]);
      case "registerProject": {
        const created = project(`p${nextId++}`);
        projects.push(created);
        return Promise.resolve(created);
      }
      case "listProjectTools": {
        const [projectId] = args as Parameters<Commands["listProjectTools"]>;
        return Promise.resolve(
          toolRecords(projectId, toolsByProject.get(projectId) ?? []),
        );
      }
      case "listProjectSkillAssignments":
        return Promise.resolve([]);
      case "configureProjectTools": {
        const [projectId, tools] = args as Parameters<
          Commands["configureProjectTools"]
        >;
        toolsByProject.set(projectId, tools);
        return Promise.resolve(toolRecords(projectId, tools));
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
    ).toEqual([
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
    mockInvoke.mockImplementation((command, ...args) => {
      if (command === "configureProjectTools") {
        return base("configureProjectTools", ...args).then(() =>
          Promise.reject({ code: "INTERNAL", message: "disk full" }),
        );
      }
      return base(command, ...args);
    });

    await act(async () => {
      await expect(
        result.current.configureTools(["claude_code"]),
      ).rejects.toMatchObject({ code: "INTERNAL" });
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
    expect(order[0]).toBe("configureProjectTools");
    expect(order).toContain("listProjectSkillAssignments");
    expect(order).toContain("listProjects");
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
    expect(callsTo("getProjectGitignoreStatus")).toEqual([["p9"]]);

    await act(async () => {
      await result.current.updateGitignore("p9", {
        add_to_gitignore: false,
        add_to_exclude: true,
      });
    });
    expect(callsTo("updateProjectGitignore")).toEqual([["p9", false, true]]);
  });
});
