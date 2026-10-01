// useCandidatePick — one picker's logic (ticket 16): the listing and its
// context, the selection, the search, and the visible-and-checked rule
// (#47). The surface, the install and its report are the Add flow's
// (useAddSkillFlow.test.ts).

import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { PickerCandidate } from "../lib/addFlow";
import { useCandidatePick } from "./useCandidatePick";

type Candidate = { name: string; subpath: string; description: string | null; valid: boolean };

const candidate = (name: string, valid = true, description: string | null = null): Candidate => ({
  name,
  subpath: `skills/${name}`,
  description,
  valid,
});

const toPicker = (c: Candidate): PickerCandidate => ({
  id: c.subpath,
  name: c.name,
  description: c.description,
  subpath: c.subpath,
  unavailable: c.valid ? null : "invalid",
});

function render() {
  return renderHook(() => useCandidatePick<Candidate, string>({ toPicker }));
}

const LISTING = [candidate("alpha"), candidate("beta", true, "a linter"), candidate("broken", false), candidate("gamma")];

describe("useCandidatePick", () => {
  it("starts empty, without a context", () => {
    const { result } = render();
    expect(result.current.ctx).toBeNull();
    expect(result.current.candidates).toEqual([]);
    expect(result.current.summary).toEqual({ total: 0, checked: 0, hidden: 0, hiddenChecked: 0 });
  });

  it("open keeps the context and checks every selectable candidate", () => {
    const { result } = render();
    act(() => result.current.open("https://github.com/x/y", LISTING));
    expect(result.current.ctx).toBe("https://github.com/x/y");
    expect(result.current.selected).toEqual({
      "skills/alpha": true,
      "skills/beta": true,
      "skills/broken": false,
      "skills/gamma": true,
    });
    expect(result.current.picked.map((c) => c.name)).toEqual(["alpha", "beta", "gamma"]);
  });

  it("a preselection checks only what it names (and never an unavailable one)", () => {
    const { result } = render();
    act(() => result.current.open("ctx", LISTING, ["skills/beta", "skills/broken"]));
    expect(result.current.picked.map((c) => c.name)).toEqual(["beta"]);
    expect(result.current.selected["skills/broken"]).toBe(false);
  });

  it("an unavailable candidate cannot be checked", () => {
    const { result } = render();
    act(() => result.current.open("ctx", LISTING));
    act(() => result.current.toggle("skills/broken", true));
    expect(result.current.selected["skills/broken"]).toBe(false);
  });

  it("the search hides candidates; only checked AND visible ones are picked", () => {
    const { result } = render();
    act(() => result.current.open("ctx", LISTING));
    // Matches the description too.
    act(() => result.current.setQuery("LINT"));
    expect(result.current.visible.map((c) => c.name)).toEqual(["beta"]);
    expect(result.current.picked.map((c) => c.name)).toEqual(["beta"]);
    expect(result.current.summary).toEqual({ total: 4, checked: 1, hidden: 3, hiddenChecked: 2 });
    act(() => result.current.setQuery(""));
    expect(result.current.picked.map((c) => c.name)).toEqual(["alpha", "beta", "gamma"]);
  });

  it("toggleAll touches only the visible, selectable candidates", () => {
    const { result } = render();
    act(() => result.current.open("ctx", LISTING));
    act(() => result.current.setQuery("a"));
    // alpha, beta (description), gamma; broken hidden? "broken" has no "a".
    act(() => result.current.toggleAll(false));
    expect(result.current.selected).toEqual({
      "skills/alpha": false,
      "skills/beta": false,
      "skills/broken": false,
      "skills/gamma": false,
    });
    act(() => result.current.setQuery("gamma"));
    act(() => result.current.toggleAll(true));
    act(() => result.current.setQuery(""));
    expect(result.current.picked.map((c) => c.name)).toEqual(["gamma"]);
  });

  it("a new listing clears the search; reset forgets everything", () => {
    const { result } = render();
    act(() => result.current.open("one", LISTING));
    act(() => result.current.setQuery("beta"));
    act(() => result.current.open("two", [candidate("delta")]));
    expect(result.current.query).toBe("");
    expect(result.current.ctx).toBe("two");
    act(() => result.current.reset());
    expect(result.current.ctx).toBeNull();
    expect(result.current.selected).toEqual({});
  });
});
