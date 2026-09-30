// Popover, DropdownMenu and Tooltip contracts (round-19 D2): roles and
// names, and Escape closes the layer and returns focus to its trigger.

import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { useRef, useState } from "react";
import { afterEach, describe, expect, it } from "vitest";
import { Popover, PopoverContent, PopoverTitle, PopoverTrigger } from "./popover";
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
