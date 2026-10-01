// Checkbox and Switch contracts (round-19 D2): role, name, and aria-checked
// that reflects the controlled prop.

import { useState } from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
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

  it("keeps focus while disabled by its own action (focusableWhenDisabled), refusing every change", () => {
    const onCheckedChange = vi.fn();
    render(<Checkbox aria-label="Amp" checked={false} disabled focusableWhenDisabled onCheckedChange={onCheckedChange} />);
    const box = screen.getByRole("checkbox", { name: "Amp" });
    box.focus();
    expect(document.activeElement).toBe(box);
    expect(box.getAttribute("aria-disabled")).toBe("true");
    expect(box.hasAttribute("aria-readonly")).toBe(false);
    expect(box.hasAttribute("data-focusable-when-disabled")).toBe(true);
    expect(box.getAttribute("tabindex")).not.toBe("-1");
    fireEvent.click(box);
    fireEvent.keyDown(box, { key: " " });
    expect(onCheckedChange).not.toHaveBeenCalled();
    expect(box.getAttribute("aria-checked")).toBe("false");
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

  it("disabled is aria-disabled, out of the tab order and not activatable", () => {
    const onCheckedChange = vi.fn();
    render(<Switch aria-label="Pi" checked disabled onCheckedChange={onCheckedChange} />);
    const control = screen.getByRole("switch", { name: "Pi" });
    expect(control.getAttribute("aria-disabled")).toBe("true");
    expect(control.getAttribute("tabindex")).toBe("-1");
    fireEvent.click(control);
    expect(onCheckedChange).not.toHaveBeenCalled();
    expect(control.getAttribute("aria-checked")).toBe("true");
  });

  it("focusableWhenDisabled keeps focus and the tab stop, says aria-disabled, and refuses every change", () => {
    const onCheckedChange = vi.fn();
    render(
      <Switch
        aria-label="alpha enabled"
        checked
        disabled
        focusableWhenDisabled
        onCheckedChange={onCheckedChange}
      />,
    );
    const control = screen.getByRole("switch", { name: "alpha enabled" });
    expect(control.getAttribute("aria-disabled")).toBe("true");
    expect(control.getAttribute("aria-readonly")).toBeNull();
    expect(control.getAttribute("tabindex")).toBe("0");
    control.focus();
    expect(document.activeElement).toBe(control);

    fireEvent.click(control);
    fireEvent.keyDown(control, { key: " " });
    fireEvent.keyUp(control, { key: " " });
    fireEvent.keyDown(control, { key: "Enter" });

    expect(onCheckedChange).not.toHaveBeenCalled();
    expect(control.getAttribute("aria-checked")).toBe("true");
    expect(document.activeElement).toBe(control);
  });

  it("keeps focus on the switch when its own action disables it, and works again once enabled", () => {
    function RowSwitch() {
      const [checked, setChecked] = useState(true);
      const [running, setRunning] = useState(false);
      return (
        <>
          <Switch
            aria-label="beta enabled"
            checked={checked}
            disabled={running}
            focusableWhenDisabled
            onCheckedChange={(next) => {
              setChecked(next);
              setRunning(true);
            }}
          />
          <button type="button" onClick={() => setRunning(false)}>
            settle
          </button>
        </>
      );
    }
    render(<RowSwitch />);
    const control = screen.getByRole("switch", { name: "beta enabled" });
    control.focus();
    fireEvent.click(control);

    expect(control.getAttribute("aria-checked")).toBe("false");
    expect(control.getAttribute("aria-disabled")).toBe("true");
    expect(control.getAttribute("tabindex")).toBe("0");
    expect(document.activeElement).toBe(control);

    fireEvent.click(screen.getByRole("button", { name: "settle" }));
    expect(control.getAttribute("aria-disabled")).toBeNull();
    fireEvent.click(control);
    expect(control.getAttribute("aria-checked")).toBe("true");
  });

  it("removes the thumb's slide under reduced motion", () => {
    render(<ControlledSwitch />);
    const thumb = document.querySelector("[data-slot=switch-thumb]");
    expect(thumb?.className).toContain("motion-reduce:transition-none");
  });
});
