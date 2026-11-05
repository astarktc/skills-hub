// Tabs, Segmented and Chip contracts (round-19 D2): roles and the selected /
// pressed state that follows the value.

import { useState } from "react";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./tabs";
import { Segmented, SegmentedItem } from "./segmented";
import { RadioGroup, RadioGroupItem } from "./radio-group";
import { Chip } from "./badge";

afterEach(cleanup);

describe("Tabs", () => {
  it("is a tablist whose selected tab controls the visible panel", () => {
    render(
      <Tabs defaultValue="general">
        <TabsList variant="line" aria-label="Settings panes">
          <TabsTrigger value="general">General</TabsTrigger>
          <TabsTrigger value="tools">Tools</TabsTrigger>
        </TabsList>
        <TabsContent value="general">General pane</TabsContent>
        <TabsContent value="tools">Tools pane</TabsContent>
      </Tabs>,
    );
    expect(screen.getByRole("tablist", { name: "Settings panes" })).toBeTruthy();
    const general = screen.getByRole("tab", { name: "General" });
    const tools = screen.getByRole("tab", { name: "Tools" });
    expect(general.getAttribute("aria-selected")).toBe("true");

    fireEvent.click(tools);

    expect(tools.getAttribute("aria-selected")).toBe("true");
    expect(general.getAttribute("aria-selected")).toBe("false");
    expect(screen.getByRole("tabpanel").textContent).toBe("Tools pane");
  });
});

describe("Tabs, pane variant (the Settings frame)", () => {
  function Panes() {
    const [pane, setPane] = useState("general");
    return (
      <Tabs variant="pane" value={pane} onValueChange={(next) => setPane(String(next))}>
        <TabsList aria-label="Settings panes">
          <TabsTrigger value="general">General</TabsTrigger>
          <TabsTrigger value="appearance">Appearance</TabsTrigger>
        </TabsList>
        <TabsContent value="general">
          <button type="button">Browse</button>
        </TabsContent>
        <TabsContent value="appearance">
          <button type="button">Light</button>
        </TabsContent>
      </Tabs>
    );
  }

  it("is a vertical tablist: ↓ moves to the next pane and shows it", async () => {
    render(<Panes />);
    const list = screen.getByRole("tablist", { name: "Settings panes" });
    expect(list.getAttribute("aria-orientation")).toBe("vertical");
    const general = screen.getByRole("tab", { name: "General" });
    act(() => general.focus());
    // Base UI moves focus in a microtask.
    await act(async () => {
      fireEvent.keyDown(general, { key: "ArrowDown" });
    });
    const appearance = screen.getByRole("tab", { name: "Appearance" });
    expect(document.activeElement).toBe(appearance);
    expect(appearance.getAttribute("aria-selected")).toBe("true");
    expect(screen.getByRole("tabpanel").textContent).toBe("Light");
  });

  it("keeps the panel out of the tab order: Tab goes to the pane's first control", () => {
    render(<Panes />);
    expect(screen.getByRole("tabpanel").getAttribute("tabindex")).toBe("-1");
  });

  it("a static pane (no focusable control) is itself the Tab stop after the pane list", () => {
    render(
      <Tabs variant="pane" value="general">
        <TabsList aria-label="Shortcuts panes">
          <TabsTrigger value="general">General</TabsTrigger>
        </TabsList>
        <TabsContent value="general" staticPane>
          <dl>
            <dt>⌘K</dt>
            <dd>Open the palette</dd>
          </dl>
        </TabsContent>
      </Tabs>,
    );
    const panel = screen.getByRole("tabpanel");
    expect(panel.getAttribute("tabindex")).toBe("0");
    act(() => panel.focus());
    expect(document.activeElement).toBe(panel);
  });
});

describe("RadioGroup", () => {
  function Theme() {
    const [theme, setTheme] = useState("system");
    return (
      <RadioGroup aria-label="Theme" value={theme} onValueChange={setTheme}>
        <RadioGroupItem value="system">System</RadioGroupItem>
        <RadioGroupItem value="light">Light</RadioGroupItem>
        <RadioGroupItem value="dark">Dark</RadioGroupItem>
      </RadioGroup>
    );
  }

  it("is a named radiogroup of radios whose aria-checked follows the value", () => {
    render(<Theme />);
    expect(screen.getByRole("radiogroup", { name: "Theme" })).toBeTruthy();
    const system = screen.getByRole("radio", { name: "System" });
    const dark = screen.getByRole("radio", { name: "Dark" });
    expect(system.getAttribute("aria-checked")).toBe("true");
    fireEvent.click(dark);
    expect(dark.getAttribute("aria-checked")).toBe("true");
    expect(system.getAttribute("aria-checked")).toBe("false");
  });

  it("an arrow key moves and selects", async () => {
    render(<Theme />);
    const system = screen.getByRole("radio", { name: "System" });
    act(() => system.focus());
    await act(async () => {
      fireEvent.keyDown(system, { key: "ArrowRight" });
    });
    const light = screen.getByRole("radio", { name: "Light" });
    expect(document.activeElement).toBe(light);
    expect(light.getAttribute("aria-checked")).toBe("true");
  });
});

function ViewSwitch() {
  const [view, setView] = useState("grouped");
  return (
    <Segmented aria-label="View" value={view} onValueChange={setView}>
      <SegmentedItem value="grouped">Grouped</SegmentedItem>
      <SegmentedItem value="flat">Flat</SegmentedItem>
    </Segmented>
  );
}

describe("Segmented", () => {
  it("is a named group with exactly one pressed item, never none", () => {
    render(<ViewSwitch />);
    expect(screen.getByRole("group", { name: "View" })).toBeTruthy();
    const grouped = screen.getByRole("button", { name: "Grouped" });
    const flat = screen.getByRole("button", { name: "Flat" });
    expect(grouped.getAttribute("aria-pressed")).toBe("true");

    fireEvent.click(flat);
    expect(flat.getAttribute("aria-pressed")).toBe("true");
    expect(grouped.getAttribute("aria-pressed")).toBe("false");

    fireEvent.click(flat);
    expect(flat.getAttribute("aria-pressed")).toBe("true");
  });
});

describe("Chip", () => {
  it("is a toggle button whose aria-pressed follows the value", () => {
    function IssuesChip() {
      const [on, setOn] = useState(false);
      return (
        <Chip pressed={on} onPressedChange={setOn}>
          Issues
        </Chip>
      );
    }
    render(<IssuesChip />);
    const chip = screen.getByRole("button", { name: "Issues" });
    expect(chip.getAttribute("aria-pressed")).toBe("false");
    fireEvent.click(chip);
    expect(chip.getAttribute("aria-pressed")).toBe("true");
  });
});
