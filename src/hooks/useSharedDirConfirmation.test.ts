// Tests at the shared-dir confirmation seam: the decision (does this tool
// share its skills dir?), the label arithmetic (which other members), and
// the one pending-confirmation value both flows drive through the Modal.
// No backend, no window.confirm — the hook takes the tool list as input.

import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import {
  useSharedDirConfirmation,
  type SharedDirTool,
} from "./useSharedDirConfirmation";

// claude and pi share a skills dir (backend-owned grouping); cursor stands
// alone and its shared_with is the singleton group.
const TOOLS: SharedDirTool[] = [
  { id: "claude", label: "CLAUDE", sharedWith: ["claude", "pi"] },
  { id: "pi", label: "PI", sharedWith: ["claude", "pi"] },
  { id: "cursor", label: "CURSOR", sharedWith: ["cursor"] },
];

function render(tools: SharedDirTool[] = TOOLS) {
  return renderHook(() => useSharedDirConfirmation(tools));
}

describe("useSharedDirConfirmation decision", () => {
  it("needs confirmation only for a tool sharing its dir", () => {
    const { result } = render();

    expect(result.current.needsConfirmation("claude")).toBe(true);
    expect(result.current.needsConfirmation("pi")).toBe(true);
    expect(result.current.needsConfirmation("cursor")).toBe(false);
    expect(result.current.needsConfirmation("unknown")).toBe(false);
  });
});

describe("useSharedDirConfirmation labels", () => {
  it("lists the other members' labels in the group's order", () => {
    const { result } = render();

    expect(result.current.sharedLabels("claude")).toEqual(["PI"]);
    expect(result.current.sharedLabels("pi")).toEqual(["CLAUDE"]);
  });

  it("falls back to the id for a member with no known label", () => {
    const { result } = render([
      { id: "claude", label: "CLAUDE", sharedWith: ["claude", "ghost"] },
    ]);

    expect(result.current.sharedLabels("claude")).toEqual(["ghost"]);
  });

  it("has no labels for a standalone or unknown tool", () => {
    const { result } = render();

    expect(result.current.sharedLabels("cursor")).toEqual([]);
    expect(result.current.sharedLabels("unknown")).toEqual([]);
  });
});

describe("useSharedDirConfirmation request", () => {
  it("resolves true immediately, with no pending value, when not shared", async () => {
    const { result } = render();

    let answer: boolean | null = null;
    await act(async () => {
      answer = await result.current.request("cursor");
    });

    expect(answer).toBe(true);
    expect(result.current.pending).toBeNull();
  });

  it("exposes the pending confirmation and resolves with the operator's answer", async () => {
    const { result } = render();

    let answer: boolean | null = null;
    act(() => {
      void result.current.request("claude").then((ok) => {
        answer = ok;
      });
    });

    expect(result.current.pending).not.toBeNull();
    expect(result.current.pending!.toolKey).toBe("claude");
    expect(result.current.pending!.toolLabel).toBe("CLAUDE");
    expect(result.current.pending!.labels).toEqual(["PI"]);
    expect(answer).toBeNull();

    await act(async () => {
      result.current.pending!.resolve(true);
    });

    expect(answer).toBe(true);
    expect(result.current.pending).toBeNull();
  });

  it("resolves false when the operator declines", async () => {
    const { result } = render();

    let answer: boolean | null = null;
    act(() => {
      void result.current.request("pi").then((ok) => {
        answer = ok;
      });
    });

    await act(async () => {
      result.current.pending!.resolve(false);
    });

    expect(answer).toBe(false);
    expect(result.current.pending).toBeNull();
  });

  it("cancel resolves the pending confirmation as declined", async () => {
    const { result } = render();

    let answer: boolean | null = null;
    act(() => {
      void result.current.request("claude").then((ok) => {
        answer = ok;
      });
    });

    await act(async () => {
      result.current.cancel();
    });

    expect(answer).toBe(false);
    expect(result.current.pending).toBeNull();
  });

  it("cancel with nothing pending is a no-op", () => {
    const { result } = render();

    act(() => {
      result.current.cancel();
    });

    expect(result.current.pending).toBeNull();
  });
});
