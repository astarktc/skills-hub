// Tests at the SkillLibrary seam: the managed-skill catalog and which skill
// each of its dialogs is about. Every verb on a skill is the action door's
// (useSkillActions.test.ts); here the reporter is a stub and the backend is
// the invokeTauri module seam — the shape App.tsx wires.

import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ManagedSkill } from "../components/skills/types";
import type { SkillLibraryDeps } from "./useSkillLibrary";
import type { StatusReporter } from "./useStatusReporter";

vi.mock("../lib/tauri", () => ({
  isTauri: true,
  invokeTauri: vi.fn(),
}));
const pickFolder = vi.fn<(options: { title: string; multiple?: boolean }) => Promise<string[]>>();
vi.mock("../lib/platform", () => ({
  createProgressChannel: async (onMessage: (message: unknown) => void) => ({ onmessage: onMessage }),
  pickFolder: (options: { title: string; multiple?: boolean }) => pickFolder(options),
}));

import { invokeTauri, type CommandName } from "../lib/tauri";
import { useSkillLibrary as useSkillLibraryHook } from "./useSkillLibrary";
import { useFocusReconciliation, useSurfaceState } from "./useSurfaceState";

const mockInvoke = vi.mocked(
  invokeTauri as unknown as (command: CommandName, ...args: unknown[]) => Promise<unknown>,
);

const t = (key: string, opts?: Record<string, unknown>) =>
  opts ? `${key} ${JSON.stringify(opts)}` : key;

function skill(id: string, name: string): ManagedSkill {
  return {
    id,
    name,
    description: null,
    source_type: "local",
    source_ref: null,
    imported_from_tool: null,
    central_path: `/hub/${name}`,
    created_at: 0,
    updated_at: 0,
    last_sync_at: null,
    status: "active",
    invocation_mode: "user-and-model",
    invocation_override: null,
    targets: [],
    disabled_at: null,
    refreshable: true,
    unlocatable: null,
    detachable: false,
    health: null,
  };
}

/** The library as App composes it, with the surface's focus read out. */
function useSkillLibrary(deps: Omit<SkillLibraryDeps, "surface">) {
  const surface = useSurfaceState();
  const library = useSkillLibraryHook({ ...deps, surface });
  useFocusReconciliation(surface, "skill", library.managedSkillIds);
  const { focused } = surface;
  return {
    ...library,
    surface,
    detailSkill:
      focused?.kind === "skill"
        ? (library.managedSkills.find((row) => row.id === focused.id) ?? null)
        : null,
  };
}

function makeDeps(skills: ManagedSkill[] = [skill("s1", "alpha")]) {
  mockInvoke.mockImplementation((command) =>
    Promise.resolve(command === "getManagedSkills" ? skills : null),
  );
  const reporter = {
    loading: false,
    notify: vi.fn(),
    notifyError: vi.fn(),
    // The lifecycle stand-in: run the body once, answer its value.
    runAction: vi.fn(async (_opts: unknown, body: (action: unknown) => Promise<unknown>) =>
      body({}),
    ) as unknown as StatusReporter["runAction"],
  };
  return { deps: { t, reporter }, reporter, skills };
}

async function renderLibrary(setup: ReturnType<typeof makeDeps>) {
  const rendered = renderHook(() => useSkillLibrary(setup.deps));
  await waitFor(() => expect(rendered.result.current.managedSkills).toEqual(setup.skills));
  return rendered;
}

beforeEach(() => {
  vi.clearAllMocks();
});
afterEach(cleanup);

describe("the catalog", () => {
  it("is null until the first load, then the backend's list; a failed read is an error Notification", async () => {
    const setup = makeDeps();
    const { result } = await renderLibrary(setup);
    expect(result.current.managedSkillIds).toEqual(["s1"]);
    const failure = { code: "OTHER", message: "db" };
    mockInvoke.mockRejectedValueOnce(failure);
    await act(async () => {
      await result.current.loadManagedSkills();
    });
    expect(setup.reporter.notifyError).toHaveBeenCalledWith(failure);
    expect(result.current.managedSkills).toEqual(setup.skills);
  });

  it("replaceCatalog swaps the whole list without a read", async () => {
    const setup = makeDeps();
    const { result } = await renderLibrary(setup);
    mockInvoke.mockClear();
    const next = [skill("s2", "beta")];
    act(() => result.current.replaceCatalog(next));
    expect(result.current.managedSkills).toEqual(next);
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("the focused row follows the catalog and clears when gone", async () => {
    const setup = makeDeps();
    const { result } = await renderLibrary(setup);
    act(() => result.current.surface.focus({ kind: "skill", id: "s1" }));
    expect(result.current.detailSkill).toEqual(setup.skills[0]);
    const updated = { ...setup.skills[0], description: "updated" };
    act(() => result.current.replaceCatalog([updated]));
    expect(result.current.detailSkill).toEqual(updated);
    act(() => result.current.replaceCatalog([]));
    expect(result.current.detailSkill).toBeNull();
  });

  it("isSkillNameTaken matches case-insensitively", async () => {
    const setup = makeDeps([skill("s1", "Alpha")]);
    const { result } = await renderLibrary(setup);
    expect(result.current.isSkillNameTaken("alpha")).toBe(true);
    expect(result.current.isSkillNameTaken("ALPHA")).toBe(true);
    expect(result.current.isSkillNameTaken("beta")).toBe(false);
  });
});

describe("dialog subjects", () => {
  it("resolves each dialog's skill against the catalog", async () => {
    const setup = makeDeps();
    const { result } = await renderLibrary(setup);
    act(() => result.current.surface.openDialog({ kind: "deleteSkill", skillIds: ["s1", "gone"] }));
    expect(result.current.pendingDelete).toEqual({ skills: [setup.skills[0]], report: null });
    act(() => result.current.handleCloseDelete());
    expect(result.current.pendingDelete).toBeNull();
    act(() => result.current.surface.openDialog({ kind: "confirmCount", verb: "disable", skillIds: ["s1"] }));
    expect(result.current.countConfirm).toEqual({ verb: "disable", skills: [setup.skills[0]] });
    act(() => result.current.handleCloseCountConfirm());
    expect(result.current.countConfirm).toBeNull();
    act(() => result.current.surface.openDialog({ kind: "assignProject", skillIds: ["s1"] }));
    expect(result.current.assignProjectSkills).toEqual([setup.skills[0]]);
    act(() => result.current.surface.openDialog({ kind: "invocationEdit", skillId: "s1" }));
    expect(result.current.invocationEditSkill).toEqual(setup.skills[0]);
    act(() => result.current.closeInvocationEdit());
    expect(result.current.invocationEditSkill).toBeNull();
  });

  it("a dialog stays put while an action runs", async () => {
    const setup = makeDeps();
    const { result, rerender } = await renderLibrary(setup);
    act(() => result.current.surface.openDialog({ kind: "deleteSkill", skillIds: ["s1"] }));
    setup.deps.reporter.loading = true;
    rerender();
    act(() => result.current.handleCloseDelete());
    expect(result.current.pendingDelete?.skills).toEqual([setup.skills[0]]);
  });

  it("warns and withdraws Change source when its skill is gone", async () => {
    const setup = makeDeps();
    const { result } = await renderLibrary(setup);
    act(() => result.current.surface.openDialog({ kind: "repoint", skillId: "s1", name: "alpha", preselect: "git" }));
    expect(result.current.pendingRepointSkill).toEqual(setup.skills[0]);
    act(() => result.current.replaceCatalog([]));
    await waitFor(() =>
      expect(setup.reporter.notify).toHaveBeenCalledWith("warning", 'errors.skillGone {"name":"alpha"}'),
    );
    expect(result.current.repointSelection).toBeNull();
  });
});

describe("Change source's folder button", () => {
  async function openRepoint() {
    const setup = makeDeps();
    const rendered = await renderLibrary(setup);
    act(() =>
      rendered.result.current.surface.openDialog({ kind: "repoint", skillId: "s1", name: "alpha", preselect: "local" }),
    );
    return { setup, ...rendered };
  }

  it("answers the picked folder", async () => {
    const { result } = await openRepoint();
    pickFolder.mockResolvedValueOnce(["/new/alpha"]);
    let picked: string | null = null;
    await act(async () => {
      picked = await result.current.pickRepointFolder();
    });
    expect(picked).toBe("/new/alpha");
    expect(pickFolder).toHaveBeenCalledWith({ title: 'changeSource.selectFolderTitle {"name":"alpha"}' });
  });

  it("a cancelled picker answers null and reports nothing", async () => {
    const { result, setup } = await openRepoint();
    pickFolder.mockResolvedValueOnce([]);
    let picked: string | null = "unset";
    await act(async () => {
      picked = await result.current.pickRepointFolder();
    });
    expect(picked).toBeNull();
    expect(setup.reporter.notifyError).not.toHaveBeenCalled();
    expect(result.current.repointSelection).not.toBeNull();
  });

  it("a picker failure is an error Notification (the one failure dialect)", async () => {
    const { result, setup } = await openRepoint();
    const failure = { code: "OTHER", message: "dialog failed" };
    pickFolder.mockRejectedValueOnce(failure);
    let picked: string | null = "unset";
    await act(async () => {
      picked = await result.current.pickRepointFolder();
    });
    expect(picked).toBeNull();
    expect(setup.reporter.notifyError).toHaveBeenCalledWith(failure);
  });
});

describe("the Re-point preview (D15)", () => {
  it("is a cancellable read under the lifecycle, answering the resolved target", async () => {
    const setup = makeDeps();
    const preview = { resolved_name: "beta", ref: "main", subpath: "skills/b", name_matches: false, candidates: null };
    mockInvoke.mockImplementation((command) =>
      Promise.resolve(command === "getManagedSkills" ? setup.skills : command === "previewRepointSource" ? preview : null),
    );
    const { result } = await renderLibrary(setup);
    let answer: unknown;
    await act(async () => {
      answer = await result.current.previewRepoint("s1", { kind: "git", url: "https://github.com/o/r" });
    });
    expect(answer).toEqual({ kind: "preview", preview });
    expect(mockInvoke).toHaveBeenCalledWith("previewRepointSource", "s1", { kind: "git", url: "https://github.com/o/r" });
    expect(setup.reporter.runAction).toHaveBeenCalledWith(
      expect.objectContaining({ read: true, cancel: "previewRepointSource", subject: { kind: "skill", id: "s1" } }),
      expect.any(Function),
    );
  });

  it("answers a refusal as a typed value for the field, never a notification", async () => {
    const setup = makeDeps();
    const refusal = { code: "INVALID_GITHUB_URL", url: "owner/repo" };
    mockInvoke.mockImplementation((command) =>
      command === "previewRepointSource" ? Promise.reject(refusal) : Promise.resolve(command === "getManagedSkills" ? setup.skills : null),
    );
    const { result } = await renderLibrary(setup);
    let answer: unknown;
    await act(async () => {
      answer = await result.current.previewRepoint("s1", { kind: "git", url: "owner/repo" });
    });
    expect(answer).toEqual({ kind: "error", error: refusal });
    expect(setup.reporter.notify).not.toHaveBeenCalled();
    expect(setup.reporter.notifyError).not.toHaveBeenCalled();
  });
});
