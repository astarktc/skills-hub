import { describe, expect, it } from "vitest";
import type { DuplicateBehaviour, ToolInfoDto } from "../components/skills/types";
import {
  duplicatePairs,
  duplicateWarning,
  harnessListGroups,
  pairsInvolving,
} from "./harnessPresentation";

function tool(
  key: string,
  installed: boolean,
  duplicates?: { keys: string[]; behaviour: DuplicateBehaviour },
  readers: ToolInfoDto["readers"] = [],
): ToolInfoDto {
  return {
    key,
    label: key,
    installed,
    shared_with: [key],
    constituents: [],
    readers,
    placement: null,
    duplicate_readers: duplicates ?? null,
  };
}

// Cursor reads Claude Code's and Codex's folders (the backend's pairs);
// fx-like "fx" loads none of a duplicate; goose is not installed.
const TOOLS: ToolInfoDto[] = [
  tool("cursor", true, { keys: ["claude_code", "codex"], behaviour: "shows_all" }),
  tool("claude_code", true),
  tool("codex", true),
  tool("fx", true, { keys: ["claude_code"], behaviour: "loads_none" }),
  tool("cortex", true, { keys: ["claude_code"], behaviour: "flags_conflict" }),
  tool("goose", false),
];

const LABELS: Record<string, string> = {
  cursor: "Cursor",
  claude_code: "Claude Code",
  codex: "Codex",
  fx: "fx",
  cortex: "Cortex Code",
};
const labelOf = (key: string) => LABELS[key] ?? key;
const t = (key: string, opts?: Record<string, unknown>) =>
  key === "common.listSeparator" ? ", " : key === "common.listAnd" ? " and " : `${key} ${JSON.stringify(opts)}`;

describe("duplicatePairs", () => {
  it("keeps only the pairs whose two harnesses are both selected", () => {
    expect(duplicatePairs(TOOLS, new Set(["cursor", "claude_code"]))).toEqual([
      { reader: "cursor", other: "claude_code", behaviour: "shows_all" },
    ]);
    expect(duplicatePairs(TOOLS, new Set(["cursor", "claude_code", "codex"]))).toEqual([
      { reader: "cursor", other: "claude_code", behaviour: "shows_all" },
      { reader: "cursor", other: "codex", behaviour: "shows_all" },
    ]);
  });

  it("has no pair when only one side is selected", () => {
    expect(duplicatePairs(TOOLS, new Set(["cursor"]))).toEqual([]);
    expect(duplicatePairs(TOOLS, new Set(["claude_code", "codex"]))).toEqual([]);
  });
});

describe("pairsInvolving", () => {
  const pairs = duplicatePairs(TOOLS, new Set(["cursor", "claude_code", "codex", "fx"]));

  it("finds a harness as the reader and as the one read", () => {
    expect(pairsInvolving(pairs, "cursor").map((p) => p.other)).toEqual(["claude_code", "codex"]);
    expect(pairsInvolving(pairs, "claude_code").map((p) => p.reader)).toEqual(["cursor", "fx"]);
    expect(pairsInvolving(pairs, "goose")).toEqual([]);
  });
});

describe("duplicateWarning", () => {
  it("names the reader and the folder it also reads (one pair)", () => {
    const pairs = duplicatePairs(TOOLS, new Set(["cursor", "claude_code"]));
    expect(duplicateWarning(pairs, labelOf, t)).toBe(
      'settingsDialog.harnesses.duplicates.showsTwice {"reader":"Cursor","harnesses":"Claude Code","count":1}',
    );
  });

  it("lists every folder one reader also reads in one sentence", () => {
    const pairs = duplicatePairs(TOOLS, new Set(["cursor", "claude_code", "codex"]));
    expect(duplicateWarning(pairsInvolving(pairs, "cursor"), labelOf, t)).toBe(
      'settingsDialog.harnesses.duplicates.showsTwice {"reader":"Cursor","harnesses":"Claude Code and Codex","count":2}',
    );
  });

  it("says invocation breaks for a harness that loads none of a duplicate", () => {
    const pairs = duplicatePairs(TOOLS, new Set(["fx", "claude_code"]));
    expect(duplicateWarning(pairs, labelOf, t)).toBe(
      'settingsDialog.harnesses.duplicates.loadsNone {"reader":"fx","harnesses":"Claude Code","count":1}',
    );
  });

  it("says the harness flags a conflict for one that flags it", () => {
    const pairs = duplicatePairs(TOOLS, new Set(["cortex", "claude_code"]));
    expect(duplicateWarning(pairs, labelOf, t)).toBe(
      'settingsDialog.harnesses.duplicates.flagsConflict {"reader":"Cortex Code","harnesses":"Claude Code","count":1}',
    );
  });

  it("joins one sentence per reader, in pair order, into one line", () => {
    const pairs = duplicatePairs(TOOLS, new Set(["cursor", "claude_code", "fx"]));
    expect(duplicateWarning(pairsInvolving(pairs, "claude_code"), labelOf, t)).toBe(
      'settingsDialog.harnesses.duplicates.showsTwice {"reader":"Cursor","harnesses":"Claude Code","count":1} ' +
        'settingsDialog.harnesses.duplicates.loadsNone {"reader":"fx","harnesses":"Claude Code","count":1}',
    );
  });

  it("is null without a pair", () => {
    expect(duplicateWarning([], labelOf, t)).toBeNull();
  });
});

describe("harnessListGroups", () => {
  const agents = tool("agents_skills", true, undefined, [
    { key: "zed", label: "Zed", installed: true },
    { key: "warp", label: "Warp", installed: false },
  ]);

  it("keeps detected harnesses in catalog order and collapses the rest, readers included", () => {
    const groups = harnessListGroups([agents, ...TOOLS]);
    expect(groups.detected.map((entry) => entry.tool.key)).toEqual([
      "agents_skills",
      "cursor",
      "claude_code",
      "codex",
      "fx",
      "cortex",
    ]);
    expect(groups.detected[0].readers.map((r) => r.key)).toEqual(["zed"]);
    expect(groups.undetected.tools.map((entry) => entry.key)).toEqual(["goose"]);
    expect(groups.undetected.readers).toEqual([{ key: "warp", label: "Warp", installed: false, reads: "agents_skills" }]);
    expect(groups.undetected.count).toBe(2);
  });

  it("puts an undetected harness in the group even while it is selected (its row explains it)", () => {
    const groups = harnessListGroups([tool("cursor", false), tool("pi", true)]);
    expect(groups.detected.map((entry) => entry.tool.key)).toEqual(["pi"]);
    expect(groups.undetected.tools.map((entry) => entry.key)).toEqual(["cursor"]);
  });
});
