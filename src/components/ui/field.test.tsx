// Field's contract (round-19 D2): the label is associated with its control,
// helper and error text describe it, invalid is aria-invalid.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { Checkbox } from "./checkbox";
import {
  Field,
  FieldDescription,
  FieldError,
  FieldLabel,
  Input,
  Select,
} from "./field";

afterEach(cleanup);

describe("Field", () => {
  it("associates the label with the input, so the placeholder is never the name", () => {
    render(
      <Field>
        <FieldLabel>Repository URL</FieldLabel>
        <Input placeholder="https://github.com/…" />
      </Field>,
    );
    const input = screen.getByRole("textbox", { name: "Repository URL" });
    expect(input.getAttribute("placeholder")).toBe("https://github.com/…");
  });

  it("describes the control by its helper text", () => {
    render(
      <Field>
        <FieldLabel>Name</FieldLabel>
        <Input />
        <FieldDescription>Optional — defaults to the folder name.</FieldDescription>
      </Field>,
    );
    const input = screen.getByRole("textbox", { name: "Name" });
    const helper = screen.getByText("Optional — defaults to the folder name.");
    expect(input.getAttribute("aria-describedby")).toBe(helper.id);
    expect(input.getAttribute("aria-invalid")).toBeNull();
  });

  it("marks an invalid control and adds the error to its description", () => {
    render(
      <Field invalid>
        <FieldLabel>URL</FieldLabel>
        <Input />
        <FieldDescription>A git URL.</FieldDescription>
        <FieldError>Not a git URL.</FieldError>
      </Field>,
    );
    const input = screen.getByRole("textbox", { name: "URL" });
    const helper = screen.getByText("A git URL.");
    const error = screen.getByRole("alert");
    expect(input.getAttribute("aria-invalid")).toBe("true");
    expect(input.getAttribute("aria-describedby")?.split(" ")).toEqual([
      helper.id,
      error.id,
    ]);
  });

  it("renders no error without a message", () => {
    render(
      <Field>
        <FieldLabel>URL</FieldLabel>
        <Input />
        <FieldError>{null}</FieldError>
      </Field>,
    );
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("associates the label with a native select", () => {
    render(
      <Field>
        <FieldLabel>Sort</FieldLabel>
        <Select defaultValue="name">
          <option value="name">Name</option>
          <option value="updated">Updated</option>
        </Select>
      </Field>,
    );
    expect(screen.getByRole("combobox", { name: "Sort" })).toBeTruthy();
  });

  it("follows an explicit control id: the label points at the id the input renders", () => {
    render(
      <Field>
        <FieldLabel>URL</FieldLabel>
        <Input id="repo-url" />
      </Field>,
    );
    const input = screen.getByRole("textbox", { name: "URL" }) as HTMLInputElement;
    expect(input.id).toBe("repo-url");
    expect(input.labels?.length).toBe(1);
    expect(input.labels?.[0].getAttribute("for")).toBe("repo-url");
  });

  it("labels the generated-id input exactly once", () => {
    render(
      <Field>
        <FieldLabel>Name</FieldLabel>
        <Input />
      </Field>,
    );
    const input = screen.getByRole("textbox", { name: "Name" }) as HTMLInputElement;
    expect(input.labels?.length).toBe(1);
  });

  it("names a Checkbox composed inside a Field from the FieldLabel", () => {
    render(
      <Field>
        <FieldLabel>Cursor</FieldLabel>
        <Checkbox />
      </Field>,
    );
    const box = screen.getByRole("checkbox", { name: "Cursor" });
    const labelId = box.getAttribute("aria-labelledby");
    expect(labelId).toBeTruthy();
    expect(document.getElementById(labelId ?? "")?.textContent).toBe("Cursor");
  });

  it("names a Checkbox with an explicit id inside a Field", () => {
    render(
      <Field>
        <FieldLabel>Pi</FieldLabel>
        <Checkbox id="tool-pi" />
      </Field>,
    );
    expect(screen.getByRole("checkbox", { name: "Pi" })).toBeTruthy();
    expect(screen.getByText("Pi").getAttribute("for")).toBe("tool-pi");
  });
});
