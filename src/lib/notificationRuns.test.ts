import { describe, expect, it } from "vitest";
import type { Notification } from "../hooks/useNotificationHistory";
import { groupRuns, historyText, rowActions, rowText } from "./notificationRuns";

let nextId = 100;
const row = (over: Partial<Notification>): Notification => ({
  id: nextId--,
  kind: "info",
  title: "row",
  at: 1_700_000_000_000,
  ...over,
});

describe("groupRuns (D11: the panel grouped by run)", () => {
  it("groups a run's rows under its headline, newest run first; a standalone notification is its own group", () => {
    const history: Notification[] = [
      row({ runId: 2, role: "headline", kind: "warning", title: "12 refreshed · 1 failed", runLabel: "Refresh" }),
      row({ runId: 2, role: "entry", kind: "error", title: "alpha failed" }),
      row({ runId: 2, role: "entry", kind: "warning", title: "beta → Cursor: to review", review: true }),
      row({ kind: "error", title: "Copy failed" }),
      row({ runId: 1, role: "headline", kind: "success", title: "3 synced", runLabel: "Sync" }),
    ];
    const groups = groupRuns(history);
    expect(groups.map((group) => [group.key, group.label, group.kind, group.head.title, group.items.map((i) => i.title), group.reviewCount])).toEqual([
      ["run:2", "Refresh", "warning", "12 refreshed · 1 failed", ["alpha failed", "beta → Cursor: to review"], 1],
      [`note:${history[3].id}`, null, "error", "Copy failed", [], 0],
      ["run:1", "Sync", "success", "3 synced", [], 0],
    ]);
  });

  it("a run without a headline is headed by its first item (not repeated), toned by its strongest item", () => {
    const groups = groupRuns([
      row({ runId: 5, role: "entry", kind: "warning", title: "first", runLabel: "Sync" }),
      row({ runId: 5, role: "entry", kind: "error", title: "second" }),
    ]);
    expect(groups).toHaveLength(1);
    expect(groups[0].head.title).toBe("first");
    expect(groups[0].items.map((item) => item.title)).toEqual(["second"]);
    expect(groups[0].kind).toBe("error");
    expect(groups[0].label).toBe("Sync");
  });
});

describe("rowActions (D11: rows carry actions)", () => {
  const exists = (id: string) => id !== "gone";
  const subject = { skillId: "s1", skillName: "alpha" };

  it("Show for a skill still in the catalog; nothing for one that left it", () => {
    expect(rowActions(row({ subject }), exists)).toEqual([{ kind: "show", skillId: "s1" }]);
    expect(rowActions(row({ subject: { skillId: "gone", skillName: "x" }, repair: { verb: "update" } }), exists)).toEqual([]);
    expect(rowActions(row({}), exists)).toEqual([]);
  });

  it("a failed pair offers Retry; an unanswered collision offers Overwrite… (the one ask)", () => {
    expect(rowActions(row({ subject: { ...subject, tool: "cursor" }, repair: { verb: "retry", tool: "cursor" } }), exists)).toEqual([
      { kind: "show", skillId: "s1" },
      { kind: "retry", skillId: "s1", tool: "cursor" },
    ]);
    expect(
      rowActions(row({ subject: { ...subject, tool: "cursor" }, repair: { verb: "overwrite", tool: "cursor" }, review: true }), exists),
    ).toEqual([
      { kind: "show", skillId: "s1" },
      { kind: "overwrite", skillId: "s1", tool: "cursor" },
    ]);
  });

  it("gone upstream (404) offers Change source… instead of a retry", () => {
    const error = { code: "GITHUB_SKILL_NOT_FOUND", url: "https://github.com/o/r" } as const;
    expect(rowActions(row({ subject, error, repair: { verb: "update" } }), exists)).toEqual([
      { kind: "show", skillId: "s1" },
      { kind: "changeSource", skillId: "s1" },
    ]);
  });

  it.each([
    ["rate limit", { code: "RATE_LIMITED", resetMinutes: 12 } as const],
    ["auth", { code: "GIT_CLONE_FAILED", kind: "auth", detail: "" } as const],
  ])("%s offers Retry and Open Settings → GitHub", (_name, error) => {
    expect(rowActions(row({ subject, error, repair: { verb: "update" } }), exists)).toEqual([
      { kind: "show", skillId: "s1" },
      { kind: "update", skillId: "s1" },
      { kind: "githubSettings" },
    ]);
    // Even with the skill gone, the token is still the repair.
    expect(rowActions(row({ error }), exists)).toEqual([{ kind: "githubSettings" }]);
  });

  it("a project row offers Show only; a kept Delete target offers the Delete again", () => {
    expect(
      rowActions(row({ subject: { ...subject, tool: "claude", projectId: "p1" }, repair: { verb: "retry", tool: "claude" } }), exists),
    ).toEqual([{ kind: "show", skillId: "s1" }]);
    expect(rowActions(row({ subject, repair: { verb: "delete" } }), exists)).toEqual([
      { kind: "show", skillId: "s1" },
      { kind: "delete", skillId: "s1" },
    ]);
  });
});

describe("copy text", () => {
  it("a row copies its title then its message; the history copies each run with its items", () => {
    expect(rowText(row({ title: "t", message: "m" }))).toBe("t\nm");
    expect(rowText(row({ title: "t" }))).toBe("t");
    const groups = groupRuns([
      row({ runId: 1, role: "headline", kind: "warning", title: "1 synced · 1 folder to review", runLabel: "Sync" }),
      row({ runId: 1, role: "entry", kind: "warning", title: "alpha → Cursor", message: "/c", review: true }),
    ]);
    expect(historyText(groups)).toBe(
      [
        "2023-11-14T22:13:20.000Z [warning] Sync: 1 synced · 1 folder to review",
        "  - [review] alpha → Cursor",
        "      /c",
      ].join("\n"),
    );
  });
});
