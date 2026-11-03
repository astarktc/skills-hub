// The CommandPalette primitive's contract (round-19 ticket 23; D2, ADR-0006 §7):
// a named modal dialog holding a combobox input that controls a listbox of
// options in named groups, the highlighted option as the input's active
// descendant, Enter runs it, a refused option stays reachable and says why,
// Escape closes it and focus returns to the opener with no layer left behind.
// Base UI's internals are not re-tested; jsdom judges no pixels.

import { useState } from "react";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CommandPalette, type CommandPaletteGroup, type CommandPaletteItem } from "./command-palette";
import { HELD_REFUSAL_EVENT, type HeldRefusalDetail } from "./held-refusal";
import { openLayerCount } from "./layer-stack";

afterEach(cleanup);

const GROUPS: CommandPaletteGroup[] = [
  {
    id: "places",
    label: "Go to",
    items: [
      { id: "place:skills", label: "My Skills" },
      { id: "place:projects", label: "Projects" },
    ],
  },
  {
    id: "actions",
    label: "Actions",
    items: [{ id: "all:update", label: "Update all", refusal: "Deploy is running" }],
  },
  { id: "empty", label: "Nothing here", items: [] },
];

function Harness({
  groups = GROUPS,
  onSelect = () => {},
  onComplete = () => {},
}: {
  groups?: CommandPaletteGroup[];
  onSelect?: (item: CommandPaletteItem) => void;
  onComplete?: (open: boolean) => void;
}) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const shown = groups.map((group) => ({
    ...group,
    items: group.items.filter((item) => item.label.toLowerCase().includes(query.toLowerCase())),
  }));
  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>
        Row
      </button>
      <CommandPalette
        open={open}
        onOpenChange={setOpen}
        onOpenChangeComplete={onComplete}
        label="Command palette"
        inputLabel="Find a skill, a place or an action"
        placeholder="Type…"
        query={query}
        onQueryChange={setQuery}
        groups={shown}
        emptyText="Nothing matches"
        footer={<span>Enter run</span>}
        onSelect={(item) => {
          onSelect(item);
          setOpen(false);
        }}
      />
    </>
  );
}

async function openPalette() {
  const opener = screen.getByRole("button", { name: "Row" });
  opener.focus();
  fireEvent.click(opener);
  const dialog = await screen.findByRole("dialog", { name: "Command palette" });
  const input = within(dialog).getByRole("combobox", { name: "Find a skill, a place or an action" });
  await waitFor(() => expect(document.activeElement).toBe(input));
  return { opener, dialog, input };
}

const activeOption = (input: HTMLElement) => {
  const id = input.getAttribute("aria-activedescendant");
  return id ? document.getElementById(id) : null;
};

/** Type into the input the way a keyboard does: an `input` event carrying `inputType`, which `autoHighlight` keys off. */
const typeQuery = (input: HTMLElement, value: string) =>
  fireEvent.input(input, { target: { value }, inputType: "insertText" });

describe("CommandPalette", () => {
  it("is a named dialog whose focused combobox controls a listbox of options in named groups", async () => {
    render(<Harness />);
    const { dialog, input } = await openPalette();
    expect(input.getAttribute("aria-autocomplete")).toBe("list");
    expect(input.getAttribute("aria-expanded")).toBe("true");
    const listbox = within(dialog).getByRole("listbox");
    expect(input.getAttribute("aria-controls")).toBe(listbox.id);
    // Empty groups are skipped; each group is named by its label.
    const groups = within(listbox).getAllByRole("group");
    expect(groups.map((group) => group.getAttribute("aria-labelledby") && document.getElementById(group.getAttribute("aria-labelledby")!)?.textContent)).toEqual([
      "Go to",
      "Actions",
    ]);
    expect(within(groups[0]).getAllByRole("option").map((option) => option.textContent)).toEqual(["My Skills", "Projects"]);
    // The foot describes the input.
    expect(document.getElementById(input.getAttribute("aria-describedby")!)?.textContent).toBe("Enter run");
  });

  it("highlights nothing on open (Enter is a no-op), ↓ highlights the first option as the active descendant while focus stays in the input", async () => {
    const onSelect = vi.fn();
    render(<Harness onSelect={onSelect} />);
    const { input } = await openPalette();
    expect(activeOption(input)).toBeNull();
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onSelect).not.toHaveBeenCalled();
    fireEvent.keyDown(input, { key: "ArrowDown" });
    await waitFor(() => expect(activeOption(input)?.textContent).toBe("My Skills"));
    fireEvent.keyDown(input, { key: "ArrowDown" });
    await waitFor(() => expect(activeOption(input)?.textContent).toBe("Projects"));
    expect(document.activeElement).toBe(input);
  });

  it("highlights the first match once a query is typed", async () => {
    render(<Harness />);
    const { input } = await openPalette();
    typeQuery(input, "my");
    await waitFor(() => expect(activeOption(input)?.textContent).toBe("My Skills"));
  });

  it("runs the highlighted option on Enter, then reports the close complete with focus back on the opener", async () => {
    const onSelect = vi.fn();
    const onComplete = vi.fn();
    render(<Harness onSelect={onSelect} onComplete={onComplete} />);
    const { opener, input } = await openPalette();
    typeQuery(input, "proj");
    await waitFor(() => expect(activeOption(input)?.textContent).toBe("Projects"));
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onSelect).toHaveBeenCalledWith(expect.objectContaining({ id: "place:projects" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await waitFor(() => expect(onComplete).toHaveBeenLastCalledWith(false));
    await waitFor(() => expect(document.activeElement).toBe(opener));
    expect(openLayerCount()).toBe(0);
  });

  it("keeps a refused option reachable, aria-disabled with its reason shown; Enter says why and runs nothing", async () => {
    const onSelect = vi.fn();
    render(<Harness onSelect={onSelect} />);
    const { dialog, input } = await openPalette();
    const refused = within(dialog).getByRole("option", { name: "Update all" });
    expect(refused.getAttribute("aria-disabled")).toBe("true");
    // The label is the name; the refusal is the description, never part of the name.
    expect(refused.getAttribute("aria-label")).toBeNull();
    expect(document.getElementById(refused.getAttribute("aria-describedby")!)?.textContent).toBe("Deploy is running");
    const heard: (string | null)[] = [];
    const listen = (event: Event) => heard.push((event as CustomEvent<HeldRefusalDetail>).detail.reason);
    document.addEventListener(HELD_REFUSAL_EVENT, listen);
    try {
      // Nothing is highlighted on open: three ↓ reach the third option.
      fireEvent.keyDown(input, { key: "ArrowDown" });
      fireEvent.keyDown(input, { key: "ArrowDown" });
      fireEvent.keyDown(input, { key: "ArrowDown" });
      await waitFor(() => expect(activeOption(input)).toBe(refused));
      fireEvent.keyDown(input, { key: "Enter" });
    } finally {
      document.removeEventListener(HELD_REFUSAL_EVENT, listen);
    }
    expect(heard).toEqual(["Deploy is running"]);
    expect(onSelect).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog", { name: "Command palette" })).toBeTruthy();
  });

  it("closes on Escape and returns focus to the element that opened it, leaving no layer", async () => {
    render(<Harness />);
    const { opener, input } = await openPalette();
    fireEvent.keyDown(input, { key: "Escape", code: "Escape" });
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await waitFor(() => expect(document.activeElement).toBe(opener));
    expect(openLayerCount()).toBe(0);
  });

  it("makes the app behind it inert while open", async () => {
    render(<Harness />);
    const { opener } = await openPalette();
    await waitFor(() => expect(opener.closest("[inert]")).not.toBeNull());
    await act(async () => {
      fireEvent.keyDown(document.activeElement!, { key: "Escape", code: "Escape" });
    });
    await waitFor(() => expect(opener.closest("[inert]")).toBeNull());
  });

  it("says so when nothing matches", async () => {
    render(<Harness />);
    const { dialog, input } = await openPalette();
    typeQuery(input, "zzz");
    await waitFor(() => expect(within(dialog).getByRole("status").textContent).toBe("Nothing matches"));
    expect(within(dialog).queryAllByRole("option")).toEqual([]);
  });
});
