// Tests at the file browser's loading seam: the folder listing and the open
// file's text through invokeTauri, the default file, a selection, a new
// document version re-reading, stale answers dropped, failures as values.

import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/tauri", () => ({
  isTauri: true,
  invokeTauri: vi.fn(),
}));

import { invokeTauri, type CommandName } from "../lib/tauri";
import { useSkillDocument, type SkillDocumentRef } from "./useSkillDocument";

const mockInvoke = vi.mocked(
  invokeTauri as unknown as (command: CommandName, ...args: unknown[]) => Promise<unknown>,
);

const FILES: Record<string, { path: string; size: number }[]> = {
  "/central/docx": [
    { path: "reference/guide.md", size: 20 },
    { path: "SKILL.md", size: 10 },
  ],
  "/cache/preview": [{ path: "docs/readme.md", size: 5 }],
};

function stubBackend(overrides: Partial<Record<CommandName, (...args: unknown[]) => Promise<unknown>>> = {}) {
  mockInvoke.mockImplementation(async (command, ...args) => {
    const override = overrides[command];
    if (override) return override(...args);
    switch (command) {
      case "listSkillFiles":
        return FILES[args[0] as string] ?? [];
      case "readSkillFile":
        return `text of ${args[0] as string}/${args[1] as string}`;
      default:
        throw new Error(`unexpected command ${command}`);
    }
  });
}

beforeEach(() => {
  mockInvoke.mockReset();
  stubBackend();
});
afterEach(cleanup);

describe("useSkillDocument", () => {
  it("lists the folder and opens SKILL.md, reading it", async () => {
    const { result } = renderHook(() => useSkillDocument({ root: "/central/docx", version: 1 }));
    expect(result.current.loadingFiles).toBe(true);
    await waitFor(() => expect(result.current.content).toBe("text of /central/docx/SKILL.md"));
    expect(result.current.activeFile).toBe("SKILL.md");
    expect(result.current.tree.map((node) => node.name)).toEqual(["reference", "SKILL.md"]);
    expect(result.current.loadingFiles).toBe(false);
    expect(result.current.loadingContent).toBe(false);
    expect(result.current.expanded.size).toBe(0);
  });

  it("reads the file the operator selects and keeps its folder open", async () => {
    const { result } = renderHook(() => useSkillDocument({ root: "/central/docx", version: 1 }));
    await waitFor(() => expect(result.current.activeFile).toBe("SKILL.md"));
    act(() => result.current.selectFile("reference/guide.md"));
    await waitFor(() => expect(result.current.content).toBe("text of /central/docx/reference/guide.md"));
    expect(result.current.expanded.has("reference")).toBe(true);
    act(() => result.current.toggleDir("reference"));
    expect(result.current.expanded.has("reference")).toBe(false);
    act(() => result.current.toggleDir("reference"));
    expect(result.current.expanded.has("reference")).toBe(true);
  });

  it("expands the folders of a default file that is not at the root", async () => {
    const { result } = renderHook(() => useSkillDocument({ root: "/cache/preview", version: 1 }));
    await waitFor(() => expect(result.current.activeFile).toBe("docs/readme.md"));
    expect([...result.current.expanded]).toEqual(["docs"]);
  });

  it("re-reads when the document's version changes and resets the selection for a new root", async () => {
    let doc: SkillDocumentRef = { root: "/central/docx", version: 1 };
    const { result, rerender } = renderHook(() => useSkillDocument(doc));
    await waitFor(() => expect(result.current.activeFile).toBe("SKILL.md"));
    act(() => result.current.selectFile("reference/guide.md"));
    const lists = () => mockInvoke.mock.calls.filter(([command]) => command === "listSkillFiles").length;
    const before = lists();
    doc = { root: "/central/docx", version: 2 };
    rerender();
    await waitFor(() => expect(lists()).toBe(before + 1));
    await waitFor(() => expect(result.current.activeFile).toBe("SKILL.md"));
    doc = { root: "/cache/preview", version: 1 };
    rerender();
    await waitFor(() => expect(result.current.activeFile).toBe("docs/readme.md"));
  });

  it("answers listing and reading failures as typed values, not throws", async () => {
    stubBackend({
      listSkillFiles: async () => {
        throw { code: "SOURCE_PATH_MISSING", path: "/central/docx" };
      },
    });
    const { result } = renderHook(() => useSkillDocument({ root: "/central/docx", version: 1 }));
    await waitFor(() => expect(result.current.listError).toEqual({ code: "SOURCE_PATH_MISSING", path: "/central/docx" }));
    expect(result.current.files).toEqual([]);
    expect(result.current.activeFile).toBeNull();
    expect(result.current.loadingFiles).toBe(false);

    stubBackend({
      readSkillFile: async () => {
        throw { code: "OTHER", message: "not utf-8" };
      },
    });
    const other = renderHook(() => useSkillDocument({ root: "/cache/preview", version: 3 }));
    await waitFor(() => expect(other.result.current.contentError).toEqual({ code: "OTHER", message: "not utf-8" }));
    expect(other.result.current.content).toBe("");
  });

  it("drops a stale listing when the document changes before it lands", async () => {
    let release: (value: unknown) => void = () => {};
    stubBackend({
      listSkillFiles: (root) =>
        root === "/central/docx"
          ? new Promise((resolve) => {
              release = resolve;
            })
          : Promise.resolve(FILES["/cache/preview"]),
    });
    let doc: SkillDocumentRef | null = { root: "/central/docx", version: 1 };
    const { result, rerender } = renderHook(() => useSkillDocument(doc));
    doc = { root: "/cache/preview", version: 1 };
    rerender();
    await waitFor(() => expect(result.current.activeFile).toBe("docs/readme.md"));
    await act(async () => release(FILES["/central/docx"]));
    expect(result.current.files).toEqual(FILES["/cache/preview"]);
    doc = null;
    rerender();
    expect(result.current.activeFile).toBeNull();
    expect(result.current.loadingFiles).toBe(false);
  });
});
