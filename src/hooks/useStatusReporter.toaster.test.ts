// The reporter ↔ toaster seam (ticket 14 audit A1): "View report" moves focus
// from a toast into the Notifications panel. The real sonner is mounted here —
// its focus return (to the element focused before the toast, when focus
// leaves its toaster) is exactly what competed with the panel's initial
// focus — and the surface door stands in for the Dialog primitive: opening
// the panel focuses its initial control, and the page behind turns inert.

import { act, fireEvent, render, renderHook, waitFor } from "@testing-library/react";
import { createElement } from "react";
import { Toaster } from "sonner";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { useStatusReporter } from "./useStatusReporter";

vi.mock("../lib/tauri", () => ({
  isTauri: true,
  invokeTauri: vi.fn().mockResolvedValue(undefined),
}));

const t = (key: string, opts?: Record<string, unknown>) => (opts ? `${key} ${JSON.stringify(opts)}` : key);

beforeAll(() => {
  // sonner reads the colour scheme; jsdom has no matchMedia.
  window.matchMedia ??= ((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addEventListener: () => {},
    removeEventListener: () => {},
    addListener: () => {},
    removeListener: () => {},
    dispatchEvent: () => false,
  })) as unknown as typeof window.matchMedia;
});

afterEach(() => {
  document.body.innerHTML = "";
});

/**
 * The page (a sidebar button focused before the toast) and a door whose
 * pushDialog opens a stand-in panel the way the Dialog primitive does:
 * initial focus on its run, the page inert behind it.
 */
function mount() {
  const page = document.createElement("div");
  const before = document.createElement("button");
  before.textContent = "My Skills";
  page.append(before);
  document.body.append(page);
  const panel = document.createElement("div");
  panel.setAttribute("role", "dialog");
  const initial = document.createElement("button");
  initial.textContent = "the run";
  panel.append(initial);
  const stack: Array<{ kind: string; runId?: number }> = [];
  let focusAtOpen: Element | null = null;
  // Every element that took focus once the panel was asked for: the failure
  // was sonner pulling focus back onto the page after the panel took it
  // (Chrome keeps that nested focus; jsdom lets the outer one win, so the
  // sequence itself is the assertion).
  const focusedAfterOpen: Element[] = [];
  document.addEventListener("focusin", (event) => {
    if (stack.length > 0) focusedAfterOpen.push(event.target as Element);
  });
  const door = {
    isOpen: (kind: string) => stack.some((layer) => layer.kind === kind),
    updateDialog: vi.fn(),
    pushDialog: vi.fn((layer: { kind: string; runId?: number }) => {
      focusAtOpen = document.activeElement;
      stack.push(layer);
      document.body.append(panel);
      // The primitive takes initial focus after it mounts, then the page
      // behind goes inert (focus left on it would drop to <body>).
      queueMicrotask(() => {
        initial.focus();
        page.setAttribute("inert", "");
        if (page.contains(document.activeElement)) (document.activeElement as HTMLElement).blur();
      });
    }),
  };
  render(createElement(Toaster, { position: "top-right" }));
  const hook = renderHook(() => useStatusReporter(t, door as never));
  return { before, initial, door, hook, focusAtOpen: () => focusAtOpen, focusedAfterOpen };
}

async function showReportToast(hook: ReturnType<typeof mount>["hook"]) {
  await act(async () => {
    await hook.result.current.applyOutcome({
      toast: { kind: "warning", tone: "failure", message: "1 refreshed · 1 failed" },
      errors: [{ title: "alpha failed", message: "offline" }],
      warnings: [],
      notes: [],
      completion: { reload: false, closeModal: false, conflict: false },
    });
  });
  let button: HTMLButtonElement | null = null;
  await waitFor(() => {
    button = document.querySelector<HTMLButtonElement>("[data-sonner-toast] [data-button]");
    expect(button?.textContent).toBe("runReport.viewReport");
  });
  return button!;
}

/** Focus after the panel opened and the toast left the screen (sonner's own removal included). */
async function settled() {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 600));
  });
  await waitFor(() => expect(document.querySelector("[data-sonner-toast]")).toBeNull());
}

describe("View report hands focus from the toast to the panel (A1)", () => {
  it("keyboard: the toaster's return completes before the panel opens, and the panel keeps focus", async () => {
    const m = mount();
    m.before.focus();
    const view = await showReportToast(m.hook);
    // Shift+Tab lands on the toast's action: sonner remembers My Skills.
    act(() => view.focus());
    expect(document.activeElement).toBe(view);
    act(() => {
      fireEvent.keyDown(view, { key: "Enter" });
      view.click();
    });
    expect(m.door.pushDialog).toHaveBeenCalledWith({ kind: "notifications", runId: expect.any(Number) });
    // The return happened first, onto the still-usable page — not into the toast.
    expect(m.focusAtOpen()).toBe(m.before);
    await settled();
    expect(m.focusedAfterOpen).toEqual([m.initial]);
    expect(document.activeElement).toBe(m.initial);
  });

  it("pointer: a click that focused the action lands in the panel the same way", async () => {
    const m = mount();
    m.before.focus();
    const view = await showReportToast(m.hook);
    // (sonner's pointer handlers swipe; the click path is mousedown → focus → click.)
    act(() => {
      fireEvent.mouseDown(view);
      view.focus();
      fireEvent.mouseUp(view);
      view.click();
    });
    await settled();
    expect(m.focusedAfterOpen).toEqual([m.initial]);
    expect(document.activeElement).toBe(m.initial);
  });

  it("a click that never focused the toast (focus stayed on the page) is unaffected", async () => {
    const m = mount();
    m.before.focus();
    const view = await showReportToast(m.hook);
    act(() => view.click());
    expect(m.focusAtOpen()).toBe(m.before);
    await settled();
    expect(m.focusedAfterOpen).toEqual([m.initial]);
    expect(document.activeElement).toBe(m.initial);
  });
});
