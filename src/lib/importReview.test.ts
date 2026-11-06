import { describe, expect, it } from "vitest";
import { describeCommandError } from "../commandError";
import type {
  BatchTargetOutcome,
  ImportReport,
  OnboardingGroup,
  OnboardingPlan,
  OnboardingVariant,
} from "../components/skills/types";
import {
  defaultChoices,
  discoveryInvitation,
  failedGroups,
  importPolicy,
  importResultRows,
  importReviewModel,
  importSelections,
  importedSkillIds,
  keptByChoice,
  mergeImportReports,
  type ImportChoices,
} from "./importReview";

const t = (key: string, opts?: Record<string, unknown>) =>
  key === "common.listSeparator" ? ", " : opts ? `${key} ${JSON.stringify(opts)}` : key;
const labels = { claude_code: "Claude Code", cursor: "Cursor", pi: "Pi", codex: "Codex" };

const variant = (tool: string, name: string, overrides: Partial<OnboardingVariant> = {}): OnboardingVariant => ({
  tool,
  name,
  path: `/h/${tool}/${name}`,
  fingerprint: `${name}-body`,
  is_link: false,
  link_target: null,
  description: null,
  modified_at: null,
  same_as_managed: null,
  ...overrides,
});
const group = (name: string, variants: OnboardingVariant[], overrides: Partial<OnboardingGroup> = {}): OnboardingGroup => ({
  name,
  variants,
  has_conflict: new Set(variants.map((v) => v.fingerprint)).size > 1,
  name_is_managed: false,
  managed_skill_id: null,
  ...overrides,
});

/** first-run in miniature: a Match, a Conflict, a managed name, an invalid manifest. */
const PLAN: OnboardingPlan = {
  total_tools_scanned: 4,
  group_count: 3,
  copy_count: 6,
  groups: [
    group("ego-browser", [variant("claude_code", "ego-browser"), variant("codex", "ego-browser")]),
    group("handoff", [
      variant("claude_code", "handoff", { fingerprint: "a", description: "Compact the conversation", modified_at: 1 }),
      variant("pi", "handoff", { fingerprint: "b", description: "A short note", modified_at: 2 }),
    ]),
    group(
      "tdd",
      [variant("cursor", "tdd", { same_as_managed: true }), variant("pi", "tdd", { fingerprint: "fork", same_as_managed: false })],
      { has_conflict: true, name_is_managed: true, managed_skill_id: "tdd-id" },
    ),
  ],
  invalid: [{ tool: "cursor", name: "broken-skill", path: "/h/cursor/broken-skill", reason: "invalid_frontmatter" }],
};

const model = (choices: ImportChoices, autoDeploy: boolean, deployTools: string[] = ["claude_code", "cursor"]) =>
  importReviewModel(PLAN, choices, { t, toolLabelById: labels, autoDeploy, deployTools });

describe("discoveryInvitation (#98)", () => {
  it("invites a review of the discovered skills, counting the invalid manifests beside them", () => {
    expect(discoveryInvitation(PLAN)).toEqual({ skills: 3, invalid: 1 });
  });

  it("still invites a review when only invalid manifests remain: unmanaged copies stay visible", () => {
    expect(discoveryInvitation({ ...PLAN, group_count: 0, copy_count: 0, groups: [] })).toEqual({ skills: 0, invalid: 1 });
  });

  it("a review of invalid manifests alone lists them and refuses Import with what would make one importable", () => {
    const invalidOnly = { ...PLAN, group_count: 0, copy_count: 0, groups: [] };
    const review = importReviewModel(invalidOnly, defaultChoices(invalidOnly), {
      t,
      toolLabelById: labels,
      autoDeploy: true,
      deployTools: [],
    });
    expect(review.invalidRows.map((row) => row.name)).toEqual(["broken-skill"]);
    expect(review.action.refusal).toBe("importReview.action.nothingImportable");
  });

  it("invites nothing when the harnesses hold nothing unmanaged, or before a scan", () => {
    expect(discoveryInvitation({ ...PLAN, group_count: 0, copy_count: 0, groups: [], invalid: [] })).toBeNull();
    expect(discoveryInvitation(null)).toBeNull();
  });
});

describe("defaultChoices (D14)", () => {
  it("checks a new name whose copies agree; a Conflict, a managed name and a failed group start unchecked", () => {
    const choices = defaultChoices(PLAN);
    expect(choices.selected).toEqual({ "ego-browser": true, handoff: false, tdd: false });
    expect(choices.replace).toEqual({});
    expect(choices.removeOthers).toEqual({});
    expect(choices.removeOriginals).toBe(true);
    expect(defaultChoices(PLAN, { "ego-browser": "rate limited" }).selected["ego-browser"]).toBe(false);
  });
});

describe("importReviewModel", () => {
  it("counts skills and copies separately, the harnesses holding them and the invalid manifests", () => {
    const review = model(defaultChoices(PLAN), true);
    expect(review.counts).toEqual({ skills: 3, copies: 6, harnesses: 4, invalid: 1 });
    expect(review.invalidRows).toEqual([
      {
        key: "cursor\n/h/cursor/broken-skill",
        name: "broken-skill",
        toolLabel: "Cursor",
        path: "/h/cursor/broken-skill",
        reason: "localSkillInvalid.invalidFrontmatter",
      },
    ]);
  });

  it("states no fate for a group that does not go ahead", () => {
    const handoff = model(defaultChoices(PLAN), true).importRows.find((row) => row.name === "handoff")!;
    expect(handoff.checked).toBe(false);
    expect(handoff.copies.map((copy) => copy.fate)).toEqual([null, null]);
  });

  it("auto-deploy on: the chosen copy becomes the skill and is linked; a kept copy in a deploy harness is asked about", () => {
    const choices = { ...defaultChoices(PLAN), selected: { "ego-browser": true, handoff: true } };
    const handoff = model(choices, true, ["pi"]).importRows.find((row) => row.name === "handoff")!;
    expect(handoff.others).toBe(1);
    expect(handoff.copies.map((copy) => [copy.toolLabel, copy.chosen, copy.fate])).toEqual([
      ["Claude Code", true, "chosenLinked"],
      ["Pi", false, "askFirst"],
    ]);
    // Not a deploy harness: left in place.
    const elsewhere = model(choices, true, ["cursor"]).importRows.find((row) => row.name === "handoff")!;
    expect(elsewhere.copies[1].fate).toBe("leftInPlace");
  });

  it("the keep / remove choice decides a differing copy's fate; the originals checkbox the identical ones'", () => {
    const base = defaultChoices(PLAN);
    const choices: ImportChoices = {
      ...base,
      selected: { ...base.selected, handoff: true },
      variant: { ...base.variant, handoff: "/h/pi/handoff" },
      removeOthers: { handoff: true },
    };
    const handoff = model(choices, false).importRows.find((row) => row.name === "handoff")!;
    expect(handoff.chosenPath).toBe("/h/pi/handoff");
    expect(handoff.copies.map((copy) => [copy.toolLabel, copy.fate])).toEqual([
      ["Claude Code", "removed"],
      ["Pi", "chosenRemoved"],
    ]);
    // Removing them comes before any deploy (#98): with auto-deploy off nothing replaces them.
    expect(handoff.removeOthersThen).toBe("nothingDeployed");
    const deploying = model(choices, true).importRows.find((row) => row.name === "handoff")!;
    expect(deploying.removeOthersThen).toBe("deployed");
    const kept = model({ ...choices, removeOthers: {}, removeOriginals: false }, false).importRows.find(
      (row) => row.name === "handoff",
    )!;
    expect(kept.copies.map((copy) => copy.fate)).toEqual(["leftInPlace", "chosenLeft"]);
    expect(kept.removeOthersThen).toBeNull();
  });

  it("a managed row compares each copy with the managed copy: Replace links the same one and asks about the other", () => {
    const leave = model(defaultChoices(PLAN), true).managedRows[0];
    expect(leave).toMatchObject({ name: "tdd", replace: false, differs: 1, harnesses: ["Cursor", "Pi"] });
    expect(leave.copies.map((copy) => [copy.sameAsManaged, copy.fate])).toEqual([
      [true, null],
      [false, null],
    ]);
    const replace = model({ ...defaultChoices(PLAN), replace: { tdd: true } }, true).managedRows[0];
    expect(replace.copies.map((copy) => copy.fate)).toEqual(["linked", "askFirst"]);
  });

  it("a checked Match group says its copies' one fate; a Conflict or an unchecked group says none", () => {
    const choices = defaultChoices(PLAN);
    const fateOf = (review: ReturnType<typeof model>, name: string) =>
      review.importRows.find((row) => row.name === name)!.groupFate;
    expect(fateOf(model(choices, true), "ego-browser")).toBe("linked");
    expect(fateOf(model(choices, false), "ego-browser")).toBe("removed");
    expect(fateOf(model({ ...choices, removeOriginals: false }, false), "ego-browser")).toBe("leftInPlace");
    expect(fateOf(model({ ...choices, selected: { handoff: true } }, true), "handoff")).toBeNull();
    expect(fateOf(model({ ...choices, selected: {} }, true), "ego-browser")).toBeNull();
  });

  it("the policy counts the checked groups' identical originals and names their harnesses", () => {
    const review = model(defaultChoices(PLAN), false);
    expect(review.policy).toEqual({
      autoDeploy: false,
      identical: 2,
      identicalHarnesses: ["Claude Code", "Codex"],
      deployHarnesses: ["Claude Code", "Cursor"],
      removeOriginals: true,
    });
  });

  it("the button names the action, and refuses when nothing would happen", () => {
    const choices = defaultChoices(PLAN);
    expect(model(choices, true).action).toEqual({ label: "importReview.action.importAndDeploy", refusal: null });
    expect(model(choices, false).action).toEqual({
      label: t("importReview.action.importAndRemove", { count: 2 }),
      refusal: null,
    });
    expect(model({ ...choices, removeOriginals: false }, false).action).toEqual({
      label: "importReview.action.import",
      refusal: null,
    });
    const nothing = { ...choices, selected: {} };
    expect(model(nothing, true).action).toEqual({ label: "importReview.action.import", refusal: "importReview.action.nothing" });
    expect(model({ ...nothing, replace: { tdd: true } }, true).action).toEqual({
      label: t("importReview.action.replace", { count: 1 }),
      refusal: null,
    });
  });
});

describe("importSelections / importPolicy", () => {
  it("sends checked groups, the chosen copy, the remove choice of a Conflict and Replace for a managed name", () => {
    const base = defaultChoices(PLAN);
    const choices: ImportChoices = {
      ...base,
      selected: { ...base.selected, handoff: true },
      variant: { ...base.variant, handoff: "/h/pi/handoff" },
      removeOthers: { handoff: true },
      replace: { tdd: true },
    };
    expect(importSelections(PLAN, choices)).toEqual([
      { group_name: "ego-browser", chosen_path: "/h/claude_code/ego-browser", name: null },
      { group_name: "handoff", chosen_path: "/h/pi/handoff", name: null, remove_divergent: true },
      { group_name: "tdd", chosen_path: "/h/cursor/tdd", name: null, replace: true },
    ]);
    expect(importSelections(PLAN, base)).toEqual([
      { group_name: "ego-browser", chosen_path: "/h/claude_code/ego-browser", name: null },
    ]);
  });

  it("states auto-deploy with its harnesses, or the originals choice", () => {
    const choices = { ...defaultChoices(PLAN), removeOriginals: false };
    expect(importPolicy(choices, { autoDeploy: true, tools: ["cursor"] })).toEqual({
      auto_sync: true,
      tools: ["cursor"],
      remove_originals: false,
    });
    expect(importPolicy(choices, { autoDeploy: false, tools: ["cursor"] })).toEqual({
      auto_sync: false,
      tools: null,
      remove_originals: false,
    });
  });
});

const synced = (tool: string): BatchTargetOutcome => ({
  skill_id: "s",
  skill_name: "s",
  tool_key: tool,
  status: { status: "synced", outcome: { mode_used: "symlink", target_path: `/t/${tool}`, replaced: false } },
});

describe("importResultRows", () => {
  const report: ImportReport = {
    groups: [
      {
        group_name: "ego-browser",
        status: {
          status: "imported",
          skill_id: "e",
          skill_name: "ego-browser",
          central_path: "/c/ego-browser",
          targets: [
            synced("claude_code"),
            synced("cursor"),
            { ...synced("pi"), status: { status: "declined" } },
            {
              ...synced("codex"),
              status: { status: "failed", error: { code: "TARGET_EXISTS", path: "/x", modifiedAt: null, sameContent: false } },
            },
          ],
          forced_tools: [],
          originals: [{ path: "/h/pi/e", tool: "pi", status: { status: "kept_divergent" } }],
        },
      },
      {
        group_name: "handoff",
        status: {
          status: "imported",
          skill_id: "h",
          skill_name: "handoff",
          central_path: "/c/handoff",
          targets: [],
          forced_tools: [],
          originals: [
            { path: "/h/claude_code/handoff", tool: "claude_code", status: { status: "removed" } },
            { path: "/h/pi/handoff", tool: "pi", status: { status: "removed" } },
            { path: "/h/cursor/handoff", tool: "cursor", status: { status: "left_in_place" } },
          ],
        },
      },
      { group_name: "tdd", status: { status: "replaced", skill_id: "t", skill_name: "tdd", central_path: "/c/tdd", targets: [synced("cursor")] } },
      { group_name: "pdf", status: { status: "skipped", reason: { reason: "name_managed", skill_id: "p" } } },
      { group_name: "mcp", status: { status: "failed", error: { code: "RATE_LIMITED", resetMinutes: 3 } } },
    ],
  };

  it("one row per sent group: where it deployed, each original's fate per harness, skips with why, failures with Retry", () => {
    const rows = importResultRows(report, { t, toolLabelById: labels });
    expect(rows.map((row) => [row.name, row.status, row.attention, row.retryable])).toEqual([
      ["ego-browser", "imported", true, false],
      ["handoff", "imported", false, false],
      ["tdd", "replaced", false, false],
      ["pdf", "skipped", false, false],
      ["mcp", "failed", true, true],
    ]);
    expect(rows[0].lines).toEqual([
      t("importReview.result.deployed", { harnesses: "Claude Code, Cursor" }),
      t("importReview.result.declined", { harnesses: "Pi" }),
      t("importReview.result.toReview", { harnesses: "Codex" }),
      t("importReview.result.keptDifferent", { harnesses: "Pi" }),
    ]);
    expect(rows[1].lines).toEqual([
      "importReview.result.libraryOnly",
      t("importReview.result.removed", { harnesses: "Claude Code, Pi" }),
      t("importReview.result.leftInPlace", { harnesses: "Cursor" }),
    ]);
    expect(rows[2].lines).toEqual([t("importReview.result.linked", { harnesses: "Cursor" })]);
    expect(rows[3].lines).toEqual([t("importReview.result.nameManaged", { name: "pdf" })]);
    expect(rows[4].lines[0]).toContain("errors.rateLimited");
  });

  it("a Replace whose every copy was kept in the ask reads Kept, not Replaced", () => {
    const declined = { ...synced("cursor"), status: { status: "declined" as const } };
    const rows = importResultRows(
      { groups: [{ group_name: "tdd", status: { status: "replaced", skill_id: "t", skill_name: "tdd", central_path: "/c/tdd", targets: [declined] } }] },
      { t, toolLabelById: labels },
    );
    expect(rows.map((row) => [row.status, row.lines])).toEqual([
      ["kept", [t("importReview.result.declined", { harnesses: "Cursor" })]],
    ]);
  });

  it("a Retry's outcomes replace their groups in place; failures, imported ids", () => {
    const retry: ImportReport = {
      groups: [{ group_name: "mcp", status: { ...report.groups[1].status, skill_id: "m" } as ImportReport["groups"][0]["status"] }],
    };
    const merged = mergeImportReports(report, retry);
    expect(merged.groups.map((g) => [g.group_name, g.status.status])).toEqual([
      ["ego-browser", "imported"],
      ["handoff", "imported"],
      ["tdd", "replaced"],
      ["pdf", "skipped"],
      ["mcp", "imported"],
    ]);
    expect(Object.keys(failedGroups(report, t))).toEqual(["mcp"]);
    expect(failedGroups(merged, t)).toEqual({});
    expect(importedSkillIds(merged)).toEqual(["e", "h", "m"]);
  });
});

describe("importResultRows by settled targets (audit F3 / F4 / F7)", () => {
  const replaced = (targets: BatchTargetOutcome[]): ImportReport => ({
    groups: [{ group_name: "tdd", status: { status: "replaced", skill_id: "t", skill_name: "tdd", central_path: "/c/tdd", targets } }],
  });
  const skip = (tool: string, code: "TARGET_DISABLED" | "TOOL_NOT_INSTALLED"): BatchTargetOutcome => ({
    ...synced(tool),
    status: { status: "skipped", error: { code, tool } },
  });
  const declined = (tool: string): BatchTargetOutcome => ({ ...synced(tool), status: { status: "declined" } });
  const unanswered = (tool: string): BatchTargetOutcome => ({
    ...synced(tool),
    status: { status: "failed", error: { code: "TARGET_EXISTS", path: `/t/${tool}`, modifiedAt: null, sameContent: false } },
  });
  const disabledReason = (tool: string) => describeCommandError({ code: "TARGET_DISABLED", tool }, t);
  const rows = (report: ImportReport, keptByChoice?: ReadonlySet<string>) =>
    importResultRows(report, { t, toolLabelById: labels, keptByChoice });

  it("an all-skipped Replace is a neutral Skipped row stating each harness's reason — never a green Replaced", () => {
    const [row] = rows(replaced([skip("cursor", "TARGET_DISABLED"), skip("pi", "TOOL_NOT_INSTALLED")]));
    expect(row.status).toBe("skipped");
    expect(row.attention).toBe(false);
    expect(row.lines).toEqual([
      t("importReview.result.skippedTarget", { harness: "Cursor", reason: disabledReason("cursor") }),
      t("importReview.result.skippedTarget", { harness: "Pi", reason: "errors.toolNotInstalled" }),
    ]);
  });

  it("a mixed synced / skipped Replace reads Replaced, with the skip as a neutral line", () => {
    const [row] = rows(replaced([synced("cursor"), skip("pi", "TARGET_DISABLED")]));
    expect([row.status, row.attention]).toEqual(["replaced", false]);
    expect(row.lines).toEqual([
      t("importReview.result.linked", { harnesses: "Cursor" }),
      t("importReview.result.skippedTarget", { harness: "Pi", reason: disabledReason("pi") }),
    ]);
  });

  it("a declined + unanswered Replace with nothing replaced is not Kept: Skipped, its folder to review", () => {
    const [row] = rows(replaced([declined("cursor"), unanswered("pi")]));
    expect([row.status, row.attention]).toEqual(["skipped", true]);
    expect(row.lines).toEqual([
      t("importReview.result.declined", { harnesses: "Cursor" }),
      t("importReview.result.toReview", { harnesses: "Pi" }),
    ]);
  });

  it("an imported group's skipped target is a neutral line too", () => {
    const [row] = rows({
      groups: [
        {
          group_name: "e",
          status: {
            status: "imported",
            skill_id: "e",
            skill_name: "e",
            central_path: "/c/e",
            targets: [synced("claude_code"), skip("cursor", "TARGET_DISABLED")],
            forced_tools: [],
            originals: [],
          },
        },
      ],
    });
    expect([row.status, row.attention]).toEqual(["imported", false]);
    expect(row.lines[1]).toBe(
      t("importReview.result.skippedTarget", { harness: "Cursor", reason: disabledReason("cursor") }),
    );
  });

  it("a different copy the operator chose to keep needs no attention; one nobody chose does", () => {
    const report: ImportReport = {
      groups: [
        {
          group_name: "handoff",
          status: {
            status: "imported",
            skill_id: "h",
            skill_name: "handoff",
            central_path: "/c/handoff",
            targets: [],
            forced_tools: [],
            originals: [{ path: "/h/pi/handoff", tool: "pi", status: { status: "kept_divergent" } }],
          },
        },
      ],
    };
    expect(rows(report, new Set(["handoff"]))[0].attention).toBe(false);
    expect(rows(report)[0].attention).toBe(true);
    expect(rows(report, new Set(["handoff"]))[0].lines).toContain(t("importReview.result.keptDifferent", { harnesses: "Pi" }));
  });

  it("a copy whose comparison was unknown says it was left in place, not a raw removal failure", () => {
    const [row] = rows({
      groups: [
        {
          group_name: "alpha",
          status: {
            status: "imported",
            skill_id: "a",
            skill_name: "alpha",
            central_path: "/c/alpha",
            targets: [],
            forced_tools: [],
            originals: [
              {
                path: "/h/codex/alpha",
                tool: "codex",
                status: { status: "failed", error: { code: "IDENTITY_UNKNOWN", path: "/h/codex/alpha", detail: "x" } },
              },
            ],
          },
        },
      ],
    });
    expect(row.attention).toBe(true);
    expect(row.lines).toContain(t("importReview.result.identityUnknown", { harness: "Codex" }));
  });
});

describe("keptByChoice", () => {
  it("names the Conflicts sent without remove_divergent — the operator kept their other copies", () => {
    const selections = importSelections(PLAN, {
      ...defaultChoices(PLAN),
      selected: { "ego-browser": true, handoff: true },
    });
    expect([...keptByChoice(PLAN, selections)]).toEqual(["handoff"]);
    const removing = importSelections(PLAN, {
      ...defaultChoices(PLAN),
      selected: { handoff: true },
      removeOthers: { handoff: true },
    });
    expect([...keptByChoice(PLAN, removing)]).toEqual([]);
  });
});
