import { describe, expect, it } from "vitest";
import type { BatchTargetOutcome } from "../components/skills/types";
import {
  invalidReasonText,
  matchesQuery,
  pickSummary,
  pickSummaryText,
  pickedCandidates,
  rejectSelection,
  settlementComplete,
  settlementRows,
  visibleCandidates,
  type PickerCandidate,
  type SettlementEntry,
} from "./addFlow";

const t = (key: string, opts?: Record<string, unknown>) => (opts ? `${key} ${JSON.stringify(opts)}` : key);

const pc = (name: string, unavailable: string | null = null, description: string | null = null): PickerCandidate => ({
  id: `skills/${name}`,
  name,
  description,
  subpath: `skills/${name}`,
  unavailable,
});

const FOUR = [pc("lint-staged-rules"), pc("commit-helper"), pc("changelog-writer"), pc("release-notes")];
const ALL = Object.fromEntries(FOUR.map((c) => [c.id, true]));

describe("the visible-and-checked rule (#47)", () => {
  it("matches name, subpath and description, case-insensitively; blank matches all", () => {
    expect(matchesQuery(pc("alpha", null, "Lints staged files"), "LINT")).toBe(true);
    expect(matchesQuery(pc("alpha"), "skills/al")).toBe(true);
    expect(matchesQuery(pc("alpha"), "  ")).toBe(true);
    expect(matchesQuery(pc("alpha"), "beta")).toBe(false);
  });

  it("the gate's line: 3 of 4 checked · 1 hidden by filter", () => {
    // "o" hides lint-staged-rules alone.
    expect(visibleCandidates(FOUR, "o").map((c) => c.name)).toEqual([
      "commit-helper",
      "changelog-writer",
      "release-notes",
    ]);
    const summary = pickSummary(FOUR, ALL, "o");
    expect(summary).toEqual({ total: 4, checked: 3, hidden: 1, hiddenChecked: 1 });
    expect(pickSummaryText(summary, t)).toBe(
      'add.summary.checked {"checked":3,"total":4} · add.summary.hidden {"count":1}',
    );
    expect(pickSummaryText(pickSummary(FOUR, ALL, ""), t)).toBe('add.summary.checked {"checked":4,"total":4}');
  });

  it("only checked, visible and selectable candidates are picked", () => {
    const listing = [...FOUR, pc("draft", "Missing name")];
    const selected = { ...ALL, "skills/draft": true, "skills/commit-helper": false };
    expect(pickedCandidates(listing, selected, "").map((c) => c.name)).toEqual([
      "lint-staged-rules",
      "changelog-writer",
      "release-notes",
    ]);
    expect(pickedCandidates(listing, selected, "notes").map((c) => c.name)).toEqual(["release-notes"]);
  });
});

describe("rejectSelection", () => {
  const taken = (name: string) => name.toLowerCase() === "taken";
  it("refuses an empty selection, a custom name on several, duplicates and taken names", () => {
    expect(rejectSelection([], "", taken, t)).toBe("errors.selectAtLeastOneSkill");
    expect(rejectSelection([{ name: "a" }, { name: "b" }], "x", taken, t)).toBe("errors.multiSelectNoCustomName");
    expect(rejectSelection([{ name: "a" }, { name: "a" }], "", taken, t)).toBe(
      'errors.duplicateSelectedSkills {"name":"a"}',
    );
    expect(rejectSelection([{ name: "Taken" }], "", taken, t)).toBe('errors.skillAlreadyExists {"name":"Taken"}');
    // A lone pick is checked under its custom name.
    expect(rejectSelection([{ name: "Taken" }], "free", taken, t)).toBeNull();
    expect(rejectSelection([{ name: "free" }], "taken", taken, t)).toBe('errors.skillAlreadyExists {"name":"taken"}');
  });
});

it("invalidReasonText names every invalid-manifest code in words", () => {
  expect(invalidReasonText("missing_skill_md", t)).toBe("localSkillInvalid.missingSkillMd");
  expect(invalidReasonText("inside_tool_dir", t)).toBe("localSkillInvalid.insideToolDir");
  expect(invalidReasonText(null, t)).toBe("localSkillInvalid.unknown");
});

describe("settlementRows", () => {
  const labels = { claude_code: "Claude Code", codex: "Codex", amp: "Amp", windsurf: "Windsurf" };
  const ctx = { t, toolLabelById: labels };
  const target = (tool: string, status: BatchTargetOutcome["status"]): BatchTargetOutcome => ({
    skill_id: "id",
    skill_name: "lint",
    tool_key: tool,
    status,
  });
  const synced = { status: "synced", outcome: { mode_used: "symlink", target_path: "/t", replaced: false } } as const;
  const installed = (report: BatchTargetOutcome[]): SettlementEntry => ({
    id: "skills/lint",
    name: "lint",
    nameOverride: null,
    phase: "settled",
    result: {
      name: "lint",
      status: "installed",
      result: { skill_id: "id", name: "lint", central_path: "/hub/lint", content_hash: null },
      deployment: { status: "reported", report },
    },
  });

  it("reads the batch's phases while it runs", () => {
    const rows = settlementRows(
      [
        { id: "a", name: "a", nameOverride: null, phase: "awaiting" },
        { id: "b", name: "b", nameOverride: null, phase: "adding" },
        { id: "c", name: "c", nameOverride: null, phase: "queued" },
      ],
      ctx,
    );
    expect(rows.map((row) => [row.status, row.retryable])).toEqual([
      ["awaiting", false],
      ["adding", false],
      ["queued", false],
    ]);
  });

  it("an installed row says what its first sync did: deployed, kept, to review, failed, skipped", () => {
    const [row] = settlementRows(
      [
        installed([
          target("codex", synced),
          target("claude_code", { status: "declined" }),
          target("windsurf", { status: "skipped", error: { code: "TOOL_NOT_INSTALLED", tool: "windsurf" } }),
        ]),
      ],
      ctx,
    );
    expect(row).toMatchObject({ status: "installed", attention: false, retryable: false });
    expect(row.detail).toBe(
      [
        'add.settlement.deployed {"count":1}',
        'add.settlement.kept {"tools":"Claude Code"}',
        'add.settlement.skipped {"tools":"Windsurf"}',
      ].join(" · "),
    );

    const [review] = settlementRows(
      [
        installed([
          target("claude_code", {
            status: "failed",
            error: { code: "TARGET_EXISTS", path: "/x", modifiedAt: 1, sameContent: false },
          }),
          target("amp", { status: "skipped", error: { code: "TOOL_NOT_WRITABLE", tool: "amp", path: "/a" } }),
        ]),
      ],
      ctx,
    );
    // An explicit install's unwritable Tool is a failure (as the install fold says).
    expect(review.attention).toBe(true);
    expect(review.detail).toContain('add.settlement.toReview {"count":1,"tools":"Claude Code"}');
    expect(review.detail).toContain("add.settlement.notDeployed");
    expect(review.detail).not.toContain("add.settlement.deployed");
  });

  it("a failed row carries its typed reason and Retry; a cancelled one is marked and can be added again", () => {
    const rows = settlementRows(
      [
        { id: "f", name: "f", nameOverride: null, phase: "settled", result: { name: "f", status: "failed", error: { code: "RATE_LIMITED", resetMinutes: 12 } } },
        { id: "c", name: "c", nameOverride: null, phase: "settled", result: { name: "c", status: "cancelled" } },
      ],
      ctx,
    );
    expect(rows).toEqual([
      { id: "f", name: "f", status: "failed", detail: 'errors.rateLimited {"minutes":12}', attention: false, retryable: true },
      { id: "c", name: "c", status: "cancelled", detail: "add.settlement.cancelledDetail", attention: false, retryable: true },
    ]);
  });

  it("settlementComplete: every sent candidate installed", () => {
    expect(settlementComplete([])).toBe(false);
    expect(settlementComplete([installed([])])).toBe(true);
    expect(
      settlementComplete([installed([]), { id: "c", name: "c", nameOverride: null, phase: "settled", result: { name: "c", status: "cancelled" } }]),
    ).toBe(false);
    expect(settlementComplete([{ id: "q", name: "q", nameOverride: null, phase: "queued" }])).toBe(false);
  });
});
