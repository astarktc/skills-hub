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

  it("cancelLoading fires the backend cancel and resets the loading surface", () => {
    const { result } = renderHook(() => useStatusReporter(t));

    act(() => {
      result.current.setLoading(true);
      result.current.setLoadingStartAt(123);
      result.current.setActionMessage("working…");
    });
    act(() => {
      result.current.cancelLoading();
    });

    expect(invokeTauri).toHaveBeenCalledWith("cancel_current_operation");
    expect(result.current.loading).toBe(false);
    expect(result.current.loadingStartAt).toBeNull();
    expect(result.current.actionMessage).toBeNull();
  });

  it("formatError localizes via describeCommandError and silences CANCELLED", () => {
    const { result } = renderHook(() => useStatusReporter(t));

    expect(result.current.formatError({ code: "TARGET_EXISTS" })).toBe(
      "errors.targetExists",
    );
    expect(result.current.formatError({ code: "CANCELLED" })).toBeNull();
  });
});
