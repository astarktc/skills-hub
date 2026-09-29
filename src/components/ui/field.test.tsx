// Field's contract (round-19 D2): the label is associated with its control,
// helper and error text describe it, invalid is aria-invalid.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
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
});
