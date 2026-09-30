// Tests for the surface module: "what is on screen" as one value. Route
// transitions and their hash round-trip (jsdom `location` / `history`), the
// catalog invariant (a focused id that leaves its catalog clears itself),
// per-surface selection, the dialog stack (one dialog at a time, nested
// layers, top-layer-only close, offered dialogs) and asks (settled exactly
// once: `confirmed` / `declined` by a button, `dismissed` otherwise).

import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  formatHash,
  parseHash,
  useFocusReconciliation,
  useSurfaceState,
  type AskAnswer,
  type AskAnswerOf,
  type ExplorePreview,
  type Route,
} from "./useSurfaceState";
import type { OverwriteRow } from "../lib/overwriteAsk";

const row = (skillId: string): OverwriteRow => ({
  skillId,
  skillName: skillId,
  toolKey: "claude",
  toolLabel: "Claude",
  path: `/t/${skillId}`,
  modifiedAt: null,
  sameContent: false,
  scope: { kind: "global" },
});

const preview: ExplorePreview = {
  sourceUrl: "https://github.com/acme/tools",
  name: "tools",
  summary: null,
  cachePath: "/cache/tools",
  fetchedAt: 1,
};

/** Start every test at a clean address with no fragment. */
beforeEach(() => {
  window.history.replaceState(null, "", "/");
});
// No vitest globals, so Testing Library's auto-cleanup is off: unmount each
// hook explicitly, or its window listeners answer the next test's events.
afterEach(cleanup);

const render = () => renderHook(() => useSurfaceState());

describe("hash grammar", () => {
  const routes: Route[] = [
    { area: "skills", focused: null },
    { area: "skills", focused: { kind: "skill", id: "s 1/é" } },
    { area: "projects", focused: null },
    { area: "projects", focused: { kind: "project", id: "p1" } },
    { area: "settings", focused: null },
    { area: "settings", focused: { kind: "settings", pane: "sync" } },
    { area: "explore", focused: null },
  ];

  it("round-trips area and focused subject", () => {
    for (const route of routes) {
      expect(parseHash(formatHash(route))).toEqual(route);
    }
    expect(formatHash({ area: "skills", focused: { kind: "skill", id: "a/b" } })).toBe(
      "#/skills/a%2Fb",
    );
  });

  it("never addresses an Explore preview", () => {
    expect(formatHash({ area: "explore", focused: { kind: "explore", ref: preview } })).toBe(
      "#/explore",
    );
  });

  it("addresses nothing for an empty or unknown hash", () => {
    expect(parseHash("")).toBeNull();
    expect(parseHash("#/nowhere")).toBeNull();
    expect(parseHash("#/skills/a/b")).toBeNull();
    expect(parseHash("#/skills/%E0%A4%A")).toBeNull();
    expect(parseHash("#/skills/")).toEqual({ area: "skills", focused: null });
  });
});

describe("route and hash", () => {
  it("opens on My Skills and normalises an empty address without a history entry", () => {
    const before = window.history.length;
    const { result } = render();

    expect(result.current.route).toEqual({ area: "skills", focused: null });
    expect(window.location.hash).toBe("#/skills");
    expect(window.history.length).toBe(before);
  });

  it("deep-links to a focused skill from the hash it opened on", () => {
    window.history.replaceState(null, "", "/#/skills/s1");
    const { result } = render();

    expect(result.current.area).toBe("skills");
    expect(result.current.focused).toEqual({ kind: "skill", id: "s1" });
  });

  it("focus shows the subject's area and pushes its address; unfocus returns", () => {
    const { result } = render();
    const before = window.history.length;

    act(() => result.current.focus({ kind: "project", id: "p1" }));
    expect(result.current.area).toBe("projects");
    expect(window.location.hash).toBe("#/projects/p1");
    expect(window.history.length).toBe(before + 1);

    act(() => result.current.unfocus());
    expect(result.current.route).toEqual({ area: "projects", focused: null });
    expect(window.location.hash).toBe("#/projects");

    act(() => result.current.open("settings"));
    expect(window.location.hash).toBe("#/settings");
    // Opening the area already showing is not a navigation.
    const settled = window.history.length;
    act(() => result.current.open("settings"));
    expect(window.history.length).toBe(settled);
  });

  it("follows a hash change (a typed or linked address)", async () => {
    const { result } = render();

    act(() => {
      window.location.hash = "#/projects/p9";
    });
    await waitFor(() =>
      expect(result.current.focused).toEqual({ kind: "project", id: "p9" }),
    );
  });

  it("Back returns to the previous area", async () => {
    const { result } = render();
    act(() => result.current.focus({ kind: "skill", id: "s1" }));
    act(() => result.current.open("explore"));
    expect(window.location.hash).toBe("#/explore");

    act(() => window.history.back());
    await waitFor(() =>
      expect(result.current.route).toEqual({
        area: "skills",
        focused: { kind: "skill", id: "s1" },
      }),
    );

    act(() => window.history.back());
    await waitFor(() => expect(result.current.route).toEqual({ area: "skills", focused: null }));
  });

  it("an unknown address is replaced by the current one", async () => {
    const { result } = render();
    act(() => result.current.open("projects"));

    act(() => {
      window.location.hash = "#/nowhere";
    });
    await waitFor(() => expect(window.location.hash).toBe("#/projects"));
    expect(result.current.area).toBe("projects");
  });

  it("keeps an Explore preview as the focused subject without addressing it", async () => {
    const { result } = render();
    act(() => result.current.focus({ kind: "explore", ref: preview }));
    expect(result.current.focused).toEqual({ kind: "explore", ref: preview });
    expect(window.location.hash).toBe("#/explore");

    // A hash event describing the current place is not a navigation.
    act(() => {
      window.dispatchEvent(new HashChangeEvent("hashchange"));
    });
    expect(result.current.focused).toEqual({ kind: "explore", ref: preview });

    act(() => result.current.unfocus());
    expect(result.current.route).toEqual({ area: "explore", focused: null });
  });
});

describe("catalog reconciliation", () => {
  it("clears a focused skill that left the catalog, replacing (not pushing) the address", () => {
    const { result } = render();
    act(() => result.current.focus({ kind: "skill", id: "gone" }));
    act(() => result.current.select("skills", ["gone", "kept"]));
    act(() => result.current.openDialog({ kind: "deleteSkill", skillIds: ["gone"] }));
    const before = window.history.length;

    act(() => result.current.reconcile("skill", ["kept"]));

    expect(result.current.route).toEqual({ area: "skills", focused: null });
    expect(window.location.hash).toBe("#/skills");
    expect(window.history.length).toBe(before);
    expect([...result.current.selection.skills]).toEqual(["kept"]);
    // A dialog about the skill goes with it.
    expect(result.current.stack).toEqual([]);
  });

  it("keeps a subject that is still in the catalog, and ignores other kinds", () => {
    const { result } = render();
    act(() => result.current.focus({ kind: "project", id: "p1" }));
    act(() => result.current.openDialog({ kind: "projectRemove", projectId: "p1" }));

    act(() => result.current.reconcile("skill", []));
    act(() => result.current.reconcile("project", ["p1"]));
    expect(result.current.focused).toEqual({ kind: "project", id: "p1" });
    expect(result.current.top).toEqual({ kind: "projectRemove", projectId: "p1" });

    act(() => result.current.reconcile("project", []));
    expect(result.current.focused).toBeNull();
    expect(result.current.top).toBeNull();
  });

  it("a deep link survives the catalog's unloaded first render", () => {
    window.history.replaceState(null, "", "/#/skills/s1");
    const { result, rerender } = renderHook(
      ({ ids }: { ids: string[] | null }) => {
        const surface = useSurfaceState();
        useFocusReconciliation(surface, "skill", ids);
        return surface;
      },
      { initialProps: { ids: null as string[] | null } },
    );
    expect(result.current.focused).toEqual({ kind: "skill", id: "s1" });

    rerender({ ids: ["s1", "s2"] });
    expect(result.current.focused).toEqual({ kind: "skill", id: "s1" });

    rerender({ ids: ["s2"] });
    expect(result.current.focused).toBeNull();
  });

  it("a stale link to an id the loaded catalog never had clears itself", async () => {
    const { result } = renderHook(() => {
      const surface = useSurfaceState();
      useFocusReconciliation(surface, "project", ["p1"]);
      return surface;
    });
    act(() => {
      window.location.hash = "#/projects/nope";
    });
    await waitFor(() => expect(window.location.hash).toBe("#/projects"));
    expect(result.current.route).toEqual({ area: "projects", focused: null });

    act(() => result.current.focus({ kind: "project", id: "p1" }));
    expect(result.current.focused).toEqual({ kind: "project", id: "p1" });
  });
});

describe("selection", () => {
  it("is per surface: select, toggle and clear leave the others alone", () => {
    const { result } = render();
    act(() => result.current.select("skills", ["a", "b"]));
    act(() => result.current.toggle("matrix", "x"));
    act(() => result.current.toggle("skills", "a"));

    expect([...result.current.selection.skills]).toEqual(["b"]);
    expect([...result.current.selection.matrix]).toEqual(["x"]);
    expect(result.current.selection.explore.size).toBe(0);

    act(() => result.current.clearSelection("skills"));
    expect(result.current.selection.skills.size).toBe(0);
    expect([...result.current.selection.matrix]).toEqual(["x"]);
  });

  it("never enters the address", () => {
    const { result } = render();
    act(() => result.current.select("skills", ["a"]));
    act(() => result.current.openDialog({ kind: "addSkill" }));
    expect(window.location.hash).toBe("#/skills");
  });
});

describe("dialog stack", () => {
  it("shows one dialog at a time: opening replaces", () => {
    const { result } = render();
    act(() => result.current.openDialog({ kind: "addSkill" }));
    act(() => result.current.openDialog({ kind: "deleteSkill", skillIds: ["s1"] }));

    expect(result.current.stack).toEqual([{ kind: "deleteSkill", skillIds: ["s1"] }]);
    expect(result.current.isOpen("addSkill")).toBe(false);
    expect(result.current.layer("deleteSkill")).toEqual({ kind: "deleteSkill", skillIds: ["s1"] });
  });

  it("updates a dialog in place: the Delete confirmation becomes its report, layers above kept (ticket 13)", () => {
    const { result } = render();
    act(() => result.current.openDialog({ kind: "deleteSkill", skillIds: ["s1", "s2"] }));
    act(() => result.current.pushDialog({ kind: "notifications" }));
    const report = [{ skillId: "s1", name: "alpha", error: { code: "OTHER", message: "x" } }];
    act(() => result.current.updateDialog({ kind: "deleteSkill", skillIds: ["s1", "s2"], report }));
    expect(result.current.stack).toEqual([
      { kind: "deleteSkill", skillIds: ["s1", "s2"], report },
      { kind: "notifications" },
    ]);
    // Nothing of the kind open: nothing changes.
    act(() => result.current.updateDialog({ kind: "confirmCount", verb: "disable", skillIds: ["s1"] }));
    expect(result.current.stack.map((layer) => layer.kind)).toEqual(["deleteSkill", "notifications"]);
  });

  it("a dialog about several skills stays while any of them does", () => {
    const { result } = render();
    act(() => result.current.openDialog({ kind: "confirmCount", verb: "disable", skillIds: ["a", "b"] }));
    act(() => result.current.reconcile("skill", ["b"]));
    expect(result.current.isOpen("confirmCount")).toBe(true);
    act(() => result.current.reconcile("skill", []));
    expect(result.current.isOpen("confirmCount")).toBe(false);
  });

  it("layers a nested step and closes only the top layer", () => {
    const { result } = render();
    act(() => result.current.openDialog({ kind: "addSkill" }));
    act(() => result.current.pushDialog({ kind: "gitPick" }));

    // An operator close aimed at a lower layer (Escape reaching both) is ignored.
    act(() => result.current.closeDialog("addSkill"));
    expect(result.current.stack.map((l) => l.kind)).toEqual(["addSkill", "gitPick"]);

    act(() => result.current.closeDialog("gitPick"));
    expect(result.current.stack.map((l) => l.kind)).toEqual(["addSkill"]);

    act(() => result.current.closeDialog("addSkill"));
    expect(result.current.top).toBeNull();
  });

  it("a flow dismisses its own dialog wherever it sits", () => {
    const { result } = render();
    act(() => result.current.openDialog({ kind: "addSkill" }));
    act(() => result.current.pushDialog({ kind: "gitPick" }));

    act(() => result.current.dismissDialog("addSkill"));
    expect(result.current.stack.map((l) => l.kind)).toEqual(["gitPick"]);
    act(() => result.current.dismissDialog("toolConfig"));
    expect(result.current.stack.map((l) => l.kind)).toEqual(["gitPick"]);
  });

  it("an offered dialog waits until no dialog is open", () => {
    const { result } = render();
    act(() => result.current.offerDialog({ kind: "newTools" }));
    act(() => result.current.offerDialog({ kind: "update" }));
    expect(result.current.stack.map((l) => l.kind)).toEqual(["newTools"]);
    expect(result.current.isOpen("update")).toBe(false);

    act(() => result.current.closeDialog("newTools"));
    expect(result.current.stack.map((l) => l.kind)).toEqual(["update"]);

    act(() => result.current.offerDialog({ kind: "newTools" }));
    act(() => result.current.withdrawDialog("newTools"));
    act(() => result.current.closeDialog("update"));
    expect(result.current.top).toBeNull();
  });
});

describe("asks", () => {
  it("layers above the open dialog and settles once with the operator's answer", async () => {
    const { result } = render();
    act(() => result.current.openDialog({ kind: "addSkill" }));
    let answer!: Promise<AskAnswer>;
    act(() => {
      answer = result.current.ask({ kind: "sharedDir", toolKey: "claude", toolLabel: "Claude", labels: ["Pi"] });
    });
    const pending = result.current.pendingAsk("sharedDir");
    expect(pending).toMatchObject({ toolKey: "claude", labels: ["Pi"] });
    expect(result.current.stack.map((l) => l.kind)).toEqual(["addSkill", "sharedDir"]);

    act(() => {
      pending!.answer("confirmed");
      pending!.answer("declined");
    });
    await expect(answer).resolves.toBe("confirmed");
    expect(result.current.stack.map((l) => l.kind)).toEqual(["addSkill"]);
  });

  it("an explicit decline is `declined`, never a dismissal", async () => {
    const { result } = render();
    let answer!: Promise<AskAnswerOf<"overwrite">>;
    act(() => {
      answer = result.current.ask({ kind: "overwrite", rows: [row("s1")] });
    });
    act(() => result.current.pendingAsk("overwrite")!.answer("declined"));
    await expect(answer).resolves.toBe("declined");
    expect(result.current.top).toBeNull();
  });

  it("a newer ask of the same kind dismisses the displaced one, whose answer goes stale", async () => {
    const { result } = render();
    let first!: Promise<AskAnswerOf<"overwrite">>;
    let second!: Promise<AskAnswerOf<"overwrite">>;
    act(() => {
      first = result.current.ask({ kind: "overwrite", rows: [row("s1")] });
    });
    const displaced = result.current.pendingAsk("overwrite")!;
    act(() => {
      second = result.current.ask({ kind: "overwrite", rows: [row("s2")] });
    });
    await expect(first).resolves.toBe("dismissed");
    expect(result.current.pendingAsk("overwrite")?.rows).toEqual([row("s2")]);

    act(() => displaced.answer("confirmed"));
    expect(result.current.pendingAsk("overwrite")?.rows).toEqual([row("s2")]);

    act(() => result.current.pendingAsk("overwrite")!.answer("confirmed"));
    await expect(second).resolves.toBe("confirmed");
    expect(result.current.top).toBeNull();
  });

  it("an ask closed by the operator (Escape) or dropped with its stack is dismissed", async () => {
    const { result } = render();
    let dropped!: Promise<AskAnswerOf<"overwrite">>;
    act(() => {
      dropped = result.current.ask({ kind: "overwrite", rows: [row("s1")] });
    });
    act(() => result.current.openDialog({ kind: "addSkill" }));
    await expect(dropped).resolves.toBe("dismissed");

    let closed!: Promise<AskAnswerOf<"overwrite">>;
    act(() => {
      closed = result.current.ask({ kind: "overwrite", rows: [row("s1")] });
    });
    act(() => result.current.closeDialog("overwrite"));
    await expect(closed).resolves.toBe("dismissed");
    expect(result.current.stack.map((l) => l.kind)).toEqual(["addSkill"]);
  });

  it("the overwrite ask can answer per row, in row order", async () => {
    const { result } = render();
    let answer!: Promise<AskAnswerOf<"overwrite">>;
    act(() => {
      answer = result.current.ask({ kind: "overwrite", rows: [row("s1"), row("s2")] });
    });
    act(() =>
      result.current.pendingAsk("overwrite")!.answer({ choices: ["keep", "overwrite"] }),
    );
    await expect(answer).resolves.toEqual({ choices: ["keep", "overwrite"] });
    expect(result.current.top).toBeNull();
  });

  it("unmounting dismisses a pending ask", async () => {
    const { result, unmount } = render();
    let answer!: Promise<AskAnswerOf<"overwrite">>;
    act(() => {
      answer = result.current.ask({ kind: "overwrite", rows: [row("s1")] });
    });
    unmount();
    await expect(answer).resolves.toBe("dismissed");
  });
});
