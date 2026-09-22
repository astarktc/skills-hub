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
  deleteOutcome,
  invocationEditOutcome,
  importOutcome,
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

describe("refreshOutcome: one precedence and completion policy", () => {
  const cases = [
    {
      name: "success",
      rows: [refreshed()],
      severity: "success",
      key: "status.refreshCompleted",
      errors: 0,
      warnings: 0,
      close: true,
    },
    {
      name: "acquisition failure",
      rows: [failed],
      severity: "warning",
      key: 'status.refreshSummary {"refreshed":0,"failed":1}',
      errors: 1,
      warnings: 0,
      close: false,
    },
    {
      name: "skipped",
      rows: [skipped],
      severity: "warning",
      key: 'status.refreshSummarySkipped {"refreshed":0,"failed":0,"skipped":1}',
      errors: 0,
      warnings: 1,
      close: false,
    },
    {
      name: "target failure",
      rows: [changed("target")],
      severity: "warning",
      key: "partialFailure",
      errors: 1,
      warnings: 0,
      close: true,
    },
    {
      name: "reassert failure",
      rows: [changed("reassert")],
      severity: "warning",
      key: "partialFailure",
      errors: 1,
      warnings: 0,
      close: true,
    },
    {
      name: "conflict",
      rows: [changed("conflict")],
      severity: "warning",
      key: "invocationEdit.refreshCompletedWithEdits",
      errors: 0,
      warnings: 1,
      close: true,
    },
    {
      name: "acquisition skips are warnings with their own reason",
      rows: [skippedGone, skippedStale],
      severity: "warning",
      key: 'status.refreshSummarySkipped {"refreshed":0,"failed":0,"skipped":2}',
      errors: 0,
      warnings: 2,
      close: false,
    },
    {
      name: "failed beats skipped",
      rows: [failed, skipped],
      severity: "warning",
      key: 'status.refreshSummary {"refreshed":0,"failed":1}',
      errors: 1,
      warnings: 1,
      close: false,
    },
    {
      name: "conflict beats failed and skipped",
      rows: [
        changed("conflict"),
        failed,
        skipped,
        changed("target"),
        changed("reassert"),
      ],
      severity: "warning",
      key: "invocationEdit.refreshCompletedWithEdits",
      errors: 3,
      warnings: 2,
      close: false,
    },
    {
      name: "empty batch",
      rows: [],
      severity: "success",
      key: "status.refreshCompleted",
      errors: 0,
      warnings: 0,
      close: false,
    },
  ];
  it.each(cases)(
    "$name",
    ({ rows, severity, key, errors, warnings, close }) => {
      const report = refreshReport(rows);
      const before = JSON.stringify(report);
      const batch = refreshOutcome(report, ctx);
      const single = refreshOutcome(report, {
        ...ctx,
        single: { name: "git", success: "single success" },
      });
      expect(batch.toast?.kind).toBe(severity);
      expect(batch.toast?.message).toBe(key);
      expect(batch.errors).toHaveLength(errors);
      expect(batch.warnings).toHaveLength(warnings);
      expect(batch.completion).toEqual({
        reload: true,
        closeModal: close,
        conflict: key === "invocationEdit.refreshCompletedWithEdits",
      });
      expect(single.completion).toEqual({ ...batch.completion, reload: false });
      expect(single.errors).toEqual(batch.errors);
      expect(single.warnings).toEqual(batch.warnings);
      if (batch.completion.conflict) {
        expect(single.toast?.kind).toBe(batch.toast?.kind);
        expect(single.toast?.message).toBe(
          'invocationEdit.updateCompletedWithConflict {"name":"git"}',
        );
      } else if (severity === "success") {
        expect(single.toast).toEqual({ kind: "success", message: "single success" });
      } else {
        // A batch of one never repeats its single entry as a count summary.
        expect(single.toast).toBeNull();
      }
      expect(JSON.stringify(report)).toBe(before);
    },
  );

  it("acquisition skips preserve both reasons; the batch summary is neutral in both locales", () => {
    const out = refreshOutcome(refreshReport([skippedGone, skippedStale]), ctx);
    expect(out.warnings).toEqual([
      { title: 'errors.refreshSkippedTitle {"name":"gone"}', message: "errors.refreshSkippedSkillGone" },
      { title: 'errors.refreshSkippedTitle {"name":"stale"}', message: "errors.refreshSkippedStaleAcquisition" },
    ]);
    expect(out.completion).toEqual({ reload: true, closeModal: false, conflict: false });
    expect(out.toast).toEqual({ kind: "warning", message: 'status.refreshSummarySkipped {"refreshed":0,"failed":0,"skipped":2}' });
    expect(resources.en.translation.status.refreshSummarySkipped).toBe("{{refreshed}} skills refreshed, {{failed}} failed, {{skipped}} skipped.");
    expect(resources.zh.translation.status.refreshSummarySkipped).toBe("已刷新 {{refreshed}} 个 Skills，{{failed}} 个失败，{{skipped}} 个已跳过。");
  });

  it("returns a repair id, never a callback or captured ManagedSkill", () => {
    const out = refreshOutcome(refreshReport([failed]), ctx);
    expect(out.errors[0].action).toEqual({
      label: "gitRepoint.action",
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
  ] satisfies { name: string; propagation: InvocationEditReport["propagation"]["targets"]; errors: { title: string; message: string }[] }[])("$name: central-settled completion, no unconditional success", ({ propagation, errors }) => {
    const report: InvocationEditReport = { skill_id: "s1", skill_name: "alpha", propagation: { targets: propagation } };
    const before = JSON.stringify(report);
    expect(invocationEditOutcome(report, ctx)).toEqual({
      toast: errors.length ? null : { kind: "success", message: "invocationEdit.saved" },
      errors, warnings: [], completion: { reload: false, closeModal: true, conflict: false },
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
  record_deleted: false,
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
      record_deleted: false,
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
    expect(unsync.toast).toEqual({ kind: "warning", message: t("unsyncPartial", { count: 2, failed: 3 }) });
    expect(project.toast).toEqual({ kind: "warning", message: t("projects.removeKept", { count: 2, failed: 3 }) });
    expect(configure.toast).toEqual({ kind: "warning", message: t("projects.toolRemovalKept", { count: 2, failed: 3 }) });
    expect(deleted.toast).toEqual({ kind: "warning", message: t("status.skillDeleteKept", { failed: 3 }) });
    expect(deleted.completion).toEqual({ reload: true, closeModal: false, conflict: false });
    expect(JSON.stringify(report)).toBe(before);
  });

  it("delete succeeds for removed targets and has kept/retry copy in both locales", () => {
    expect(deleteOutcome({ ...removal(false), central_removed: true, record_deleted: true }, ctx)).toEqual({
      toast: { kind: "success", message: "status.skillRemoved" },
      errors: [], warnings: [], completion: { reload: true, closeModal: true, conflict: false },
    });
    expect(resources.en.translation.errors.deleteKeptTargetTitle).toContain("{{tool}}");
    expect(resources.zh.translation.errors.deleteKeptTargetTitle).toContain("{{tool}}");
    expect(resources.en.translation.status.skillDeleteKept).toBe("Skill kept: {{failed}} targets could not be removed. You can retry.");
    expect(resources.zh.translation.status.skillDeleteKept).toBe("技能已保留：{{failed}} 个同步目标无法移除。你可以重试。");
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
                kind: fail ? "warning" : "success",
                message: fail
                  ? t("unsyncPartial", { count: 0, failed: 1 })
                  : t("unsyncAllComplete", { count: 1 }),
              }
            : action === "toggle" && !fail
              ? { kind: "success", message: "status.syncDisabled" }
              : null,
        );
      }
    },
  );
  it("a zero-target report warns instead of claiming an unsync happened", () => {
    const nothing: RemovalReport = { targets: [], central_removed: false, record_deleted: false };
    for (const action of ["all", "skill", "toggle", "bulkUnassign"] as const) {
      const out = removalOutcome(nothing, { ...ctx, action });
      expect(out.toast).toEqual({
        kind: "warning",
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
      record_deleted: false,
    };
    expect(removalOutcome(report, { ...ctx, action: "bulkUnassign" })).toEqual({
      toast: { kind: "success", message: t("projects.bulkUnassignSuccess", { count: 2 }) },
      errors: [],
      warnings: [],
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
        kind: "warning",
        message: t("projects.bulkUnassignPartial", { count: 1, failed: 1 }),
      },
      errors: [{ title: t("errors.unsyncFailedTitle", { tool: "pi" }), message: "busy" }],
      warnings: [],
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
      record_deleted: false,
    };
    const kept = [
      { title: t("errors.projectRemovalFailedTitle", { tool: "pi" }), message: "denied" },
    ];
    expect(projectRemovalOutcome(report, { ...ctx, action: "removeProject" })).toEqual({
      toast: { kind: "warning", message: t("projects.removeKept", { count: 1, failed: 1 }) },
      errors: kept,
      warnings: [],
      completion: { reload: false, closeModal: false, conflict: false },
    });
    expect(projectRemovalOutcome(report, { ...ctx, action: "configureTools" })).toEqual({
      toast: { kind: "warning", message: t("projects.toolRemovalKept", { count: 1, failed: 1 }) },
      errors: kept,
      warnings: [],
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
      record_deleted: false,
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
      record_deleted: false,
    };
    const nothing: RemovalReport = { targets: [], central_removed: false, record_deleted: false };
    for (const report of [clean, nothing]) {
      expect(projectRemovalOutcome(report, { ...ctx, action: "removeProject" })).toEqual({
        toast: { kind: "success", message: "projects.removeComplete" },
        errors: [],
        warnings: [],
        completion: { reload: false, closeModal: true, conflict: false },
      });
      expect(projectRemovalOutcome(report, { ...ctx, action: "configureTools" })).toEqual({
        toast: null,
        errors: [],
        warnings: [],
        completion: { reload: false, closeModal: true, conflict: false },
      });
    }
  });
  it("delete's successful report reloads and closes", () => {
    expect(deleteOutcome({ targets: [], central_removed: true, record_deleted: true }, ctx)).toEqual({
      toast: { kind: "success", message: "status.skillRemoved" },
      errors: [],
      warnings: [],
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
      expect(out.errors).toEqual([
        {
          title: t("errors.syncFailedTitle", { name: "s2-name", tool: "CLAUDE" }),
          message: "denied",
        },
        {
          title: t("errors.syncFailedTitle", { name: "s1-name", tool: "unknown" }),
          message: t("errors.pathOutsideToolDirs", { path: "/x" }),
        },
      ]);
      expect(out.warnings).toEqual([]);
      // The project world applies the view the mutation returned.
      expect(out.completion).toEqual({ reload: false, closeModal: false, conflict: false });
    }
    expect(JSON.stringify(report)).toBe(before);
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
        kind: "warning",
        message: t("projects.bulkAssignPartial", { assigned: 2, failed: 1 }),
      },
      resync: {
        kind: "warning",
        message: t("projects.resyncPartial", { synced: 2, failed: 1 }),
      },
      resyncAll: {
        kind: "warning",
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
    expect(outs.toggleOn.toast).toEqual({ kind: "success", message: "status.syncEnabled" });
    expect(outs.bulkAssign.toast).toEqual({
      kind: "success",
      message: t("projects.bulkAssignSuccess", { count: 1 }),
    });
    expect(outs.resync.toast).toEqual({
      kind: "success",
      message: t("projects.resyncSuccess", { synced: 1 }),
    });
    expect(outs.resyncAll.toast).toEqual({
      kind: "success",
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
      kind: "success",
      message: "projects.bulkAssignNothing",
    });
    expect(projectSyncOutcome({ items: [] }, { ...ctx, action: "resync" }).toast).toEqual({
      kind: "success",
      message: t("projects.resyncSuccess", { synced: 0 }),
    });
  });

  it("carries the new copy in both locales", () => {
    const en = resources.en.translation.projects;
    const zh = resources.zh.translation.projects;
    for (const catalog of [en, zh]) {
      expect(catalog.bulkAssignSuccess_other).toContain("{{count}}");
      expect(catalog.bulkUnassignSuccess_other).toContain("{{count}}");
      expect(catalog.bulkUnassignPartial).toContain("{{failed}}");
      expect(catalog.resyncAllPartial).toContain("{{failed}}");
      expect(catalog.resyncAllSuccess).toContain("{{synced}}");
    }
    expect(en.bulkAssignSuccess_one).toBe("Assigned to {{count}} tool");
    expect(resources.en.translation.localSkillInvalid.insideToolDir).toBe(
      "This folder is a Tool's own skills copy — use Import instead",
    );
    expect(resources.zh.translation.localSkillInvalid.insideToolDir).toBeTruthy();
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
      expect(out.toast).toBeNull();
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
            message:
              action === "toggle"
                ? "status.syncEnabled"
                : "status.syncCompleted",
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
    expect(out.toast).toEqual({ kind: "success", message: "status.syncCompleted" });
  });
  it("explicit TARGET_EXISTS toggle shows the path and neither reloads nor succeeds", () => {
    const out = syncOutcome(
      [
          {
            ...syncReport[0],
            status: {
              status: "skipped",
              error: { code: "TARGET_EXISTS", path: "/conflict" },
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
    expect(out.toast).toEqual({ kind: "warning", message: t("status.importPartial", { imported: 2, failed: 1 }) });
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
              error: { code: "TARGET_EXISTS", path: "/conflict" },
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
  ])("$name", ({ status, errors, warnings }) => {
    const out = importOutcome(importReport(status), ctx);
    expect(out.errors).toHaveLength(errors);
    expect(out.warnings).toHaveLength(warnings);
    expect(out.completion).toEqual({
      reload: true,
      closeModal: errors + warnings === 0,
      conflict: false,
    });
    expect(out.toast?.kind).toBe(errors + warnings ? "warning" : "success");
    expect(out.toast?.message).toBe(
      status.status === "failed"
        ? t("status.importPartial", { imported: 0, failed: 1 })
        : errors + warnings ? "partialFailure" : "status.importCompleted",
    );
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
    expect(out.toast).toEqual({ kind: "success", message: "status.importCompleted" });
  });
  it("keeps forced-tool explanations as separate detail lines with label fallback", () => {
    expect(
      importOutcome(
        importReport(imported({ forced_tools: ["claude", "unknown"] })),
        ctx,
      ).toast,
    ).toEqual({
      kind: "success",
      message: "status.importCompleted",
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
                error: { code: "TARGET_EXISTS", path: "/conflict" },
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
    expect(out.toast).toEqual({
      kind: "success",
      message:
        source === "selection"
          ? "status.selectedSkillsInstalled"
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
      expect(out.toast?.kind).toBe("warning");
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
    expect(out.toast?.kind).toBe("warning");
  });
});
