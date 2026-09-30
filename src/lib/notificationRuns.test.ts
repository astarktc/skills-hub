import { describe, expect, it } from "vitest";
import type { Notification } from "../hooks/useNotificationHistory";
import { arrivalAnnouncement, groupRuns, historyText, rowActions, rowText, rowTitle, staleCollision } from "./notificationRuns";

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

  it("a failed pair offers Retry; an unanswered collision offers Overwrite… by the route that met it (A3)", () => {
    expect(rowActions(row({ subject: { ...subject, tool: "cursor" }, repair: { verb: "retry", tool: "cursor" } }), exists)).toEqual([
      { kind: "show", skillId: "s1" },
      { kind: "retry", skillId: "s1", tool: "cursor" },
    ]);
    expect(
      rowActions(row({ subject: { ...subject, tool: "cursor" }, repair: { verb: "overwrite", tool: "cursor", via: "sync" }, review: true }), exists),
    ).toEqual([
      { kind: "show", skillId: "s1" },
      { kind: "overwrite", skillId: "s1", tool: "cursor", via: "sync" },
    ]);
    // An Enable's collision keeps its route: answered by the pair's Enable, never an ordinary sync.
    expect(
      rowActions(row({ subject: { ...subject, tool: "cursor" }, repair: { verb: "overwrite", tool: "cursor", via: "enable" }, review: true }), exists),
    ).toEqual([
      { kind: "show", skillId: "s1" },
      { kind: "overwrite", skillId: "s1", tool: "cursor", via: "enable" },
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

const inProject = (projectId: string) => `in ${projectId === "p1" ? "monorepo" : projectId}:`;

describe("copy text", () => {
  it("a row copies its title then its message; the history copies each run with its items", () => {
    expect(rowText(row({ title: "t", message: "m" }), inProject)).toBe("t\nm");
    expect(rowText(row({ title: "t" }), inProject)).toBe("t");
    const groups = groupRuns([
      row({ runId: 1, role: "headline", kind: "warning", title: "1 synced · 1 folder to review", runLabel: "Sync" }),
      row({ runId: 1, role: "entry", kind: "warning", title: "alpha → Cursor", message: "/c", review: true }),
    ]);
    expect(historyText(groups, inProject)).toBe(
      [
        "2023-11-14T22:13:20.000Z [warning] Sync: 1 synced · 1 folder to review",
        "  - [review] alpha → Cursor",
        "      /c",
      ].join("\n"),
    );
  });
});

describe("project identity (N1: one project-aware presentation)", () => {
  const projectSubject = { skillId: "s1", skillName: "alpha", tool: "claude_code", projectId: "p1" };

  it("a single failed project toggle promoted to its run's heading keeps its project — on screen and in both copies", () => {
    const groups = groupRuns([
      row({
        runId: 1,
        role: "entry",
        kind: "error",
        runLabel: "Sync",
        title: "Sync failed: alpha -> Claude Code",
        message: "Permission denied",
        subject: projectSubject,
      }),
    ]);
    expect(groups[0].head.role).toBe("entry");
    expect(rowTitle(groups[0].head, inProject)).toBe("in monorepo: Sync failed: alpha -> Claude Code");
    expect(rowText(groups[0].head, inProject)).toBe("in monorepo: Sync failed: alpha -> Claude Code\nPermission denied");
    expect(historyText(groups, inProject)).toBe(
      "2023-11-14T22:13:20.000Z [error] Sync: in monorepo: Sync failed: alpha -> Claude Code\n    Permission denied",
    );
  });

  it("a project item under a headline is placed too; a title that names its project already is not prefixed twice", () => {
    const groups = groupRuns([
      row({ runId: 2, role: "headline", kind: "warning", title: "3 synced · 1 failed", runLabel: "Re-sync all" }),
      row({ runId: 2, role: "entry", kind: "error", title: "alpha -> Cursor", subject: { ...projectSubject, projectId: "p2" } }),
      row({
        runId: 2,
        role: "entry",
        kind: "error",
        title: "Sync failed in monorepo: alpha -> Claude Code",
        subject: { ...projectSubject, projectInTitle: true },
      }),
    ]);
    expect(groups[0].items.map((item) => rowTitle(item, inProject))).toEqual([
      "in p2: alpha -> Cursor",
      "Sync failed in monorepo: alpha -> Claude Code",
    ]);
    // A headline is about the run, never one project.
    expect(rowTitle(groups[0].head, inProject)).toBe("3 synced · 1 failed");
  });
});

describe("staleCollision (A3: the report is history, the pair is now)", () => {
  const skill = (disabled_at: number | null, status?: "disabled" | "synced" | "error") => ({
    disabled_at,
    targets: status ? [{ tool: "claude", status }] : [],
  });

  it("an Enable's collision is answered while the pair is still its disabled marker and the skill is on", () => {
    expect(staleCollision(skill(null, "disabled"), "claude", "enable")).toBeNull();
    expect(staleCollision(skill(9, "disabled"), "claude", "enable")).toBe("skillDisabled");
    expect(staleCollision(skill(null, "synced"), "claude", "enable")).toBe("resolved");
    expect(staleCollision(skill(null, "error"), "claude", "enable")).toBe("changed");
    expect(staleCollision(skill(null), "claude", "enable")).toBe("changed");
  });

  it("a sync's collision is answered by re-syncing, never over a Disable made since", () => {
    expect(staleCollision(skill(null), "claude", "sync")).toBeNull();
    expect(staleCollision(skill(null, "error"), "claude", "sync")).toBeNull();
    expect(staleCollision(skill(null, "disabled"), "claude", "sync")).toBe("pairDisabled");
    expect(staleCollision(skill(3), "claude", "sync")).toBe("skillDisabled");
    expect(staleCollision(skill(null, "synced"), "claude", "sync")).toBe("resolved");
  });
});

describe("arrivalAnnouncement (ticket 14 audit N4: what the open panel announces)", () => {
  const inProject = (id: string) => `in ${id}:`;
  const opened = groupRuns([row({ runId: 1, role: "headline", kind: "success", title: "3 synced" })]);

  it("announces nothing while the newest group is the one the panel opened on", () => {
    expect(arrivalAnnouncement(opened, "run:1", inProject)).toBe("");
    expect(arrivalAnnouncement([], null, inProject)).toBe("");
  });

  it("announces a standalone note recorded while open (a stale collision's answer)", () => {
    const note = row({ kind: "info", title: "Nothing to overwrite: alpha is already deployed to Cursor." });
    const groups = groupRuns([note, ...opened.map((group) => group.head)]);
    expect(arrivalAnnouncement(groups, "run:1", inProject)).toBe(
      "Nothing to overwrite: alpha is already deployed to Cursor.",
    );
  });

  it("announces a run settled from the panel by its headline, a project row with its project", () => {
    const retried = groupRuns([
      row({ runId: 2, role: "entry", kind: "error", title: "alpha failed", subject: { skillId: "a", skillName: "alpha", projectId: "mono" } }),
    ]);
    expect(arrivalAnnouncement(retried, "run:1", inProject)).toBe("in mono: alpha failed");
  });

  it("announces the first arrival after the panel opened empty, or after Clear", () => {
    expect(arrivalAnnouncement(opened, null, inProject)).toBe("3 synced");
  });
});
