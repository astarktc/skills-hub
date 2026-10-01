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
  within,
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
import { Popover, PopoverContent, PopoverTitle, PopoverTrigger } from "./popover";

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

/**
 * An ask answered through a pending value (the overwrite ask's shape): the
 * answer clears `pending`, so the dialog closes in the same render as its
 * body loses its data. The popup keeps its identity; only the per-ask body
 * is keyed, so a displacing ask starts with fresh state.
 */
function PendingAsk() {
  const [pending, setPending] = useState<{ id: string } | null>(null);
  const [answers, setAnswers] = useState<string[]>([]);
  const answer = (value: string) => {
    setAnswers((prev) => [...prev, value]);
    setPending(null);
  };
  return (
    <>
      <button type="button" onClick={() => setPending({ id: "first" })}>
        Enable
      </button>
      <output aria-label="answers">{answers.join(",")}</output>
      <Dialog
        open={pending !== null}
        onOpenChange={(next) => {
          if (!next) answer("dismissed");
        }}
      >
        <DialogContent showCloseButton={false}>
          <DialogHeader>
            <DialogTitle>Folder already exists</DialogTitle>
          </DialogHeader>
          <AskBody
            key={pending?.id ?? "none"}
            pending={pending}
            onAnswer={answer}
            onDisplace={() => setPending({ id: "second" })}
          />
        </DialogContent>
      </Dialog>
    </>
  );
}

function AskBody({
  pending,
  onAnswer,
  onDisplace,
}: {
  pending: { id: string } | null;
  onAnswer: (value: string) => void;
  onDisplace: () => void;
}) {
  const [touched, setTouched] = useState(false);
  return (
    <DialogFooter>
      <button type="button" aria-pressed={touched} onClick={() => setTouched(true)}>
        Choose {pending?.id ?? "none"}
      </button>
      <button type="button" onClick={onDisplace} disabled={!pending}>
        Displace
      </button>
      <button type="button" onClick={() => onAnswer("declined")} disabled={!pending}>
        Keep existing
      </button>
    </DialogFooter>
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

describe("Dialog opened from a menu item (ticket 14, D21)", () => {
  function FromMenu() {
    const [menu, setMenu] = useState(false);
    const [open, setOpen] = useState(false);
    return (
      <>
        <button type="button" id="row-menu" aria-haspopup="menu" onClick={() => setMenu(true)}>
          More actions
        </button>
        {menu ? (
          <div role="menu" aria-labelledby="row-menu">
            <button
              type="button"
              role="menuitem"
              onClick={() => {
                // The dialog opens from the item; the menu closes after it
                // (as Base UI's menu does, once the press has settled).
                setOpen(true);
                setTimeout(() => setMenu(false), 0);
              }}
            >
              Change source…
            </button>
          </div>
        ) : null}
        <Dialog open={open} onOpenChange={setOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Change source</DialogTitle>
            </DialogHeader>
          </DialogContent>
        </Dialog>
      </>
    );
  }

  it("returns focus to the menu's trigger, not the item that is gone", async () => {
    render(<FromMenu />);
    const trigger = screen.getByRole("button", { name: "More actions" });
    fireEvent.click(trigger);
    const item = screen.getByRole("menuitem", { name: "Change source…" });
    item.focus();
    fireEvent.click(item);
    await openedDialog("Change source");
    pressEscape();
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await waitFor(() => expect(document.activeElement).toBe(trigger));
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

describe("Dialog answered through a pending value that disappears as it closes", () => {
  async function openAsk() {
    const utils = render(<PendingAsk />);
    const opener = screen.getByRole("button", { name: "Enable" });
    opener.focus();
    fireEvent.click(opener);
    await openedDialog("Folder already exists");
    await waitFor(() => expect(utils.container.hasAttribute("inert")).toBe(true));
    return { ...utils, opener };
  }

  async function expectReleased(container: HTMLElement, opener: HTMLElement) {
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });
    // No closed ghost popup, no layer left behind, focus back on the opener.
    expect(document.querySelector('[data-slot="dialog-content"]')).toBeNull();
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(container.hasAttribute("inert")).toBe(false);
    expect(document.querySelector("[inert]")).toBeNull();
    expect(document.activeElement).toBe(opener);
  }

  it("Escape answers `dismissed`, releases every layer and returns focus", async () => {
    const { container, opener } = await openAsk();
    pressEscape();
    await expectReleased(container, opener);
    expect(screen.getByRole("status", { name: "answers" }).textContent).toBe("dismissed");
  });

  it("an explicit answer releases every layer and returns focus", async () => {
    const { container, opener } = await openAsk();
    fireEvent.click(screen.getByRole("button", { name: "Keep existing" }));
    await expectReleased(container, opener);
    expect(screen.getByRole("status", { name: "answers" }).textContent).toBe("declined");
  });

  it("a displacing ask keeps the popup and starts its body fresh", async () => {
    await openAsk();
    const popup = screen.getByRole("dialog", { name: "Folder already exists" });
    fireEvent.click(screen.getByRole("button", { name: "Choose first" }));
    expect(
      screen.getByRole("button", { name: "Choose first" }).getAttribute("aria-pressed"),
    ).toBe("true");

    fireEvent.click(screen.getByRole("button", { name: "Displace" }));

    const next = await screen.findByRole("button", { name: "Choose second" });
    expect(next.getAttribute("aria-pressed")).toBe("false");
    expect(screen.getByRole("dialog", { name: "Folder already exists" })).toBe(popup);
  });
});

describe("ConfirmDialog", () => {
  function Ask({
    tone,
    initialFocus,
    onCancel,
    onConfirm,
  }: {
    tone: "accent" | "danger";
    initialFocus?: "confirm" | "cancel";
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
        initialFocus={initialFocus}
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

  it("puts focus on Cancel for a large non-destructive question (initialFocus cancel)", async () => {
    render(<Ask tone="accent" initialFocus="cancel" onCancel={() => {}} onConfirm={() => {}} />);
    await openedDialog("Overwrite?");
    await waitFor(() =>
      expect(document.activeElement).toBe(screen.getByRole("button", { name: "Keep existing" })),
    );
  });
});

describe("Dialog placed as a sheet", () => {
  function Sheet() {
    const [open, setOpen] = useState(false);
    return (
      <>
        <button type="button" onClick={() => setOpen(true)}>
          Open docx
        </button>
        <Dialog open={open} onOpenChange={setOpen}>
          <DialogContent placement="sheet" showCloseButton={false} aria-labelledby="sheet-heading">
            <h2 id="sheet-heading">docx</h2>
            <button type="button">Restore</button>
          </DialogContent>
        </Dialog>
      </>
    );
  }

  it("keeps the dialog contract: named by its own heading, Escape closes, focus returns", async () => {
    render(<Sheet />);
    const trigger = screen.getByRole("button", { name: "Open docx" });
    trigger.focus();
    fireEvent.click(trigger);
    const sheet = await openedDialog("docx");
    expect(sheet.dataset.placement).toBe("sheet");
    pressEscape();
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await waitFor(() => expect(document.activeElement).toBe(trigger));
  });
});

describe("Dialog return targets are tested, and a layer closing beneath another hands over (ticket 12 I1)", () => {
  /**
   * The inspector's shape: a sheet opened from a row; inside it a repair
   * opens a confirmation whose action removes the repair. The fallback chain
   * is row → sheet heading; the sheet stays open, so the row stays inert.
   */
  function SheetWithRepair() {
    const [sheet, setSheet] = useState(false);
    const [confirm, setConfirm] = useState(false);
    const [repairable, setRepairable] = useState(true);
    const row = useRef<HTMLButtonElement>(null);
    const heading = useRef<HTMLHeadingElement>(null);
    return (
      <>
        <button ref={row} type="button" onClick={() => setSheet(true)}>
          local-notes
        </button>
        <Dialog open={sheet} onOpenChange={setSheet}>
          <DialogContent placement="sheet" showCloseButton={false} aria-labelledby="repair-sheet-heading">
            <h2 ref={heading} id="repair-sheet-heading" tabIndex={-1}>
              local-notes
            </h2>
            {repairable ? (
              <button type="button" onClick={() => setConfirm(true)}>
                Detach
              </button>
            ) : null}
          </DialogContent>
        </Dialog>
        <Dialog open={confirm} onOpenChange={setConfirm}>
          <DialogContent
            fallbackFocus={(usable) => [row.current, heading.current].find((el) => el && usable(el)) ?? null}
          >
            <DialogHeader>
              <DialogTitle>Detach local-notes?</DialogTitle>
            </DialogHeader>
            <DialogFooter>
              <button
                type="button"
                onClick={() => {
                  setRepairable(false);
                  setConfirm(false);
                }}
              >
                Confirm detach
              </button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </>
    );
  }

  it("skips a fallback the remaining sheet keeps inert: focus lands on the sheet's heading, never the row", async () => {
    render(<SheetWithRepair />);
    fireEvent.click(screen.getByRole("button", { name: "local-notes" }));
    await openedDialog("local-notes");
    const repair = screen.getByRole("button", { name: "Detach" });
    repair.focus();
    fireEvent.click(repair);
    await openedDialog("Detach local-notes?");

    fireEvent.click(screen.getByRole("button", { name: "Confirm detach" }));

    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Detach local-notes?" })).toBeNull());
    await waitFor(() => expect(document.activeElement).toBe(screen.getByRole("heading", { name: "local-notes" })));
    expect(screen.getByRole("dialog", { name: "local-notes" })).toBeTruthy();
  });

  /**
   * Re-point's shape: the dialog's Continue raises a progress layer above it;
   * the action closes the dialog while that layer is still up, removing the
   * control that opened the dialog; then the progress layer closes.
   */
  function ActionUnderProgress() {
    const [source, setSource] = useState(true);
    const [dialog, setDialog] = useState(false);
    const [progress, setProgress] = useState(false);
    const heading = useRef<HTMLHeadingElement>(null);
    return (
      <>
        <h2 ref={heading} tabIndex={-1}>
          deprecated-helper
        </h2>
        {source ? (
          <button type="button" onClick={() => setDialog(true)}>
            Change source
          </button>
        ) : null}
        <Dialog open={dialog} onOpenChange={setDialog}>
          <DialogContent fallbackFocus={(usable) => (heading.current && usable(heading.current) ? heading.current : null)}>
            <DialogHeader>
              <DialogTitle>Change source</DialogTitle>
            </DialogHeader>
            <DialogFooter>
              <button type="button" onClick={() => setProgress(true)}>
                Continue
              </button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
        <Dialog open={progress} onOpenChange={() => {}} dismissible={false}>
          <DialogContent showCloseButton={false}>
            <DialogHeader>
              <DialogTitle>Working</DialogTitle>
            </DialogHeader>
            <DialogFooter>
              <button
                type="button"
                onClick={() => {
                  setSource(false);
                  setDialog(false);
                }}
              >
                Settle
              </button>
              <button type="button" onClick={() => setProgress(false)}>
                Finish
              </button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </>
    );
  }

  it("the dialog beneath moves no focus as it closes; the layer above returns it by the dialog's chain", async () => {
    render(<ActionUnderProgress />);
    const opener = screen.getByRole("button", { name: "Change source" });
    opener.focus();
    fireEvent.click(opener);
    await openedDialog("Change source");
    const cont = screen.getByRole("button", { name: "Continue" });
    cont.focus();
    fireEvent.click(cont);
    const progress = await openedDialog("Working");

    const settle = screen.getByRole("button", { name: "Settle" });
    settle.focus();
    fireEvent.click(settle);
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Change source" })).toBeNull());
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    // Nothing stole focus out of the layer still on top.
    expect(progress.contains(document.activeElement)).toBe(true);

    fireEvent.click(screen.getByRole("button", { name: "Finish" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await waitFor(() => expect(document.activeElement).toBe(screen.getByRole("heading", { name: "deprecated-helper" })));
  });
});

describe("Dialog interceptEscape (ticket 12 I6)", () => {
  function Expandable() {
    const [open, setOpen] = useState(true);
    const [expanded, setExpanded] = useState(true);
    return (
      <Dialog
        open={open}
        onOpenChange={setOpen}
        interceptEscape={() => {
          if (!expanded) return false;
          setExpanded(false);
          return true;
        }}
      >
        <DialogContent placement="sheet" aria-labelledby="files-heading">
          <h2 id="files-heading">Files</h2>
          <button type="button">{expanded ? "Collapse" : "Expand"}</button>
        </DialogContent>
      </Dialog>
    );
  }

  it("spends the first Escape on the layer's own step and stays open; the next one dismisses", async () => {
    render(<Expandable />);
    await openedDialog("Files");
    pressEscape();
    await waitFor(() => expect(screen.getByRole("button", { name: "Expand" })).toBeTruthy());
    expect(screen.getByRole("dialog", { name: "Files" })).toBeTruthy();
    pressEscape();
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });
});

describe("A confirmation raised from inside a popover in a dialog (ticket 16 A1)", () => {
  /**
   * Add's shape: a dialog holds a popover of checkboxes; toggling a
   * shared-folder Tool raises a confirmation that is a sibling of the dialog
   * (the surface stack's ask), in its own portal. One key dismisses one
   * layer, and each layer returns focus to the control that raised it.
   */
  function PopoverAsk() {
    const [add, setAdd] = useState(false);
    const [ask, setAsk] = useState(false);
    const [amp, setAmp] = useState(false);
    return (
      <>
        <button type="button" onClick={() => setAdd(true)}>
          Add skill
        </button>
        <Dialog open={add} onOpenChange={setAdd}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Add</DialogTitle>
            </DialogHeader>
            <DialogBody>
              <Popover>
                <PopoverTrigger>Edit Tools</PopoverTrigger>
                <PopoverContent>
                  <PopoverTitle>Tool scope</PopoverTitle>
                  <button type="button" aria-pressed={amp} onClick={() => setAsk(true)}>
                    Amp
                  </button>
                  <button type="button">Cursor</button>
                </PopoverContent>
              </Popover>
            </DialogBody>
          </DialogContent>
        </Dialog>
        <ConfirmDialog
          open={ask}
          title="Shared skills directory"
          confirmLabel="Continue"
          cancelLabel="Cancel"
          onConfirm={() => {
            setAmp((value) => !value);
            setAsk(false);
          }}
          onCancel={() => setAsk(false)}
        />
      </>
    );
  }

  /** A pointer press as the browser sends it: pointerdown, mousedown, click. */
  function press(element: HTMLElement) {
    fireEvent.pointerDown(element);
    fireEvent.mouseDown(element);
    fireEvent.pointerUp(element);
    fireEvent.mouseUp(element);
    fireEvent.click(element);
  }

  async function openAsk() {
    render(<PopoverAsk />);
    const opener = screen.getByRole("button", { name: "Add skill" });
    opener.focus();
    fireEvent.click(opener);
    await openedDialog("Add");
    const trigger = screen.getByRole("button", { name: "Edit Tools" });
    trigger.focus();
    fireEvent.click(trigger);
    const popover = await screen.findByRole("dialog", { name: "Tool scope" });
    const amp = within(popover).getByRole("button", { name: "Amp" });
    amp.focus();
    fireEvent.click(amp);
    const ask = await openedDialog("Shared skills directory");
    return { opener, trigger, popover, amp, ask };
  }

  async function unwind(opener: HTMLElement, trigger: HTMLElement, amp: HTMLElement) {
    // The popover outlived the ask, and focus is back on the control that raised it.
    expect(screen.getByRole("dialog", { name: "Tool scope" })).toBeTruthy();
    await waitFor(() => expect(document.activeElement).toBe(amp));
    expect(amp.closest("[inert]")).toBeNull();
    // Tab moves on inside it: nothing the ask's focus-out took out of the tab order stays out.
    const popover = screen.getByRole("dialog", { name: "Tool scope" });
    for (const control of within(popover).getAllByRole("button")) {
      expect(control.getAttribute("tabindex")).not.toBe("-1");
    }
    // Next Escape: the popover alone → its trigger.
    pressEscape();
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Tool scope" })).toBeNull());
    await waitFor(() => expect(document.activeElement).toBe(trigger));
    expect(screen.getByRole("dialog", { name: "Add" })).toBeTruthy();
    // Next Escape: Add → its opener.
    pressEscape();
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await waitFor(() => expect(document.activeElement).toBe(opener));
  }

  it("makes the popover inert beneath the confirmation", async () => {
    const { popover } = await openAsk();
    await waitFor(() => expect(popover.closest("[inert]")).not.toBeNull());
  });

  it("Escape closes only the confirmation; then the popover; then the dialog", async () => {
    const { opener, trigger, amp } = await openAsk();
    pressEscape();
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Shared skills directory" })).toBeNull());
    expect(amp.getAttribute("aria-pressed")).toBe("false");
    await unwind(opener, trigger, amp);
  });

  it("an explicit Cancel keeps the popover and returns focus to the checkbox", async () => {
    const { opener, trigger, amp, ask } = await openAsk();
    press(within(ask).getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Shared skills directory" })).toBeNull());
    expect(amp.getAttribute("aria-pressed")).toBe("false");
    await unwind(opener, trigger, amp);
  });

  it("Continue applies the answer, keeps the popover and returns focus to the checkbox", async () => {
    const { opener, trigger, amp, ask } = await openAsk();
    press(within(ask).getByRole("button", { name: "Continue" }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Shared skills directory" })).toBeNull());
    expect(amp.getAttribute("aria-pressed")).toBe("true");
    await unwind(opener, trigger, amp);
  });
});
