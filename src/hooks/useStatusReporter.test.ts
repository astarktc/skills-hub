// Tests at the StatusReporter seam: the interface every world hook reports
// through. Side channels are mocked at their module seams — sonner (toast
// one-shots render there) and src/lib/tauri.ts (cancelLoading fires the
// backend cancel command through it).

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
    info: vi.fn(),
  },
}));
vi.mock("../lib/tauri", () => ({
  isTauri: true,
  invokeTauri: vi.fn().mockResolvedValue(undefined),
}));

import { toast } from "sonner";
import { invokeTauri } from "../lib/tauri";
import { useStatusReporter } from "./useStatusReporter";

const t = (key: string, opts?: Record<string, unknown>) =>
  opts ? `${key} ${JSON.stringify(opts)}` : key;

beforeEach(() => {
  vi.clearAllMocks();
});

// The lifetimes the reporter owns (spec Q5): errors never auto-dismiss,
// successes flash. Warning/info are asserted in the `notify` block.
const ERROR_OPTIONS = {
  description: undefined,
  duration: Infinity,
  closeButton: true,
};
const SUCCESS_OPTIONS = { description: undefined, duration: 2000 };

describe("useStatusReporter", () => {
  it("renders a success toast once, then auto-clears the trigger", async () => {
    const { result } = renderHook(() => useStatusReporter(t));

    act(() => {
      result.current.setSuccessToastMessage("done!");
    });

    expect(toast.success).toHaveBeenCalledWith("done!", SUCCESS_OPTIONS);
    // The one-shot flag clears in a microtask; a second render must not
    // re-fire the toast.
    await waitFor(() => expect(toast.success).toHaveBeenCalledTimes(1));
  });

  // Toast lifetime is owned here and nowhere else: an error stays until the
  // operator closes it, a warning lingers, a success/info flashes.
  describe("notify", () => {
    it("an error stays on screen until closed, with its message as description", () => {
      const { result } = renderHook(() => useStatusReporter(t));

      act(() => {
        result.current.notify("error", "Install failed", "disk full");
      });

      expect(toast.error).toHaveBeenCalledWith("Install failed", {
        description: "disk full",
        duration: Infinity,
        closeButton: true,
      });
    });

    it("a warning lingers 5 s", () => {
      const { result } = renderHook(() => useStatusReporter(t));

      act(() => {
        result.current.notify("warning", "Skipped", "already present");
      });

      expect(toast.warning).toHaveBeenCalledWith("Skipped", {
        description: "already present",
        duration: 5000,
      });
    });

    it("a success flashes 2 s", () => {
      const { result } = renderHook(() => useStatusReporter(t));

      act(() => {
        result.current.notify("success", "Installed");
      });

      expect(toast.success).toHaveBeenCalledWith("Installed", {
        description: undefined,
        duration: 2000,
      });
    });

    it("an info flashes 2 s", () => {
      const { result } = renderHook(() => useStatusReporter(t));

      act(() => {
        result.current.notify("info", "Checking for updates");
      });

      expect(toast.info).toHaveBeenCalledWith("Checking for updates", {
        description: undefined,
        duration: 2000,
      });
    });
  });

  // The session's notification history (spec Q3/Q4): every notify() both
  // toasts and records; the ring is bounded; only errors and warnings count
  // as unread until the operator opens the panel.
  describe("notification history", () => {
    it("notify records an entry newest first and toasts it", () => {
      const before = Date.now();
      const { result } = renderHook(() => useStatusReporter(t));

      act(() => {
        result.current.notify("success", "Installed", "skill-a");
        result.current.notify("error", "Install failed", "disk full");
      });

      expect(toast.success).toHaveBeenCalledTimes(1);
      expect(toast.error).toHaveBeenCalledTimes(1);
      expect(result.current.notifications).toHaveLength(2);
      const [newest, oldest] = result.current.notifications;
      expect(newest).toMatchObject({
        kind: "error",
        title: "Install failed",
        message: "disk full",
      });
      expect(oldest).toMatchObject({ kind: "success", title: "Installed" });
      expect(newest.at).toBeGreaterThanOrEqual(before);
      expect(newest.id).not.toBe(oldest.id);
    });

    it("the setters land in the history too (setError, showActionErrors)", () => {
      const { result } = renderHook(() => useStatusReporter(t));

      act(() => {
        result.current.setError("it broke");
        result.current.showActionErrors([{ title: "skill-b", message: "b failed" }]);
      });

      // setError is an effect-rendered trigger, so it lands after the
      // synchronous showActionErrors; only presence is asserted.
      expect(result.current.notifications.map((n) => n.title).sort()).toEqual(
        ["it broke", "skill-b"],
      );
      expect(result.current.unreadCount).toBe(2);
    });

    it("keeps only the last 100 entries, dropping the oldest", () => {
      const { result } = renderHook(() => useStatusReporter(t));

      act(() => {
        for (let i = 1; i <= 101; i++) {
          result.current.notify("info", `n${i}`);
        }
      });

      expect(result.current.notifications).toHaveLength(100);
      expect(result.current.notifications[0].title).toBe("n101");
      expect(result.current.notifications[99].title).toBe("n2");
      expect(
        result.current.notifications.some((n) => n.title === "n1"),
      ).toBe(false);
    });

    it("counts only errors and warnings as unread", () => {
      const { result } = renderHook(() => useStatusReporter(t));
      expect(result.current.unreadCount).toBe(0);

      act(() => {
        result.current.notify("error", "e");
        result.current.notify("warning", "w");
        result.current.notify("success", "s");
        result.current.notify("info", "i");
      });

      expect(result.current.unreadCount).toBe(2);
    });

    it("markAllRead zeroes the unread count; later entries count again", () => {
      const { result } = renderHook(() => useStatusReporter(t));

      act(() => {
        result.current.notify("error", "e1");
        result.current.notify("warning", "w1");
      });
      act(() => {
        result.current.markAllRead();
      });

      expect(result.current.unreadCount).toBe(0);
      // Reading does not discard: the entries stay listed.
      expect(result.current.notifications).toHaveLength(2);

      act(() => {
        result.current.notify("error", "e2");
      });

      expect(result.current.unreadCount).toBe(1);
    });

    it("clearNotifications empties the history and the unread count", () => {
      const { result } = renderHook(() => useStatusReporter(t));

      act(() => {
        result.current.notify("error", "e1");
        result.current.notify("success", "s1");
      });
      act(() => {
        result.current.clearNotifications();
      });

      expect(result.current.notifications).toEqual([]);
      expect(result.current.unreadCount).toBe(0);
    });
  });

  it("renders an error toast and clears the action message with it", async () => {
    const { result } = renderHook(() => useStatusReporter(t));

    act(() => {
      result.current.setActionMessage("syncing…");
      result.current.setError("it broke");
    });

    expect(toast.error).toHaveBeenCalledWith("it broke", ERROR_OPTIONS);
    await waitFor(() => expect(result.current.actionMessage).toBeNull());
  });

  it("showActionErrors silences empty messages and counts the rest", () => {
    const { result } = renderHook(() => useStatusReporter(t));

    act(() => {
      result.current.showActionErrors([
        { title: "skill-a", message: "" }, // silenced (e.g. cancelled)
        { title: "skill-b", message: "b failed" },
        { title: "skill-c", message: "c failed" },
      ]);
    });

    expect(toast.error).toHaveBeenCalledWith("skill-b", {
      ...ERROR_OPTIONS,
      description: 'b failed' + 'errors.moreCount {"count":1}',
    });
  });

  // A batch with many per-target failures (Refresh all) must stay readable:
  // the screen gets one toast, the history gets every entry.
  it("showActionErrors toasts once but records every visible entry as its own error", () => {
    const { result } = renderHook(() => useStatusReporter(t));

    act(() => {
      result.current.showActionErrors([
        { title: "skill-a", message: "a failed" },
        { title: "skill-b", message: "b failed" },
        { title: "skill-c", message: "c failed" },
      ]);
    });

    expect(toast.error).toHaveBeenCalledTimes(1);
    expect(toast.error).toHaveBeenCalledWith("skill-a", {
      ...ERROR_OPTIONS,
      description: 'a failed' + 'errors.moreCount {"count":2}',
    });
    // Newest first, and the head's row carries the plain message: the
    // "+N more" belongs to the toast, the panel lists the N themselves.
    expect(
      result.current.notifications.map((n) => [n.kind, n.title, n.message]),
    ).toEqual([
      ["error", "skill-c", "c failed"],
      ["error", "skill-b", "b failed"],
      ["error", "skill-a", "a failed"],
    ]);
    expect(result.current.unreadCount).toBe(3);
  });

  it("showActionErrors with only silenced entries shows nothing", () => {
    const { result } = renderHook(() => useStatusReporter(t));

    act(() => {
      result.current.showActionErrors([{ title: "skill-a", message: "" }]);
    });

    expect(toast.error).not.toHaveBeenCalled();
  });

  it("cancelLoading fires the backend cancel and resets the loading surface", async () => {
    const { result } = renderHook(() => useStatusReporter(t));
    let finish: () => void = () => {};
    const pending = new Promise<void>((resolve) => {
      finish = resolve;
    });

    let run: Promise<unknown> = Promise.resolve();
    act(() => {
      run = result.current.runAction({ message: "working…" }, () => pending);
    });
    expect(result.current.loading).toBe(true);

    act(() => {
      result.current.cancelLoading();
    });

    expect(invokeTauri).toHaveBeenCalledWith("cancelCurrentOperation");
    expect(result.current.loading).toBe(false);
    expect(result.current.loadingStartAt).toBeNull();
    expect(result.current.actionMessage).toBeNull();

    await act(async () => {
      finish();
      await run;
    });
  });

  it("formatError localizes via describeCommandError and silences CANCELLED", () => {
    const { result } = renderHook(() => useStatusReporter(t));

    expect(result.current.formatError({ code: "TARGET_EXISTS" })).toBe(
      "errors.targetExists",
    );
    expect(result.current.formatError({ code: "CANCELLED" })).toBeNull();
  });
});

// The action lifecycle invariant lives in exactly one place — runAction — so
// it is tested exactly once, here, through the reporter's own interface.
describe("useStatusReporter runAction", () => {
  /** A body whose completion the test controls. */
  function deferred<T>() {
    let resolve: (value: T) => void = () => {};
    let reject: (err: unknown) => void = () => {};
    const promise = new Promise<T>((res, rej) => {
      resolve = res;
      reject = rej;
    });
    return { promise, resolve, reject };
  }

  it("opens the loading surface with the message, then resets it on success and toasts", async () => {
    const { result } = renderHook(() => useStatusReporter(t));
    const body = deferred<number>();

    let run: Promise<number | undefined> = Promise.resolve(undefined);
    act(() => {
      run = result.current.runAction(
        { message: "working…", successToast: "done!" },
        () => body.promise,
      );
    });

    expect(result.current.loading).toBe(true);
    expect(result.current.loadingStartAt).toEqual(expect.any(Number));
    expect(result.current.actionMessage).toBe("working…");
    expect(toast.success).not.toHaveBeenCalled();

    let value: number | undefined;
    await act(async () => {
      body.resolve(42);
      value = await run;
    });

    expect(value).toBe(42);
    expect(result.current.loading).toBe(false);
    expect(result.current.loadingStartAt).toBeNull();
    expect(result.current.actionMessage).toBeNull();
    await waitFor(() =>
      expect(toast.success).toHaveBeenCalledWith("done!", SUCCESS_OPTIONS),
    );
    expect(toast.error).not.toHaveBeenCalled();
  });

  it("a successToast function receives the body's value", async () => {
    const { result } = renderHook(() => useStatusReporter(t));

    await act(async () => {
      await result.current.runAction(
        { successToast: (count: number) => `removed ${count}` },
        async () => 3,
      );
    });

    await waitFor(() =>
      expect(toast.success).toHaveBeenCalledWith("removed 3", SUCCESS_OPTIONS),
    );
  });

  it("without a successToast, completes silently", async () => {
    const { result } = renderHook(() => useStatusReporter(t));

    await act(async () => {
      await result.current.runAction({}, async () => undefined);
    });

    expect(result.current.loading).toBe(false);
    expect(toast.success).not.toHaveBeenCalled();
    expect(toast.error).not.toHaveBeenCalled();
  });

  it("routes a thrown failure through formatError to the error toast and still resets", async () => {
    const { result } = renderHook(() => useStatusReporter(t));

    let value: unknown = "unset";
    await act(async () => {
      value = await result.current.runAction(
        { message: "working…", successToast: "done!" },
        async () => {
          throw { code: "TARGET_EXISTS" };
        },
      );
    });

    expect(value).toBeUndefined();
    expect(result.current.loading).toBe(false);
    expect(result.current.loadingStartAt).toBeNull();
    await waitFor(() =>
      expect(toast.error).toHaveBeenCalledWith(
        "errors.targetExists",
        ERROR_OPTIONS,
      ),
    );
    expect(toast.success).not.toHaveBeenCalled();
    await waitFor(() => expect(result.current.actionMessage).toBeNull());
  });

  it("a cancelled command resets silently: no error, no success toast", async () => {
    const { result } = renderHook(() => useStatusReporter(t));

    await act(async () => {
      await result.current.runAction({ successToast: "done!" }, async () => {
        throw { code: "CANCELLED" };
      });
    });

    expect(result.current.loading).toBe(false);
    expect(result.current.loadingStartAt).toBeNull();
    expect(result.current.actionMessage).toBeNull();
    expect(toast.error).not.toHaveBeenCalled();
    expect(toast.success).not.toHaveBeenCalled();
  });

  it("handOff ends the action with neither toast nor error (another surface takes over)", async () => {
    const { result } = renderHook(() => useStatusReporter(t));

    let value: unknown = "unset";
    await act(async () => {
      value = await result.current.runAction(
        { message: "scanning…", successToast: "installed!" },
        async (action) => action.handOff(),
      );
    });

    expect(value).toBeUndefined();
    expect(result.current.loading).toBe(false);
    expect(result.current.loadingStartAt).toBeNull();
    expect(result.current.actionMessage).toBeNull();
    expect(toast.success).not.toHaveBeenCalled();
    expect(toast.error).not.toHaveBeenCalled();
  });

  it("fail ends the action with the given message and no success toast", async () => {
    const { result } = renderHook(() => useStatusReporter(t));

    let value: unknown = "unset";
    await act(async () => {
      value = await result.current.runAction(
        { successToast: "installed!" },
        async (action) => action.fail("name taken"),
      );
    });

    expect(value).toBeUndefined();
    expect(result.current.loading).toBe(false);
    await waitFor(() =>
      expect(toast.error).toHaveBeenCalledWith("name taken", ERROR_OPTIONS),
    );
    expect(toast.success).not.toHaveBeenCalled();
  });

  it("fail(null) is the silent-cancel contract: resets without any toast", async () => {
    const { result } = renderHook(() => useStatusReporter(t));

    await act(async () => {
      await result.current.runAction({ successToast: "x" }, async (action) =>
        action.fail(null),
      );
    });

    expect(result.current.loading).toBe(false);
    expect(toast.error).not.toHaveBeenCalled();
    expect(toast.success).not.toHaveBeenCalled();
  });

  it("starting an action clears a pending error so it cannot re-fire", async () => {
    const { result } = renderHook(() => useStatusReporter(t));

    act(() => {
      result.current.setError("stale");
    });
    expect(toast.error).toHaveBeenCalledTimes(1);

    await act(async () => {
      await result.current.runAction({}, async () => undefined);
    });

    expect(toast.error).toHaveBeenCalledTimes(1);
  });
});
