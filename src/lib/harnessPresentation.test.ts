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
    folder_key: key,
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
  const K = "settingsDialog.harnesses.duplicates";

  it("on the reader's row: names the folder it also reads (one pair)", () => {
    const pairs = duplicatePairs(TOOLS, new Set(["cursor", "claude_code"]));
    expect(duplicateWarning(pairs, "cursor", labelOf, t)).toBe(
      `${K}.reads.showsTwice {"reader":"Cursor","harnesses":"Claude Code","count":1}`,
    );
  });

  it("on the read harness's row: names the reader (one pair)", () => {
    const pairs = duplicatePairs(TOOLS, new Set(["cursor", "claude_code"]));
    expect(duplicateWarning(pairs, "claude_code", labelOf, t)).toBe(
      `${K}.readBy.showsTwice {"readers":"Cursor","harness":"Claude Code","count":1}`,
    );
  });

  it("lists every folder one reader also reads in one sentence", () => {
    const pairs = duplicatePairs(TOOLS, new Set(["cursor", "claude_code", "codex"]));
    expect(duplicateWarning(pairs, "cursor", labelOf, t)).toBe(
      `${K}.reads.showsTwice {"reader":"Cursor","harnesses":"Claude Code and Codex","count":2}`,
    );
  });

  it("says invocation breaks for a reader that loads none of a duplicate", () => {
    const pairs = duplicatePairs(TOOLS, new Set(["fx", "claude_code"]));
    expect(duplicateWarning(pairs, "fx", labelOf, t)).toBe(
      `${K}.reads.loadsNone {"reader":"fx","harnesses":"Claude Code","count":1}`,
    );
  });

  it("says the reader flags a conflict for one that flags it", () => {
    const pairs = duplicatePairs(TOOLS, new Set(["cortex", "claude_code"]));
    expect(duplicateWarning(pairs, "cortex", labelOf, t)).toBe(
      `${K}.reads.flagsConflict {"reader":"Cortex Code","harnesses":"Claude Code","count":1}`,
    );
  });

  it("gathers the readers of one folder by consequence, the gravest first", () => {
    const pairs = duplicatePairs(TOOLS, new Set(["cursor", "claude_code", "fx", "cortex"]));
    expect(duplicateWarning(pairs, "claude_code", labelOf, t)).toBe(
      `${K}.readBy.loadsNone {"readers":"fx","harness":"Claude Code","count":1} ` +
        `${K}.readBy.flagsConflict {"readers":"Cortex Code","harness":"Claude Code","count":1} ` +
        `${K}.readBy.showsTwice {"readers":"Cursor","harness":"Claude Code","count":1}`,
    );
    const many = [
      tool("a", true, { keys: ["dir"], behaviour: "shows_all" }),
      tool("b", true, { keys: ["dir"], behaviour: "shadows" }),
      tool("dir", true),
    ];
    expect(duplicateWarning(duplicatePairs(many, new Set(["a", "b", "dir"])), "dir", labelOf, t)).toBe(
      `${K}.readBy.showsTwice {"readers":"a and b","harness":"dir","count":2}`,
    );
  });

  it("names one folder once when two selected harnesses share it (backend shared_with)", () => {
    // Crush reads Amp's folder, which Kimi deploys to too: one folder, not two.
    const shared: ToolInfoDto[] = [
      tool("crush", true, { keys: ["amp", "kimi"], behaviour: "unknown" }),
      { ...tool("amp", true), shared_with: ["amp", "kimi"] },
      { ...tool("kimi", true), shared_with: ["amp", "kimi"] },
    ];
    const names = (key: string) => ({ crush: "Crush", amp: "Amp", kimi: "Kimi" })[key] ?? key;
    const line = duplicateWarning(duplicatePairs(shared, new Set(["crush", "amp", "kimi"])), "crush", names, t);
    expect(line).toBe(
      `settingsDialog.harnesses.duplicates.reads.showsTwice ${JSON.stringify({ reader: "Crush", harnesses: "Amp / Kimi", count: 1 })}`,
    );
    // Only Kimi selected beside Crush: the folder is named by the one selected sharer.
    const lone = duplicateWarning(duplicatePairs(shared, new Set(["crush", "kimi"])), "crush", names, t);
    expect(lone).toContain('"harnesses":"Kimi"');
  });

  it("is null without a pair involving the harness", () => {
    const pairs = duplicatePairs(TOOLS, new Set(["cursor", "claude_code"]));
    expect(duplicateWarning(pairs, "codex", labelOf, t)).toBeNull();
    expect(duplicateWarning([], "cursor", labelOf, t)).toBeNull();
  });
});

describe("harnessListGroups", () => {
  const agents = tool("agents_skills", true, undefined, [
    { key: "zed", label: "Zed", installed: true, duplicate_readers: null },
    { key: "warp", label: "Warp", installed: false, duplicate_readers: null },
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
    expect(groups.undetected.readers).toEqual([
      { key: "warp", label: "Warp", installed: false, duplicate_readers: null, reads: "agents_skills" },
    ]);
    expect(groups.undetected.count).toBe(2);
  });

  it("puts an undetected harness in the group even while it is selected (its row explains it)", () => {
    const groups = harnessListGroups([tool("cursor", false), tool("pi", true)]);
    expect(groups.detected.map((entry) => entry.tool.key)).toEqual(["pi"]);
    expect(groups.undetected.tools.map((entry) => entry.key)).toEqual(["cursor"]);
  });
});

// round 20 ticket 05 rows: a reader-only harness's pairs ride on its reader
// entry under the `.agents/skills` entry, whose folder is its deploy dir. The
// pair is live when that entry and the other harness are both selected.
describe("reader-only pairs", () => {
  const READ_ONLY: ToolInfoDto[] = [
    tool("agents_skills", true, undefined, [
      {
        key: "fx_reader",
        label: "fx",
        installed: true,
        duplicate_readers: { keys: ["claude_code", "codex"], behaviour: "loads_none" },
      },
      { key: "zed", label: "Zed", installed: true, duplicate_readers: null },
    ]),
    tool("cursor", true, { keys: ["agents_skills", "claude_code"], behaviour: "shows_all" }),
    tool("claude_code", true),
    tool("codex", true),
  ];
  const names = (key: string) =>
    ({ agents_skills: ".agents/skills", fx_reader: "fx", cursor: "Cursor", claude_code: "Claude Code", codex: "Codex" })[
      key
    ] ?? key;
  const K = "settingsDialog.harnesses.duplicates";

  it("pairs the reader with the other harness via its group entry while that entry is selected", () => {
    expect(duplicatePairs(READ_ONLY, new Set(["claude_code"]))).toEqual([]);
    expect(duplicatePairs(READ_ONLY, new Set(["agents_skills", "claude_code"]))).toEqual([
      { reader: "fx_reader", other: "claude_code", behaviour: "loads_none", via: "agents_skills" },
    ]);
  });

  it("involves the group entry, the reader and the other harness", () => {
    const pairs = duplicatePairs(READ_ONLY, new Set(["agents_skills", "claude_code"]));
    for (const key of ["agents_skills", "fx_reader", "claude_code"]) expect(pairsInvolving(pairs, key)).toHaveLength(1);
    expect(pairsInvolving(pairs, "codex")).toEqual([]);
  });

  it("warns on the group entry's row (the Deploy-now ask's subject) that invocation breaks in the reader", () => {
    const pairs = duplicatePairs(READ_ONLY, new Set(["agents_skills", "claude_code", "codex", "cursor"]));
    expect(duplicateWarning(pairs, "agents_skills", names, t)).toBe(
      `${K}.reads.loadsNone {"reader":"fx","harnesses":"Claude Code and Codex","count":2} ` +
        `${K}.readBy.showsTwice {"readers":"Cursor","harness":".agents/skills","count":1}`,
    );
  });

  it("without its group entry, pairs the reader among two selected folders it reads (round 20 rows review)", () => {
    // fx reads Claude Code's and Codex's folders: deploying one skill to both breaks `$name` in fx, AGENTS or not.
    expect(duplicatePairs(READ_ONLY, new Set(["claude_code", "codex"]))).toEqual([
      { reader: "fx_reader", other: "claude_code", behaviour: "loads_none", among: true },
      { reader: "fx_reader", other: "codex", behaviour: "loads_none", among: true },
    ]);
    const pairs = duplicatePairs(READ_ONLY, new Set(["claude_code", "codex"]));
    expect(duplicateWarning(pairs, "claude_code", names, t)).toBe(
      `${K}.among.loadsNone {"reader":"fx","harnesses":"Claude Code and Codex"}`,
    );
    expect(pairsInvolving(pairs, "codex")).toHaveLength(1);
  });

  it("warns on the other harness's row that the reader reads its folder", () => {
    const pairs = duplicatePairs(READ_ONLY, new Set(["agents_skills", "claude_code"]));
    expect(duplicateWarning(pairs, "claude_code", names, t)).toBe(
      `${K}.readBy.loadsNone {"readers":"fx","harness":"Claude Code","count":1}`,
    );
  });
});
