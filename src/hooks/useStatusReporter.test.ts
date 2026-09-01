// Tests at the StatusReporter seam: the interface every world hook reports
// through. Side channels are mocked at their module seams — sonner (toast
// one-shots render there) and src/lib/tauri.ts (cancelLoading fires the
// backend cancel command through it).

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
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

describe("useStatusReporter", () => {
  it("renders a success toast once, then auto-clears the trigger", async () => {
    const { result } = renderHook(() => useStatusReporter(t));

    act(() => {
      result.current.setSuccessToastMessage("done!");
    });

    expect(toast.success).toHaveBeenCalledWith("done!", { duration: 1800 });
    // The one-shot flag clears in a microtask; a second render must not
    // re-fire the toast.
    await waitFor(() => expect(toast.success).toHaveBeenCalledTimes(1));
  });

  it("renders an error toast and clears the action message with it", async () => {
    const { result } = renderHook(() => useStatusReporter(t));

    act(() => {
      result.current.setActionMessage("syncing…");
      result.current.setError("it broke");
    });

    expect(toast.error).toHaveBeenCalledWith("it broke", { duration: 2600 });
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

    expect(toast.error).toHaveBeenCalledWith(
      'skill-b\nb failed' + 'errors.moreCount {"count":1}',
      { duration: 3200 },
    );
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

    expect(invokeTauri).toHaveBeenCalledWith("cancel_current_operation");
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
      expect(toast.success).toHaveBeenCalledWith("done!", { duration: 1800 }),
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
      expect(toast.success).toHaveBeenCalledWith("removed 3", {
        duration: 1800,
      }),
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
      expect(toast.error).toHaveBeenCalledWith("errors.targetExists", {
        duration: 2600,
      }),
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
      expect(toast.error).toHaveBeenCalledWith("name taken", {
        duration: 2600,
      }),
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
