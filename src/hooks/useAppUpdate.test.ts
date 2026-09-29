// Tests at the AppUpdate seam: the one updater lifecycle that Settings, the
// startup notice and (later) the title bar read. The updater crosses the
// platform seam (mocked here), the ignored version is a preference (mocked),
// and the notice is a surface dialog (the real surface module, as App
// composes it).

import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AppUpdateHandle } from "../lib/platform";
import type { StatusReporter } from "./useStatusReporter";

const checkForUpdate = vi.fn<() => Promise<AppUpdateHandle | null>>();
const appVersion = vi.fn<() => Promise<string>>();
vi.mock("../lib/platform", () => ({
  checkForUpdate: () => checkForUpdate(),
  appVersion: () => appVersion(),
}));
const ignored = { value: null as string | null };
vi.mock("../lib/preferences", () => ({
  ignoredUpdateVersionPreference: {
    read: () => ignored.value,
    write: (value: string) => {
      ignored.value = value;
    },
  },
}));

import { useAppUpdate } from "./useAppUpdate";
import { useSurfaceState } from "./useSurfaceState";

function makeReporter(): Pick<StatusReporter, "notify" | "formatError"> {
  return {
    notify: vi.fn(),
    formatError: vi.fn((err: unknown) => `formatted:${(err as Error).message}`),
  };
}

/** A found release whose install and notes the test controls. */
function release(version: string, overrides?: Partial<AppUpdateHandle>): AppUpdateHandle {
  return {
    version,
    downloadAndInstall: vi.fn(async () => {}),
    releaseNotes: vi.fn(async () => `notes for ${version}`),
    ...overrides,
  };
}

function renderUpdate(reporter = makeReporter()) {
  return renderHook(() => {
    const surface = useSurfaceState();
    const update = useAppUpdate({ reporter, surface });
    return { ...update, noticeOpen: surface.isOpen("update") };
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  ignored.value = null;
  appVersion.mockResolvedValue("1.2.18");
});
afterEach(cleanup);

describe("useAppUpdate startup check", () => {
  it("offers the notice for a found release and fetches its notes", async () => {
    checkForUpdate.mockResolvedValue(release("9.9.9"));
    const { result } = renderUpdate();
    await waitFor(() => expect(result.current.notes).toBe("notes for 9.9.9"));
    expect(result.current.status).toBe("available");
    expect(result.current.version).toBe("9.9.9");
    expect(result.current.noticeOpen).toBe(true);
    expect(result.current.currentVersion).toBe("1.2.18");
  });

  it("stays quiet when current, ignored, or failing", async () => {
    for (const answer of ["current", "ignored", "failing"] as const) {
      ignored.value = answer === "ignored" ? "9.9.9" : null;
      if (answer === "current") checkForUpdate.mockResolvedValueOnce(null);
      else if (answer === "ignored") checkForUpdate.mockResolvedValueOnce(release("9.9.9"));
      else checkForUpdate.mockRejectedValueOnce(new Error("offline"));
      const reporter = makeReporter();
      const { result, unmount } = renderUpdate(reporter);
      await waitFor(() => expect(checkForUpdate).toHaveBeenCalled());
      await waitFor(() => expect(result.current.status).toBe("idle"));
      expect(result.current.noticeOpen).toBe(false);
      expect(result.current.error).toBeNull();
      expect(reporter.notify).not.toHaveBeenCalled();
      unmount();
      checkForUpdate.mockClear();
    }
  });
});

describe("useAppUpdate — one state for Settings and the notice", () => {
  it("a release found at startup is what Settings shows, and dismissing the notice keeps it", async () => {
    checkForUpdate.mockResolvedValue(release("9.9.9"));
    const { result } = renderUpdate();
    await waitFor(() => expect(result.current.noticeOpen).toBe(true));
    act(() => result.current.dismiss());
    expect(result.current.noticeOpen).toBe(false);
    expect(result.current.status).toBe("available");
    expect(result.current.version).toBe("9.9.9");
    expect(ignored.value).toBeNull();
  });

  it("Settings' check reports up-to-date, a found release (even an ignored one) without the notice, or an error", async () => {
    checkForUpdate.mockResolvedValueOnce(null);
    const { result } = renderUpdate();
    await waitFor(() => expect(checkForUpdate).toHaveBeenCalledTimes(1));

    checkForUpdate.mockResolvedValueOnce(null);
    await act(async () => {
      await result.current.check();
    });
    expect(result.current.status).toBe("up-to-date");

    ignored.value = "9.9.9";
    checkForUpdate.mockResolvedValueOnce(release("9.9.9"));
    await act(async () => {
      await result.current.check();
    });
    expect(result.current.status).toBe("available");
    expect(result.current.version).toBe("9.9.9");
    expect(result.current.noticeOpen).toBe(false);

    checkForUpdate.mockRejectedValueOnce(new Error("offline"));
    await act(async () => {
      await result.current.check();
    });
    expect(result.current.status).toBe("error");
    expect(result.current.error).toBe("formatted:offline");
  });

  it("installing from either place drives the one status to done", async () => {
    const found = release("9.9.9");
    checkForUpdate.mockResolvedValue(found);
    const { result } = renderUpdate();
    await waitFor(() => expect(result.current.status).toBe("available"));
    let installing!: Promise<void>;
    act(() => {
      installing = result.current.install();
    });
    expect(result.current.status).toBe("installing");
    await act(async () => {
      await installing;
    });
    expect(found.downloadAndInstall).toHaveBeenCalledTimes(1);
    expect(result.current.status).toBe("done");
  });

  it("a failed install keeps the release to retry, says why, and lands in the history", async () => {
    checkForUpdate.mockResolvedValue(
      release("9.9.9", { downloadAndInstall: vi.fn(async () => { throw new Error("disk full"); }) }),
    );
    const reporter = makeReporter();
    const { result } = renderUpdate(reporter);
    await waitFor(() => expect(result.current.status).toBe("available"));
    await act(async () => {
      await result.current.install();
    });
    expect(result.current.status).toBe("available");
    expect(result.current.error).toBe("formatted:disk full");
    expect(reporter.notify).toHaveBeenCalledWith("error", "formatted:disk full");
  });

  it("Don't remind me records the version and closes the notice", async () => {
    checkForUpdate.mockResolvedValue(release("9.9.9"));
    const { result } = renderUpdate();
    await waitFor(() => expect(result.current.noticeOpen).toBe(true));
    act(() => result.current.ignore());
    expect(ignored.value).toBe("9.9.9");
    expect(result.current.noticeOpen).toBe(false);
  });
});
