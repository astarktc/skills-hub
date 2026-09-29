// Tests at the ToolCatalog seam: one global tool-status load on mount, the
// labels / installedness / shared-dir map derived from it, and the project
// tool entries read on demand. The backend is mocked at the invokeTauri seam.

import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ToolInfoDto, ToolStatusDto } from "../components/skills/types";

vi.mock("../lib/tauri", () => ({
  isTauri: true,
  invokeTauri: vi.fn(),
}));

import { invokeTauri, type CommandName } from "../lib/tauri";
import { useToolCatalog } from "./useToolCatalog";

const mockInvoke = vi.mocked(
  invokeTauri as unknown as (
    command: CommandName,
    ...args: unknown[]
  ) => Promise<unknown>,
);

// i18next stand-in: `tools.cursor` has a catalog label, the rest fall back
// to the backend's label (the caller's defaultValue).
const t = (key: string, opts?: Record<string, unknown>) =>
  key === "tools.cursor" ? "Cursor (i18n)" : String(opts?.defaultValue ?? key);

function toolInfo(key: string, shared_with: string[] = [key]): ToolInfoDto {
  return { key, label: key.toUpperCase(), installed: true, shared_with, constituents: [] };
}

const GLOBAL: ToolStatusDto = {
  tools: [
    toolInfo("claude", ["claude", "pi"]),
    toolInfo("pi", ["claude", "pi"]),
    toolInfo("cursor"),
    { ...toolInfo("goose"), installed: false },
  ],
  installed: ["claude", "pi", "cursor"],
  newly_installed: ["cursor"],
};

const PROJECT: ToolStatusDto = {
  tools: [toolInfo("claude"), toolInfo("project_only")],
  installed: ["claude"],
  newly_installed: [],
};

function stubBackend() {
  mockInvoke.mockImplementation((command) => {
    switch (command) {
      case "getToolStatus":
        return Promise.resolve(GLOBAL);
      case "getProjectToolStatus":
        return Promise.resolve(PROJECT);
      default:
        return Promise.reject(new Error(`unexpected ${command}`));
    }
  });
}

beforeEach(() => {
  vi.clearAllMocks();
});
afterEach(cleanup);

describe("useToolCatalog", () => {
  it("loads the global tool status once and derives labels, installedness and shared dirs", async () => {
    stubBackend();
    const { result, rerender } = renderHook(() => useToolCatalog({ t }));
    await waitFor(() => expect(result.current.toolStatus).toEqual(GLOBAL));
    rerender();

    expect(mockInvoke.mock.calls.filter(([cmd]) => cmd === "getToolStatus")).toHaveLength(1);
    expect(result.current.tools).toEqual([
      { id: "claude", label: "CLAUDE" },
      { id: "pi", label: "PI" },
      { id: "cursor", label: "Cursor (i18n)" },
      { id: "goose", label: "GOOSE" },
    ]);
    expect(result.current.toolLabelById).toMatchObject({ cursor: "Cursor (i18n)", goose: "GOOSE" });
    expect(result.current.installedToolIds).toEqual(["claude", "pi", "cursor"]);
    expect(result.current.installedTools.map((tool) => tool.id)).toEqual(["claude", "pi", "cursor"]);
    // Only groups of two or more: a standalone tool maps to nothing.
    expect(result.current.sharedToolIdsByToolId).toEqual({
      claude: ["claude", "pi"],
      pi: ["claude", "pi"],
    });
    // The project entries are not read until asked for.
    expect(mockInvoke).not.toHaveBeenCalledWith("getProjectToolStatus");
    expect(result.current.projectToolStatus).toBeNull();
  });

  it("reads the project tool entries on demand, and their labels join the label map", async () => {
    stubBackend();
    const { result } = renderHook(() => useToolCatalog({ t }));
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());

    await act(async () => {
      await result.current.loadProjectToolStatus();
    });
    expect(result.current.projectToolStatus).toEqual(PROJECT);
    expect(result.current.toolLabelById.project_only).toBe("PROJECT_ONLY");
    // A second ask re-reads (a tool installed meanwhile shows up).
    await act(async () => {
      await result.current.loadProjectToolStatus();
    });
    expect(mockInvoke.mock.calls.filter(([cmd]) => cmd === "getProjectToolStatus")).toHaveLength(2);
  });

  it("a failed project read rejects to its caller and keeps the last entries", async () => {
    stubBackend();
    const { result } = renderHook(() => useToolCatalog({ t }));
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());
    mockInvoke.mockRejectedValueOnce({ code: "OTHER", message: "boom" });
    await expect(
      act(async () => {
        await result.current.loadProjectToolStatus();
      }),
    ).rejects.toEqual({ code: "OTHER", message: "boom" });
    expect(result.current.projectToolStatus).toBeNull();
  });

  it("a failed global load leaves an empty catalog, not an error", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    mockInvoke.mockRejectedValue(new Error("detect failed"));
    const { result } = renderHook(() => useToolCatalog({ t }));
    await waitFor(() => expect(warn).toHaveBeenCalled());
    expect(result.current.toolStatus).toBeNull();
    expect(result.current.tools).toEqual([]);
    expect(result.current.installedToolIds).toEqual([]);
    warn.mockRestore();
  });
});
