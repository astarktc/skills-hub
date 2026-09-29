// The focus ring must actually draw (round-19 ticket 07, review B1). A class
// string is not proof: Tailwind 4's `outline-none` sets
// `--tw-outline-style: none` and `focus-visible:outline-2` draws
// `outline-style: var(--tw-outline-style)`, so a ring spelled without an
// explicit style computes to `none`. This test renders every focusable
// primitive, compiles the classes it actually carries with the installed
// Tailwind, and resolves the focus-visible cascade from that CSS.
// (jsdom has no :focus-visible cascade; the rendered-page proof is in the
// ticket's Comments.)

import {
  cleanup,
  fireEvent,
  render,
  screen,
} from "@testing-library/react";
import { compile } from "tailwindcss";
import { afterEach, describe, expect, it } from "vitest";
import { Chip } from "./badge";
import { Button } from "./button";
import { Checkbox } from "./checkbox";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "./dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./dropdown-menu";
import { Input, Select, Textarea } from "./field";
import { Segmented, SegmentedItem } from "./segmented";
import { Switch } from "./switch";
import { Tabs, TabsList, TabsTrigger } from "./tabs";

afterEach(cleanup);

// ---------------------------------------------------------------------------
// A minimal cascade over Tailwind's compiled (nested) CSS: enough to resolve
// `outline-*` for one element carrying a set of classes, focused or not.

type Rule = { selector: string; declarations: [string, string][]; conditional: boolean };

function parseCss(css: string) {
  const rules: Rule[] = [];
  const initialValues = new Map<string, string>();
  const text = css.replace(/\/\*[\s\S]*?\*\//g, "");
  const walk = (body: string, parentSelector: string | null, conditional: boolean) => {
    let i = 0;
    let pending = "";
    const declarations: [string, string][] = [];
    const flushDeclarations = (chunk: string) => {
      for (const part of chunk.split(";")) {
        const colon = part.indexOf(":");
        if (colon > 0 && !part.trim().startsWith("@")) {
          declarations.push([part.slice(0, colon).trim(), part.slice(colon + 1).trim()]);
        }
      }
    };
    while (i < body.length) {
      const ch = body[i];
      if (ch === "{") {
        let depth = 1;
        let j = i + 1;
        while (j < body.length && depth > 0) {
          if (body[j] === "{") depth++;
          else if (body[j] === "}") depth--;
          j++;
        }
        const inner = body.slice(i + 1, j - 1);
        const lastSemicolon = pending.lastIndexOf(";");
        flushDeclarations(pending.slice(0, lastSemicolon + 1));
        const prelude = pending.slice(lastSemicolon + 1).trim();
        pending = "";
        if (prelude.startsWith("@property")) {
          const name = prelude.split(/\s+/)[1];
          const initial = /initial-value\s*:\s*([^;]+)/.exec(inner);
          if (initial) initialValues.set(name, initial[1].trim());
        } else if (prelude.startsWith("@layer")) {
          walk(inner, parentSelector, conditional);
        } else if (prelude.startsWith("@")) {
          walk(inner, parentSelector, true);
        } else {
          const selector =
            parentSelector === null
              ? prelude
              : prelude.includes("&")
                ? prelude.replace(/&/g, parentSelector)
                : `${parentSelector} ${prelude}`;
          walk(inner, selector, conditional);
        }
        i = j;
        continue;
      }
      pending += ch;
      i++;
    }
    flushDeclarations(pending);
    if (parentSelector !== null && declarations.length > 0) {
      rules.push({ selector: parentSelector, declarations, conditional });
    }
  };
  walk(text, null, false);
  return { rules, initialValues };
}

/** Classes and pseudo-classes of a single compound selector, or null. */
function compound(selector: string): { classes: string[]; pseudos: string[] } | null {
  const classes: string[] = [];
  const pseudos: string[] = [];
  let i = 0;
  const readName = () => {
    let name = "";
    while (i < selector.length) {
      const ch = selector[i];
      if (ch === "\\") {
        name += selector[i + 1];
        i += 2;
      } else if (/[.:#[\s>+~,]/.test(ch)) break;
      else {
        name += ch;
        i++;
      }
    }
    return name;
  };
  while (i < selector.length) {
    const ch = selector[i];
    if (ch === ".") {
      i++;
      classes.push(readName());
    } else if (ch === ":") {
      i++;
      pseudos.push(readName());
    } else return null;
  }
  return { classes, pseudos };
}

function resolveOutline(css: string, classList: string[], focusVisible: boolean) {
  const { rules, initialValues } = parseCss(css);
  const has = new Set(classList);
  const applicable: { declarations: [string, string][]; specificity: number; order: number }[] = [];
  rules.forEach((rule, order) => {
    if (rule.conditional) return;
    for (const part of rule.selector.split(",")) {
      const parsed = compound(part.trim());
      if (!parsed || parsed.classes.length === 0) continue;
      if (!parsed.classes.every((name) => has.has(name))) continue;
      const pseudosMatch = parsed.pseudos.every(
        (pseudo) => focusVisible && pseudo === "focus-visible",
      );
      if (!pseudosMatch) continue;
      applicable.push({
        declarations: rule.declarations,
        specificity: parsed.classes.length + parsed.pseudos.length,
        order,
      });
      break;
    }
  });
  applicable.sort((a, b) => a.specificity - b.specificity || a.order - b.order);
  const computed = (property: string, depth = 0): string | undefined => {
    let value: string | undefined;
    for (const rule of applicable) {
      for (const [prop, val] of rule.declarations) if (prop === property) value = val;
    }
    if (value === undefined && property.startsWith("--")) value = initialValues.get(property);
    const reference = value && /^var\((--[\w-]+)\)$/.exec(value);
    if (reference && depth < 4) return computed(reference[1], depth + 1);
    return value;
  };
  return {
    style: computed("outline-style") ?? "none",
    width: computed("outline-width"),
    color: applicable
      .flatMap((rule) => rule.declarations)
      .filter(([prop]) => prop === "outline-color")
      .map(([, value]) => value)
      .pop(),
  };
}

async function compiledFor(classList: string[]) {
  const compiler = await compile("@theme inline { --color-ring: var(--ring); }\n@tailwind utilities;");
  return compiler.build(classList);
}

async function focusedOutline(element: Element) {
  const classList = element.className.toString().split(/\s+/).filter(Boolean);
  const css = await compiledFor(classList);
  return {
    focused: resolveOutline(css, classList, true),
    rest: resolveOutline(css, classList, false),
  };
}

async function expectRing(element: Element | null) {
  expect(element).not.toBeNull();
  const { focused, rest } = await focusedOutline(element as Element);
  expect(focused).toEqual({ style: "solid", width: "2px", color: "var(--ring)" });
  expect(rest.style).toBe("none");
}

// ---------------------------------------------------------------------------

describe("the cascade resolver", () => {
  it("reproduces the defect: a ring without an explicit style computes to none", async () => {
    const classList = [
      "outline-none",
      "focus-visible:outline-2",
      "focus-visible:outline-offset-2",
      "focus-visible:outline-ring",
    ];
    expect(resolveOutline(await compiledFor(classList), classList, true).style).toBe("none");
    const fixed = [...classList, "focus-visible:outline-solid"];
    expect(resolveOutline(await compiledFor(fixed), fixed, true).style).toBe("solid");
  });
});

describe("every focusable primitive draws a solid 2 px ring on focus-visible", () => {
  it("Button", async () => {
    render(<Button>Save</Button>);
    await expectRing(screen.getByRole("button", { name: "Save" }));
  });

  it("Chip", async () => {
    render(<Chip>Issues</Chip>);
    await expectRing(screen.getByRole("button", { name: "Issues" }));
  });

  it("Checkbox", async () => {
    render(<Checkbox aria-label="Cursor" />);
    await expectRing(screen.getByRole("checkbox", { name: "Cursor" }));
  });

  it("Switch", async () => {
    render(<Switch aria-label="Enabled" />);
    await expectRing(screen.getByRole("switch", { name: "Enabled" }));
  });

  it("Tabs trigger", async () => {
    render(
      <Tabs defaultValue="a">
        <TabsList>
          <TabsTrigger value="a">Overview</TabsTrigger>
        </TabsList>
      </Tabs>,
    );
    await expectRing(screen.getByRole("tab", { name: "Overview" }));
  });

  it("Segmented item", async () => {
    render(
      <Segmented aria-label="Grouping" value="grouped" onValueChange={() => {}}>
        <SegmentedItem value="grouped">Grouped</SegmentedItem>
      </Segmented>,
    );
    await expectRing(screen.getByRole("button", { name: "Grouped" }));
  });

  it("Input, Textarea and Select (1 px offset)", async () => {
    render(
      <>
        <Input aria-label="URL" />
        <Textarea aria-label="Notes" />
        <Select aria-label="Sort">
          <option>Name</option>
        </Select>
      </>,
    );
    for (const element of [
      screen.getByRole("textbox", { name: "URL" }),
      screen.getByRole("textbox", { name: "Notes" }),
      screen.getByRole("combobox", { name: "Sort" }),
    ]) {
      await expectRing(element);
      expect(element.className).toContain("focus-visible:outline-offset-1");
    }
  });

  it("Dialog Close", async () => {
    render(
      <Dialog open onOpenChange={() => {}}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Settings</DialogTitle>
          </DialogHeader>
        </DialogContent>
      </Dialog>,
    );
    await screen.findByRole("dialog", { name: "Settings" });
    await expectRing(screen.getByRole("button", { name: "close" }));
  });

  it("DropdownMenu item, drawn inset so the scrolling popup cannot clip it", async () => {
    render(
      <DropdownMenu>
        <DropdownMenuTrigger aria-label="More actions">⋯</DropdownMenuTrigger>
        <DropdownMenuContent>
          <DropdownMenuItem>Edit</DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>,
    );
    fireEvent.click(screen.getByRole("button", { name: "More actions" }));
    const item = await screen.findByRole("menuitem", { name: "Edit" });
    await expectRing(item);
    expect(item.className).toContain("focus-visible:-outline-offset-2");
  });
});
