import { describe, expect, it } from "vitest";
import type {
  BatchSyncReportDto,
  ImportGroupStatusDto,
  ImportReportDto,
  InstallResultDto,
  RefreshReportDto,
  RemovalReportDto,
  SkillRefreshResultDto,
} from "../bindings";
import {
  deleteOutcome,
  importOutcome,
  installOutcome,
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
const refreshed = (id = "git"): SkillRefreshResultDto => ({
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
const failed: SkillRefreshResultDto = {
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
const skipped: SkillRefreshResultDto = {
  skill_id: "local",
  skill_name: "local",
  status: { status: "skipped", state: "source_missing" },
};
function changed(
  kind: "conflict" | "target" | "reassert",
): SkillRefreshResultDto {
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
function refreshReport(skills: SkillRefreshResultDto[]): RefreshReportDto {
  return {
    skills,
    refreshed: skills.filter((s) => s.status.status === "refreshed").length,
    failed: skills.filter((s) => s.status.status === "failed").length,
    skipped: skills.filter((s) => s.status.status === "skipped").length,
    target_failures: skills.reduce(
      (n, s) =>
        n +
        (s.status.status === "refreshed"
          ? s.status.targets.filter((t) => t.status.status === "failed")
              .length + Number(Boolean(s.status.reassert_error))
          : 0),
      0,
    ),
  };
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
      key: "status.refreshSummary",
      errors: 1,
      warnings: 0,
      close: false,
    },
    {
      name: "skipped",
      rows: [skipped],
      severity: "warning",
      key: "status.refreshSummarySkipped",
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
      name: "failed beats skipped",
      rows: [failed, skipped],
      severity: "warning",
      key: "status.refreshSummary",
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
      expect(batch.toast?.message).toContain(key);
      expect(batch.errors).toHaveLength(errors);
      expect(batch.warnings).toHaveLength(warnings);
      expect(batch.completion).toEqual({
        reload: true,
        closeModal: close,
        conflict: rows.some(
          (s) =>
            s.status.status === "refreshed" && Boolean(s.status.edit_conflict),
        ),
      });
      expect(single.completion).toEqual(batch.completion);
      expect(single.errors).toEqual(batch.errors);
      expect(single.warnings).toEqual(batch.warnings);
      expect(single.toast?.kind).toBe(batch.toast?.kind);
      expect(single.toast?.message).toBe(
        batch.completion.conflict
          ? t("invocationEdit.updateCompletedWithConflict", { name: "git" })
          : severity === "success"
            ? "single success"
            : batch.toast?.message,
      );
      expect(JSON.stringify(report)).toBe(before);
    },
  );

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

const removal = (fail: boolean): RemovalReportDto => ({
  targets: [
    {
      scope: { scope: "global" },
      tool: "unknown",
      path: "/target",
      status: fail
        ? { status: "failed", error: { code: "OTHER", message: "busy" } }
        : { status: "removed" },
    },
  ],
  removed: fail ? 0 : 1,
  failed: fail ? 1 : 0,
});
describe("removalOutcome", () => {
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
  it("delete's null result reloads and closes; command errors are not reports", () => {
    expect(deleteOutcome(null, ctx)).toEqual({
      toast: { kind: "success", message: "status.skillRemoved" },
      errors: [],
      warnings: [],
      completion: { reload: true, closeModal: true, conflict: false },
    });
  });
});

const syncReport: BatchSyncReportDto = {
  results: [
    {
      skill_id: "git",
      skill_name: "git",
      tool: "claude",
      status: { status: "synced", mode_used: "copy" },
    },
    {
      skill_id: "git",
      skill_name: "git",
      tool: "unknown",
      status: { status: "failed", error: { code: "OTHER", message: "failed" } },
    },
    {
      skill_id: "git",
      skill_name: "git",
      tool: "absent",
      status: {
        status: "skipped",
        error: { code: "TOOL_NOT_INSTALLED", tool: "absent" },
      },
    },
    {
      skill_id: "git",
      skill_name: "git",
      tool: "claude",
      status: {
        status: "skipped",
        error: { code: "TOOL_NOT_WRITABLE", tool: "claude", path: "/locked" },
      },
    },
  ],
  synced: 1,
  failed: 1,
  skipped: 2,
};
describe("syncOutcome", () => {
  it.each([
    { action: "bulk", errors: 1 },
    { action: "install", errors: 2 },
    { action: "toggle", errors: 3 },
  ] as const)(
    "$action selects the intended failures/skips",
    ({ action, errors }) => {
      const out = syncOutcome(syncReport, { ...ctx, action });
      expect(out.errors).toHaveLength(errors);
      expect(out.errors[0]).toEqual({
        title: t("errors.syncFailedTitle", { name: "git", tool: "unknown" }),
        message: "failed",
      });
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
      { results: [syncReport.results[0]], synced: 1, skipped: 0, failed: 0 },
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
  it("explicit TARGET_EXISTS toggle shows the path and neither reloads nor succeeds", () => {
    const out = syncOutcome(
      {
        results: [
          {
            ...syncReport.results[0],
            status: {
              status: "skipped",
              error: { code: "TARGET_EXISTS", path: "/conflict" },
            },
          },
        ],
        synced: 0,
        skipped: 1,
        failed: 0,
      },
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
    Extract<ImportGroupStatusDto, { status: "imported" }>
  > = {},
): ImportGroupStatusDto => ({
  status: "imported",
  skill_id: "git",
  skill_name: "git",
  targets: [],
  forced_tools: [],
  originals: [],
  ...overrides,
});
const importReport = (status: ImportGroupStatusDto): ImportReportDto => ({
  groups: [{ group_name: "git", status }],
  imported: Number(status.status === "imported"),
  failed: Number(status.status === "failed"),
});
describe("importOutcome", () => {
  it.each([
    { name: "success", status: imported(), errors: 0, warnings: 0 },
    {
      name: "refused group",
      status: {
        status: "failed",
        error: { code: "SKILL_INVALID", reason: "missing_skill_md" },
      } as ImportGroupStatusDto,
      errors: 1,
      warnings: 0,
    },
    {
      name: "target failed",
      status: imported({
        targets: [
          {
            ...syncReport.results[0],
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
      status: imported({ targets: [syncReport.results[3]] }),
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
              ...syncReport.results[0],
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
