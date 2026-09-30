import i18next from "i18next";
import { describe, expect, it } from "vitest";
import { resources } from "../i18n/resources";
import type {
  BatchTargetOutcome,
  ImportGroupStatus,
  ImportReport,
  InstallResultDto,
  InvocationEditReport,
  ProjectSyncOutcome,
  ProjectSyncReport,
  RefreshReport,
  RemovalReport,
  SkillRefreshOutcome,
} from "../bindings";
import {
  combineOutcomes,
  compositeHeadline,
  deleteOutcome,
  HEADLINE_DONE_KEY,
  headlineTone,
  enableOutcome,
  invocationEditOutcome,
  importOutcome,
  installBatchSettlements,
  installOutcome,
  projectRemovalOutcome,
  projectSyncOutcome,
  refreshOutcome,
  removalOutcome,
  syncOutcome,
  type InstallDeployment,
  type InstallSettlement,
} from "./reportOutcome";

const t = (key: string, opts?: Record<string, unknown>) =>
  opts ? `${key} ${JSON.stringify(opts)}` : key;
const ctx = {
  t,
  toolLabelById: { claude: "CLAUDE" },
  canRepoint: (id: string) => id === "git",
};
const refreshed = (id = "git"): SkillRefreshOutcome => ({
  skill_id: id,
  skill_name: id,
  status: {
    status: "refreshed",
    content_hash: null,
    source_revision: null,
    targets: [],
    reassert_error: null,
    edit_conflict: null,
  },
});
const failed: SkillRefreshOutcome = {
  skill_id: "git",
  skill_name: "git",
  status: {
    status: "failed",
    error: {
      code: "GITHUB_SKILL_NOT_FOUND",
      url: "https://github.com/old/repo",
    },
  },
};
const skipped: SkillRefreshOutcome = {
  skill_id: "local",
  skill_name: "local",
  status: { status: "skipped", state: "source_missing" },
};
const skippedGone: SkillRefreshOutcome = {
  skill_id: "gone",
  skill_name: "gone",
  status: { status: "skipped_acquisition", reason: "skill_gone" },
};
const skippedStale: SkillRefreshOutcome = {
  skill_id: "stale",
  skill_name: "stale",
  status: { status: "skipped_acquisition", reason: "stale_acquisition" },
};
function changed(
  kind: "conflict" | "target" | "reassert",
): SkillRefreshOutcome {
  const row = refreshed();
  if (row.status.status !== "refreshed") throw new Error("fixture");
  if (kind === "conflict")
    row.status.edit_conflict = {
      base_mode: "user-and-model",
      upstream_mode: "model-only",
      override_mode: "user-only",
    };
  if (kind === "target")
    row.status.targets.push({
      scope: { scope: "project", project_id: "project", tool: "claude" },
      status: {
        status: "failed",
        error: { code: "OTHER", message: "target failed" },
      },
    });
  if (kind === "reassert")
    row.status.reassert_error = { code: "OTHER", message: "reassert failed" };
  return row;
}
function refreshReport(skills: SkillRefreshOutcome[]): RefreshReport {
  return { skills };
}

/** The composite line as the fake `t` renders it: parts joined in order. */
const line = (...parts: [key: string, count: number][]) =>
  parts.map(([key, count]) => t(key, { count })).join(" · ");
const refreshedN = (count: number) => ["outcome.done.refreshed", count] as [string, number];
const cancelledRow = (id: string): SkillRefreshOutcome => ({
  skill_id: id,
  skill_name: id,
  status: { status: "cancelled" },
});

describe("compositeHeadline / headlineTone", () => {
  it("renders parts in the fixed order, omitting zero counts", () => {
    expect(
      compositeHeadline(t, "synced", {
        review: 1, cancelled: 2, declined: 3, skipped: 4, targetsFailed: 5, failed: 6, done: 7,
      }),
    ).toBe(line(
      ["outcome.done.synced", 7],
      ["outcome.failed", 6],
      ["outcome.targetsFailed", 5],
      ["outcome.skipped", 4],
      ["outcome.declined", 3],
      ["outcome.cancelled", 2],
      ["outcome.review.edits", 1],
    ));
    expect(compositeHeadline(t, "assigned", { failed: 2 })).toBe(line(["outcome.failed", 2]));
    expect(compositeHeadline(t, "enabled", {})).toBe(line(["outcome.done.enabled", 0]));
    expect(compositeHeadline(t, "imported", { review: 1 }, "originals")).toBe(line(["outcome.review.originals", 1]));
  });
  it("tones by failure \u203a conflict \u203a skipped \u203a declined \u203a success; a cancel alone is neutral", () => {
    const all = { done: 1, failed: 1, targetsFailed: 1, skipped: 1, declined: 1, cancelled: 1, review: 1 };
    expect(headlineTone(all)).toBe("failure");
    expect(headlineTone({ ...all, failed: 0 })).toBe("failure");
    expect(headlineTone({ ...all, failed: 0, targetsFailed: 0 })).toBe("conflict");
    expect(headlineTone({ ...all, failed: 0, targetsFailed: 0, review: 0 })).toBe("skipped");
    expect(headlineTone({ done: 1, declined: 1, cancelled: 1 })).toBe("declined");
    expect(headlineTone({ done: 1, cancelled: 1 })).toBe("neutral");
    expect(headlineTone({ cancelled: 1 })).toBe("neutral");
    expect(headlineTone({ done: 3 })).toBe("success");
    expect(headlineTone({})).toBe("success");
  });
});

describe("refreshOutcome: composite headline, tone precedence, completion policy", () => {
  const cases = [
    {
      name: "success",
      rows: [refreshed()],
      tone: "success",
      message: line(refreshedN(1)),
      errors: 0,
      warnings: 0,
      close: true,
    },
    {
      name: "acquisition failure",
      rows: [failed],
      tone: "failure",
      message: line(["outcome.failed", 1]),
      errors: 1,
      warnings: 0,
      close: false,
    },
    {
      name: "skipped",
      rows: [skipped],
      tone: "skipped",
      message: line(["outcome.skipped", 1]),
      errors: 0,
      warnings: 1,
      close: false,
    },
    {
      name: "target failure",
      rows: [changed("target")],
      tone: "failure",
      message: line(refreshedN(1), ["outcome.targetsFailed", 1]),
      errors: 1,
      warnings: 0,
      close: true,
    },
    {
      name: "reassert failure",
      rows: [changed("reassert")],
      tone: "failure",
      message: line(refreshedN(1), ["outcome.targetsFailed", 1]),
      errors: 1,
      warnings: 0,
      close: true,
    },
    {
      name: "conflict",
      rows: [changed("conflict")],
      tone: "conflict",
      message: line(refreshedN(1), ["outcome.review.edits", 1]),
      errors: 0,
      warnings: 1,
      close: true,
    },
    {
      name: "acquisition skips are warnings with their own reason",
      rows: [skippedGone, skippedStale],
      tone: "skipped",
      message: line(["outcome.skipped", 2]),
      errors: 0,
      warnings: 2,
      close: false,
    },
    {
      // refreshSummary used to drop the skipped count whenever anything failed.
      name: "failed keeps the skipped count",
      rows: [failed, skipped],
      tone: "failure",
      message: line(["outcome.failed", 1], ["outcome.skipped", 1]),
      errors: 1,
      warnings: 1,
      close: false,
    },
    {
      // A conflict is additive news about a skill that refreshed (ADR-0004):
      // it never outranks a failure in tone, and the line still counts it.
      name: "failure outranks conflict; every kind is counted",
      rows: [
        changed("conflict"),
        failed,
        skipped,
        changed("target"),
        changed("reassert"),
      ],
      tone: "failure",
      message: line(
        refreshedN(3),
        ["outcome.failed", 1],
        ["outcome.targetsFailed", 2],
        ["outcome.skipped", 1],
        ["outcome.review.edits", 1],
      ),
      errors: 3,
      warnings: 2,
      close: false,
    },
    {
      name: "empty batch",
      rows: [],
      tone: "success",
      message: line(refreshedN(0)),
      errors: 0,
      warnings: 0,
      close: false,
    },
    {
      // Cancel before anything applied: pre-settled skips stay warnings, the
      // cancelled rows raise nothing, and nothing claims a failure.
      name: "cancelled",
      rows: [cancelledRow("a"), cancelledRow("b"), skipped],
      tone: "neutral",
      message: "outcome.refreshCancelled",
      errors: 0,
      warnings: 1,
      close: false,
    },
  ] as const;
  const KIND = { success: "success", failure: "warning", conflict: "warning", skipped: "warning", neutral: "info" } as const;
  it.each(cases)(
    "$name",
    ({ rows, tone, message, errors, warnings, close }) => {
      const report = refreshReport([...rows]);
      const before = JSON.stringify(report);
      const batch = refreshOutcome(report, ctx);
      const single = refreshOutcome(report, {
        ...ctx,
        single: { name: "git", success: "single success" },
      });
      expect(batch.toast).toEqual({ kind: KIND[tone], tone, message });
      expect(batch.errors).toHaveLength(errors);
      expect(batch.warnings).toHaveLength(warnings);
      expect(batch.notes).toEqual([]);
      const conflict = rows.some(
        (row) => row.status.status === "refreshed" && row.status.edit_conflict !== null,
      );
      expect(batch.completion).toEqual({ reload: true, closeModal: close, conflict });
      expect(single.completion).toEqual({ ...batch.completion, reload: false });
      expect(single.errors).toEqual(batch.errors);
      expect(single.warnings).toEqual(batch.warnings);
      if (conflict) {
        // The batch of one keeps its sentence; its tone is the batch's.
        expect(single.toast).toEqual({
          kind: KIND[tone],
          tone,
          message: 'invocationEdit.updateCompletedWithConflict {"name":"git"}',
        });
      } else if (tone === "success") {
        expect(single.toast).toEqual({ kind: "success", tone: "success", message: "single success" });
      } else if (tone === "neutral") {
        expect(single.toast).toEqual({ kind: "info", tone: "neutral", message: 'outcome.updateCancelled {"name":"git"}' });
      } else {
        // A batch of one never repeats its single entry as a count summary.
        expect(single.toast).toBeNull();
      }
      expect(JSON.stringify(report)).toBe(before);
    },
  );

  it("the observed mixed Refresh reads every count, failure-toned, with the conflict completion", () => {
    // round-18 flows.md: 5 skills failed, 6 targets failed, 4 skipped, 1 Edit
    // conflict used to read "Refresh complete — invocation Edit conflicts need review".
    const rows = [
      ...Array.from({ length: 6 }, () => changed("target")),
      ...Array.from({ length: 5 }, (_, i) => refreshed(`ok${i}`)),
      changed("conflict"),
      ...Array.from({ length: 5 }, () => failed),
      ...Array.from({ length: 4 }, () => skipped),
    ];
    const out = refreshOutcome(refreshReport(rows), ctx);
    expect(out.toast).toEqual({
      kind: "warning",
      tone: "failure",
      message: line(
        refreshedN(12),
        ["outcome.failed", 5],
        ["outcome.targetsFailed", 6],
        ["outcome.skipped", 4],
        ["outcome.review.edits", 1],
      ),
    });
    expect(out.errors).toHaveLength(11);
    expect(out.warnings).toHaveLength(5);
    expect(out.completion).toEqual({ reload: true, closeModal: false, conflict: true });
  });

  it("renders the observed line in English", async () => {
    const i18n = i18next.createInstance();
    await i18n.init({ resources, lng: "en", interpolation: { escapeValue: false } });
    const en = i18n.t.bind(i18n) as unknown as typeof t;
    expect(
      compositeHeadline(en, "refreshed", { done: 12, failed: 5, targetsFailed: 6, skipped: 4, review: 1 }),
    ).toBe("12 refreshed · 5 failed · 6 targets failed · 4 skipped · 1 Edit to review");
    expect(compositeHeadline(en, "installed", { done: 2, cancelled: 3 })).toBe("2 installed · 3 cancelled");
    expect(compositeHeadline(en, "synced", { done: 1, declined: 1, targetsFailed: 1 })).toBe(
      "1 synced · 1 target failed · 1 kept existing",
    );
    expect(compositeHeadline(en, "imported", { done: 3, review: 2 }, "originals")).toBe(
      "3 imported · 2 originals to review",
    );
    expect(compositeHeadline(en, "removed", {})).toBe("0 removed");
    expect(en("outcome.refreshCancelled")).toBe("Refresh cancelled — nothing was changed");
    expect(en("outcome.declinedLine", { name: "ts-reset", tool: "Claude Code" })).toBe(
      "ts-reset: kept your existing Claude Code folder",
    );
    // Every verb has its plural forms in the catalog.
    for (const key of Object.values(HEADLINE_DONE_KEY)) {
      expect(en(key, { count: 1 })).not.toBe(key);
      expect(en(key, { count: 2 })).not.toBe(key);
    }
  });

  it("Edit saved-with-drift reads saved first, in English", async () => {
    const i18n = i18next.createInstance();
    await i18n.init({ resources, lng: "en", interpolation: { escapeValue: false } });
    const en = i18n.t.bind(i18n) as unknown as typeof t;
    const failure = (tool: string): InvocationEditReport["propagation"]["targets"][number] => ({
      scope: { scope: "global", tool },
      status: { status: "failed", error: { code: "OTHER", message: "blocked" } },
    });
    const out = invocationEditOutcome(
      { skill_id: "s1", skill_name: "alpha", propagation: { targets: [failure("claude"), failure("cursor")] } },
      { t: en },
    );
    expect(out.toast).toEqual({
      kind: "warning",
      tone: "failure",
      message: "Invocation setting saved \u00b7 2 targets failed",
    });
    expect(out.completion.closeModal).toBe(true);
  });

  it("acquisition skips preserve both reasons; the batch line counts them", () => {
    const out = refreshOutcome(refreshReport([skippedGone, skippedStale]), ctx);
    expect(out.warnings).toEqual([
      { title: 'errors.refreshSkippedTitle {"name":"gone"}', message: "errors.refreshSkippedSkillGone" },
      { title: 'errors.refreshSkippedTitle {"name":"stale"}', message: "errors.refreshSkippedStaleAcquisition" },
    ]);
    expect(out.completion).toEqual({ reload: true, closeModal: false, conflict: false });
    expect(out.toast).toEqual({ kind: "warning", tone: "skipped", message: line(["outcome.skipped", 2]) });
  });

  it("returns a repair id, never a callback or captured ManagedSkill", () => {
    const out = refreshOutcome(refreshReport([failed]), ctx);
    expect(out.errors[0].action).toEqual({
      label: "changeSource.action",
      skillId: "git",
      skillName: "git",
    });
    expect(out.errors[0].message).toContain("https://github.com/old/repo");
    expect(
      refreshOutcome(refreshReport([failed]), {
        ...ctx,
        canRepoint: () => false,
      }).errors[0].action,
    ).toBeUndefined();
    expect(
      refreshOutcome(
        refreshReport([
          {
            ...failed,
            status: {
              status: "failed",
              error: { code: "OTHER", message: "other" },
            },
          },
        ]),
        ctx,
      ).errors[0].action,
    ).toBeUndefined();
  });

  it("orders skill errors before target/reassert errors, skips before conflicts; silent propagation skips", () => {
    const success = refreshed();
    if (success.status.status !== "refreshed") throw new Error("fixture");
    success.status.targets.push({
      scope: { scope: "global", tool: "unknown" },
      status: { status: "skipped", reason: { reason: "link_follows_source" } },
    });
    const out = refreshOutcome(
      refreshReport([
        changed("target"),
        changed("conflict"),
        failed,
        changed("reassert"),
        skipped,
        success,
      ]),
      ctx,
    );
    expect(out.errors.map((e) => e.title)).toEqual([
      t("errors.updateFailedTitle", { name: "git" }),
      t("errors.propagationFailedTitle", { name: "git", tool: "CLAUDE" }),
      t("errors.reassertFailedTitle", { name: "git" }),
    ]);
    expect(out.warnings).toEqual([
      {
        title: t("errors.refreshSkippedTitle", { name: "local" }),
        message: "errors.refreshSkippedSourceMissing",
      },
      {
        title: t("invocationEdit.warningTitle", { name: "git" }),
        message: t("invocationEdit.refreshWarning", {
          name: "git",
          upstream: "invocationMode.modelOnly",
          override: "invocationMode.userOnly",
        }),
      },
    ]);
  });
  it("renders central-missing skips", () => {
    expect(
      refreshOutcome(
        refreshReport([
          {
            ...skipped,
            status: { status: "skipped", state: "central_missing" },
          },
        ]),
        ctx,
      ).warnings[0].message,
    ).toBe("errors.refreshSkippedCentralMissing");
  });
});

describe("invocationEditOutcome", () => {
  const globalFailure: InvocationEditReport["propagation"]["targets"][number] = {
    scope: { scope: "global", tool: "claude" },
    status: { status: "failed", error: { code: "OTHER", message: "global blocked" } },
  };
  const projectFailure: InvocationEditReport["propagation"]["targets"][number] = {
    scope: { scope: "project", project_id: "p1", tool: "unknown" },
    status: { status: "failed", error: { code: "OTHER", message: "project blocked" } },
  };
  it.each([
    { name: "no targets / no-op", propagation: [], errors: [] },
    { name: "synced", propagation: [{ scope: { scope: "global", tool: "claude" }, status: { status: "synced", mode_used: "copy" } }], errors: [] },
    { name: "global failure", propagation: [globalFailure], errors: [{ title: 'errors.propagationFailedTitle {"name":"alpha","tool":"CLAUDE"}', message: "global blocked" }] },
    { name: "project failure", propagation: [projectFailure], errors: [{ title: 'errors.propagationFailedTitle {"name":"alpha","tool":"unknown"}', message: "project blocked" }] },
    { name: "both failures", propagation: [globalFailure, projectFailure], errors: [
      { title: 'errors.propagationFailedTitle {"name":"alpha","tool":"CLAUDE"}', message: "global blocked" },
      { title: 'errors.propagationFailedTitle {"name":"alpha","tool":"unknown"}', message: "project blocked" },
    ] },
    { name: "expected skips", propagation: [
      { scope: { scope: "global", tool: "claude" }, status: { status: "skipped", reason: { reason: "link_follows_source" } } },
      { scope: { scope: "global", tool: "absent" }, status: { status: "skipped", reason: { reason: "tool_not_installed", tool: "absent" } } },
      { scope: { scope: "global", tool: "unknown" }, status: { status: "skipped", reason: { reason: "unknown_tool", tool: "unknown" } } },
      { scope: { scope: "project", project_id: "p1", tool: "claude" }, status: { status: "skipped", reason: { reason: "project_unavailable", project_id: "p1" } } },
    ], errors: [] },
  ] satisfies { name: string; propagation: InvocationEditReport["propagation"]["targets"]; errors: { title: string; message: string }[] }[])("$name: central-settled completion, saved even with drift", ({ propagation, errors }) => {
    const report: InvocationEditReport = { skill_id: "s1", skill_name: "alpha", propagation: { targets: propagation } };
    const before = JSON.stringify(report);
    expect(invocationEditOutcome(report, ctx)).toEqual({
      // The Edit saved centrally either way (ADR-0004): drift says so, warning-toned.
      toast: errors.length
        ? { kind: "warning", tone: "failure", message: `invocationEdit.saved · ${t("outcome.targetsFailed", { count: errors.length })}` }
        : { kind: "success", tone: "success", message: "invocationEdit.saved" },
      errors, warnings: [], notes: [], completion: { reload: false, closeModal: true, conflict: false },
    });
    expect(JSON.stringify(report)).toBe(before);
  });
});

const removal = (fail: boolean): RemovalReport => ({
  targets: [
    {
      rows: [{ scope: "global_target", id: "t1", skill_id: "s1", tool: "unknown" }],
      path: "/target",
      status: fail
        ? { status: "failed", error: { code: "OTHER", message: "busy" } }
        : { status: "removed" },
    },
  ],
  central_removed: false,
  record_deleted: false, cleared_markers: [],
});
describe("removalOutcome", () => {
  it("counts shared artifact rows and emits failures in target/row order for every removal action", () => {
    const report: RemovalReport = {
      targets: [
        {
          path: "/removed",
          rows: [
            { scope: "global_target", id: "r1", skill_id: "s1", tool: "claude" },
            { scope: "global_target", id: "r2", skill_id: "s1", tool: "pi" },
          ],
          status: { status: "removed" },
        },
        {
          path: "/kept",
          rows: [
            { scope: "global_target", id: "t1", skill_id: "s1", tool: "claude" },
            { scope: "assignment", id: "a1", project_id: "p1", skill_id: "s1", tool: "unknown" },
          ],
          status: { status: "failed", error: { code: "PATH_OUTSIDE_TOOL_DIRS", path: "/kept" } },
        },
        {
          path: "/also-kept",
          rows: [{ scope: "assignment", id: "a2", project_id: "p2", skill_id: "s1", tool: "pi" }],
          status: { status: "failed", error: { code: "OTHER", message: "busy" } },
        },
      ],
      central_removed: false,
      record_deleted: false, cleared_markers: [],
    };
    const before = JSON.stringify(report);
    const unsync = removalOutcome(report, { ...ctx, action: "all" });
    const project = projectRemovalOutcome(report, { ...ctx, action: "removeProject" });
    const configure = projectRemovalOutcome(report, { ...ctx, action: "configureTools" });
    const deleted = deleteOutcome(report, ctx);
    for (const [out, key] of [
      [unsync, "errors.unsyncFailedTitle"],
      [project, "errors.projectRemovalFailedTitle"],
      [configure, "errors.projectRemovalFailedTitle"],
      [deleted, "errors.deleteKeptTargetTitle"],
    ] as const) {
      expect(out.errors).toEqual([
        { title: t(key, { tool: "CLAUDE" }), message: t("errors.pathOutsideToolDirs", { path: "/kept" }) },
        { title: t(key, { tool: "unknown" }), message: t("errors.pathOutsideToolDirs", { path: "/kept" }) },
        { title: t(key, { tool: "pi" }), message: "busy" },
      ]);
      expect(out.completion.closeModal).toBe(false);
    }
    expect(unsync.toast).toEqual({ kind: "warning", tone: "failure", message: t("unsyncPartial", { count: 2, failed: 3 }) });
    expect(project.toast).toEqual({ kind: "warning", tone: "failure", message: t("projects.removeKept", { count: 2, failed: 3 }) });
    expect(configure.toast).toEqual({ kind: "warning", tone: "failure", message: t("projects.toolRemovalKept", { count: 2, failed: 3 }) });
    expect(deleted.toast).toEqual({ kind: "warning", tone: "failure", message: t("status.skillDeleteKept", { failed: 3 }) });
    expect(deleted.completion).toEqual({ reload: true, closeModal: false, conflict: false });
    expect(JSON.stringify(report)).toBe(before);
  });

  it("delete succeeds for removed targets and has kept/retry copy", () => {
    expect(deleteOutcome({ ...removal(false), central_removed: true, record_deleted: true, cleared_markers: [] }, ctx)).toEqual({
      toast: { kind: "success", tone: "success", message: "status.skillRemoved" },
      errors: [], warnings: [], notes: [], completion: { reload: true, closeModal: true, conflict: false },
    });
    expect(resources.en.translation.errors.deleteKeptTargetTitle).toContain("{{tool}}");
    expect(resources.en.translation.status.skillDeleteKept).toBe("Skill kept: {{failed}} targets could not be removed. You can retry.");
  });

  it.each(["all", "skill", "toggle"] as const)(
    "%s removal settles kept rows without claiming success",
    (action) => {
      for (const fail of [false, true]) {
        const out = removalOutcome(removal(fail), { ...ctx, action });
        expect(out.errors).toEqual(
          fail
            ? [
                {
                  title: t("errors.unsyncFailedTitle", { tool: "unknown" }),
                  message: "busy",
                },
              ]
            : [],
        );
        expect(out.completion).toEqual({
          reload: action !== "toggle" || !fail,
          closeModal: !fail,
          conflict: false,
        });
        expect(out.toast).toEqual(
          action === "all"
            ? {
                kind: fail ? "warning" : "success", tone: fail ? "failure" : "success",
                message: fail
                  ? t("unsyncPartial", { count: 0, failed: 1 })
                  : t("unsyncAllComplete", { count: 1 }),
              }
            : action === "toggle" && !fail
              ? { kind: "success", tone: "success", message: "status.syncDisabled" }
              : null,
        );
      }
    },
  );
  it("a zero-target report warns instead of claiming an unsync happened", () => {
    const nothing: RemovalReport = { targets: [], central_removed: false, record_deleted: false, cleared_markers: [] };
    for (const action of ["all", "skill", "toggle", "bulkUnassign"] as const) {
      const out = removalOutcome(nothing, { ...ctx, action });
      expect(out.toast).toEqual({
        kind: "warning", tone: "skipped",
        message: "unsyncNothingPlanned",
      });
      expect(out.errors).toEqual([]);
      expect(out.completion.closeModal).toBe(false);
    }
  });
  it("bulk unassign counts removed rows, names kept ones, and never reloads", () => {
    const report: RemovalReport = {
      targets: [
        {
          rows: [{ scope: "assignment", id: "a1", project_id: "p1", skill_id: "s1", tool: "claude" }],
          path: "/work/p1/.claude/skills/s1",
          status: { status: "removed" },
        },
        {
          rows: [{ scope: "assignment", id: "a2", project_id: "p1", skill_id: "s1", tool: "pi" }],
          path: "/work/p1/.pi/skills/s1",
          status: { status: "removed" },
        },
      ],
      central_removed: false,
      record_deleted: false, cleared_markers: [],
    };
    expect(removalOutcome(report, { ...ctx, action: "bulkUnassign" })).toEqual({
      toast: { kind: "success", tone: "success", message: t("projects.bulkUnassignSuccess", { count: 2 }) },
      errors: [],
      warnings: [],
      notes: [],
      completion: { reload: false, closeModal: true, conflict: false },
    });
    const kept: RemovalReport = {
      ...report,
      targets: [
        report.targets[0],
        {
          ...report.targets[1],
          status: { status: "failed", error: { code: "OTHER", message: "busy" } },
        },
      ],
    };
    expect(removalOutcome(kept, { ...ctx, action: "bulkUnassign" })).toEqual({
      toast: {
        kind: "warning", tone: "failure",
        message: t("projects.bulkUnassignPartial", { count: 1, failed: 1 }),
      },
      errors: [{ title: t("errors.unsyncFailedTitle", { tool: "pi" }), message: "busy" }],
      warnings: [],
      notes: [],
      completion: { reload: false, closeModal: false, conflict: false },
    });
  });
  it("a project removal reports each kept target and keeps its modal open", () => {
    const report: RemovalReport = {
      targets: [
        {
          rows: [{ scope: "assignment", id: "a1", project_id: "p1", skill_id: "s1", tool: "claude" }],
          path: "/work/p1/.claude/skills/a",
          status: { status: "removed" },
        },
        {
          rows: [{ scope: "assignment", id: "a2", project_id: "p1", skill_id: "s1", tool: "pi" }],
          path: "/work/p1/.pi/skills/a",
          status: { status: "failed", error: { code: "OTHER", message: "denied" } },
        },
      ],
      central_removed: false,
      record_deleted: false, cleared_markers: [],
    };
    const kept = [
      { title: t("errors.projectRemovalFailedTitle", { tool: "pi" }), message: "denied" },
    ];
    expect(projectRemovalOutcome(report, { ...ctx, action: "removeProject" })).toEqual({
      toast: { kind: "warning", tone: "failure", message: t("projects.removeKept", { count: 1, failed: 1 }) },
      errors: kept,
      warnings: [],
      notes: [],
      completion: { reload: false, closeModal: false, conflict: false },
    });
    expect(projectRemovalOutcome(report, { ...ctx, action: "configureTools" })).toEqual({
      toast: { kind: "warning", tone: "failure", message: t("projects.toolRemovalKept", { count: 1, failed: 1 }) },
      errors: kept,
      warnings: [],
      notes: [],
      completion: { reload: false, closeModal: false, conflict: false },
    });
    // The tool label map applies to the failed target's title.
    const relabelled = { ...report, targets: [{ ...report.targets[1], rows: [{ ...report.targets[1].rows[0], tool: "claude" }] }] };
    expect(
      projectRemovalOutcome(relabelled, { ...ctx, action: "removeProject" }).errors[0].title,
    ).toBe(t("errors.projectRemovalFailedTitle", { tool: "CLAUDE" }));
  });
  it("a tool key absent from the label map falls back to the raw key", () => {
    // The fold never invents a label: a key the map lacks (and the fresh-launch
    // case of an empty map) surfaces as-is. The binder is responsible for
    // handing the projects world the startup-loaded map (round 14 D4).
    const report: RemovalReport = {
      targets: [
        {
          rows: [{ scope: "assignment", id: "a1", project_id: "p1", skill_id: "s1", tool: "claude" }],
          path: "/work/p1/.claude/skills/a",
          status: { status: "failed", error: { code: "OTHER", message: "denied" } },
        },
      ],
      central_removed: false,
      record_deleted: false, cleared_markers: [],
    };
    for (const toolLabelById of [{}, { pi: "Pi" }] as Record<string, string>[]) {
      for (const action of ["removeProject", "configureTools"] as const) {
        expect(
          projectRemovalOutcome(report, { ...ctx, toolLabelById, action }).errors,
        ).toEqual([
          { title: t("errors.projectRemovalFailedTitle", { tool: "claude" }), message: "denied" },
        ]);
      }
    }
    expect(
      projectRemovalOutcome(report, { ...ctx, action: "removeProject" }).errors[0].title,
    ).toBe(t("errors.projectRemovalFailedTitle", { tool: "CLAUDE" }));
  });
  it("a clean or empty project removal closes without a nothing-planned warning", () => {
    const clean: RemovalReport = {
      targets: [
        {
          rows: [{ scope: "assignment", id: "a1", project_id: "p1", skill_id: "s1", tool: "pi" }],
          path: "/work/p1/.pi/skills/a",
          status: { status: "removed" },
        },
      ],
      central_removed: false,
      record_deleted: false, cleared_markers: [],
    };
    const nothing: RemovalReport = { targets: [], central_removed: false, record_deleted: false, cleared_markers: [] };
    for (const report of [clean, nothing]) {
      expect(projectRemovalOutcome(report, { ...ctx, action: "removeProject" })).toEqual({
        toast: { kind: "success", tone: "success", message: "projects.removeComplete" },
        errors: [],
        warnings: [],
        notes: [],
        completion: { reload: false, closeModal: true, conflict: false },
      });
      expect(projectRemovalOutcome(report, { ...ctx, action: "configureTools" })).toEqual({
        toast: null,
        errors: [],
        warnings: [],
        notes: [],
        completion: { reload: false, closeModal: true, conflict: false },
      });
    }
  });
  it("delete's successful report reloads and closes", () => {
    expect(deleteOutcome({ targets: [], central_removed: true, record_deleted: true, cleared_markers: [] }, ctx)).toEqual({
      toast: { kind: "success", tone: "success", message: "status.skillRemoved" },
      errors: [],
      warnings: [],
      notes: [],
      completion: { reload: true, closeModal: true, conflict: false },
    });
  });
});

const projectItem = (
  tool: string,
  status: ProjectSyncOutcome["status"],
  skill = "s1",
): ProjectSyncOutcome => ({
  assignment_id: `p1:${skill}:${tool}`,
  project_id: "p1",
  skill_id: skill,
  skill_name: `${skill}-name`,
  tool,
  status,
});
const denied = { status: "failed", error: { code: "OTHER", message: "denied" } } as const;
describe("projectSyncOutcome", () => {
  const actions = ["toggleOn", "bulkAssign", "resync", "resyncAll"] as const;

  it("names every failed row by skill and tool label with the typed error copy, in report order", () => {
    const report: ProjectSyncReport = {
      items: [
        projectItem("claude", { status: "synced" }),
        projectItem("claude", denied, "s2"),
        projectItem("pi", { status: "already_assigned" }),
        projectItem("unknown", {
          status: "failed",
          error: { code: "PATH_OUTSIDE_TOOL_DIRS", path: "/x" },
        }),
      ],
    };
    const before = JSON.stringify(report);
    for (const action of actions) {
      const out = projectSyncOutcome(report, { ...ctx, action });
      // A resync of every project names the project too — the same skill × tool
      // can fail in two projects — falling back to the id when no name is known.
      const title = (name: string, tool: string) =>
        action === "resyncAll"
          ? t("errors.syncFailedInProjectTitle", { name, tool, project: "p1" })
          : t("errors.syncFailedTitle", { name, tool });
      expect(out.errors).toEqual([
        { title: title("s2-name", "CLAUDE"), message: "denied" },
        {
          title: title("s1-name", "unknown"),
          message: t("errors.pathOutsideToolDirs", { path: "/x" }),
        },
      ]);
      expect(out.warnings).toEqual([]);
      // The project world applies the view the mutation returned.
      expect(out.completion).toEqual({ reload: false, closeModal: false, conflict: false });
    }
    expect(JSON.stringify(report)).toBe(before);
  });

  it("resync-all names the project by its label when one is supplied", () => {
    const report: ProjectSyncReport = { items: [projectItem("claude", denied)] };
    const out = projectSyncOutcome(report, {
      ...ctx,
      action: "resyncAll",
      projectLabelById: { p1: "My App" },
    });
    expect(out.errors[0]?.title).toBe(
      t("errors.syncFailedInProjectTitle", { name: "s1-name", tool: "CLAUDE", project: "My App" }),
    );
  });

  it("derives the counters per action: failure outranks success", () => {
    const mixed: ProjectSyncReport = {
      items: [
        projectItem("claude", { status: "synced" }),
        projectItem("pi", { status: "synced" }),
        projectItem("cursor", { status: "already_assigned" }),
        projectItem("codex", denied),
      ],
    };
    const toasts = Object.fromEntries(
      actions.map((action) => [action, projectSyncOutcome(mixed, { ...ctx, action }).toast]),
    );
    expect(toasts).toEqual({
      // A failed batch-of-one is its error entry alone, never a success claim.
      toggleOn: null,
      bulkAssign: {
        kind: "warning", tone: "failure",
        message: t("projects.bulkAssignPartial", { assigned: 2, failed: 1 }),
      },
      resync: {
        kind: "warning", tone: "failure",
        message: t("projects.resyncPartial", { synced: 2, failed: 1 }),
      },
      resyncAll: {
        kind: "warning", tone: "failure",
        message: t("projects.resyncAllPartial", { synced: 2, failed: 1 }),
      },
    });
  });

  it("a clean report succeeds with per-action copy and closes", () => {
    const clean: ProjectSyncReport = {
      items: [
        projectItem("claude", { status: "synced" }),
        projectItem("pi", { status: "already_assigned" }),
      ],
    };
    const outs = Object.fromEntries(
      actions.map((action) => [action, projectSyncOutcome(clean, { ...ctx, action })]),
    );
    expect(outs.toggleOn.toast).toEqual({ kind: "success", tone: "success", message: "status.syncEnabled" });
    expect(outs.bulkAssign.toast).toEqual({
      kind: "success", tone: "success",
      message: t("projects.bulkAssignSuccess", { count: 1 }),
    });
    expect(outs.resync.toast).toEqual({
      kind: "success", tone: "success",
      message: t("projects.resyncSuccess", { synced: 1 }),
    });
    expect(outs.resyncAll.toast).toEqual({
      kind: "success", tone: "success",
      message: t("projects.resyncAllSuccess", { synced: 1 }),
    });
    for (const out of Object.values(outs)) {
      expect(out.errors).toEqual([]);
      expect(out.completion).toEqual({ reload: false, closeModal: true, conflict: false });
    }
  });

  it("a saturated bulk assign says so instead of counting zero; an empty resync still reports", () => {
    const saturated: ProjectSyncReport = {
      items: [projectItem("claude", { status: "already_assigned" })],
    };
    expect(projectSyncOutcome(saturated, { ...ctx, action: "bulkAssign" }).toast).toEqual({
      kind: "success", tone: "success",
      message: "projects.bulkAssignNothing",
    });
    expect(projectSyncOutcome({ items: [] }, { ...ctx, action: "resync" }).toast).toEqual({
      kind: "success", tone: "success",
      message: t("projects.resyncSuccess", { synced: 0 }),
    });
  });

  it("carries the new copy", () => {
    const en = resources.en.translation.projects;
    expect(en.bulkAssignSuccess_other).toContain("{{count}}");
    expect(en.bulkUnassignSuccess_other).toContain("{{count}}");
    expect(en.bulkUnassignPartial).toContain("{{failed}}");
    expect(en.resyncAllPartial).toContain("{{failed}}");
    expect(en.resyncAllSuccess).toContain("{{synced}}");
    expect(en.bulkAssignSuccess_one).toBe("Assigned to {{count}} tool");
    expect(resources.en.translation.localSkillInvalid.insideToolDir).toBe(
      "This folder is a Tool's own skills copy — use Import instead",
    );
  });
});

const syncReport: BatchTargetOutcome[] = [
    {
      skill_id: "git",
      skill_name: "git",
      tool_key: "claude",
      status: { status: "synced", outcome: { mode_used: "copy", target_path: "/target", replaced: false } },
    },
    {
      skill_id: "git",
      skill_name: "git",
      tool_key: "unknown",
      status: { status: "failed", error: { code: "OTHER", message: "failed" } },
    },
    {
      skill_id: "git",
      skill_name: "git",
      tool_key: "absent",
      status: {
        status: "skipped",
        error: { code: "TOOL_NOT_INSTALLED", tool: "absent" },
      },
    },
    {
      skill_id: "git",
      skill_name: "git",
      tool_key: "claude",
      status: {
        status: "skipped",
        error: { code: "TOOL_NOT_WRITABLE", tool: "claude", path: "/locked" },
      },
    },
];
describe("syncOutcome", () => {
  it.each([
    { action: "bulk", errors: 1, warnings: 1 },
    { action: "install", errors: 2, warnings: 1 },
    { action: "toggle", errors: 3, warnings: 0 },
  ] as const)(
    "$action selects the intended failures/skips",
    ({ action, errors, warnings }) => {
      const out = syncOutcome(syncReport, { ...ctx, action });
      expect(out.errors).toHaveLength(errors);
      expect(out.errors[0]).toEqual({
        title: t("errors.syncFailedTitle", { name: "git", tool: "unknown" }),
        message: "failed",
      });
      // A not-detected skip is a per-tool warning for bulk/install (the
      // selection is stale, nothing failed); an explicit toggle keeps it
      // as the error it already was.
      expect(out.warnings).toEqual(
        warnings
          ? [
              {
                title: t("errors.syncSkippedNotInstalledTitle", { tool: "absent" }),
                message: t("errors.syncSkippedNotInstalledMessage", { count: 1 }),
              },
            ]
          : [],
      );
      // Bulk answers the composite line; install leaves the toast to
      // installOutcome; a failed toggle is its error entries alone.
      expect(out.toast).toEqual(
        action === "bulk"
          ? {
              kind: "warning",
              tone: "failure",
              message: line(["outcome.done.synced", 1], ["outcome.failed", 1], ["outcome.skipped", 2]),
            }
          : null,
      );
      expect(out.completion).toEqual({
        reload: action !== "toggle",
        closeModal: false,
        conflict: false,
      });
    },
  );
  it.each(["bulk", "install", "toggle"] as const)("%s success", (action) => {
    const out = syncOutcome(
      [syncReport[0]],
      { ...ctx, action },
    );
    expect(out.errors).toEqual([]);
    expect(out.completion).toEqual({
      reload: true,
      closeModal: true,
      conflict: false,
    });
    expect(out.toast).toEqual(
      action === "install"
        ? null
        : {
            kind: "success",
            tone: "success",
            message:
              action === "toggle"
                ? "status.syncEnabled"
                : line(["outcome.done.synced", 1]),
          },
    );
  });
  it("folds one warning per not-detected tool across skills, labelled, without touching completion", () => {
    const skip = (skill: string, tool: string): BatchTargetOutcome => ({
      skill_id: skill,
      skill_name: skill,
      tool_key: tool,
      status: { status: "skipped", error: { code: "TOOL_NOT_INSTALLED", tool } },
    });
    const out = syncOutcome(
      [syncReport[0], skip("a", "claude"), skip("b", "claude"), skip("a", "absent")],
      { ...ctx, action: "bulk" },
    );
    expect(out.errors).toEqual([]);
    expect(out.warnings).toEqual([
      {
        title: t("errors.syncSkippedNotInstalledTitle", { tool: "CLAUDE" }),
        message: t("errors.syncSkippedNotInstalledMessage", { count: 2 }),
      },
      {
        title: t("errors.syncSkippedNotInstalledTitle", { tool: "absent" }),
        message: t("errors.syncSkippedNotInstalledMessage", { count: 1 }),
      },
    ]);
    expect(out.completion).toEqual({ reload: true, closeModal: true, conflict: false });
    // A stale selection fails nothing, but the line counts the skips and
    // the skipped tone outranks success (D10).
    expect(out.toast).toEqual({
      kind: "warning",
      tone: "skipped",
      message: line(["outcome.done.synced", 1], ["outcome.skipped", 3]),
    });
  });
  it("explicit TARGET_EXISTS toggle shows the path and neither reloads nor succeeds", () => {
    const out = syncOutcome(
      [
          {
            ...syncReport[0],
            status: {
              status: "skipped",
              error: { code: "TARGET_EXISTS", path: "/conflict", modifiedAt: null, sameContent: false },
            },
          },
      ],
      { ...ctx, action: "toggle" },
    );
    expect(out.errors[0].message).toBe(
      t("errors.targetExistsDetail", { path: "/conflict" }),
    );
    expect(out.completion.reload).toBe(false);
    expect(out.toast).toBeNull();
  });

  // D10: "Keep existing" is the operator's answer — never an error entry,
  // never a failure; one quiet note per pair.
  const declinedRow = (skill: string, tool: string): BatchTargetOutcome => ({
    skill_id: skill,
    skill_name: skill,
    tool_key: tool,
    status: { status: "declined" },
  });
  it("a declined-only bulk sync is one quiet line per pair, declined-toned", () => {
    const out = syncOutcome(
      [declinedRow("ts-reset", "claude"), declinedRow("zod", "cursor")],
      { ...ctx, action: "bulk" },
    );
    const notes = [
      t("outcome.declinedLine", { name: "ts-reset", tool: "CLAUDE" }),
      t("outcome.declinedLine", { name: "zod", tool: "cursor" }),
    ];
    expect(out).toEqual({
      toast: {
        kind: "info",
        tone: "declined",
        message: line(["outcome.declined", 2]),
        detail: notes.join("\n"),
      },
      errors: [],
      warnings: [],
      notes,
      completion: { reload: true, closeModal: true, conflict: false },
      // The structured counts behind the line, for combineOutcomes.
      tally: { done: [{ verb: "synced", count: 0 }], counts: { failed: 0, skipped: 0, declined: 2 } },
    });
  });
  it("a declined pair beside synced ones keeps the declined tone; beside a failure, failure wins", () => {
    const withSynced = syncOutcome([syncReport[0], declinedRow("git", "cursor")], { ...ctx, action: "bulk" });
    expect(withSynced.toast).toMatchObject({
      tone: "declined",
      message: line(["outcome.done.synced", 1], ["outcome.declined", 1]),
    });
    const withFailure = syncOutcome([syncReport[1], declinedRow("git", "cursor")], { ...ctx, action: "bulk" });
    expect(withFailure.toast).toMatchObject({
      kind: "warning",
      tone: "failure",
      message: line(["outcome.failed", 1], ["outcome.declined", 1]),
    });
    expect(withFailure.errors).toHaveLength(1);
  });
  it("a declined toggle says what was kept instead of claiming the sync", () => {
    const out = syncOutcome([declinedRow("ts-reset", "claude")], { ...ctx, action: "toggle" });
    expect(out.toast).toEqual({
      kind: "info",
      tone: "declined",
      message: t("outcome.declinedLine", { name: "ts-reset", tool: "CLAUDE" }),
    });
    expect(out.errors).toEqual([]);
    expect(out.completion).toEqual({ reload: true, closeModal: true, conflict: false });
  });
});

const imported = (
  overrides: Partial<
    Extract<ImportGroupStatus, { status: "imported" }>
  > = {},
): ImportGroupStatus => ({
  status: "imported",
  skill_id: "git",
  skill_name: "git",
  targets: [],
  forced_tools: [],
  originals: [],
  ...overrides,
});
const importReport = (status: ImportGroupStatus): ImportReport => ({
  groups: [{ group_name: "git", status }],
});
describe("importOutcome", () => {
  it("derives mixed group totals without counting target/original failures as failed imports", () => {
    const report: ImportReport = {
      groups: [
        { group_name: "clean", status: imported() },
        { group_name: "partial", status: imported({ targets: [syncReport[1]], originals: [
          { tool: "claude", path: "/original", status: { status: "failed", error: { code: "OTHER", message: "busy" } } },
        ] }) },
        { group_name: "failed", status: { status: "failed", error: { code: "OTHER", message: "admission failed" } } },
      ],
    };
    const before = JSON.stringify(report);
    const out = importOutcome(report, ctx);
    // Groups are the unit; a target or cleanup failure under an imported
    // group is a failed target, never a failed import.
    expect(out.toast).toEqual({
      kind: "warning",
      tone: "failure",
      message: line(["outcome.done.imported", 2], ["outcome.failed", 1], ["outcome.targetsFailed", 2]),
    });
    expect(out.errors).toHaveLength(3);
    expect(out.completion).toEqual({ reload: true, closeModal: false, conflict: false });
    expect(JSON.stringify(report)).toBe(before);
  });
  it.each([
    { name: "success", status: imported(), errors: 0, warnings: 0 },
    {
      name: "refused group",
      status: {
        status: "failed",
        error: { code: "SKILL_INVALID", reason: "missing_skill_md" },
      } as ImportGroupStatus,
      errors: 1,
      warnings: 0,
    },
    {
      name: "target failed",
      status: imported({
        targets: [
          {
            ...syncReport[0],
            status: {
              status: "failed",
              error: { code: "TARGET_EXISTS", path: "/conflict", modifiedAt: null, sameContent: false },
            },
          },
        ],
      }),
      errors: 1,
      warnings: 0,
    },
    {
      name: "target skipped",
      status: imported({ targets: [syncReport[3]] }),
      errors: 1,
      warnings: 0,
    },
    {
      name: "divergent original",
      status: imported({
        originals: [
          {
            tool: "claude",
            path: "/original",
            status: { status: "kept_divergent" },
          },
        ],
      }),
      errors: 0,
      warnings: 1,
    },
    {
      name: "cleanup failed",
      status: imported({
        originals: [
          {
            tool: "unknown",
            path: "/original",
            status: {
              status: "failed",
              error: { code: "OTHER", message: "busy" },
            },
          },
        ],
      }),
      errors: 1,
      warnings: 0,
    },
    {
      name: "removed original",
      status: imported({
        originals: [
          { tool: "claude", path: "/original", status: { status: "removed" } },
        ],
      }),
      errors: 0,
      warnings: 0,
    },
    {
      name: "original left in place by choice",
      status: imported({
        originals: [
          { tool: "claude", path: "/original", status: { status: "left_in_place" } },
        ],
      }),
      errors: 0,
      warnings: 0,
    },
    {
      name: "managed name without a decision",
      status: {
        status: "skipped",
        reason: { reason: "name_managed", skill_id: "s1" },
      } as ImportGroupStatus,
      errors: 0,
      warnings: 1,
    },
    {
      name: "replaced over an identical copy",
      status: {
        status: "replaced",
        skill_id: "s1",
        skill_name: "git",
        targets: [syncReport[0]],
      } as ImportGroupStatus,
      errors: 0,
      warnings: 0,
    },
    {
      name: "replace answered TARGET_EXISTS",
      status: {
        status: "replaced",
        skill_id: "s1",
        skill_name: "git",
        targets: [
          {
            ...syncReport[0],
            status: { status: "failed", error: { code: "TARGET_EXISTS", path: "/copy", modifiedAt: null, sameContent: false } },
          },
        ],
      } as ImportGroupStatus,
      errors: 1,
      warnings: 0,
    },
  ])("$name", ({ name, status, errors, warnings }) => {
    const out = importOutcome(importReport(status), ctx);
    expect(out.errors).toHaveLength(errors);
    expect(out.warnings).toHaveLength(warnings);
    expect(out.completion).toEqual({
      reload: true,
      closeModal: errors + warnings === 0,
      conflict: false,
    });
    const imported1: [string, number] = ["outcome.done.imported", 1];
    const expected: Record<string, [string, string]> = {
      "success": ["success", line(imported1)],
      "refused group": ["failure", line(["outcome.failed", 1])],
      "target failed": ["failure", line(imported1, ["outcome.targetsFailed", 1])],
      "target skipped": ["failure", line(imported1, ["outcome.targetsFailed", 1])],
      "divergent original": ["conflict", line(imported1, ["outcome.review.originals", 1])],
      "cleanup failed": ["failure", line(imported1, ["outcome.targetsFailed", 1])],
      "removed original": ["success", line(imported1)],
      "original left in place by choice": ["success", line(imported1)],
      "managed name without a decision": ["skipped", line(["outcome.skipped", 1])],
      "replaced over an identical copy": ["success", line(imported1)],
      "replace answered TARGET_EXISTS": ["failure", line(imported1, ["outcome.targetsFailed", 1])],
    };
    const [tone, message] = expected[name];
    expect(out.toast).toEqual({ kind: tone === "success" ? "success" : "warning", tone, message });
  });
  it("reports a not-detected target as a per-tool warning that does not hold the import open", () => {
    const out = importOutcome(importReport(imported({ targets: [syncReport[2]] })), ctx);
    expect(out.errors).toEqual([]);
    expect(out.warnings).toEqual([
      {
        title: t("errors.syncSkippedNotInstalledTitle", { tool: "absent" }),
        message: t("errors.syncSkippedNotInstalledMessage", { count: 1 }),
      },
    ]);
    expect(out.completion).toEqual({ reload: true, closeModal: true, conflict: false });
    // Import's unit is the group: a stale tool selection is a warning entry,
    // not a skipped import, so the line stays clean.
    expect(out.toast).toEqual({ kind: "success", tone: "success", message: line(["outcome.done.imported", 1]) });
  });
  it("keeps forced-tool explanations as separate detail lines with label fallback", () => {
    expect(
      importOutcome(
        importReport(imported({ forced_tools: ["claude", "unknown"] })),
        ctx,
      ).toast,
    ).toEqual({
      kind: "success",
      tone: "success",
      message: line(["outcome.done.imported", 1]),
      detail: [
        t("status.importSourceToolForced", { name: "git", tool: "CLAUDE" }),
        t("status.importSourceToolForced", { name: "git", tool: "unknown" }),
      ].join("\n"),
    });
  });
  it("preserves actionable target paths and divergent-original warnings", () => {
    const out = importOutcome(
      importReport(
        imported({
          targets: [
            {
              ...syncReport[0],
              status: {
                status: "failed",
                error: { code: "TARGET_EXISTS", path: "/conflict", modifiedAt: null, sameContent: false },
              },
            },
          ],
          originals: [
            {
              tool: "claude",
              path: "/original",
              status: { status: "kept_divergent" },
            },
          ],
        }),
      ),
      ctx,
    );
    expect(out.errors[0].message).toBe(
      t("errors.syncTargetExistsMessage", { path: "/conflict" }),
    );
    expect(out.warnings[0]).toEqual({
      title: t("errors.importKeptDivergentTitle", {
        name: "git",
        tool: "CLAUDE",
      }),
      message: t("errors.importKeptDivergentMessage", { path: "/original" }),
    });
  });
  it("a declined target is a quiet note after the forced-tool lines, never an error", () => {
    const out = importOutcome(
      importReport(imported({
        forced_tools: ["claude"],
        targets: [{ ...syncReport[0], tool_key: "cursor", status: { status: "declined" } }],
      })),
      ctx,
    );
    const note = t("outcome.declinedLine", { name: "git", tool: "cursor" });
    expect(out.errors).toEqual([]);
    expect(out.notes).toEqual([note]);
    expect(out.completion).toEqual({ reload: true, closeModal: true, conflict: false });
    expect(out.toast).toEqual({
      kind: "info",
      tone: "declined",
      message: line(["outcome.done.imported", 1], ["outcome.declined", 1]),
      detail: [t("status.importSourceToolForced", { name: "git", tool: "CLAUDE" }), note].join("\n"),
    });
  });
});

const installed: InstallResultDto = {
  skill_id: "git",
  name: "git",
  central_path: "/hub/git",
  content_hash: null,
};
describe("installOutcome", () => {
  it.each(["git", "local", "selection"] as const)("%s result", (source) => {
    const out = installOutcome(
      [
        {
          name: "git",
          status: "installed",
          result: installed,
          deployment: { status: "disabled" },
        },
      ],
      { ...ctx, source },
    );
    expect(out.completion).toEqual({
      reload: true,
      closeModal: true,
      conflict: false,
    });
    // A clean single git/local install keeps its sentence; the picker's
    // batch answers the count line.
    expect(out.toast).toEqual({
      kind: "success",
      tone: "success",
      message:
        source === "selection"
          ? line(["outcome.done.installed", 1])
          : source === "git"
            ? "status.gitSkillCreated"
            : "status.localSkillCreated",
    });
  });
  it.each([
    { status: "no-targets" },
    { status: "failed", error: new Error("deploy request failed") },
    { status: "reported", report: syncReport },
  ] satisfies InstallDeployment[])(
    "installed bytes still reload/close after deployment $status",
    (deployment) => {
      const out = installOutcome(
        [{ name: "git", status: "installed", result: installed, deployment }],
        { ...ctx, source: "selection" },
      );
      expect(out.errors.length).toBeGreaterThan(0);
      expect(out.toast).toEqual({
        kind: "warning",
        tone: "failure",
        message: line(["outcome.done.installed", 1], ["outcome.targetsFailed", out.errors.length]),
      });
      expect(out.completion).toEqual({
        reload: true,
        closeModal: true,
        conflict: false,
      });
    },
  );
  it.each(
    (
      [
        [],
        [{ name: "git", status: "failed", error: new Error("clone failed") }],
      ] satisfies InstallSettlement[][]
    ).map((report) => ({ report })),
  )(
    "no installed bytes keeps the picker open and avoids reload/success",
    ({ report }) => {
      const out = installOutcome(report, { ...ctx, source: "selection" });
      expect(out.completion).toEqual({
        reload: false,
        closeModal: false,
        conflict: false,
      });
      expect(out.toast).toBeNull();
      expect(out.errors).toHaveLength(report.length);
    },
  );
  it("collects a partial batch in candidate order and completes", () => {
    const out = installOutcome(
      [
        { name: "first", status: "failed", error: new Error("clone failed") },
        {
          name: "git",
          status: "installed",
          result: installed,
          deployment: { status: "no-targets" },
        },
      ],
      { ...ctx, source: "selection" },
    );
    expect(out.errors).toEqual([
      {
        title: t("errors.importFailedTitle", { name: "first" }),
        message: "clone failed",
      },
      {
        title: t("errors.unsyncedTitle", { name: "git" }),
        message: "errors.noSyncTargets",
      },
    ]);
    expect(out.completion.reload).toBe(true);
    expect(out.toast).toEqual({
      kind: "warning",
      tone: "failure",
      message: line(["outcome.done.installed", 1], ["outcome.failed", 1], ["outcome.targetsFailed", 1]),
    });
  });
  // Shape only: ticket 04's install batch answers `cancelled` for the
  // candidates it never reached after the operator cancelled.
  const clean = (name: string): InstallSettlement => ({
    name, status: "installed", result: { ...installed, name }, deployment: { status: "disabled" },
  });
  const cancelledCandidate = (name: string): InstallSettlement => ({ name, status: "cancelled" });
  it("a cancelled tail is counted, raises no entry, and reads neutral", () => {
    const out = installOutcome(
      [clean("a"), clean("b"), cancelledCandidate("c"), cancelledCandidate("d"), cancelledCandidate("e")],
      { ...ctx, source: "selection" },
    );
    expect(out.errors).toEqual([]);
    expect(out.warnings).toEqual([]);
    expect(out.toast).toEqual({
      kind: "info",
      tone: "neutral",
      message: line(["outcome.done.installed", 2], ["outcome.cancelled", 3]),
    });
    expect(out.completion).toEqual({ reload: true, closeModal: true, conflict: false });
  });
  it("a batch cancelled before its first candidate installs nothing and keeps the picker open", () => {
    const out = installOutcome(
      [cancelledCandidate("a"), cancelledCandidate("b")],
      { ...ctx, source: "selection" },
    );
    expect(out.toast).toEqual({ kind: "info", tone: "neutral", message: line(["outcome.cancelled", 2]) });
    expect(out.completion).toEqual({ reload: false, closeModal: false, conflict: false });
  });
  it("a failure before the cancel keeps the failure tone", () => {
    const out = installOutcome(
      [clean("a"), { name: "b", status: "failed", error: new Error("clone failed") }, cancelledCandidate("c")],
      { ...ctx, source: "selection" },
    );
    expect(out.toast).toMatchObject({
      tone: "failure",
      message: line(["outcome.done.installed", 1], ["outcome.failed", 1], ["outcome.cancelled", 1]),
    });
    expect(out.errors).toHaveLength(1);
  });
  it("a declined deployment pair is a quiet note on the install's line", () => {
    const out = installOutcome(
      [{
        name: "git",
        status: "installed",
        result: installed,
        deployment: {
          status: "reported",
          report: [syncReport[0], { ...syncReport[0], tool_key: "cursor", status: { status: "declined" } }],
        },
      }],
      { ...ctx, source: "git" },
    );
    const note = t("outcome.declinedLine", { name: "git", tool: "cursor" });
    expect(out.errors).toEqual([]);
    expect(out.notes).toEqual([note]);
    expect(out.toast).toEqual({
      kind: "info",
      tone: "declined",
      message: line(["outcome.done.installed", 1], ["outcome.declined", 1]),
      detail: note,
    });
  });
});

// Review round 2026-09-29 (ticket 01, F6): a Disabled pair is skipped by
// design in every report kind — never an error entry, never a failure toast.
describe("Disabled is a skip in every fold", () => {
  const disabledTarget: BatchTargetOutcome = {
    skill_id: "s1",
    skill_name: "alpha",
    tool_key: "claude",
    status: { status: "skipped", error: { code: "TARGET_DISABLED", tool: "claude" } },
  };
  const disabledPropagation = [
    { scope: { scope: "global", tool: "claude" }, status: { status: "skipped", reason: { reason: "disabled" } } },
    { scope: { scope: "project", project_id: "p1", tool: "claude" }, status: { status: "skipped", reason: { reason: "disabled" } } },
  ] satisfies InvocationEditReport["propagation"]["targets"];
  const refreshedDisabled = (): SkillRefreshOutcome => {
    const row = refreshed();
    if (row.status.status !== "refreshed") throw new Error("fixture");
    row.status.targets.push(...disabledPropagation);
    return row;
  };
  const cases: [string, () => { errors: unknown[]; toast: { kind: string } | null }][] = [
    ["sync bulk", () => syncOutcome([disabledTarget], { ...ctx, action: "bulk" })],
    ["sync install", () => syncOutcome([disabledTarget], { ...ctx, action: "install" })],
    ["sync toggle", () => syncOutcome([disabledTarget], { ...ctx, action: "toggle" })],
    ["install", () => installOutcome(
      [{ name: "git", status: "installed", result: installed, deployment: { status: "reported", report: [disabledTarget] } }],
      { ...ctx, source: "git" },
    )],
    ["import", () => importOutcome(importReport(imported({ targets: [disabledTarget] })), ctx)],
    ["refresh", () => refreshOutcome(refreshReport([refreshedDisabled()]), ctx)],
    ["edit", () => invocationEditOutcome(
      { skill_id: "s1", skill_name: "alpha", propagation: { targets: disabledPropagation } },
      ctx,
    )],
    ["enable", () => enableOutcome({ global: [disabledTarget], projects: { items: [] } }, ctx)],
    ["project resync", () => projectSyncOutcome(
      { items: [projectItem("claude", { status: "skipped", reason: { reason: "disabled" } })] },
      { ...ctx, action: "resync" },
    )],
  ];
  it.each(cases)("%s", (_name, fold) => {
    const out = fold();
    expect(out.errors).toEqual([]);
    expect(out.toast?.kind ?? "none").not.toBe("error");
    if (out.toast) expect(JSON.stringify(out.toast)).not.toContain("partialFailure");
  });
  it("import of a disabled pair completes like a clean import", () => {
    expect(importOutcome(importReport(imported({ targets: [disabledTarget] })), ctx).toast).toEqual({
      kind: "success", tone: "success",
      message: t("outcome.done.imported", { count: 1 }),
    });
  });
});

// F7: disabled assignments are recorded, not synced — counted as such.
describe("projectSyncOutcome counts disabled skips", () => {
  const skip = { status: "skipped", reason: { reason: "disabled" } } as const;
  it("an all-skipped bulk assign says assigned-not-synced, never 'already assigned'", () => {
    const out = projectSyncOutcome(
      { items: [projectItem("claude", skip), projectItem("pi", skip)] },
      { ...ctx, action: "bulkAssign" },
    );
    expect(out.toast).toEqual({
      kind: "warning", tone: "skipped",
      message: t("disabledSkips.bulkAssignNotSynced", { count: 2 }),
    });
    expect(out.errors).toEqual([]);
    expect(out.completion.closeModal).toBe(true);
  });
  it("mixed synced/skipped and synced/skipped/failed bulk assigns carry every count", () => {
    const mixed = { items: [projectItem("claude", { status: "synced" }), projectItem("pi", skip)] };
    expect(projectSyncOutcome(mixed, { ...ctx, action: "bulkAssign" }).toast).toEqual({
      kind: "warning", tone: "skipped",
      message: t("disabledSkips.bulkAssignMixed", { assigned: 1, skipped: 1 }),
    });
    const partial = { items: [...mixed.items, projectItem("codex", denied)] };
    const out = projectSyncOutcome(partial, { ...ctx, action: "bulkAssign" });
    expect(out.toast).toEqual({
      kind: "warning", tone: "failure",
      message: t("disabledSkips.bulkAssignPartial", { assigned: 1, failed: 1, skipped: 1 }),
    });
    expect(out.errors).toHaveLength(1);
  });
  it("resync summaries include the skipped count", () => {
    const report = { items: [projectItem("claude", { status: "synced" }), projectItem("pi", skip)] };
    expect(projectSyncOutcome(report, { ...ctx, action: "resync" }).toast).toEqual({
      kind: "success", tone: "success",
      message: t("disabledSkips.resyncSuccess", { synced: 1, skipped: 1 }),
    });
    const partial = { items: [...report.items, projectItem("codex", denied)] };
    expect(projectSyncOutcome(partial, { ...ctx, action: "resyncAll" }).toast).toEqual({
      kind: "warning", tone: "failure",
      message: t("disabledSkips.resyncAllPartial", { synced: 1, failed: 1, skipped: 1 }),
    });
  });
});

// F1: clearing a marker (a row owning no artifact) is work done, not "nothing planned".
describe("removalOutcome with cleared markers", () => {
  it("a report that only cleared a marker row is a success", () => {
    const report: RemovalReport = {
      targets: [],
      central_removed: false,
      record_deleted: false,
      cleared_markers: [{ scope: "assignment", id: "a1", project_id: "p1", skill_id: "s1", tool: "claude" }],
    };
    const out = removalOutcome(report, { ...ctx, action: "toggle" });
    expect(out.toast).toEqual({ kind: "success", tone: "success", message: "status.syncDisabled" });
    expect(out.errors).toEqual([]);
  });

  // N1: a cleared marker is an unassignment, so bulk unassign counts it (the
  // copy says "removed from N tools", not that an artifact was removed).
  const marker = (tool: string) =>
    ({ scope: "assignment", id: `m-${tool}`, project_id: "p1", skill_id: "s1", tool }) as const;
  it("a markers-only bulk unassign counts every cleared assignment", () => {
    const report: RemovalReport = {
      targets: [],
      central_removed: false,
      record_deleted: false,
      cleared_markers: [marker("claude"), marker("pi")],
    };
    expect(removalOutcome(report, { ...ctx, action: "bulkUnassign" })).toEqual({
      toast: { kind: "success", tone: "success", message: t("projects.bulkUnassignSuccess", { count: 2 }) },
      errors: [],
      warnings: [],
      notes: [],
      completion: { reload: false, closeModal: true, conflict: false },
    });
  });
  it("a mixed bulk unassign counts removed targets and cleared markers together", () => {
    const report: RemovalReport = {
      targets: [
        {
          rows: [{ scope: "assignment", id: "a1", project_id: "p1", skill_id: "s1", tool: "codex" }],
          path: "/work/p1/.codex/skills/s1",
          status: { status: "removed" },
        },
      ],
      central_removed: false,
      record_deleted: false,
      cleared_markers: [marker("claude")],
    };
    expect(removalOutcome(report, { ...ctx, action: "bulkUnassign" }).toast).toEqual({
      kind: "success", tone: "success",
      message: t("projects.bulkUnassignSuccess", { count: 2 }),
    });
  });
});

describe("ticket 04: the install batch report and the project Keep", () => {
  it("installBatchSettlements maps candidates in order and tells 'not deployed' from 'no Tools'", () => {
    const report = {
      candidates: [
        {
          subpath: "s/a",
          name: "a",
          status: {
            status: "installed" as const,
            skill: { skill_id: "a", name: "a", central_path: "/hub/a", content_hash: null },
            targets: [],
          },
        },
        { subpath: "s/b", name: "b", status: { status: "failed" as const, error: { code: "OTHER" as const, message: "x" } } },
        { subpath: "s/c", name: "c", status: { status: "cancelled" as const } },
      ],
    };
    const kinds = (deploy: boolean, tools: string[]) =>
      installBatchSettlements(report, { deploy, tools }).map((s) =>
        s.status === "installed" ? `installed:${s.deployment.status}` : s.status,
      );
    expect(kinds(false, ["claude_code"])).toEqual(["installed:disabled", "failed", "cancelled"]);
    expect(kinds(true, [])).toEqual(["installed:no-targets", "failed", "cancelled"]);
    expect(kinds(true, ["claude_code"])).toEqual(["installed:reported", "failed", "cancelled"]);
  });

  it("a declined project assignment is a quiet note, never a failure", () => {
    const out = projectSyncOutcome(
      {
        items: [
          { assignment_id: "x", project_id: "p", skill_id: "a", skill_name: "alpha", tool: "claude_code", status: { status: "declined" } },
        ],
      },
      { t, action: "toggleOn" },
    );
    expect(out.errors).toEqual([]);
    expect(out.notes).toHaveLength(1);
    expect(out.toast?.tone).toBe("declined");
  });
});

describe("an answered overwrite ask inside Propagation-shaped reports (ticket 09 B2)", () => {
  it("a kept pair is a declined count and a quiet note on the Refresh headline, never a failure", () => {
    const out = refreshOutcome({ skills: [refreshed("a"), refreshed("b")] }, {
      ...ctx,
      declined: [{ skillName: "a", tool: "claude" }],
    });
    expect(out.errors).toEqual([]);
    expect(out.notes).toEqual(['outcome.declinedLine {"name":"a","tool":"CLAUDE"}']);
    expect(out.toast).toMatchObject({ tone: "declined", detail: out.notes[0] });
    expect(out.toast?.message).toContain("outcome.declined");
  });

  it("a batch of one keeps its sentence, toned declined, with the note as detail", () => {
    const out = refreshOutcome({ skills: [refreshed("a")] }, {
      ...ctx,
      single: { name: "a", success: "updated a" },
      declined: [{ skillName: "a", tool: "claude" }],
    });
    expect(out.toast).toMatchObject({ tone: "declined", message: "updated a" });
    expect(out.toast?.detail).toContain("outcome.declinedLine");
  });

  it("Edit's saved headline carries a kept pair as a note", () => {
    const out = invocationEditOutcome(
      { skill_id: "a", skill_name: "a", propagation: { targets: [] } },
      { ...ctx, declined: [{ skillName: "a", tool: "claude" }] },
    );
    expect(out.toast).toMatchObject({ tone: "declined", message: "invocationEdit.saved" });
    expect(out.notes).toHaveLength(1);
  });
});

describe("combineOutcomes (one action, several reports, one run)", () => {
  const part = (over: Partial<ReturnType<typeof syncOutcome>>): ReturnType<typeof syncOutcome> => ({
    toast: null,
    errors: [],
    warnings: [],
    notes: [],
    completion: { reload: false, closeModal: true, conflict: false },
    ...over,
  });

  it("with no tally, heads with the first toast, re-tones it when another part failed, and carries the others' notes", () => {
    const out = combineOutcomes(
      [
        part({ toast: { kind: "success", tone: "success", message: "3 synced" } }),
        part({
          errors: [{ title: "e", message: "m" }],
          notes: ["kept"],
          completion: { reload: true, closeModal: false, conflict: false },
        }),
      ],
      ctx,
    );
    expect(out.toast).toMatchObject({ tone: "failure", kind: "warning", message: "3 synced", detail: "kept" });
    expect(out.errors).toHaveLength(1);
    expect(out.notes).toEqual(["kept"]);
    expect(out.completion).toEqual({ reload: true, closeModal: false, conflict: false });
  });

  it("no part with a toast or a tally: none, and the notes stay rows", () => {
    const out = combineOutcomes([part({ notes: ["a"] }), part({ notes: ["b"] })], ctx);
    expect(out.toast).toBeNull();
    expect(out.notes).toEqual(["a", "b"]);
  });

  // Re-review B3-R1 (ticket 09): Deploy all = a fresh first sync + an Enable,
  // folded by the real folds. Every part's counts reach one composite line,
  // and the tone is the strongest of both (failure › conflict › skipped ›
  // declined › success) — an Enable Keep never reads as success.
  const target = (skill: string, tool: string, status: BatchTargetOutcome["status"]): BatchTargetOutcome => ({
    skill_id: skill,
    skill_name: skill,
    tool_key: tool,
    status,
  });
  const synced: BatchTargetOutcome["status"] = {
    status: "synced",
    outcome: { mode_used: "symlink", target_path: "/t", replaced: false },
  };
  const fresh = [target("beta", "claude", synced)];
  const enabledName = "alpha";
  const syncedN = (count: number) => ["outcome.done.synced", count] as [string, number];
  const enabledN = (count: number) => ["outcome.done.enabled", count] as [string, number];
  it.each([
    {
      enable: "one synced target",
      status: synced,
      tone: "success",
      message: line(syncedN(1), enabledN(1)),
      errors: 0,
      notes: 0,
    },
    {
      enable: "one declined target (Keep)",
      status: { status: "declined" } as BatchTargetOutcome["status"],
      tone: "declined",
      message: line(syncedN(1), ["outcome.declined", 1]),
      errors: 0,
      notes: 1,
    },
    {
      enable: "one Disabled skip",
      status: { status: "skipped", error: { code: "TARGET_DISABLED", tool: "cursor" } } as BatchTargetOutcome["status"],
      tone: "skipped",
      message: line(syncedN(1), ["outcome.skipped", 1]),
      errors: 0,
      notes: 0,
    },
    {
      enable: "one failed target",
      status: { status: "failed", error: { code: "OTHER", message: "denied" } } as BatchTargetOutcome["status"],
      tone: "failure",
      message: line(syncedN(1), ["outcome.failed", 1]),
      errors: 1,
      notes: 0,
    },
  ] as const)(
    "fresh sync success + Enable with $enable: one line, tone $tone",
    ({ status, tone, message, errors, notes }) => {
      const out = combineOutcomes(
        [
          syncOutcome(fresh, { ...ctx, action: "bulk" }),
          enableOutcome({ global: [target(enabledName, "cursor", status)], projects: { items: [] } }, ctx),
        ],
        ctx,
      );
      expect(out.toast).toMatchObject({ tone, message });
      expect(out.errors).toHaveLength(errors);
      expect(out.notes).toHaveLength(notes);
      // A declined pair's quiet line rides as the detail of the one headline.
      if (notes) expect(out.toast?.detail).toBe(out.notes.join("\n"));
      expect(out.completion.reload).toBe(true);
    },
  );

  it("sums a project row of Enable with the global parts, in targets", () => {
    const out = combineOutcomes(
      [
        syncOutcome([...fresh, target("beta", "cursor", { status: "declined" })], { ...ctx, action: "bulk" }),
        enableOutcome(
          {
            global: [target(enabledName, "claude", synced)],
            projects: { items: [projectItem("claude", { status: "declined" })] },
          },
          ctx,
        ),
      ],
      ctx,
    );
    expect(out.toast).toMatchObject({
      tone: "declined",
      message: line(syncedN(1), enabledN(1), ["outcome.declined", 2]),
    });
    expect(out.notes).toHaveLength(2);
  });
});
