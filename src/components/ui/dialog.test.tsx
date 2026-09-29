// The Dialog primitive's contract (round-19 D2; ADR-0006 §7): role and name,
// focus in / trapped / returned (or to a fallback), Escape closes only the top
// layer and only when dismissible, and the background is inert while open.
// Base UI's internals are not re-tested; jsdom judges no pixels.

import { useRef, useState } from "react";
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "./dialog";
import { ConfirmDialog } from "./confirm-dialog";

function pressEscape() {
  fireEvent.keyDown(document.activeElement ?? document.body, {
    key: "Escape",
    code: "Escape",
  });
}

async function openedDialog(name: string) {
  const dialog = await screen.findByRole("dialog", { name });
  await waitFor(() => expect(dialog.contains(document.activeElement)).toBe(true));
  return dialog;
}

function Single({ dismissible = true }: { dismissible?: boolean }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>
        Open settings
      </button>
      <Dialog open={open} onOpenChange={setOpen} dismissible={dismissible}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Settings</DialogTitle>
          </DialogHeader>
          <DialogBody>
            <button type="button">Inside</button>
          </DialogBody>
          <DialogFooter>
            <button type="button" onClick={() => setOpen(false)}>
              Done
            </button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

function Stacked() {
  const [parent, setParent] = useState(false);
  const [child, setChild] = useState(false);
  return (
    <>
      <button type="button" onClick={() => setParent(true)}>
        Add skill
      </button>
      <Dialog open={parent} onOpenChange={setParent}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add</DialogTitle>
          </DialogHeader>
          <DialogBody>
            <button type="button" onClick={() => setChild(true)}>
              Pick skills
            </button>
          </DialogBody>
        </DialogContent>
      </Dialog>
      <Dialog open={child} onOpenChange={setChild}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Picker</DialogTitle>
          </DialogHeader>
          <DialogBody>
            <button type="button">Candidate</button>
          </DialogBody>
        </DialogContent>
      </Dialog>
    </>
  );
}

function RemovesItsOpener({ withFallback = true }: { withFallback?: boolean }) {
  const [open, setOpen] = useState(false);
  const [rowExists, setRowExists] = useState(true);
  const heading = useRef<HTMLHeadingElement>(null);
  return (
    <>
      <h2 ref={heading} tabIndex={-1}>
        Library
      </h2>
      {rowExists ? (
        <button type="button" onClick={() => setOpen(true)}>
          Delete row
        </button>
      ) : null}
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent fallbackFocus={withFallback ? () => heading.current : undefined}>
          <DialogHeader>
            <DialogTitle>Delete?</DialogTitle>
          </DialogHeader>
          <DialogFooter>
            <button
              type="button"
              onClick={() => {
                setRowExists(false);
                setOpen(false);
              }}
            >
              Yes, delete
            </button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

// vitest runs without globals, so RTL's auto-cleanup is off: unmount
// explicitly, or a previous test's dialog would stay on the layer stack.
afterEach(() => {
  cleanup();
});

describe("Dialog", () => {
  it("is a dialog named by its title through aria-labelledby", async () => {
    render(<Single />);
    fireEvent.click(screen.getByRole("button", { name: "Open settings" }));

    const dialog = await openedDialog("Settings");
    const title = screen.getByText("Settings");
    expect(dialog.getAttribute("aria-labelledby")).toBe(title.id);
  });

  it("moves focus in, and Escape closes it and returns focus to the trigger", async () => {
    render(<Single />);
    const trigger = screen.getByRole("button", { name: "Open settings" });
    trigger.focus();
    fireEvent.click(trigger);
    await openedDialog("Settings");

    pressEscape();

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await waitFor(() => expect(document.activeElement).toBe(trigger));
  });

  it("makes the background inert while open, and only while open", async () => {
    const { container } = render(<Single />);
    fireEvent.click(screen.getByRole("button", { name: "Open settings" }));
    await openedDialog("Settings");

    await waitFor(() => expect(container.hasAttribute("inert")).toBe(true));

    fireEvent.click(screen.getByRole("button", { name: "Done" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(container.hasAttribute("inert")).toBe(false);
  });

  it("has a named 32 px Close in the corner", async () => {
    render(<Single />);
    fireEvent.click(screen.getByRole("button", { name: "Open settings" }));
    const dialog = await openedDialog("Settings");

    const close = screen.getByRole("button", { name: "close" });
    expect(dialog.contains(close)).toBe(true);
    expect(close.className).toContain("size-8");
  });

  it("ignores Escape and disables Close while not dismissible", async () => {
    render(<Single dismissible={false} />);
    fireEvent.click(screen.getByRole("button", { name: "Open settings" }));
    await openedDialog("Settings");

    pressEscape();
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(screen.getByRole("dialog", { name: "Settings" })).toBeTruthy();
    expect(
      (screen.getByRole("button", { name: "close" }) as HTMLButtonElement)
        .disabled,
    ).toBe(true);
  });

  it("closes only the top layer on Escape, and focus returns into the layer beneath", async () => {
    render(<Stacked />);
    fireEvent.click(screen.getByRole("button", { name: "Add skill" }));
    await openedDialog("Add");
    const pick = screen.getByRole("button", { name: "Pick skills" });
    pick.focus();
    fireEvent.click(pick);
    await openedDialog("Picker");

    pressEscape();

    await waitFor(() =>
      expect(screen.queryByRole("dialog", { name: "Picker" })).toBeNull(),
    );
    expect(screen.getByRole("dialog", { name: "Add" })).toBeTruthy();
    await waitFor(() => expect(document.activeElement).toBe(pick));

    pressEscape();
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("makes the layer beneath inert while a layer is stacked on it", async () => {
    render(<Stacked />);
    fireEvent.click(screen.getByRole("button", { name: "Add skill" }));
    const parent = await openedDialog("Add");
    fireEvent.click(screen.getByRole("button", { name: "Pick skills" }));
    await openedDialog("Picker");

    await waitFor(() => expect(parent.hasAttribute("inert")).toBe(true));
  });

  it("sends focus to the fallback when its opener no longer exists", async () => {
    render(<RemovesItsOpener />);
    const opener = screen.getByRole("button", { name: "Delete row" });
    opener.focus();
    fireEvent.click(opener);
    await openedDialog("Delete?");

    fireEvent.click(screen.getByRole("button", { name: "Yes, delete" }));

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await waitFor(() =>
      expect(document.activeElement).toBe(
        screen.getByRole("heading", { name: "Library" }),
      ),
    );
  });
});

describe("Dialog without a fallback, when its lone opener is removed", () => {
  it("returns focus to the main landmark, never to <body>", async () => {
    render(
      <main aria-label="Library surface">
        <RemovesItsOpener withFallback={false} />
      </main>,
    );
    const opener = screen.getByRole("button", { name: "Delete row" });
    opener.focus();
    fireEvent.click(opener);
    await openedDialog("Delete?");

    fireEvent.click(screen.getByRole("button", { name: "Yes, delete" }));

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    const main = screen.getByRole("main", { name: "Library surface" });
    await waitFor(() => expect(document.activeElement).toBe(main));
    expect(document.activeElement).not.toBe(document.body);
    // Programmatically focusable, never a Tab stop.
    expect(main.getAttribute("tabindex")).toBe("-1");
  });
});

describe("ConfirmDialog", () => {
  function Ask({
    tone,
    onCancel,
    onConfirm,
  }: {
    tone: "accent" | "danger";
    onCancel: () => void;
    onConfirm: () => void;
  }) {
    return (
      <ConfirmDialog
        open
        title="Overwrite?"
        confirmLabel="Overwrite"
        cancelLabel="Keep existing"
        tone={tone}
        onCancel={onCancel}
        onConfirm={onConfirm}
      >
        One folder already exists.
      </ConfirmDialog>
    );
  }

  it("answers cancel on Escape and puts focus on the safe answer for danger", async () => {
    const answers: string[] = [];
    render(
      <Ask
        tone="danger"
        onCancel={() => answers.push("cancel")}
        onConfirm={() => answers.push("confirm")}
      />,
    );
    await openedDialog("Overwrite?");
    await waitFor(() =>
      expect(document.activeElement).toBe(
        screen.getByRole("button", { name: "Keep existing" }),
      ),
    );

    pressEscape();

    await waitFor(() => expect(answers).toEqual(["cancel"]));
  });

  it("puts focus on the confirm answer for accent and has no corner Close", async () => {
    render(<Ask tone="accent" onCancel={() => {}} onConfirm={() => {}} />);
    await openedDialog("Overwrite?");

    await waitFor(() =>
      expect(document.activeElement).toBe(
        screen.getByRole("button", { name: "Overwrite" }),
      ),
    );
    expect(screen.queryByRole("button", { name: "close" })).toBeNull();
  });
});
