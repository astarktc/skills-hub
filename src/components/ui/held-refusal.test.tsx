// The held-control refusal contract (round-19 ticket 27, D19): a Button,
// Switch or Checkbox held disabled by a reason keeps focus, refuses every
// change, and says so — one bubbling HELD_REFUSAL_EVENT per activation
// (pointer click, Space, Enter) carrying its stated reason. The primitive
// itself announces nothing (no live region of its own): the binder turns the
// event into the reporter's one polite toast (`useHeldRefusal`).

import type * as React from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Button } from "./button";
import { Checkbox } from "./checkbox";
import { HELD_REFUSAL_EVENT, type HeldRefusalDetail } from "./held-refusal";
import { Switch } from "./switch";

const REASON = "Disable is running — wait for it to finish";

let heard: Array<string | null> = [];
const listen = (event: Event) => heard.push((event as CustomEvent<HeldRefusalDetail>).detail.reason);

beforeEach(() => {
  heard = [];
  document.addEventListener(HELD_REFUSAL_EVENT, listen);
});
afterEach(() => {
  document.removeEventListener(HELD_REFUSAL_EVENT, listen);
  cleanup();
});

type Held = { held: boolean; change: () => void; extra?: Record<string, unknown> };

const CONTROLS: Array<{
  name: string;
  role: "button" | "switch" | "checkbox";
  render: (props: Held) => React.ReactElement;
  /** The value that must not change, read off the element. */
  state: (element: HTMLElement) => string | null;
}> = [
  {
    name: "Switch",
    role: "switch",
    render: ({ held, change, extra }) => (
      <Switch
        aria-label="alpha enabled"
        checked
        disabled={held}
        focusableWhenDisabled
        onCheckedChange={change}
        title={held ? REASON : undefined}
        aria-description={held ? REASON : undefined}
        {...extra}
      />
    ),
    state: (element) => element.getAttribute("aria-checked"),
  },
  {
    name: "Checkbox",
    role: "checkbox",
    render: ({ held, change, extra }) => (
      <Checkbox
        aria-label="alpha on Cursor"
        checked={false}
        disabled={held}
        focusableWhenDisabled
        onCheckedChange={change}
        title={held ? REASON : undefined}
        aria-description={held ? REASON : undefined}
        {...extra}
      />
    ),
    state: (element) => element.getAttribute("aria-checked"),
  },
  {
    name: "Button",
    role: "button",
    render: ({ held, change, extra }) => (
      <Button
        disabled={held}
        focusableWhenDisabled
        onClick={change}
        title={held ? REASON : undefined}
        aria-description={held ? REASON : undefined}
        {...extra}
      >
        Sync
      </Button>
    ),
    state: (element) => element.getAttribute("aria-disabled"),
  },
];

describe.each(CONTROLS)("$name held by a reason (ticket 27)", ({ role, render: view, state }) => {
  it("a pointer click dispatches one refusal with the stated reason, changes nothing and keeps focus", () => {
    const change = vi.fn();
    render(view({ held: true, change }));
    const control = screen.getByRole(role);
    const before = state(control);
    control.focus();

    fireEvent.click(control, { detail: 1 });

    expect(heard).toEqual([REASON]);
    expect(change).not.toHaveBeenCalled();
    expect(state(control)).toBe(before);
    expect(document.activeElement).toBe(control);
  });

  it("Space and Enter each dispatch one refusal; the click a key synthesizes is not counted again", () => {
    const change = vi.fn();
    render(view({ held: true, change }));
    const control = screen.getByRole(role);
    control.focus();

    fireEvent.keyDown(control, { key: " " });
    fireEvent.keyUp(control, { key: " " });
    // A native button (or Base UI's dispatch) turns the key into a click with detail 0.
    fireEvent.click(control, { detail: 0 });
    fireEvent.keyDown(control, { key: "Enter" });
    // Holding the key down is one activation, not a stream of them.
    fireEvent.keyDown(control, { key: "Enter", repeat: true });
    fireEvent.keyDown(control, { key: "Tab" });

    expect(heard).toEqual([REASON, REASON]);
    expect(change).not.toHaveBeenCalled();
    expect(document.activeElement).toBe(control);
  });

  it("falls back to the title when the control states no aria-description", () => {
    render(view({ held: true, change: vi.fn(), extra: { "aria-description": undefined } }));
    fireEvent.click(screen.getByRole(role), { detail: 1 });
    expect(heard).toEqual([REASON]);
  });

  it("carries a null reason when the control states none", () => {
    render(view({ held: true, change: vi.fn(), extra: { "aria-description": undefined, title: undefined } }));
    fireEvent.click(screen.getByRole(role), { detail: 1 });
    expect(heard).toEqual([null]);
  });

  it("still runs the caller's own capture handler", () => {
    const onClickCapture = vi.fn();
    render(view({ held: true, change: vi.fn(), extra: { onClickCapture } }));
    fireEvent.click(screen.getByRole(role), { detail: 1 });
    expect(onClickCapture).toHaveBeenCalledTimes(1);
    expect(heard).toEqual([REASON]);
  });

  it("renders no live region of its own: the one announcement is the binder's toast", () => {
    const { container } = render(view({ held: true, change: vi.fn() }));
    fireEvent.click(screen.getByRole(role), { detail: 1 });
    expect(container.querySelector("[aria-live], [role='status'], [role='alert']")).toBeNull();
  });

  it("takes the pointer (the click lands on it, not on the row beneath); other disabled controls stay pointer-transparent", () => {
    render(view({ held: true, change: vi.fn() }));
    const control = screen.getByRole(role);
    expect(control.hasAttribute("data-focusable-when-disabled")).toBe(true);
    expect(control.className).toContain("aria-disabled:not-data-focusable-when-disabled:pointer-events-none");
    expect(control.className).not.toMatch(/(^|\s)aria-disabled:pointer-events-none(\s|$)/);
  });

  it("an enabled control dispatches nothing", () => {
    const change = vi.fn();
    render(view({ held: false, change }));
    const control = screen.getByRole(role);
    control.focus();
    fireEvent.click(control, { detail: 1 });
    fireEvent.keyDown(control, { key: "Enter" });
    expect(heard).toEqual([]);
  });
});

describe.each(CONTROLS.filter(({ role }) => role !== "button"))("$name held label activation", ({ role, render: view, state }) => {
  it("a click on its wrapping label shows one refusal, without changing state", () => {
    const change = vi.fn();
    render(<label>{view({ held: true, change })}<span>Target label</span></label>);
    const control = screen.getByRole(role);
    const before = state(control);
    control.focus();

    fireEvent.click(screen.getByText("Target label"), { detail: 1 });

    expect(heard).toEqual([REASON]);
    expect(change).not.toHaveBeenCalled();
    expect(state(control)).toBe(before);
    expect(document.activeElement).toBe(control);
  });

  it("an associated label uses the current reason and stops refusing when enabled", () => {
    const change = vi.fn();
    const inputRef = { current: null as HTMLInputElement | null };
    const labeled = (held: boolean, reason: string) => <>
      <label htmlFor="held-target">External label</label>
      {view({ held, change, extra: { id: "held-target", inputRef, "aria-description": reason } })}
    </>;
    const { rerender, unmount } = render(labeled(true, REASON));
    expect(inputRef.current?.tagName).toBe("INPUT");
    fireEvent.click(screen.getByText("External label"), { detail: 1 });
    rerender(labeled(true, "Folder not found"));
    fireEvent.click(screen.getByText("External label"), { detail: 1 });
    expect(heard).toEqual([REASON, "Folder not found"]);
    expect(change).not.toHaveBeenCalled();

    rerender(labeled(false, "Folder not found"));
    fireEvent.click(screen.getByText("External label"), { detail: 1 });
    expect(heard).toEqual([REASON, "Folder not found"]);
    expect(change).toHaveBeenCalledTimes(1);
    unmount();
    expect(inputRef.current).toBeNull();
  });

  it("preserves callback inputRef cleanup and removes the input observer on unmount", () => {
    const cleanupRef = vi.fn();
    const inputRef = vi.fn<(element: HTMLInputElement | null) => () => void>(() => cleanupRef);
    const { unmount } = render(view({ held: true, change: vi.fn(), extra: { inputRef } }));
    const forwarded = inputRef.mock.calls[0][0]!;
    expect(forwarded).not.toBeNull();
    const refusal = vi.fn();
    forwarded.addEventListener(HELD_REFUSAL_EVENT, refusal);
    fireEvent.click(forwarded, { detail: 1 });
    expect(refusal).toHaveBeenCalledTimes(1);
    unmount();
    expect(cleanupRef).toHaveBeenCalledTimes(1);
    expect(inputRef).toHaveBeenCalledTimes(1);
    fireEvent.click(forwarded, { detail: 1 });
    expect(refusal).toHaveBeenCalledTimes(1);
  });
});

describe("Button pending (its own action running)", () => {
  it("is held: a click dispatches one refusal, the binder names the running action", () => {
    const onClick = vi.fn();
    render(
      <Button pending onClick={onClick}>
        Refresh
      </Button>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Refresh" }), { detail: 1 });
    expect(heard).toEqual([null]);
    expect(onClick).not.toHaveBeenCalled();
  });

  it("a natively disabled button (not held) is never activated and dispatches nothing", () => {
    render(<Button disabled>Save</Button>);
    fireEvent.click(screen.getByRole("button", { name: "Save" }), { detail: 1 });
    expect(heard).toEqual([]);
  });
});
