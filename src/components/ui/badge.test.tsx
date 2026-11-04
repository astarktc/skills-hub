// The identity tone's contract (round-19 ticket 30): every hue the type admits
// renders a chip whose classes compile, against the real theme.css, to that
// hue's tint and text — and both themes define both values. A class string is
// not proof: a hue missing from `@theme inline` generates no rule at all.

import { cleanup, render, screen } from "@testing-library/react";
import { compile } from "tailwindcss";
import { afterEach, describe, expect, it } from "vitest";
import themeCss from "@/styles/theme.css?raw";
import { Badge } from "./badge";
import { IDENTITY_HUES } from "./styles";

afterEach(cleanup);

const blockOf = (selector: string) => {
  const start = themeCss.indexOf(`${selector} {`);
  expect(start).toBeGreaterThanOrEqual(0);
  return themeCss.slice(start, themeCss.indexOf("\n}", start));
};

async function compiled(classList: string[]) {
  const compiler = await compile(`${themeCss}\n@tailwind utilities;`);
  return compiler.build(classList);
}

describe("Badge tone=identity", () => {
  it.each(IDENTITY_HUES)("draws %s as its tint with its foreground, chip-sized", async (hue) => {
    render(
      <Badge tone="identity" hue={hue} data-testid="chip">
        AN
      </Badge>,
    );
    const chip = screen.getByTestId("chip");
    const classes = chip.className.split(/\s+/);
    expect(classes).toEqual(
      expect.arrayContaining([`bg-identity-${hue}`, `text-identity-${hue}-foreground`, "size-5.5", "text-2xs"]),
    );
    expect(chip.hasAttribute("hue")).toBe(false);
    const css = await compiled(classes);
    expect(css).toMatch(new RegExp(`\\.bg-identity-${hue}\\s*\\{\\s*background-color:\\s*var\\(--identity-${hue}\\)`));
    expect(css).toMatch(
      new RegExp(`\\.text-identity-${hue}-foreground\\s*\\{\\s*color:\\s*var\\(--identity-${hue}-foreground\\)`),
    );
  });

  it.each(IDENTITY_HUES)("defines %s in the dark and the light theme", (hue) => {
    for (const block of [blockOf(":root"), blockOf(":root[data-theme='light']")]) {
      expect(block).toMatch(new RegExp(`--identity-${hue}: #[0-9a-f]{6};`));
      expect(block).toMatch(new RegExp(`--identity-${hue}-foreground: #[0-9a-f]{6};`));
    }
  });

  it("admits identity only as a soft chip (type contract)", () => {
    render(
      <>
        {/* @ts-expect-error identity has no outline weight */}
        <Badge tone="identity" hue="teal" variant="outline" />
        {/* @ts-expect-error identity needs a hue */}
        <Badge tone="identity" />
        {/* @ts-expect-error a hue belongs to identity alone */}
        <Badge tone="stale" hue="teal" />
        <Badge tone="neutral" size="chip" data-testid="neutral-chip" />
      </>,
    );
    // a neutral chip (Individual sources, a local folder) is the same square
    expect(screen.getByTestId("neutral-chip").className).toContain("size-5.5");
  });
});
