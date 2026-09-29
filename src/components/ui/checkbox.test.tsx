// Checkbox and Switch contracts (round-19 D2): role, name, and aria-checked
// that reflects the controlled prop.

import { useState } from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { Checkbox } from "./checkbox";
import { Switch } from "./switch";

afterEach(cleanup);

function ControlledCheckbox() {
  const [checked, setChecked] = useState(false);
  return (
    <Checkbox aria-label="Cursor" checked={checked} onCheckedChange={setChecked} />
  );
}

describe("Checkbox", () => {
  it("is a named checkbox whose aria-checked follows the value", () => {
    render(<ControlledCheckbox />);
    const box = screen.getByRole("checkbox", { name: "Cursor" });
    expect(box.getAttribute("aria-checked")).toBe("false");

    fireEvent.click(box);

    expect(box.getAttribute("aria-checked")).toBe("true");
  });

  it("is named by a wrapping label", () => {
    render(
      <label>
        <Checkbox /> Claude Code
      </label>,
    );
    expect(screen.getByRole("checkbox", { name: "Claude Code" })).toBeTruthy();
  });

  it("reports mixed when indeterminate", () => {
    render(<Checkbox aria-label="Select all" indeterminate checked={false} />);
    expect(
      screen.getByRole("checkbox", { name: "Select all" }).getAttribute("aria-checked"),
    ).toBe("mixed");
  });

  it("reports disabled", () => {
    render(<Checkbox aria-label="Pi" disabled />);
    expect(
      screen.getByRole("checkbox", { name: "Pi" }).getAttribute("aria-disabled"),
    ).toBe("true");
  });
});

function ControlledSwitch() {
  const [checked, setChecked] = useState(true);
  return (
    <Switch aria-label="Enabled" checked={checked} onCheckedChange={setChecked} />
  );
}

describe("Switch", () => {
  it("is a named switch whose aria-checked follows the value", () => {
    render(<ControlledSwitch />);
    const control = screen.getByRole("switch", { name: "Enabled" });
    expect(control.getAttribute("aria-checked")).toBe("true");

    fireEvent.click(control);

    expect(control.getAttribute("aria-checked")).toBe("false");
  });

  it("removes the thumb's slide under reduced motion", () => {
    render(<ControlledSwitch />);
    const thumb = document.querySelector("[data-slot=switch-thumb]");
    expect(thumb?.className).toContain("motion-reduce:transition-none");
  });
});
