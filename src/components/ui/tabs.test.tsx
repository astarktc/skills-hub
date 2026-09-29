// Tabs, Segmented and Chip contracts (round-19 D2): roles and the selected /
// pressed state that follows the value.

import { useState } from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./tabs";
import { Segmented, SegmentedItem } from "./segmented";
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
