// Button's contract (round-19 D2): role + name, and disabled / pending as
// ARIA state that keeps focus where it is.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Button } from "./button";

afterEach(cleanup);

describe("Button", () => {
  it("is a button named by its content, whatever its variant and tone", () => {
    render(
      <Button variant="solid" tone="danger" size="sm">
        Delete
      </Button>,
    );
    expect(screen.getByRole("button", { name: "Delete" })).toBeTruthy();
  });

  it("an icon-only button takes its name from aria-label", () => {
    render(<Button size="icon" aria-label="Refresh all" />);
    expect(screen.getByRole("button", { name: "Refresh all" })).toBeTruthy();
  });

  it("pending is aria-busy, not activatable, and stays focusable", () => {
    const onClick = vi.fn();
    render(
      <Button pending onClick={onClick}>
        Refresh
      </Button>,
    );
    const button = screen.getByRole("button", { name: "Refresh" });

    expect(button.getAttribute("aria-busy")).toBe("true");
    expect(button.getAttribute("aria-disabled")).toBe("true");
    button.focus();
    expect(document.activeElement).toBe(button);
    fireEvent.click(button);
    expect(onClick).not.toHaveBeenCalled();
  });

  it("disabled is not activatable", () => {
    const onClick = vi.fn();
    render(
      <Button disabled onClick={onClick}>
        Save
      </Button>,
    );
    const button = screen.getByRole("button", { name: "Save" }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    fireEvent.click(button);
    expect(onClick).not.toHaveBeenCalled();
  });
});
