import { describe, expect, it } from "vitest";
import { resources } from "../i18n/resources";
import {
  MATRIX_CELL_MARK,
  MATRIX_STATES,
  matrixCell,
  matrixColumns,
  matrixFilters,
  matrixLegend,
  type MatrixCellInput,
} from "./projectPresentation";

const t = (key: string, opts?: Record<string, unknown>) =>
  opts ? `${key} ${JSON.stringify(opts)}` : key;

const skill = (over: Partial<MatrixCellInput["skill"]> = {}): MatrixCellInput["skill"] => ({
  name: "tdd",
  disabled_at: null,
  targets: [],
  ...over,
});
const tool = { key: "claude_code", label: "Claude Code" };
const cell = (input: Partial<MatrixCellInput>) =>
  matrixCell({ skill: skill(), tool, assignment: undefined, inFlight: false, ...input }, t);

describe("matrixCell", () => {
  it("an unassigned cell has no mark and says so in its name", () => {
    const out = cell({});
    expect(out).toMatchObject({ state: "unassigned", assigned: false, mark: null, word: null, repairs: [], alsoGlobal: false });
    expect(out.name).toBe(t("projectsSurface.cell.name", { skill: "tdd", tool: "Claude Code", state: "projectsSurface.cell.notAssigned" }));
  });

  it.each([
    ["synced", "synced", "synced", []],
    ["stale", "stale", "stale", ["sync"]],
    ["missing", "missing", "missing", ["sync"]],
    ["pending", "pending", "not_synced", []],
    ["error", "failed", "failed", ["sync"]],
    ["disabled", "disabled", "disabled", []],
  ] as const)("status %s reads %s with the strip's %s mark", (status, state, mark, repairs) => {
    const out = cell({ assignment: { status, last_error: null } });
    expect(out.state).toBe(state);
    expect(out.mark).toBe(mark);
    expect(out.repairs).toEqual(repairs);
    expect(out.word).toBe(`projectsSurface.state.${state}`);
    // The name carries the word and its meaning — the state is never colour or shape alone.
    expect(out.name).toContain(`projectsSurface.state.${state}`);
    expect(out.name).toContain(state === "failed" ? "projectsSurface.detail.failed" : `projectsSurface.detail.${state}`);
  });

  it("a failed cell names its error", () => {
    const out = cell({ assignment: { status: "error", last_error: "Permission denied (os error 13)" } });
    expect(out.detail).toBe(t("projectsSurface.detail.failedWith", { error: "Permission denied (os error 13)" }));
    expect(out.name).toContain("Permission denied (os error 13)");
  });

  it("D17: a live global target marks the cell 'also global' and keeps it a normal cell", () => {
    const global = skill({ targets: [{ tool: "claude_code", status: "synced", synced_at: 1 }] });
    const unassigned = matrixCell({ skill: global, tool, assignment: undefined, inFlight: false }, t);
    expect(unassigned).toMatchObject({ alsoGlobal: true, assigned: false, state: "unassigned" });
    expect(unassigned.name.endsWith(", projectsSurface.cell.alsoGlobal")).toBe(true);
    const assigned = matrixCell({ skill: global, tool, assignment: { status: "synced", last_error: null }, inFlight: false }, t);
    expect(assigned).toMatchObject({ alsoGlobal: true, assigned: true, state: "synced" });
    // A disabled global row, or a never-synced error row, is not a deployment.
    for (const target of [
      { tool: "claude_code", status: "disabled", synced_at: 1 },
      { tool: "claude_code", status: "error", synced_at: null },
      { tool: "pi", status: "synced", synced_at: 1 },
    ] as const) {
      expect(matrixCell({ skill: skill({ targets: [target] }), tool, assignment: undefined, inFlight: false }, t).alsoGlobal).toBe(false);
    }
  });

  it("a skill disabled at skill grain reads disabled whatever its row says, with no repair", () => {
    const out = matrixCell(
      { skill: skill({ disabled_at: 5 }), tool, assignment: { status: "error", last_error: "x" }, inFlight: false },
      t,
    );
    expect(out).toMatchObject({ state: "disabled", mark: "disabled", repairs: [], detail: "projectsSurface.detail.skillDisabled" });
  });

  it("a cell the running action is about says so and keeps its repair (the button pends, holding focus)", () => {
    const out = cell({ assignment: { status: "stale", last_error: null }, inFlight: true });
    expect(out).toMatchObject({ inFlight: true, word: "projectsSurface.cell.syncing", repairs: ["sync"], detail: null });
  });
});

describe("matrixColumns", () => {
  it("labels each configured Tool from the catalog and says when detection does not see it", () => {
    const labels: Record<string, string> = { claude_code: "Claude Code", windsurf: "Windsurf" };
    const tools = [{ tool: "claude_code" }, { tool: "windsurf" }, { tool: "zed" }];
    expect(matrixColumns(tools, (key) => labels[key] ?? key, ["claude_code"])).toEqual([
      { key: "claude_code", label: "Claude Code", detected: true },
      { key: "windsurf", label: "Windsurf", detected: false },
      { key: "zed", label: "zed", detected: false },
    ]);
    // Before the catalog loads nothing is called "not detected".
    expect(matrixColumns(tools, (key) => key, null).every((column) => column.detected)).toBe(true);
  });
});

describe("matrixFilters", () => {
  const rows = [
    { skill_id: "a", status: "synced" },
    { skill_id: "b", status: "stale" },
    { skill_id: "c", status: "error" },
    { skill_id: "d", status: "missing" },
    { skill_id: "e", status: "disabled" },
    { skill_id: "f", status: "pending" },
  ] as const;
  const filters = matrixFilters(rows);
  const live = (id: string) => ({ id, disabled_at: null });
  it("Assigned keeps every skill with a row here", () => {
    expect(["a", "b", "c", "d", "e", "f", "z"].filter((id) => filters.assigned(live(id)))).toEqual(["a", "b", "c", "d", "e", "f"]);
  });
  it("Issues keeps what Sync this repairs — stale, missing, failed — never a disabled skill", () => {
    expect(["a", "b", "c", "d", "e", "f"].filter((id) => filters.issues(live(id)))).toEqual(["b", "c", "d"]);
    expect(filters.issues({ id: "c", disabled_at: 3 })).toBe(false);
  });
});

describe("matrixLegend", () => {
  it("lists every assigned state with the cell's own mark and word", () => {
    expect(matrixLegend(t).map(({ state, mark, word }) => [state, mark, word])).toEqual(
      MATRIX_STATES.map((state) => [state, MATRIX_CELL_MARK[state], `projectsSurface.state.${state}`]),
    );
  });
  it("every state has its word and meaning in the catalog", () => {
    const en = resources.en.translation.projectsSurface;
    for (const state of MATRIX_STATES) {
      expect(en.state[state]).toBeTruthy();
      expect(en.detail[state]).toBeTruthy();
    }
    expect(en.detail.stale).toBe("copy differs from the skill \u2014 Sync to fix");
  });
});
