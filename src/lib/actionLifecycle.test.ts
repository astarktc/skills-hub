// The action lifecycle's pure rules: the post-mutation focus table (D21),
// the refusal (D19), the cancellable set (D20) and the pending-subject
// readers. `resolveFocusTarget` is checked against a jsdom document.

import { afterEach, describe, expect, it } from "vitest";
import {
  CANCELLABLE_COMMANDS,
  describeRefusal,
  dialogFallback,
  focusAfter,
  isPendingSkill,
  pendingCellKeys,
  refusalFor,
  resolveFirst,
  resolveFocusTarget,
  progressActivity,
  showsHeaderProgress,
  type PendingAction,
} from "./actionLifecycle";

const t = (key: string, opts?: Record<string, unknown>) =>
  opts ? `${key} ${JSON.stringify(opts)}` : key;

const pending = (over: Partial<PendingAction> = {}): PendingAction => ({
  subject: { kind: "batch" },
  label: "Refresh",
  activity: null,
  startedAt: 1,
  cancellable: false,
  cancelling: false,
  progress: null,
  ...over,
});

describe("focusAfter (D21)", () => {
  const order = ["a", "b", "c"];
  const context = (skillId: string | null, rows = order) => ({ skillId, order: rows, surface: "skills" as const });

  it.each([
    ["Delete of a middle row → the next row", "delete", true, context("b"), { kind: "row", skillId: "c" }],
    ["Delete of the first row → the next row", "delete", true, context("a"), { kind: "row", skillId: "b" }],
    ["Delete of the last row → the previous row", "delete", true, context("c"), { kind: "row", skillId: "b" }],
    ["Delete of the only row → the surface heading", "delete", true, context("a", ["a"]), { kind: "heading", surface: "skills" }],
    ["Delete of a row the list no longer shows → the first remaining row", "delete", true, context("z"), { kind: "row", skillId: "a" }],
    ["a Delete that kept the skill → the same control", "delete", false, context("b"), { kind: "trigger" }],
    ["Disable → the same control", "disable", false, context("b"), { kind: "trigger" }],
    ["Enable → the same control", "enable", false, context("b"), { kind: "trigger" }],
    ["Retry → the same control", "retry", false, context("b"), { kind: "trigger" }],
    ["Restore → the skill's health row", "restore", false, context("b"), { kind: "health", skillId: "b" }],
  ] as const)("%s", (_name, verb, removed, ctx, expected) => {
    expect(focusAfter({ verb, removed }, ctx)).toEqual(expected);
  });

  it.each([
    ["a bulk Delete steps past every removed row to the next", ["b", "c"], ["a", "b", "c", "d"], { kind: "row", skillId: "d" }],
    ["a bulk Delete of the tail → the nearest row before the first removed", ["c", "d"], ["a", "b", "c", "d"], { kind: "row", skillId: "b" }],
    ["a bulk Delete of scattered rows → the first after the last removed", ["a", "c"], ["a", "b", "c", "d"], { kind: "row", skillId: "d" }],
    ["a bulk Delete of every row → the surface heading", ["a", "b"], ["a", "b"], { kind: "heading", surface: "skills" }],
  ] as const)("%s", (_name, removedIds, rows, expected) => {
    expect(
      focusAfter({ verb: "delete", removed: true }, { skillId: removedIds[0], removedIds, order: rows, surface: "skills" }),
    ).toEqual(expected);
  });

  it("a dialog's fallback chain is the focused row, the inspector's heading, then the surface heading", () => {
    expect(dialogFallback({ focusedSkillId: "b", surface: "skills" })).toEqual([
      { kind: "row", skillId: "b" },
      { kind: "inspector", skillId: "b" },
      { kind: "heading", surface: "skills" },
    ]);
    expect(dialogFallback({ focusedSkillId: null, surface: "projects" })).toEqual([
      { kind: "heading", surface: "projects" },
    ]);
  });
});

describe("resolveFocusTarget", () => {
  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("finds rows, health rows and headings by their data attributes", () => {
    document.body.innerHTML = `
      <main>
        <h2 data-surface-heading="skills" tabindex="-1">My Skills</h2>
        <button data-skill-row="a">a</button>
        <div data-skill-health="a" tabindex="-1"></div>
      </main>`;
    expect(resolveFocusTarget({ kind: "row", skillId: "a" })?.dataset.skillRow).toBe("a");
    expect(resolveFocusTarget({ kind: "health", skillId: "a" })?.dataset.skillHealth).toBe("a");
    expect(resolveFocusTarget({ kind: "heading", surface: "skills" })?.tagName).toBe("H2");
    expect(resolveFocusTarget({ kind: "row", skillId: "gone" })).toBeNull();
  });

  it("a health target prefers the inspector's Health section over the row's marks", () => {
    document.body.innerHTML = `
      <main>
        <div data-skill-health="a" tabindex="-1"></div>
        <aside><section data-inspector-health="a" tabindex="-1"></section></aside>
      </main>`;
    expect(resolveFocusTarget({ kind: "health", skillId: "a" })?.dataset.inspectorHealth).toBe("a");
  });

  it("a heading that is not on screen falls back to the main landmark, never body", () => {
    document.body.innerHTML = "<main></main>";
    expect(resolveFocusTarget({ kind: "heading", surface: "skills" })?.tagName).toBe("MAIN");
  });

  it("the trigger counts only while connected and enabled; a chain takes the first on screen", () => {
    document.body.innerHTML = `<button id="t" disabled>x</button><button data-skill-row="b">b</button>`;
    const trigger = document.getElementById("t");
    expect(resolveFocusTarget({ kind: "trigger" }, trigger)).toBeNull();
    expect(
      resolveFirst([{ kind: "trigger" }, { kind: "row", skillId: "b" }], trigger)?.dataset.skillRow,
    ).toBe("b");
    trigger?.removeAttribute("disabled");
    expect(resolveFocusTarget({ kind: "trigger" }, trigger)).toBe(trigger);
  });

  it("a chain walked with a closing layer's usable test skips a row the sheet keeps inert (I1)", () => {
    document.body.innerHTML = `
      <main><button data-skill-row="b">b</button></main>
      <div role="dialog" data-inspector="b"><h2 data-inspector-heading="b" tabindex="-1">b</h2></div>`;
    const chain = dialogFallback({ focusedSkillId: "b", surface: "skills" });
    // Beside the list nothing stays inert: the row.
    expect(resolveFirst(chain)?.dataset.skillRow).toBe("b");
    // Under the sheet the row stays inert: the sheet's heading.
    const behindSheet = (element: Element) => !element.closest("main");
    expect(resolveFirst(chain, null, behindSheet)?.dataset.inspectorHeading).toBe("b");
  });
});

describe("one mutation at a time (D19)", () => {
  it("names what is running, or says only that something is", () => {
    expect(refusalFor(null)).toBeNull();
    expect(describeRefusal(refusalFor(pending())!, t)).toBe('lifecycle.running {"action":"Refresh"}');
    expect(describeRefusal(refusalFor(pending({ label: null }))!, t)).toBe("lifecycle.busy");
  });
});

describe("pending subjects", () => {
  it("a skill's card reads its own pending; the matrix reads its cells", () => {
    const skill = pending({ subject: { kind: "skill", id: "s1" } });
    expect(isPendingSkill(skill, "s1")).toBe(true);
    expect(isPendingSkill(skill, "s2")).toBe(false);
    expect([...pendingCellKeys(pending({ subject: { kind: "cell", ids: ["s1:pi"] } }))]).toEqual(["s1:pi"]);
    expect(pendingCellKeys(skill).size).toBe(0);
    const several = pending({ subject: { kind: "skills", ids: ["s1", "s2"] } });
    expect(isPendingSkill(several, "s2")).toBe(true);
    expect(isPendingSkill(several, "s3")).toBe(false);
  });

  it.each([
    ["a batch", pending(), true],
    ["a streamed skill action", pending({ subject: { kind: "skill", id: "s1" }, progress: { done: 1, total: 1 } }), true],
    ["a cancellable skill action", pending({ subject: { kind: "skill", id: "s1" }, cancellable: true }), true],
    ["a small skill action (inline)", pending({ subject: { kind: "skill", id: "s1" } }), false],
    ["a cell action (inline)", pending({ subject: { kind: "cell", ids: ["s1:pi"] } }), false],
    ["a bulk Remove (its dialog shows it), even streamed", pending({ subject: { kind: "skills", ids: ["s1", "s2"] }, progress: { done: 1, total: 2 } }), false],
    ["nothing running", null, false],
  ] as const)("%s shows the header progress: %s", (_name, value, expected) => {
    expect(showsHeaderProgress(value)).toBe(expected);
  });

  it("a batch that names its skills marks each of their rows pending (D19), no other", () => {
    const refresh = pending({ subject: { kind: "batch", ids: ["s1", "s2"] } });
    expect(isPendingSkill(refresh, "s1")).toBe(true);
    expect(isPendingSkill(refresh, "s3")).toBe(false);
    expect(isPendingSkill(pending(), "s1")).toBe(false);
  });

  it("the header says the action's own words, else its name running — never one line for every batch", () => {
    expect(progressActivity(pending({ activity: "Refreshing" }), t)).toBe("Refreshing");
    expect(progressActivity(pending(), t)).toBe('activity.running {"action":"Refresh"}');
    expect(progressActivity(pending({ label: null }), t)).toBe("activity.working");
    expect(progressActivity(pending({ activity: "Refreshing", cancelling: true }), t)).toBe("lifecycle.cancelling");
  });
});

describe("the cancellable set (D20)", () => {
  it("is exactly the token-taking commands plus the Add batch", () => {
    expect([...CANCELLABLE_COMMANDS].sort()).toEqual(
      [
        "cloneExploreSkill",
        "installGitSelection",
        "installSelection",
        "previewRepointSource",
        "refreshManagedSkills",
        "repointSkillSource",
        "updateManagedSkill",
      ].sort(),
    );
  });
});
