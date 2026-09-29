// Tests at the project view's action hook (B1): every project mutation runs
// under the reporter's lifecycle, so while one runs every other is refused —
// it sends no command — and `blockedReason` names the running one for the
// controls to show. Composed as the page composes it: the real reporter,
// project world and surface; the backend at the invokeTauri seam, the folder
// picker at the platform seam.

import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ProjectDto, ProjectViewDto } from "./types";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
}));
vi.mock("../../lib/tauri", () => ({
  isTauri: true,
  invokeTauri: vi.fn(),
}));
vi.mock("../../lib/platform", () => ({
  pickFolder: vi.fn(async () => ["/work/moved"]),
}));

import { invokeTauri, type CommandName } from "../../lib/tauri";
import { pickFolder } from "../../lib/platform";
import { useStatusReporter } from "../../hooks/useStatusReporter";
import { useSurfaceState } from "../../hooks/useSurfaceState";
import { useProjectState } from "./useProjectState";
import { useProjectActions } from "./useProjectActions";

const mockInvoke = vi.mocked(
  invokeTauri as unknown as (command: CommandName, ...args: unknown[]) => Promise<unknown>,
);

const t = (key: string, opts?: Record<string, unknown>) =>
  opts ? `${key} ${JSON.stringify(opts)}` : key;

const PROJECT: ProjectDto = {
  id: "p1",
  path: "/work/p1",
  name: "p1",
  created_at: 0,
  updated_at: 0,
  tool_count: 2,
  skill_count: 1,
  assignment_count: 1,
  sync_status: "synced",
  path_exists: true,
};
const VIEW: ProjectViewDto = {
  project: PROJECT,
  tools: ["claude_code", "cursor"].map((tool) => ({ id: `p1:${tool}`, project_id: "p1", tool })),
  assignments: [
    {
      id: "a1",
      project_id: "p1",
      skill_id: "s2",
      skill_name: "beta",
      tool: "cursor",
      mode: "symlink",
      status: "synced",
      last_error: null,
      synced_at: 1,
      content_hash: null,
      created_at: 1,
    },
  ],
  reconciled: true,
};

let release: () => void = () => {};

function backend() {
  const held = new Promise<void>((resolve) => {
    release = resolve;
  });
  mockInvoke.mockImplementation(async (command) => {
    switch (command) {
      case "listProjects":
        return [PROJECT];
      case "getProjectView":
      case "updateProjectPath":
        return VIEW;
      case "toggleProjectSkillAssignment":
        await held;
        return { kind: "assigned", view: VIEW, report: { items: [] } };
      default:
        return null;
    }
  });
}

function renderActions() {
  const catalog = { toolLabelById: {}, loadProjectToolStatus: vi.fn(async () => ({ tools: [], installed: [], newly_installed: [] })) };
  return renderHook(() => {
    const reporter = useStatusReporter(t);
    const surface = useSurfaceState();
    const state = useProjectState();
    const actions = useProjectActions({ t, state, catalog, reporter, surface });
    return { reporter, surface, state, actions };
  });
}

type Rendered = ReturnType<typeof renderActions>["result"];

const OTHER_MUTATIONS: [string, (result: Rendered) => Promise<unknown>][] = [
  ["register a project", (r) => r.current.actions.handleAddProject("/work/p2", { add_to_gitignore: true, add_to_exclude: false })],
  ["save the Edit dialog's ignore choice", (r) => r.current.actions.handleEditSave("p1", { add_to_gitignore: true, add_to_exclude: false })],
  ["update a missing project's path", (r) => r.current.actions.handleUpdatePath("p1")],
  ["configure the tool set", (r) => r.current.actions.handleToolConfigConfirm(["claude_code"])],
  ["remove the project", (r) => r.current.actions.handleRemoveProject()],
  ["sync the project", (r) => r.current.actions.handleResyncProject()],
  ["sync every project", (r) => r.current.actions.handleResyncAll()],
  ["toggle another cell", (r) => r.current.actions.handleToggleAssignment("s2", "cursor")],
  ["assign a row", (r) => r.current.actions.handleBulkAssign("s2")],
  ["unassign a row", (r) => r.current.actions.handleBulkUnassign("s2")],
];

beforeEach(() => {
  vi.clearAllMocks();
});
afterEach(cleanup);

describe("useProjectActions: one mutation at a time (D19)", () => {
  it.each(OTHER_MUTATIONS)("while a cell toggle runs, '%s' is refused and sends no command", async (_name, mutate) => {
    backend();
    const { result } = renderActions();
    await waitFor(() => expect(result.current.state.projectsLoading).toBe(false));
    await act(async () => {
      await result.current.state.selectProject("p1");
    });
    act(() => result.current.surface.openDialog({ kind: "projectRemove", projectId: "p1" }));

    let running: Promise<void> = Promise.resolve();
    act(() => {
      running = result.current.actions.handleToggleAssignment("s1", "claude_code");
    });
    await waitFor(() => expect(result.current.reporter.pending).not.toBeNull());
    // The one running action pends only its own cell; every other control
    // reads the refusal as its disabled reason.
    expect([...result.current.actions.pendingCells]).toEqual(["s1:claude_code"]);
    expect(result.current.actions.blockedReason).toBe('lifecycle.running {"action":"lifecycle.verbs.assign"}');

    mockInvoke.mockClear();
    await act(async () => {
      await mutate(result);
    });
    expect(mockInvoke).not.toHaveBeenCalled();
    // A path update does not even open the picker: its choice could not apply.
    expect(pickFolder).not.toHaveBeenCalled();

    await act(async () => {
      release();
      await running;
    });
    expect(result.current.actions.blockedReason).toBeNull();
  });

  it("registration, the Edit save and a path update run under the lifecycle when nothing else does", async () => {
    backend();
    const { result } = renderActions();
    await waitFor(() => expect(result.current.state.projectsLoading).toBe(false));
    mockInvoke.mockClear();
    await act(async () => {
      await result.current.actions.handleEditSave("p1", { add_to_gitignore: true, add_to_exclude: false });
    });
    await act(async () => {
      await result.current.actions.handleUpdatePath("p1");
    });
    expect(mockInvoke.mock.calls.map(([command]) => command)).toEqual(["updateProjectGitignore", "updateProjectPath"]);
    expect(pickFolder).toHaveBeenCalledTimes(1);
    expect(result.current.reporter.notifications.map((n) => n.title)).toEqual([
      "projects.updatePathSuccess",
      "projects.configureProject",
    ]);
  });
});
