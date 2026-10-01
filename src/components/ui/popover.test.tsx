// Popover, DropdownMenu and Tooltip contracts (round-19 D2): roles and
// names, and Escape closes the layer and returns focus to its trigger.

import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { useLayoutEffect, useRef, useState } from "react";
import { afterEach, describe, expect, it } from "vitest";
import { Popover, PopoverContent, PopoverTitle, PopoverTrigger } from "./popover";
import { closeLayer, openLayer } from "./layer-stack";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "./dropdown-menu";
import { Tooltip, TooltipContent, TooltipTrigger } from "./tooltip";

afterEach(cleanup);

function pressEscape() {
  fireEvent.keyDown(document.activeElement ?? document.body, {
    key: "Escape",
    code: "Escape",
  });
}

describe("Popover", () => {
  it("opens a dialog named by its title; Escape closes it and focus returns", async () => {
    render(
      <Popover>
        <PopoverTrigger>Scope</PopoverTrigger>
        <PopoverContent>
          <PopoverTitle>Tool scope</PopoverTitle>
          <button type="button">Cursor</button>
        </PopoverContent>
      </Popover>,
    );
    const trigger = screen.getByRole("button", { name: "Scope" });
    trigger.focus();
    fireEvent.click(trigger);
    expect(await screen.findByRole("dialog", { name: "Tool scope" })).toBeTruthy();
    expect(trigger.getAttribute("aria-expanded")).toBe("true");

    pressEscape();

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await waitFor(() => expect(document.activeElement).toBe(trigger));
  });
});

describe("Popover focus returned from a layer that is gone (ticket 16 A1)", () => {
  /**
   * A layer above the popover (the shared-folder ask) took focus — Base UI
   * then takes the popover's controls out of the tab order — and, as it
   * unmounted, returned focus to the popover's checkbox from nowhere (no
   * related target). Tab must move on inside the popover again.
   */
  /** A modal layer of the stack, in its own portal-less node: the ask's shape, minus Base UI. */
  function OutsideLayer({ onDone }: { onDone: () => void }) {
    useLayoutEffect(() => {
      openLayer("outside-ask");
      return () => closeLayer("outside-ask");
    }, []);
    return (
      <button type="button" onClick={onDone}>
        Continue
      </button>
    );
  }

  function WithOutsideLayer() {
    const [layer, setLayer] = useState(false);
    return (
      <>
        <Popover>
          <PopoverTrigger>Edit Tools</PopoverTrigger>
          <PopoverContent>
            <PopoverTitle>Tool scope</PopoverTitle>
            <button type="button" onClick={() => setLayer(true)}>
              Amp
            </button>
            <button type="button">Cursor</button>
          </PopoverContent>
        </Popover>
        {layer ? <OutsideLayer onDone={() => setLayer(false)} /> : null}
      </>
    );
  }

  it("puts the popover's controls back in the tab order when focus returns from nowhere", async () => {
    render(<WithOutsideLayer />);
    const trigger = screen.getByRole("button", { name: "Edit Tools" });
    trigger.focus();
    fireEvent.click(trigger);
    const popup = await screen.findByRole("dialog", { name: "Tool scope" });
    const amp = within(popup).getByRole("button", { name: "Amp" });
    const cursor = within(popup).getByRole("button", { name: "Cursor" });
    amp.focus();
    fireEvent.click(amp);
    // The layer takes focus (Base UI disables focus inside the portal it left).
    const layer = screen.getByRole("button", { name: "Continue" });
    layer.focus();
    // Base UI's deferred focus-out check runs while the layer is up: the popover stays.
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(screen.getByRole("dialog", { name: "Tool scope" })).toBe(popup);
    // In a browser Base UI now takes the portal's controls out of the tab order
    // (its `disableFocusInside`: `data-tabindex` keeps the old value); jsdom
    // dispatches no focus-out it acts on, so do exactly what it does.
    for (const control of [amp, cursor]) {
      control.dataset.tabindex = control.getAttribute("tabindex") ?? "";
      control.setAttribute("tabindex", "-1");
    }
    // The layer unmounts with focus on it; focus comes back from <body>.
    fireEvent.click(layer);
    await waitFor(() => expect(screen.queryByRole("button", { name: "Continue" })).toBeNull());
    (document.activeElement as HTMLElement | null)?.blur?.();
    amp.focus();
    expect(document.activeElement).toBe(amp);
    expect(cursor.getAttribute("tabindex")).not.toBe("-1");
    expect(amp.getAttribute("tabindex")).not.toBe("-1");
  });
});

describe("Popover without a trigger (anchor)", () => {
  function Inline() {
    const [open, setOpen] = useState(false);
    const anchor = useRef<HTMLButtonElement>(null);
    const confirm = useRef<HTMLButtonElement>(null);
    return (
      <>
        <button ref={anchor} type="button" onClick={() => setOpen(true)}>
          Enable alpha
        </button>
        <Popover open={open} onOpenChange={setOpen}>
          <PopoverContent anchor={anchor} initialFocus={confirm} finalFocus={anchor}>
            <PopoverTitle>Sync again?</PopoverTitle>
            <button ref={confirm} type="button">
              Enable
            </button>
          </PopoverContent>
        </Popover>
      </>
    );
  }

  it("opens on its own terms with focus on the named control; Escape returns focus to the anchor", async () => {
    render(<Inline />);
    const anchor = screen.getByRole("button", { name: "Enable alpha" });
    anchor.focus();
    fireEvent.click(anchor);
    const popup = await screen.findByRole("dialog", { name: "Sync again?" });
    await waitFor(() => expect(document.activeElement).toBe(within(popup).getByRole("button", { name: "Enable" })));

    pressEscape();

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await waitFor(() => expect(document.activeElement).toBe(anchor));
  });
});

describe("DropdownMenu", () => {
  it("opens a menu of menuitems; Escape closes it and focus returns", async () => {
    render(
      <DropdownMenu>
        <DropdownMenuTrigger aria-label="More actions">⋯</DropdownMenuTrigger>
        <DropdownMenuContent>
          <DropdownMenuItem>Edit</DropdownMenuItem>
          <DropdownMenuItem variant="destructive">Delete…</DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>,
    );
    const trigger = screen.getByRole("button", { name: "More actions" });
    trigger.focus();
    fireEvent.click(trigger);

    expect(await screen.findByRole("menu")).toBeTruthy();
    expect(screen.getByRole("menuitem", { name: "Delete…" })).toBeTruthy();

    pressEscape();

    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
    await waitFor(() => expect(document.activeElement).toBe(trigger));
  });
});

describe("DropdownMenu with a labelled group", () => {
  it("names the group by its label and keeps its items inside it", async () => {
    render(
      <DropdownMenu>
        <DropdownMenuTrigger aria-label="More actions">⋯</DropdownMenuTrigger>
        <DropdownMenuContent>
          <DropdownMenuGroup>
            <DropdownMenuLabel>Skill</DropdownMenuLabel>
            <DropdownMenuItem>Edit</DropdownMenuItem>
          </DropdownMenuGroup>
          <DropdownMenuSeparator />
          <DropdownMenuItem variant="destructive">Delete…</DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>,
    );
    fireEvent.click(screen.getByRole("button", { name: "More actions" }));

    const group = await screen.findByRole("group", { name: "Skill" });
    expect(within(group).getByRole("menuitem", { name: "Edit" })).toBeTruthy();
    expect(within(group).queryByRole("menuitem", { name: "Delete…" })).toBeNull();
    expect(screen.getByRole("menuitem", { name: "Delete…" })).toBeTruthy();
  });
});

describe("Tooltip", () => {
  it("adds a hint without replacing the trigger's own name", async () => {
    render(
      <Tooltip open>
        <TooltipTrigger aria-label="Refresh all">↻</TooltipTrigger>
        <TooltipContent>Fetch every source again</TooltipContent>
      </Tooltip>,
    );
    expect(screen.getByRole("button", { name: "Refresh all" })).toBeTruthy();
    expect(await screen.findByText("Fetch every source again")).toBeTruthy();
  });
});
