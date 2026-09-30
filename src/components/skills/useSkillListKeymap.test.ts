// My Skills' keymap at the hook level: the real surface module, a DOM list
// of rows, trusted-shaped keydown events on window.

import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useSurfaceState } from "../../hooks/useSurfaceState";
import { inListScope, useSkillListKeymap } from "./useSkillListKeymap";

const ORDER = ["a", "b", "c", "d"];

function mountRows(ids = ORDER) {
  document.body.innerHTML = `
    <main><div data-skill-surface="">
      <input data-skill-search="" />
      <div data-skill-list="">
        ${ids.map((id) => `<li data-skill-item="${id}"><button data-skill-row="${id}">${id}</button><button data-switch="${id}">on</button></li>`).join("")}
      </div>
      <aside data-inspector="c">
        <h2 tabindex="-1" data-inspector-heading="c">c</h2>
        <button data-files-toggle="">Collapse files</button>
        <div tabindex="0" role="region" data-skill-reader=""><p><a href="#x" data-reader-link="">link</a> text</p></div>
      </aside>
    </div></main>
    <nav><button data-sidebar="">Skills</button></nav>`;
}
const reader = () => document.querySelector<HTMLElement>("[data-skill-reader]")!;
const row = (id: string) => document.querySelector<HTMLElement>(`[data-skill-row="${id}"]`)!;

function press(key: string, init: KeyboardEventInit = {}, target: EventTarget = document.activeElement ?? document.body) {
  act(() => {
    target.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...init }));
  });
}

function setup(options: { focusedSkillId?: string | null; order?: string[]; listCovered?: boolean } = {}) {
  const onEscape = vi.fn();
  const rendered = renderHook(() => {
    const surface = useSurfaceState();
    const keymap = useSkillListKeymap({
      enabled: true,
      listCovered: options.listCovered ?? false,
      order: options.order ?? ORDER,
      surface,
      focusedSkillId: options.focusedSkillId ?? null,
      onEscape,
    });
    return { surface, keymap };
  });
  const selected = () => [...rendered.result.current.surface.selection.skills];
  return { ...rendered, onEscape, selected };
}

beforeEach(() => {
  window.location.hash = "#/skills";
  mountRows();
});
afterEach(() => {
  cleanup();
  document.body.innerHTML = "";
});

describe("j / k", () => {
  it("move focus down and up the rows, from the first when focus is in none", () => {
    setup();
    press("j");
    expect(document.activeElement).toBe(row("a"));
    press("j");
    press("j");
    expect(document.activeElement).toBe(row("c"));
    press("k");
    expect(document.activeElement).toBe(row("b"));
    // From another control of a row, the row's own neighbour.
    document.querySelector<HTMLElement>('[data-switch="d"]')!.focus();
    press("k");
    expect(document.activeElement).toBe(row("c"));
  });

  it("start from the inspector's skill when focus is in no row", () => {
    setup({ focusedSkillId: "c" });
    press("j");
    expect(document.activeElement).toBe(row("c"));
  });
});

describe("x, ⌘A, Escape", () => {
  it("x toggles the row holding focus; Escape clears first, then takes the surface's next step", () => {
    const { selected, onEscape } = setup();
    row("b").focus();
    press("x");
    expect(selected()).toEqual(["b"]);
    press("x");
    expect(selected()).toEqual([]);
    press("x");
    press("Escape");
    expect(selected()).toEqual([]);
    expect(onEscape).not.toHaveBeenCalled();
    press("Escape");
    expect(onEscape).toHaveBeenCalledTimes(1);
  });

  it("⌘A / Ctrl+A select exactly the visible subset the lens shows", () => {
    const { selected } = setup({ order: ["b", "d"] });
    row("a").focus();
    press("a", { metaKey: true });
    expect(selected()).toEqual(["b", "d"]);
    act(() => undefined);
    press("Escape");
    press("a", { ctrlKey: true });
    expect(selected()).toEqual(["b", "d"]);
  });

  it("/ focuses the search", () => {
    setup();
    press("/");
    expect(document.activeElement).toBe(document.querySelector("[data-skill-search]"));
  });
});

describe("never while typing, never under a layer", () => {
  it("ignores keys typed into a field, and every key while a dialog or menu is open", () => {
    const { selected } = setup();
    const search = document.querySelector<HTMLInputElement>("[data-skill-search]")!;
    search.focus();
    press("j");
    press("a", { metaKey: true });
    expect(document.activeElement).toBe(search);
    expect(selected()).toEqual([]);
    const dialog = document.createElement("div");
    dialog.setAttribute("role", "dialog");
    document.body.append(dialog);
    row("a").focus();
    press("x");
    press("j");
    expect(selected()).toEqual([]);
    expect(document.activeElement).toBe(row("a"));
  });
});

describe("the inspector and its reader keep their keys (audit A1)", () => {
  it.each([
    ["the reader region", () => reader()],
    ["a reader descendant", () => document.querySelector<HTMLElement>("[data-reader-link]")!],
    ["the inspector's heading", () => document.querySelector<HTMLElement>("[data-inspector-heading]")!],
    ["outside My Skills (the sidebar)", () => document.querySelector<HTMLElement>("[data-sidebar]")!],
  ])("from %s, j / k / x / slash / ⌘A leave the list alone and ⌘A stays native", (_, focusable) => {
    const { result, selected } = setup({ focusedSkillId: "c" });
    act(() => result.current.keymap.selectFromRow("c", false));
    const origin = focusable();
    origin.focus();
    for (const key of ["x", "j", "k", "/"]) press(key, {}, origin);
    expect(document.activeElement).toBe(origin);
    expect(selected()).toEqual(["c"]);
    let native = true;
    act(() => {
      const event = new KeyboardEvent("keydown", { key: "a", metaKey: true, bubbles: true, cancelable: true });
      native = origin.dispatchEvent(event);
    });
    expect(native).toBe(true);
    expect(selected()).toEqual(["c"]);
    expect(document.activeElement).toBe(origin);
  });

  it("Escape still runs the hierarchy from the reader: the selection first, then the surface's next step", () => {
    const { result, selected, onEscape } = setup({ focusedSkillId: "c" });
    act(() => result.current.keymap.selectFromRow("b", false));
    reader().focus();
    press("Escape");
    expect(selected()).toEqual([]);
    expect(onEscape).not.toHaveBeenCalled();
    expect(document.activeElement).toBe(reader());
    press("Escape");
    expect(onEscape).toHaveBeenCalledTimes(1);
  });

  it("inListScope: rows, the rail and nothing-focused are the list's; the inspector, the reader, a dialog and the sidebar are not", () => {
    expect(inListScope(row("a"))).toBe(true);
    expect(inListScope(document.body)).toBe(true);
    expect(inListScope(document.querySelector("main"))).toBe(true);
    expect(inListScope(window)).toBe(true);
    expect(inListScope(reader())).toBe(false);
    expect(inListScope(document.querySelector("[data-reader-link]"))).toBe(false);
    expect(inListScope(document.querySelector("[data-inspector-heading]"))).toBe(false);
    expect(inListScope(document.querySelector("[data-sidebar]"))).toBe(false);
    const dialog = document.createElement("div");
    dialog.setAttribute("role", "dialog");
    const inside = document.createElement("button");
    dialog.append(inside);
    document.querySelector("[data-skill-surface]")!.append(dialog);
    expect(inListScope(inside)).toBe(false);
  });
});

describe("expanded Files cover the list (audit A2)", () => {
  it("row shortcuts are refused against the covered list while Escape keeps running", () => {
    const { result, selected, onEscape } = setup({ focusedSkillId: "c", listCovered: true });
    act(() => result.current.keymap.selectFromRow("b", false));
    const toggle = document.querySelector<HTMLElement>("[data-files-toggle]")!;
    toggle.focus();
    for (const key of ["j", "k", "x", "/"]) press(key);
    press("a", { metaKey: true });
    expect(document.activeElement).toBe(toggle);
    expect(selected()).toEqual(["b"]);
    // Even from nothing-focused, never a row of the inert list.
    act(() => toggle.blur());
    press("j", {}, document.body);
    expect(document.activeElement).toBe(document.body);
    // Escape: the selection first, then the surface's next step (collapse Files).
    toggle.focus();
    press("Escape");
    expect(selected()).toEqual([]);
    expect(onEscape).not.toHaveBeenCalled();
    press("Escape");
    expect(onEscape).toHaveBeenCalledTimes(1);
  });
});

describe("pointer selection shares the anchor", () => {
  it("a shift-click selects the range from the last row toggled; a group header its source", () => {
    const { result, selected } = setup();
    act(() => result.current.keymap.selectFromRow("b", false));
    act(() => result.current.keymap.selectFromRow("d", true));
    expect(selected().sort()).toEqual(["b", "c", "d"]);
    // A range upward from the new anchor keeps what is selected.
    act(() => result.current.keymap.selectFromRow("a", true));
    expect(selected().sort()).toEqual(["a", "b", "c", "d"]);
    act(() => result.current.keymap.selectGroup(["a", "b"], false));
    expect(selected().sort()).toEqual(["c", "d"]);
    act(() => result.current.keymap.selectGroup(["a"], true));
    expect(selected().sort()).toEqual(["a", "c", "d"]);
  });
});

describe("the rail leaving with the selection", () => {
  it("clearing from inside the rail hands focus to the row last toggled, never to body", () => {
    const { result, selected } = setup();
    act(() => result.current.keymap.selectFromRow("c", false));
    const rail = document.createElement("div");
    rail.dataset.bulkRail = "";
    const disable = document.createElement("button");
    disable.setAttribute("aria-disabled", "true");
    rail.append(disable);
    document.body.append(rail);
    disable.focus();
    // A disabled-but-focusable control consumed the key to block activation: still ours.
    act(() => {
      const event = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true });
      event.preventDefault();
      disable.dispatchEvent(event);
    });
    expect(selected()).toEqual([]);
    expect(document.activeElement).toBe(row("c"));
  });
});
