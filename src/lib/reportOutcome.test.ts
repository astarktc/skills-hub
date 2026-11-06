import i18next from "i18next";
import { describe, expect, it } from "vitest";
import { resources } from "../i18n/resources";
import type {
  BatchTargetOutcome,
  EnableReport,
  ImportGroupStatus,
  ImportReport,
  InstallResult,
  InvocationEditReport,
  LibraryMoveReport,
  ProjectSyncOutcome,
  ProjectSyncReport,
  RefreshReport,
  RemovalReport,
  SkillKept,
  SkillRefreshOutcome,
} from "../bindings";
import {
  combineOutcomes,
  compositeHeadline,
  deleteOutcome,
  deleteReportRows,
  HEADLINE_DONE_KEY,
  headlineTone,
  enableOutcome,
  gitCacheCleanOutcome,
  invocationEditOutcome,
  importGroupKind,
  importOutcome,
  installBatchSettlements,
  installOutcome,
  libraryMoveCounts,
  libraryMoveOutcome,
  projectRemovalOutcome,
  projectSyncOutcome,
  refreshOutcome,
  removalOutcome,
  runItems,
  syncOutcome,
  type InstallDeployment,
  type InstallSettlement,
} from "./reportOutcome";

const t = (key: string, opts?: Record<string, unknown>) =>
  opts ? `${key} ${JSON.stringify(opts)}` : key;
const ctx = {
  t,
  toolLabelById: { claude: "CLAUDE" },
};
/** An entry's words (title + message): the shape most assertions compare; the
 * declarative subject / error / repair are pinned by their own tests. */
const words = (entries: readonly { title: string; message: string }[]) =>
  entries.map(({ title, message }) => ({ title, message }));
/** An Outcome with its entries reduced to their words. */
const plain = <O extends { errors: readonly { title: string; message: string }[]; warnings: readonly { title: string; message: string }[] }>(
  outcome: O,
) => ({ ...outcome, errors: words(outcome.errors), warnings: words(outcome.warnings) });
const refreshed = (id = "git"): SkillRefreshOutcome => ({
  skill_id: id,
  skill_name: id,
  status: {
    status: "refreshed",
    content_hash: null,
    source_revision: null,
    targets: [],
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
  status: { status: "skipped", reason: { kind: "unlocatable", state: "source_missing" } },
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
  kind: "conflict" | "target",
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
        first_deploy: false,
      },
    });
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
      ],
      tone: "failure",
      message: line(
        refreshedN(2),
        ["outcome.failed", 1],
        ["outcome.targetsFailed", 1],
        ["outcome.skipped", 1],
        ["outcome.review.edits", 1],
      ),
      errors: 2,
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
    ).toBe("12 updated · 5 failed · 6 deployments failed · 4 skipped · 1 Edit to review");
    expect(compositeHeadline(en, "installed", { done: 2, cancelled: 3 })).toBe("2 added · 3 cancelled");
    expect(compositeHeadline(en, "synced", { done: 1, declined: 1, targetsFailed: 1 })).toBe(
      "1 deployed · 1 deployment failed · 1 kept existing",
    );
    expect(compositeHeadline(en, "imported", { done: 3, review: 2 }, "originals")).toBe(
      "3 imported · 2 originals to review",
    );
    expect(compositeHeadline(en, "removed", {})).toBe("0 removed");
    expect(en("outcome.refreshCancelled")).toBe("Update all cancelled — nothing was changed");
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
      status: { status: "failed", error: { code: "OTHER", message: "blocked" }, first_deploy: false },
    });
    const out = invocationEditOutcome(
      { skill_id: "s1", skill_name: "alpha", propagation: { targets: [failure("claude"), failure("cursor")] } },
      { t: en },
    );
    expect(out.toast).toEqual({
      kind: "warning",
      tone: "failure",
      message: "Invocation setting saved \u00b7 2 deployments failed",
    });
    expect(out.completion.closeModal).toBe(true);
  });

  it("acquisition skips preserve both reasons; the batch line counts them", () => {
    const out = refreshOutcome(refreshReport([skippedGone, skippedStale]), ctx);
    expect(words(out.warnings)).toEqual([
      { title: 'errors.refreshSkippedTitle {"name":"gone"}', message: "errors.refreshSkippedSkillGone" },
      { title: 'errors.refreshSkippedTitle {"name":"stale"}', message: "errors.refreshSkippedStaleAcquisition" },
    ]);
    expect(out.completion).toEqual({ reload: true, closeModal: false, conflict: false });
    expect(out.toast).toEqual({ kind: "warning", tone: "skipped", message: line(["outcome.skipped", 2]) });
  });

  it("an entry is declarative data — subject, typed error, repair — never a callback or captured ManagedSkill", () => {
    const out = refreshOutcome(refreshReport([failed]), ctx);
    const [entry] = out.errors;
    expect(entry.subject).toEqual({ skillId: "git", skillName: "git" });
    expect(entry.error?.code).toBe("GITHUB_SKILL_NOT_FOUND");
    expect(entry.repair).toEqual({ verb: "update" });
    expect(entry.message).toContain("https://github.com/old/repo");
    expect(Object.values(entry).some((value) => typeof value === "function")).toBe(false);
  });

  it("announces no deployment: a synced global target is Propagation's, never a new arrival (D34)", () => {
    const success = refreshed("alpha");
    if (success.status.status !== "refreshed") throw new Error("fixture");
    success.status.targets.push(
      { scope: { scope: "global", tool: "claude" }, status: { status: "synced", mode_used: "symlink" } },
      { scope: { scope: "global", tool: "cursor" }, status: { status: "synced", mode_used: "copy" } },
    );
    const out = refreshOutcome(refreshReport([success]), ctx);
    expect(out.notes).toEqual([]);
    expect(out.toast).toEqual({ kind: "success", tone: "success", message: line(refreshedN(1)) });
  });

  it("a delete_incomplete skip is a skipped item offering Retry the Delete (D35)", () => {
    const train: SkillRefreshOutcome = {
      skill_id: "train",
      skill_name: "release-train",
      status: { status: "skipped", reason: { kind: "delete_incomplete" } },
    };
    const out = refreshOutcome(refreshReport([refreshed(), train]), ctx);
    expect(out.errors).toEqual([]);
    expect(out.warnings).toEqual([
      {
        title: t("updateSkip.deleteIncompleteTitle", { name: "release-train" }),
        message: "updateSkip.deleteIncompleteMessage",
        subject: { skillId: "train", skillName: "release-train" },
        repair: { verb: "delete" },
      },
    ]);
    expect(out.toast).toMatchObject({ tone: "skipped", message: line(refreshedN(1), ["outcome.skipped", 1]) });
    expect(out.completion.closeModal).toBe(false);
  });

  it("a failed first deploy reads 'Could not deploy'; an owned copy keeps 'update the synced copy'", () => {
    const success = refreshed("alpha");
    if (success.status.status !== "refreshed") throw new Error("fixture");
    success.status.targets.push(
      { scope: { scope: "global", tool: "claude" }, status: { status: "failed", error: { code: "OTHER", message: "io" }, first_deploy: true } },
      { scope: { scope: "global", tool: "cursor" }, status: { status: "failed", error: { code: "OTHER", message: "io" }, first_deploy: false } },
    );
    const out = refreshOutcome(refreshReport([success]), ctx);
    expect(out.errors.map((entry) => entry.title)).toEqual([
      t("propagationFirstDeploy.failedTitle", { name: "alpha", tool: "CLAUDE" }),
      t("errors.propagationFailedTitle", { name: "alpha", tool: "cursor" }),
    ]);
  });

  it("orders skill errors before target errors, skips before conflicts; silent propagation skips", () => {
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
        skipped,
        success,
      ]),
      ctx,
    );
    expect(out.errors.map((e) => e.title)).toEqual([
      t("errors.updateFailedTitle", { name: "git" }),
      t("errors.propagationFailedTitle", { name: "git", tool: "CLAUDE" }),
    ]);
    expect(words(out.warnings)).toEqual([
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
            status: { status: "skipped", reason: { kind: "unlocatable", state: "central_missing" } },
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
    status: { status: "failed", error: { code: "OTHER", message: "global blocked" }, first_deploy: false },
  };
  const projectFailure: InvocationEditReport["propagation"]["targets"][number] = {
    scope: { scope: "project", project_id: "p1", tool: "unknown" },
    status: { status: "failed", error: { code: "OTHER", message: "project blocked" }, first_deploy: false },
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
    expect(plain(invocationEditOutcome(report, ctx))).toEqual({
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
      rows: [{ scope: "global_target", id: "t1", skill_id: "s1", tool: "unknown", folder_key: "unknown" }],
      path: "/target",
      status: fail
        ? { status: "failed", error: { code: "OTHER", message: "busy" } }
        : { status: "removed" },
    },
  ],
  central_removed: false,
  record_deleted: false, cleared_markers: [], disabled_markers: [],
});
describe("removalOutcome", () => {
  it("counts shared artifact rows and emits failures in target/row order for every removal action", () => {
    const report: RemovalReport = {
      targets: [
        {
          path: "/removed",
          rows: [
            { scope: "global_target", id: "r1", skill_id: "s1", tool: "claude", folder_key: "claude" },
            { scope: "global_target", id: "r2", skill_id: "s1", tool: "pi", folder_key: "pi" },
          ],
          status: { status: "removed" },
        },
        {
          path: "/kept",
          rows: [
            { scope: "global_target", id: "t1", skill_id: "s1", tool: "claude", folder_key: "claude" },
            { scope: "assignment", id: "a1", project_id: "p1", skill_id: "s1", tool: "unknown", folder_key: "unknown" },
          ],
          status: { status: "failed", error: { code: "PATH_OUTSIDE_TOOL_DIRS", path: "/kept" } },
        },
        {
          path: "/also-kept",
          rows: [{ scope: "assignment", id: "a2", project_id: "p2", skill_id: "s1", tool: "pi", folder_key: "pi" }],
          status: { status: "failed", error: { code: "OTHER", message: "busy" } },
        },
      ],
      central_removed: false,
      record_deleted: false, cleared_markers: [], disabled_markers: [],
    };
    const before = JSON.stringify(report);
    const unsync = removalOutcome(report, { ...ctx, action: "all" });
    const project = projectRemovalOutcome(report, { ...ctx, action: "removeProject" });
    const configure = projectRemovalOutcome(report, { ...ctx, action: "configureTools" });
    const deleted = deleteOutcome([{ skillId: "s1", name: "alpha", report }], ctx);
    for (const [out, key] of [
      [unsync, "errors.unsyncFailedTitle"],
      [project, "errors.projectRemovalFailedTitle"],
      [configure, "errors.projectRemovalFailedTitle"],
      [deleted, "errors.deleteKeptTargetTitle"],
    ] as const) {
      expect(words(out.errors)).toEqual([
        { title: t(key, { tool: "CLAUDE" }), message: t("errors.pathOutsideToolDirs", { path: "/kept" }) },
        { title: t(key, { tool: "unknown" }), message: t("errors.pathOutsideToolDirs", { path: "/kept" }) },
        { title: t(key, { tool: "pi" }), message: "busy" },
      ]);
      // A project removal closes either way: the kept row shows in the matrix.
      expect(out.completion.closeModal).toBe(out === project);
    }
    // Counted per Deployment: the global (s1, claude) Deployment has one row
    // removed and one kept, so it counts once, as failed — only (s1, pi)
    // globally was removed.
    expect(unsync.toast).toEqual({
      kind: "warning",
      tone: "failure",
      message: [t("outcome.done.disabled", { count: 1 }), t("outcome.failed", { count: 3 })].join(" · "),
    });
    expect(project.toast).toEqual({
      kind: "warning", tone: "failure",
      message: [t("projectsSurface.removeKept"), t("outcome.done.removed", { count: 1 }), t("outcome.failed", { count: 3 })].join(" · "),
    });
    expect(configure.toast).toEqual({
      kind: "warning", tone: "failure",
      message: [t("outcome.done.removed", { count: 1 }), t("outcome.failed", { count: 3 })].join(" · "),
    });
    expect(deleted.toast).toEqual({ kind: "warning", tone: "failure", message: t("status.skillDeleteKept", { failed: 3 }) });
    expect(deleted.completion).toEqual({ reload: true, closeModal: false, conflict: false });
    expect(JSON.stringify(report)).toBe(before);
  });

  it("delete succeeds for removed targets and has kept/retry copy", () => {
    expect(deleteOutcome([{ skillId: "s1", name: "alpha", report: { ...removal(false), central_removed: true, record_deleted: true, cleared_markers: [], disabled_markers: [] } }], ctx)).toEqual({
      toast: { kind: "success", tone: "success", message: "status.skillRemoved" },
      errors: [], warnings: [], notes: [], completion: { reload: true, closeModal: true, conflict: false },
    });
    expect(resources.en.translation.errors.deleteKeptTargetTitle).toContain("{{tool}}");
    expect(resources.en.translation.status.skillDeleteKept).toBe("Skill kept: {{failed}} deployments could not be removed. You can retry.");
  });

  it.each(["all", "skill", "toggle"] as const)(
    "%s removal settles kept rows without claiming success",
    (action) => {
      for (const fail of [false, true]) {
        const out = removalOutcome(removal(fail), { ...ctx, action });
        expect(words(out.errors)).toEqual(
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
                  ? t("outcome.failed", { count: 1 })
                  : t("outcome.done.disabled", { count: 1 }),
              }
            : action === "toggle" && !fail
              ? { kind: "success", tone: "success", message: "status.syncDisabled" }
              : null,
        );
      }
    },
  );
  it("a zero-target report warns instead of claiming an unsync happened", () => {
    const nothing: RemovalReport = { targets: [], central_removed: false, record_deleted: false, cleared_markers: [], disabled_markers: [] };
    for (const action of ["toggle", "bulkUnassign"] as const) {
      const out = removalOutcome(nothing, { ...ctx, action });
      expect(out.toast).toEqual({
        kind: "warning", tone: "skipped",
        message: "unsyncNothingPlanned",
      });
      expect(words(out.errors)).toEqual([]);
      expect(out.completion.closeModal).toBe(false);
    }
  });
  it("bulk unassign counts removed rows, names kept ones, and never reloads", () => {
    const report: RemovalReport = {
      targets: [
        {
          rows: [{ scope: "assignment", id: "a1", project_id: "p1", skill_id: "s1", tool: "claude", folder_key: "claude" }],
          path: "/work/p1/.claude/skills/s1",
          status: { status: "removed" },
        },
        {
          rows: [{ scope: "assignment", id: "a2", project_id: "p1", skill_id: "s1", tool: "pi", folder_key: "pi" }],
          path: "/work/p1/.pi/skills/s1",
          status: { status: "removed" },
        },
      ],
      central_removed: false,
      record_deleted: false, cleared_markers: [], disabled_markers: [],
    };
    expect(removalOutcome(report, { ...ctx, action: "bulkUnassign" })).toEqual({
      toast: { kind: "success", tone: "success", message: t("projectsSurface.done.unassigned", { count: 2 }) },
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
    expect(plain(removalOutcome(kept, { ...ctx, action: "bulkUnassign" }))).toEqual({
      toast: {
        kind: "warning", tone: "failure",
        message: [t("projectsSurface.done.unassigned", { count: 1 }), t("outcome.failed", { count: 1 })].join(" · "),
      },
      errors: [{ title: t("errors.unsyncFailedTitle", { tool: "pi" }), message: "busy" }],
      warnings: [],
      notes: [],
      completion: { reload: false, closeModal: false, conflict: false },
    });
  });
  it("a project removal reports each kept target and closes (the kept row shows in the matrix); a tool configuration stays open", () => {
    const report: RemovalReport = {
      targets: [
        {
          rows: [{ scope: "assignment", id: "a1", project_id: "p1", skill_id: "s1", tool: "claude", folder_key: "claude" }],
          path: "/work/p1/.claude/skills/a",
          status: { status: "removed" },
        },
        {
          rows: [{ scope: "assignment", id: "a2", project_id: "p1", skill_id: "s1", tool: "pi", folder_key: "pi" }],
          path: "/work/p1/.pi/skills/a",
          status: { status: "failed", error: { code: "OTHER", message: "denied" } },
        },
      ],
      central_removed: false,
      record_deleted: false, cleared_markers: [], disabled_markers: [],
    };
    const kept = [
      { title: t("errors.projectRemovalFailedTitle", { tool: "pi" }), message: "denied" },
    ];
    expect(plain(projectRemovalOutcome(report, { ...ctx, action: "removeProject" }))).toEqual({
      toast: {
        kind: "warning", tone: "failure",
        message: [t("projectsSurface.removeKept"), t("outcome.done.removed", { count: 1 }), t("outcome.failed", { count: 1 })].join(" · "),
      },
      errors: kept,
      warnings: [],
      notes: [],
      completion: { reload: false, closeModal: true, conflict: false },
    });
    expect(plain(projectRemovalOutcome(report, { ...ctx, action: "configureTools" }))).toEqual({
      toast: { kind: "warning", tone: "failure", message: [t("outcome.done.removed", { count: 1 }), t("outcome.failed", { count: 1 })].join(" · ") },
      errors: kept,
      warnings: [],
      notes: [],
      completion: { reload: false, closeModal: false, conflict: false },
    });
    // The tool label map applies to the failed target's title, named by the
    // Deployment folder the row describes.
    const relabelled = {
      ...report,
      targets: [{ ...report.targets[1], rows: [{ ...report.targets[1].rows[0], tool: "claude", folder_key: "claude" }] }],
    };
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
          rows: [{ scope: "assignment", id: "a1", project_id: "p1", skill_id: "s1", tool: "claude", folder_key: "claude" }],
          path: "/work/p1/.claude/skills/a",
          status: { status: "failed", error: { code: "OTHER", message: "denied" } },
        },
      ],
      central_removed: false,
      record_deleted: false, cleared_markers: [], disabled_markers: [],
    };
    for (const toolLabelById of [{}, { pi: "Pi" }] as Record<string, string>[]) {
      for (const action of ["removeProject", "configureTools"] as const) {
        expect(
          words(projectRemovalOutcome(report, { ...ctx, toolLabelById, action }).errors),
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
          rows: [{ scope: "assignment", id: "a1", project_id: "p1", skill_id: "s1", tool: "pi", folder_key: "pi" }],
          path: "/work/p1/.pi/skills/a",
          status: { status: "removed" },
        },
      ],
      central_removed: false,
      record_deleted: false, cleared_markers: [], disabled_markers: [],
    };
    const nothing: RemovalReport = { targets: [], central_removed: false, record_deleted: false, cleared_markers: [], disabled_markers: [] };
    for (const report of [clean, nothing]) {
      expect(plain(projectRemovalOutcome(report, { ...ctx, action: "removeProject" }))).toEqual({
        toast: { kind: "success", tone: "success", message: "projects.removeComplete" },
        errors: [],
        warnings: [],
        notes: [],
        completion: { reload: false, closeModal: true, conflict: false },
      });
      expect(plain(projectRemovalOutcome(report, { ...ctx, action: "configureTools" }))).toEqual({
        toast: null,
        errors: [],
        warnings: [],
        notes: [],
        completion: { reload: false, closeModal: true, conflict: false },
      });
    }
  });
  it("delete's successful report reloads and closes", () => {
    expect(deleteOutcome([{ skillId: "s1", name: "alpha", report: { targets: [], central_removed: true, record_deleted: true, cleared_markers: [], disabled_markers: [] } }], ctx)).toEqual({
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
    const report: ProjectSyncReport = { skipped_projects: [],
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
      expect(words(out.errors)).toEqual([
        { title: title("s2-name", "CLAUDE"), message: "denied" },
        {
          title: title("s1-name", "unknown"),
          message: t("errors.pathOutsideToolDirs", { path: "/x" }),
        },
      ]);
      expect(words(out.warnings)).toEqual([]);
      // The project world applies the view the mutation returned.
      expect(out.completion).toEqual({ reload: false, closeModal: false, conflict: false });
    }
    expect(JSON.stringify(report)).toBe(before);
  });

  it("resync-all names the project by its label when one is supplied", () => {
    const report: ProjectSyncReport = { skipped_projects: [], items: [projectItem("claude", denied)] };
    const out = projectSyncOutcome(report, {
      ...ctx,
      action: "resyncAll",
      projectLabelById: { p1: "My App" },
    });
    expect(out.errors[0]?.title).toBe(
      t("errors.syncFailedInProjectTitle", { name: "s1-name", tool: "CLAUDE", project: "My App" }),
    );
    // The title names the project: the panel's presentation does not prefix it again (N1).
    expect(out.errors[0]?.subject).toMatchObject({ projectId: "p1", projectInTitle: true });
    const one = projectSyncOutcome(report, { ...ctx, action: "toggleOn" });
    expect(one.errors[0]?.subject?.projectInTitle).toBeUndefined();
  });

  it("derives the counters per action: failure outranks success", () => {
    const mixed: ProjectSyncReport = { skipped_projects: [],
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
        message: [t("outcome.done.assigned", { count: 2 }), t("outcome.failed", { count: 1 })].join(" · "),
      },
      resync: {
        kind: "warning", tone: "failure",
        message: [t("outcome.done.synced", { count: 2 }), t("outcome.failed", { count: 1 })].join(" · "),
      },
      resyncAll: {
        kind: "warning", tone: "failure",
        message: [t("outcome.done.synced", { count: 2 }), t("outcome.failed", { count: 1 })].join(" · "),
      },
    });
  });

  it("a clean report succeeds with per-action copy and closes", () => {
    const clean: ProjectSyncReport = { skipped_projects: [],
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
      message: t("outcome.done.assigned", { count: 1 }),
    });
    expect(outs.resync.toast).toEqual({
      kind: "success", tone: "success",
      message: t("outcome.done.synced", { count: 1 }),
    });
    expect(outs.resyncAll.toast).toEqual({
      kind: "success", tone: "success",
      message: t("outcome.done.synced", { count: 1 }),
    });
    for (const out of Object.values(outs)) {
      expect(words(out.errors)).toEqual([]);
      expect(out.completion).toEqual({ reload: false, closeModal: true, conflict: false });
    }
  });

  it("a saturated bulk assign says so instead of counting zero; an empty resync still reports", () => {
    const saturated: ProjectSyncReport = { skipped_projects: [],
      items: [projectItem("claude", { status: "already_assigned" })],
    };
    expect(projectSyncOutcome(saturated, { ...ctx, action: "bulkAssign" }).toast).toEqual({
      kind: "success", tone: "success",
      message: "projects.bulkAssignNothing",
    });
    expect(projectSyncOutcome({ skipped_projects: [], items: [] }, { ...ctx, action: "resync" }).toast).toEqual({
      kind: "success", tone: "success",
      message: t("outcome.done.synced", { count: 0 }),
    });
  });

  it("Sync All skips a project whose folder is missing with one line, never a failure per assignment", () => {
    const report: ProjectSyncReport = {
      items: [projectItem("claude", { status: "synced" }), projectItem("pi", { status: "synced" })],
      skipped_projects: [{ project_id: "p2", reason: { reason: "folder_missing" } }],
    };
    const out = projectSyncOutcome(report, { ...ctx, action: "resyncAll", projectLabelById: { p2: "archived-app" } });
    expect(words(out.errors)).toEqual([]);
    expect(words(out.warnings)).toEqual([
      { title: t("projectsSurface.folderMissingSkip", { project: "archived-app" }), message: "projectsSurface.folderMissingDetail" },
    ]);
    // The panel does not prefix "in archived-app:" again — the line names it.
    expect(out.warnings[0].subject).toEqual({ skillName: "", projectId: "p2", projectInTitle: true });
    expect(out.toast).toEqual({
      kind: "warning", tone: "skipped",
      message: [t("outcome.done.synced", { count: 2 }), t("outcome.skipped", { count: 1 })].join(" · "),
    });
    expect(out.completion).toEqual({ reload: false, closeModal: true, conflict: false });
    expect(resources.en.translation.projectsSurface.folderMissingSkip).toBe(
      "{{project}}: folder not found \u2014 Update path or Remove",
    );
  });

  it("Assign to project and toggle-on into a missing folder: the skip line, never a silent success", () => {
    const report: ProjectSyncReport = { items: [], skipped_projects: [{ project_id: "p2", reason: { reason: "folder_missing" } }] };
    const ctxNamed = { ...ctx, projectLabelById: { p2: "archived-app" } };
    const bulk = projectSyncOutcome(report, { ...ctxNamed, action: "bulkAssign" });
    expect(words(bulk.warnings)).toEqual([
      { title: t("projectsSurface.folderMissingSkip", { project: "archived-app" }), message: "projectsSurface.folderMissingDetail" },
    ]);
    expect(bulk.toast).toMatchObject({ tone: "skipped", message: t("outcome.skipped", { count: 1 }) });
    const toggle = projectSyncOutcome(report, { ...ctxNamed, action: "toggleOn" });
    expect(toggle.toast).toMatchObject({ tone: "skipped", message: t("projectsSurface.folderMissingSkip", { project: "archived-app" }) });
  });

  it("Enable counts a project skipped whole once, and names it", () => {
    const out = enableOutcome(
      {
        global: [],
        projects: { items: [], skipped_projects: [{ project_id: "p2", reason: { reason: "folder_missing" } }] },
      },
      { ...ctx, batch: true, projectLabelById: { p2: "archived-app" } },
    );
    expect(out.tally?.counts.skipped).toBe(1);
    expect(words(out.warnings)).toEqual([
      { title: t("projectsSurface.folderMissingSkip", { project: "archived-app" }), message: "projectsSurface.folderMissingDetail" },
    ]);
    expect(out.toast).toMatchObject({ tone: "skipped" });
  });

  it("a cell's Sync this confirms a batch of one; its failure is the error entry alone", () => {
    const ok = projectSyncOutcome({ skipped_projects: [], items: [projectItem("claude", { status: "synced" })] }, { ...ctx, action: "syncCell" });
    expect(ok.toast).toEqual({ kind: "success", tone: "success", message: "projectsSurface.cellSynced" });
    const failed = projectSyncOutcome({ skipped_projects: [], items: [projectItem("claude", denied)] }, { ...ctx, action: "syncCell" });
    expect(failed.toast).toBeNull();
    expect(words(failed.errors)).toEqual([{ title: t("errors.syncFailedTitle", { name: "s1-name", tool: "CLAUDE" }), message: "denied" }]);
  });

  it("carries the new copy", () => {
    expect(resources.en.translation.localSkillInvalid.insideToolDir).toBe(
      "This folder is a harness's own skills copy — use Import instead",
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
      expect(words(out.errors)[0]).toEqual({
        title: t("errors.syncFailedTitle", { name: "git", tool: "unknown" }),
        message: "failed",
      });
      // A not-detected skip is a per-tool warning for bulk/install (the
      // selection is stale, nothing failed); an explicit toggle keeps it
      // as the error it already was.
      expect(words(out.warnings)).toEqual(
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
    expect(words(out.errors)).toEqual([]);
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
    expect(words(out.errors)).toEqual([]);
    expect(words(out.warnings)).toEqual([
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
  // Ticket 11 audit Q4 / D10 / D12: a TARGET_EXISTS still in a report is a
  // collision the operator did not answer — the ask was dismissed. It is "to
  // review" (conflict tone), never a failure: no error entry, no retry, no
  // implicit Keep; the pair stays as it was.
  describe("a dismissed overwrite ask folds as \"to review\", never failed", () => {
    const exists = { code: "TARGET_EXISTS", path: "/conflict", modifiedAt: 7, sameContent: false } as const;
    const unanswered = (status: "failed" | "skipped"): BatchTargetOutcome => ({
      ...syncReport[0],
      status: { status, error: exists },
    });
    it.each(["failed", "skipped"] as const)("an explicit toggle (%s row): conflict toast, a review entry, no error", (status) => {
      const out = syncOutcome([unanswered(status)], { ...ctx, action: "toggle" });
      expect(out.errors).toEqual([]);
      expect(out.toast).toEqual({ kind: "warning", tone: "conflict", message: t("runReport.collisions", { count: 1 }) });
      expect(out.warnings).toEqual([
        {
          title: t("runReport.collisionTitle", { name: "git", tool: "CLAUDE" }),
          message: t("runReport.collisionMessage", { path: "/conflict" }),
          subject: { skillId: syncReport[0].skill_id, skillName: "git", tool: "claude" },
          error: exists,
          repair: { verb: "overwrite", tool: "claude", via: "sync" },
          review: true,
        },
      ]);
      expect(out.completion).toEqual({ reload: true, closeModal: true, conflict: false });
    });
    it("an Enable's collision keeps its route: Overwrite… answers it by Enable, not an ordinary sync (ticket 14 audit A3)", () => {
      const out = enableOutcome({ global: [unanswered("failed")], projects: { skipped_projects: [], items: [] } }, ctx);
      expect(out.errors).toEqual([]);
      expect(out.warnings).toMatchObject([{ review: true, repair: { verb: "overwrite", tool: "claude", via: "enable" } }]);
    });
    it("a bulk sync counts it in the headline as to review, beside what synced", () => {
      const out = syncOutcome([syncReport[0], { ...unanswered("failed"), tool_key: "cursor" }], { ...ctx, action: "bulk" });
      expect(out.errors).toEqual([]);
      expect(out.toast).toMatchObject({
        tone: "conflict",
        message: line(["outcome.done.synced", 1], ["runReport.collisions", 1]),
      });
      expect(out.tally?.counts.collisions).toBe(1);
    });
    it("a failure beside it still wins the tone; the collision is never counted failed", () => {
      const out = syncOutcome([syncReport[1], { ...unanswered("failed"), tool_key: "cursor" }], { ...ctx, action: "bulk" });
      expect(out.toast).toMatchObject({
        tone: "failure",
        message: line(["outcome.failed", 1], ["runReport.collisions", 1]),
      });
      expect(out.errors).toHaveLength(1);
      expect(out.warnings.filter((entry) => entry.review)).toHaveLength(1);
    });
    it("a Refresh's Propagation collision is to review, not a target failure", () => {
      const success = refreshed("alpha");
      if (success.status.status !== "refreshed") throw new Error("fixture");
      success.status.targets.push({ scope: { scope: "global", tool: "cursor" }, status: { status: "failed", error: exists, first_deploy: true } });
      const out = refreshOutcome(refreshReport([success]), ctx);
      expect(out.errors).toEqual([]);
      expect(out.toast).toMatchObject({
        tone: "conflict",
        message: line(["outcome.done.refreshed", 1], ["runReport.collisions", 1]),
      });
    });
    it("a project assignment collision is to review and names the project in its subject", () => {
      const report: ProjectSyncReport = { skipped_projects: [],
        items: [
          { assignment_id: "a1", project_id: "p1", skill_id: "s1", skill_name: "tdd", tool: "claude", status: { status: "failed", error: exists } },
        ],
      };
      const out = projectSyncOutcome(report, { ...ctx, action: "toggleOn" });
      expect(out.errors).toEqual([]);
      expect(out.toast).toEqual({ kind: "warning", tone: "conflict", message: t("runReport.collisions", { count: 1 }) });
      expect(out.warnings[0]).toMatchObject({ review: true, subject: { projectId: "p1", skillId: "s1", tool: "claude" } });
      // A project row is answered from its matrix: no overwrite repair here.
      expect(out.warnings[0].repair).toBeUndefined();
      expect(out.completion.closeModal).toBe(true);
    });
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
      tally: { done: [{ verb: "synced", count: 0 }], counts: { failed: 0, skipped: 0, declined: 2, collisions: 0 } },
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
    expect(words(out.errors)).toEqual([]);
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
  central_path: "/central/git",
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
              error: { code: "OTHER", message: "disk full" },
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
      errors: 0,
      warnings: 1,
    },
  ])("$name", ({ name, status, errors, warnings }) => {
    const out = importOutcome(importReport(status), ctx);
    expect(out.errors).toHaveLength(errors);
    expect(out.warnings).toHaveLength(warnings);
    expect(out.completion).toEqual({
      reload: true,
      closeModal: errors + warnings === 0 || name === "replace answered TARGET_EXISTS",
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
      // A Replace counts as what its targets settled as (audit F3): one
      // deployed copy is a replace; an unanswered collision replaced nothing.
      "replaced over an identical copy": ["success", line(["outcome.done.replaced", 1])],
      "replace answered TARGET_EXISTS": ["conflict", line(["outcome.skipped", 1], ["runReport.collisions", 1])],
    };
    const [tone, message] = expected[name];
    expect(out.toast).toEqual({ kind: tone === "success" ? "success" : "warning", tone, message });
  });
  it("reports a not-detected target as a per-tool warning that does not hold the import open", () => {
    const out = importOutcome(importReport(imported({ targets: [syncReport[2]] })), ctx);
    expect(words(out.errors)).toEqual([]);
    expect(words(out.warnings)).toEqual([
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
    // The collision nobody answered is to review, its path in the words.
    expect(out.errors).toEqual([]);
    expect(out.warnings.find((entry) => entry.review)?.message).toBe(
      t("runReport.collisionMessage", { path: "/conflict" }),
    );
    expect(words(out.warnings.filter((entry) => !entry.review))[0]).toEqual({
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
    expect(words(out.errors)).toEqual([]);
    expect(out.notes).toEqual([note]);
    expect(out.completion).toEqual({ reload: true, closeModal: true, conflict: false });
    expect(out.toast).toEqual({
      kind: "info",
      tone: "declined",
      message: line(["outcome.done.imported", 1], ["outcome.declinedFolders", 1]),
      detail: [t("status.importSourceToolForced", { name: "git", tool: "CLAUDE" }), note].join("\n"),
    });
  });
});

const installed: InstallResult = {
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
          : t("add.added", { name: "git" }),
    });
  });
  it.each([
    { status: "no-targets" },
    { status: "failed", error: new Error("deploy request failed") },
    { status: "reported", report: syncReport },
  ] satisfies InstallDeployment[])(
    "installed bytes still reload after deployment $status, and the dialog stays on its settlement",
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
        closeModal: false,
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
    expect(words(out.errors)).toEqual([
      {
        title: t("add.failedTitle", { name: "first" }),
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
    expect(words(out.errors)).toEqual([]);
    expect(words(out.warnings)).toEqual([]);
    expect(out.toast).toEqual({
      kind: "info",
      tone: "neutral",
      message: line(["outcome.done.installed", 2], ["outcome.cancelled", 3]),
    });
    // What landed reloads; the dialog stays on its settlement (ticket 16).
    expect(out.completion).toEqual({ reload: true, closeModal: false, conflict: false });
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
    expect(words(out.errors)).toEqual([]);
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
    ["enable", () => enableOutcome({ global: [disabledTarget], projects: { skipped_projects: [], items: [] } }, ctx)],
    ["project resync", () => projectSyncOutcome(
      { skipped_projects: [], items: [projectItem("claude", { status: "skipped", reason: { reason: "disabled" } })] },
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
      { skipped_projects: [], items: [projectItem("claude", skip), projectItem("pi", skip)] },
      { ...ctx, action: "bulkAssign" },
    );
    // The count line, with why nothing synced as its detail.
    expect(out.toast).toEqual({
      kind: "warning", tone: "skipped",
      message: t("outcome.skipped", { count: 2 }),
      detail: "projectsSurface.syncedWhileDisabled",
    });
    expect(words(out.errors)).toEqual([]);
    expect(out.completion.closeModal).toBe(true);
  });
  it("mixed synced/skipped and synced/skipped/failed bulk assigns carry every count", () => {
    const mixed = { skipped_projects: [], items: [projectItem("claude", { status: "synced" }), projectItem("pi", skip)] };
    expect(projectSyncOutcome(mixed, { ...ctx, action: "bulkAssign" }).toast).toEqual({
      kind: "warning", tone: "skipped",
      message: [t("outcome.done.assigned", { count: 1 }), t("outcome.skipped", { count: 1 })].join(" · "),
      detail: "projectsSurface.syncedWhileDisabled",
    });
    const partial = { skipped_projects: [], items: [...mixed.items, projectItem("codex", denied)] };
    const out = projectSyncOutcome(partial, { ...ctx, action: "bulkAssign" });
    expect(out.toast).toEqual({
      kind: "warning", tone: "failure",
      message: [t("outcome.done.assigned", { count: 1 }), t("outcome.failed", { count: 1 }), t("outcome.skipped", { count: 1 })].join(" · "),
      detail: "projectsSurface.syncedWhileDisabled",
    });
    expect(out.errors).toHaveLength(1);
  });
  it("resync summaries include the skipped count (D10: a skip tones the count line skipped)", () => {
    const report = { skipped_projects: [], items: [projectItem("claude", { status: "synced" }), projectItem("pi", skip)] };
    expect(projectSyncOutcome(report, { ...ctx, action: "resync" }).toast).toEqual({
      kind: "warning", tone: "skipped",
      message: [t("outcome.done.synced", { count: 1 }), t("outcome.skipped", { count: 1 })].join(" · "),
    });
    const partial = { skipped_projects: [], items: [...report.items, projectItem("codex", denied)] };
    expect(projectSyncOutcome(partial, { ...ctx, action: "resyncAll" }).toast).toEqual({
      kind: "warning", tone: "failure",
      message: [t("outcome.done.synced", { count: 1 }), t("outcome.failed", { count: 1 }), t("outcome.skipped", { count: 1 })].join(" · "),
    });
  });
});

// #95: a Disable that only records intent (a never-deployed pair's marker
// flipped `disabled`, or a skill-grain Disable with nothing deployed) worked —
// it says so instead of "Nothing was removed".
describe("removalOutcome of a Disable that only records intent", () => {
  const intent = (...tools: string[]): RemovalReport => ({
    targets: [],
    central_removed: false,
    record_deleted: false,
    cleared_markers: [],
    disabled_markers: tools.map((tool) => ({ scope: "global_target", id: `m-${tool}`, skill_id: "s1", tool, folder_key: tool })),
  });
  it("a pair's flipped marker reads as '{skill} stays off {harness}'", () => {
    const out = removalOutcome(intent("claude"), { ...ctx, action: "toggle", skillNames: { s1: "alpha" } });
    expect(out).toEqual({
      toast: { kind: "success", tone: "success", message: t("status.pairStaysOff", { name: "alpha", tool: "CLAUDE" }) },
      errors: [], warnings: [], notes: [],
      completion: { reload: true, closeModal: true, conflict: false },
    });
  });
  it("a pair Disable that also removed an artifact keeps the plain confirmation", () => {
    const report = { ...removal(false), disabled_markers: intent("claude").disabled_markers };
    expect(removalOutcome(report, { ...ctx, action: "toggle", skillNames: { s1: "alpha" } }).toast).toEqual({
      kind: "success", tone: "success", message: "status.syncDisabled",
    });
  });
  it("a bulk Disable counts flipped markers as disabled", () => {
    expect(removalOutcome(intent("claude", "pi"), { ...ctx, action: "all" }).toast).toEqual({
      kind: "success", tone: "success", message: t("outcome.done.disabled", { count: 2 }),
    });
  });
  it("a skill-grain Disable with nothing deployed records the skill disabled and says nothing was removed", () => {
    for (const action of ["skill", "all"] as const) {
      const out = removalOutcome(intent(), { ...ctx, action });
      expect(out.toast).toEqual({ kind: "success", tone: "success", message: "status.disabledNothingDeployed" });
      expect(out.completion.closeModal).toBe(true);
    }
  });
  it("a skill-grain Disable whose only rows were markers is the plain success", () => {
    const out = removalOutcome(intent("claude"), { ...ctx, action: "skill" });
    expect(out.toast).toBeNull();
    expect(out.completion).toEqual({ reload: true, closeModal: true, conflict: false });
  });
});

// F1: clearing a marker (a row owning no artifact) is work done, not "nothing planned".
describe("removalOutcome with cleared markers", () => {
  it("a report that only cleared a marker row is a success", () => {
    const report: RemovalReport = {
      targets: [],
      central_removed: false,
      record_deleted: false,
      cleared_markers: [{ scope: "assignment", id: "a1", project_id: "p1", skill_id: "s1", tool: "claude", folder_key: "claude" }],
      disabled_markers: [],
    };
    const out = removalOutcome(report, { ...ctx, action: "toggle" });
    expect(out.toast).toEqual({ kind: "success", tone: "success", message: "status.syncDisabled" });
    expect(words(out.errors)).toEqual([]);
  });

  // N1: a cleared marker is an unassignment, so bulk unassign counts it (the
  // copy says "removed from N tools", not that an artifact was removed).
  const marker = (tool: string) =>
    ({ scope: "assignment", id: `m-${tool}`, project_id: "p1", skill_id: "s1", tool, folder_key: tool }) as const;
  it("a markers-only bulk unassign counts every cleared assignment", () => {
    const report: RemovalReport = {
      targets: [],
      central_removed: false,
      record_deleted: false,
      cleared_markers: [marker("claude"), marker("pi")],
      disabled_markers: [],
    };
    expect(removalOutcome(report, { ...ctx, action: "bulkUnassign" })).toEqual({
      toast: { kind: "success", tone: "success", message: t("projectsSurface.done.unassigned", { count: 2 }) },
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
          rows: [{ scope: "assignment", id: "a1", project_id: "p1", skill_id: "s1", tool: "codex", folder_key: "codex" }],
          path: "/work/p1/.codex/skills/s1",
          status: { status: "removed" },
        },
      ],
      central_removed: false,
      record_deleted: false,
      cleared_markers: [marker("claude")],
      disabled_markers: [],
    };
    expect(removalOutcome(report, { ...ctx, action: "bulkUnassign" }).toast).toEqual({
      kind: "success", tone: "success",
      message: t("projectsSurface.done.unassigned", { count: 2 }),
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
      { skipped_projects: [],
        items: [
          { assignment_id: "x", project_id: "p", skill_id: "a", skill_name: "alpha", tool: "claude_code", status: { status: "declined" } },
        ],
      },
      { t, action: "toggleOn" },
    );
    expect(words(out.errors)).toEqual([]);
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
    expect(words(out.errors)).toEqual([]);
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
          enableOutcome({ global: [target(enabledName, "cursor", status)], projects: { skipped_projects: [], items: [] } }, ctx),
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
            projects: { skipped_projects: [], items: [projectItem("claude", { status: "declined" })] },
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

describe("Delete over several skills (ticket 13, D16)", () => {
  const kept: RemovalReport = {
    targets: [
      {
        path: "/tools/codex/release-train",
        rows: [{ scope: "global_target", id: "t1", skill_id: "rt", tool: "codex", folder_key: "codex" }],
        status: { status: "removed" },
      },
      {
        path: "/tools/claude/release-train",
        rows: [{ scope: "global_target", id: "t2", skill_id: "rt", tool: "claude", folder_key: "claude" }],
        status: { status: "failed", error: { code: "OTHER", message: "Permission denied" } },
      },
      {
        path: "/work/mono/.agents/skills/release-train",
        rows: [{ scope: "assignment", id: "a1", project_id: "p1", skill_id: "rt", tool: "claude", folder_key: "claude" }],
        status: { status: "removed" },
      },
    ],
    central_removed: false,
    record_deleted: false,
    cleared_markers: [], disabled_markers: [],
  };
  const gone: RemovalReport = { targets: [], central_removed: true, record_deleted: true, cleared_markers: [], disabled_markers: [] };
  const settled = [
    { skillId: "a", name: "alpha", report: gone },
    { skillId: "rt", name: "release-train", report: kept },
    { skillId: "b", name: "beta", error: { code: "OTHER", message: "store locked" } },
  ];

  it("reads as the count line over skills, names each kept target by skill and closes only when all went", () => {
    const out = deleteOutcome(settled, ctx);
    expect(out.toast).toEqual({
      kind: "warning",
      tone: "failure",
      message: [t("outcome.done.removed", { count: 1 }), t("outcome.failed", { count: 2 })].join(" · "),
    });
    expect(words(out.errors)).toEqual([
      { title: `release-train: ${t("errors.deleteKeptTargetTitle", { tool: "CLAUDE" })}`, message: "Permission denied" },
      { title: t("deleteReport.failedTitle", { name: "beta" }), message: "store locked" },
    ]);
    expect(out.completion).toEqual({ reload: true, closeModal: false, conflict: false });
    const clean = deleteOutcome([settled[0], { ...settled[0], skillId: "c" }], ctx);
    expect(clean.toast).toEqual({ kind: "success", tone: "success", message: t("outcome.done.removed", { count: 2 }) });
    expect(clean.completion.closeModal).toBe(true);
  });

  it("folds a CANCELLED skill as the cancelled kind: neutral, counted, no error entry, never a failure (D10, audit A4)", () => {
    const cancelled = { skillId: "c", name: "gamma", error: { code: "CANCELLED" } };
    const one = deleteOutcome([cancelled], ctx);
    expect(one.toast).toEqual({ kind: "info", tone: "neutral", message: t("deleteCancel.single", { name: "gamma" }) });
    expect(one.errors).toEqual([]);
    expect(one.completion.closeModal).toBe(true);
    const mixed = deleteOutcome([settled[0], cancelled], ctx);
    expect(mixed.toast).toEqual({
      kind: "info",
      tone: "neutral",
      message: [t("outcome.done.removed", { count: 1 }), t("outcome.cancelled", { count: 1 })].join(" · "),
    });
    expect(mixed.errors).toEqual([]);
    expect(mixed.completion.closeModal).toBe(true);
    // Beside a real failure the failure tone wins, the cancel is still counted, not an entry.
    const withFailure = deleteOutcome([settled[0], cancelled, settled[2]], ctx);
    expect(withFailure.toast).toMatchObject({
      tone: "failure",
      message: [
        t("outcome.done.removed", { count: 1 }),
        t("outcome.failed", { count: 1 }),
        t("outcome.cancelled", { count: 1 }),
      ].join(" · "),
    });
    expect(words(withFailure.errors)).toEqual([{ title: t("deleteReport.failedTitle", { name: "beta" }), message: "store locked" }]);
    expect(withFailure.completion.closeModal).toBe(false);
  });

  it("a single skill that threw before planning reads as not deleted, never as 0 kept targets", () => {
    const out = deleteOutcome([settled[2]], ctx);
    expect(out.toast).toEqual({ kind: "warning", tone: "failure", message: t("deleteReport.failedTitle", { name: "beta" }) });
    expect(words(out.errors)).toEqual([{ title: t("deleteReport.failedTitle", { name: "beta" }), message: "store locked" }]);
    // A kept target still names its count.
    expect(deleteOutcome([settled[1]], ctx).toast).toEqual({
      kind: "warning",
      tone: "failure",
      message: t("status.skillDeleteKept", { failed: 1 }),
    });
  });

  it("turns each settlement into a report row: kept first, removed-from labels, kept paths with their words", () => {
    const rows = deleteReportRows(settled, { ...ctx, projectLabelById: { p1: "monorepo" } });
    expect(rows.map((row) => row.skillId)).toEqual(["rt", "b", "a"]);
    expect(rows[0]).toEqual({
      skillId: "rt",
      name: "release-train",
      deleted: false,
      removedFrom: ["codex", "CLAUDE · monorepo"],
      kept: [
        {
          path: "/tools/claude/release-train",
          tools: ["CLAUDE"],
          error: { code: "OTHER", message: "Permission denied" },
          message: "Permission denied",
        },
      ],
      failure: null,
    });
    expect(rows[1]).toMatchObject({ deleted: false, kept: [], failure: "store locked" });
    expect(rows[2]).toMatchObject({ deleted: true, kept: [], removedFrom: [] });
  });
});

describe("Enable over several skills (ticket 13)", () => {
  it("reads as the count line over targets, even when clean", () => {
    const report: EnableReport = {
      global: [
        { skill_id: "a", skill_name: "alpha", tool_key: "claude", status: { status: "synced", outcome: { mode_used: "symlink", target_path: "/x", replaced: false } } },
      ],
      projects: { skipped_projects: [], items: [] },
    };
    const single = enableOutcome(report, ctx);
    expect(single.toast?.message).toBe("status.targetsEnabled");
    const batch = enableOutcome(report, { ...ctx, batch: true });
    expect(batch.toast).toEqual({ kind: "success", tone: "success", message: t("outcome.done.enabled", { count: 1 }) });
  });
});

// ---------------------------------------------------------------------------
// Import's group settlement (audit F3 / F4 / F7): each sent group's outcome
// is derived from its settled target rows; the headline names each kind.
// ---------------------------------------------------------------------------

describe("importGroupKind / import headline by settled targets", () => {
  const target = (tool: string, status: BatchTargetOutcome["status"]): BatchTargetOutcome => ({
    skill_id: "tdd",
    skill_name: "tdd",
    tool_key: tool,
    status,
  });
  const synced = (tool: string) =>
    target(tool, { status: "synced", outcome: { mode_used: "symlink", target_path: `/t/${tool}`, replaced: true } });
  const declined = (tool: string) => target(tool, { status: "declined" });
  const unanswered = (tool: string) =>
    target(tool, { status: "failed", error: { code: "TARGET_EXISTS", path: `/t/${tool}`, modifiedAt: null, sameContent: false } });
  const disabled = (tool: string) => target(tool, { status: "skipped", error: { code: "TARGET_DISABLED", tool } });
  const notInstalled = (tool: string) => target(tool, { status: "skipped", error: { code: "TOOL_NOT_INSTALLED", tool } });
  const replaced = (targets: BatchTargetOutcome[]): ImportGroupStatus => ({
    status: "replaced",
    skill_id: "tdd",
    skill_name: "tdd",
    central_path: "/central/tdd",
    targets,
  });
  const groups = (count: number, prefix = "s") =>
    Array.from({ length: count }, (_, i) => ({ group_name: `${prefix}${i}`, status: imported({ skill_id: `${prefix}${i}` }) }));

  it.each([
    ["every copy kept in the ask", [declined("cursor"), declined("pi"), declined("codex")], "kept"],
    ["at least one copy actually replaced", [synced("cursor"), declined("pi")], "replaced"],
    ["every target skipped (disabled)", [disabled("cursor"), disabled("pi")], "skipped"],
    ["every target skipped (not installed)", [notInstalled("cursor")], "skipped"],
    ["synced and skipped", [synced("cursor"), disabled("pi")], "replaced"],
    ["declined and unanswered: not every collision answered Keep", [declined("cursor"), unanswered("pi")], "skipped"],
    ["no target at all", [], "skipped"],
    ["nothing replaced and a target failed", [target("cursor", { status: "failed", error: { code: "OTHER", message: "busy" } })], "failed"],
  ] as const)("a Replace: %s", (_name, targets, kind) => {
    expect(importGroupKind(replaced([...targets]))).toBe(kind);
  });

  it("the group's own status decides an import, a skip and a failure", () => {
    expect(importGroupKind(imported({ targets: [declined("pi")] }))).toBe("imported");
    expect(importGroupKind({ status: "skipped", reason: { reason: "name_managed", skill_id: "s" } })).toBe("skipped");
    expect(importGroupKind({ status: "failed", error: { code: "OTHER", message: "x" } })).toBe("failed");
  });

  it("8 imported + an all-Keep Replace reads '8 imported · 1 kept' — never 9 imported", () => {
    const out = importOutcome(
      { groups: [...groups(8), { group_name: "tdd", status: replaced([declined("cursor"), declined("pi"), declined("codex")]) }] },
      ctx,
    );
    expect(out.toast?.message).toBe(line(["outcome.done.imported", 8], ["outcome.done.kept", 1]));
    expect(out.toast?.tone).toBe("success");
  });

  it("10 imported + two real Replaces reads '10 imported · 2 replaced'", () => {
    const out = importOutcome(
      {
        groups: [
          ...groups(10),
          { group_name: "tdd", status: replaced([synced("cursor"), synced("pi")]) },
          { group_name: "zod", status: replaced([synced("codex")]) },
        ],
      },
      ctx,
    );
    expect(out.toast?.message).toBe(line(["outcome.done.imported", 10], ["outcome.done.replaced", 2]));
  });

  it("an all-skipped (disabled) Replace is a skipped group, not a green replace", () => {
    const out = importOutcome({ groups: [{ group_name: "tdd", status: replaced([disabled("cursor"), disabled("pi")]) }] }, ctx);
    expect(out.toast).toMatchObject({ tone: "skipped", message: line(["outcome.skipped", 1]) });
  });

  it("a mixed synced / skipped Replace is one replaced group; its disabled skip is no failure", () => {
    const out = importOutcome({ groups: [{ group_name: "tdd", status: replaced([synced("cursor"), disabled("pi")]) }] }, ctx);
    expect(out.toast).toMatchObject({ tone: "success", message: line(["outcome.done.replaced", 1]) });
    expect(out.errors).toEqual([]);
  });

  it("a declined + unanswered Replace with nothing replaced is skipped, its unanswered folder to review", () => {
    const out = importOutcome({ groups: [{ group_name: "tdd", status: replaced([declined("cursor"), unanswered("pi")]) }] }, ctx);
    expect(out.toast).toMatchObject({
      tone: "conflict",
      message: line(["outcome.skipped", 1], ["runReport.collisions", 1]),
    });
  });

  it("a declined target under an imported or replaced group is named as an existing folder kept, not a group", () => {
    const out = importOutcome(
      {
        groups: [
          { group_name: "git", status: imported({ targets: [synced("claude"), declined("pi")] }) },
          { group_name: "tdd", status: replaced([synced("cursor"), declined("codex")]) },
        ],
      },
      ctx,
    );
    expect(out.toast?.message).toBe(
      line(["outcome.done.imported", 1], ["outcome.done.replaced", 1], ["outcome.declinedFolders", 2]),
    );
  });

  it("reads in English: '8 imported · 1 kept', '1 replaced · 2 existing folders kept', the kept-copy note", async () => {
    const i18n = i18next.createInstance();
    await i18n.init({ resources, lng: "en", interpolation: { escapeValue: false } });
    const en = i18n.t.bind(i18n) as unknown as typeof t;
    const kept = importOutcome(
      { groups: [...groups(8), { group_name: "tdd", status: replaced([declined("cursor"), declined("pi")]) }] },
      { t: en },
    );
    expect(kept.toast?.message).toBe("8 imported · 1 kept");
    const mixed = importOutcome({ groups: [{ group_name: "tdd", status: replaced([synced("cursor"), declined("pi"), declined("codex")]) }] }, { t: en });
    expect(mixed.toast?.message).toBe("1 replaced · 2 existing folders kept");
    expect(en("outcome.keptDifferentLine", { name: "handoff", count: 1, harnesses: "Pi" })).toBe(
      "handoff: 1 different copy kept in Pi",
    );
  });

  it("a different copy the operator chose to keep is a quiet note, not an original to review (F7)", () => {
    const report: ImportReport = {
      groups: [
        {
          group_name: "handoff",
          status: imported({
            skill_name: "handoff",
            originals: [{ tool: "pi", path: "/pi/handoff", status: { status: "kept_divergent" } }],
          }),
        },
      ],
    };
    const chosen = importOutcome(report, { ...ctx, keptByChoice: new Set(["handoff"]) });
    const note = t("outcome.keptDifferentLine", { name: "handoff", count: 1, harnesses: "pi" });
    expect(chosen.warnings).toEqual([]);
    expect(chosen.notes).toEqual([note]);
    expect(chosen.toast).toMatchObject({ tone: "success", message: line(["outcome.done.imported", 1]), detail: note });
    // Without the operator's keep (a sibling that diverged after the review) it is to review.
    const unresolved = importOutcome(report, ctx);
    expect(unresolved.toast).toMatchObject({
      tone: "conflict",
      message: line(["outcome.done.imported", 1], ["outcome.review.originals", 1]),
    });
  });
});

// BACKLOG #72: "1 Edit to review" sat last in its run, under every failed
// target. A run with a headline lists its skill-level items first.
describe("runItems", () => {
  const withTarget = (id: string): SkillRefreshOutcome => {
    const row = changed("target");
    return { ...row, skill_id: id, skill_name: id };
  };
  const withConflict = (id: string): SkillRefreshOutcome => {
    const row = changed("conflict");
    return { ...row, skill_id: id, skill_name: id };
  };

  it("under a headline: skill-level items first (errors, then warnings), then target-level", () => {
    const outcome = refreshOutcome(
      refreshReport([withTarget("a"), failed, withTarget("b"), skipped, withConflict("c")]),
      ctx,
    );
    expect(outcome.toast).not.toBeNull();
    const items = runItems(outcome.errors, outcome.warnings, { headline: true });
    expect(items.map((item) => [item.kind, item.subject?.skillName, item.subject?.tool ?? null])).toEqual([
      ["error", "git", null],
      ["warning", "local", null],
      ["warning", "c", null],
      ["error", "a", "claude"],
      ["error", "b", "claude"],
    ]);
  });

  it("without a headline the first item heads the run: errors first, as reported", () => {
    const errors = [{ title: "pair", message: "m", subject: { skillName: "a", tool: "claude" } }];
    const warnings = [{ title: "skill", message: "m", subject: { skillName: "b" } }];
    expect(runItems(errors, warnings, { headline: false }).map((item) => [item.kind, item.title])).toEqual([
      ["error", "pair"],
      ["warning", "skill"],
    ]);
  });
});

// ---------------------------------------------------------------------------
// Move Library folder (round 20 #03)
// ---------------------------------------------------------------------------

describe("libraryMoveOutcome", () => {
  /** Every reason the clean keeps an old copy on purpose. */
  const KEPT_REASONS = [
    "still_linked",
    "still_recorded",
    "outside_library_folder",
    "changed_since_copy",
    "destination_missing",
    "replaced",
    "stale_journal",
    "set_aside",
  ] as const;

  const moved = (id: string): LibraryMoveReport["skills"][number] => ({
    skill_id: id,
    name: id,
    status: { status: "moved", from: `/old/${id}`, to: `/new/${id}` },
  });
  const deployment = (
    id: string,
    status: LibraryMoveReport["deployments"][number]["status"],
    tool = "claude",
  ): LibraryMoveReport["deployments"][number] => ({
    skill_id: id,
    name: id,
    path: `/home/.claude/skills/${id}`,
    rows: [{ scope: "global_target", id: `${id}-row`, skill_id: id, tool, folder_key: tool }],
    status,
  });
  const report = (parts: Partial<LibraryMoveReport>): LibraryMoveReport => ({
    from: "/old",
    to: "/new",
    skills: [],
    deployments: [],
    left_behind: [{ path: "/old", reason: "old_library", skill_id: null, error: null }],
    ...parts,
  });

  it("counts moved, re-linked, skipped (missing / not ours / folder missing), failed and left behind", () => {
    const failure = { code: "REMOVAL_PERMISSION_DENIED", path: "/home/.claude/skills/b", detail: "EACCES" } as const;
    const r = report({
      skills: [moved("a"), moved("b"), moved("c"), { skill_id: "gone", name: "gone", status: { status: "skipped", reason: "missing" } }],
      deployments: [
        deployment("a", { status: "relinked", mode: "symlink" }),
        deployment("a", { status: "relinked", mode: "symlink" }, "cursor"),
        deployment("b", { status: "failed", error: failure }),
        deployment("c", { status: "skipped", reason: "not_ours" }),
        deployment("c", { status: "skipped", reason: "folder_missing" }),
        deployment("c", { status: "unchanged", reason: "copy" }),
        deployment("c", { status: "skipped", reason: "disabled" }),
      ],
      left_behind: [
        { path: "/old", reason: "old_library", skill_id: null, error: null },
        { path: "/old/notes", reason: "unrecorded", skill_id: null, error: null },
        { path: "/old/b", reason: "still_linked", skill_id: "b", error: null },
      ],
    });

    expect(libraryMoveCounts(r)).toEqual({
      moved: 3,
      missing: 1,
      relinked: 2,
      unchanged: 1,
      failed: 1,
      notOurs: 1,
      folderMissing: 1,
      disabled: 1,
      kept: 0,
      leftBehind: 2,
    });
    const outcome = libraryMoveOutcome(r, ctx);
    expect(outcome.toast).toMatchObject({
      tone: "failure",
      message: [
        'outcome.done.moved {"count":3}',
        'outcome.done.relinked {"count":2}',
        'outcome.targetsFailed {"count":1}',
        'outcome.skipped {"count":3}',
        'libraryMove.report.leftBehind {"count":2}',
      ].join(" · "),
    });
    // The failed re-point is an error offering Retry on its harness.
    expect(outcome.errors).toHaveLength(1);
    expect(outcome.errors[0]).toMatchObject({
      title: 'libraryMove.report.relinkFailedTitle {"name":"b","tool":"CLAUDE"}',
      error: failure,
      repair: { verb: "retry", tool: "claude" },
      subject: { skillId: "b", tool: "claude" },
    });
    // Unlocatable, not ours, folder missing and the kept copy are warnings.
    expect(outcome.warnings.map((entry) => entry.title)).toEqual([
      'libraryMove.report.missingTitle {"name":"gone"}',
      'libraryMove.report.notOursTitle {"name":"c","tool":"CLAUDE"}',
      'libraryMove.report.folderMissingTitle {"name":"c","tool":"CLAUDE"}',
      'libraryMove.report.stillLinkedTitle {"name":"b"}',
    ]);
    // The old folder and what stays in it are quiet notes.
    expect(outcome.notes).toEqual([
      'libraryMove.report.oldLibraryLine {"path":"/old"}',
      'libraryMove.report.unrecordedLine {"path":"/old/notes"}',
      'libraryMove.report.copiesLine {"count":1}',
      'libraryMove.report.disabledLine {"count":1}',
    ]);
    // The catalog came with the response; Settings stays open.
    expect(outcome.completion).toEqual({ reload: false, closeModal: false, conflict: false });
  });

  it("names a stored 1.3.4 report's Deployments by their folder, not a sibling's own key", () => {
    // The shape `decode_stored_report` hands out for the 1.3.4 fixture
    // (`src-tauri/src/core/tests/fixtures/library-move-report-1.3.4.json`):
    // Kimi's pre-1.4.0 rows carry the folder key the backend's rule filled in.
    const failure = { code: "OTHER", message: "permission denied" } as const;
    const r = report({
      skills: [moved("alpha")],
      deployments: [
        {
          skill_id: "alpha",
          name: "alpha",
          path: "/Users/op/.config/agents/skills/alpha",
          rows: [
            { scope: "global_target", id: "t-kimi", skill_id: "alpha", tool: "kimi_cli", folder_key: "amp" },
            { scope: "global_target", id: "t-amp", skill_id: "alpha", tool: "amp", folder_key: "amp" },
          ],
          status: { status: "failed", error: failure },
        },
        {
          skill_id: "alpha",
          name: "alpha",
          path: "/Users/op/code/app/.agents/skills/alpha",
          rows: [
            {
              scope: "assignment",
              id: "a-kimi",
              project_id: "project-app",
              skill_id: "alpha",
              tool: "kimi_cli",
              folder_key: "agents_skills",
            },
            {
              scope: "assignment",
              id: "a-amp",
              project_id: "project-app",
              skill_id: "alpha",
              tool: "amp",
              folder_key: "agents_skills",
            },
          ],
          status: { status: "skipped", reason: "not_ours" },
        },
      ],
    });
    const labels = { ...ctx, toolLabelById: { amp: "Amp folder", kimi_cli: "Kimi", agents_skills: "Agents folder" } };

    const outcome = libraryMoveOutcome(r, labels);

    expect(libraryMoveCounts(r)).toMatchObject({ failed: 1, notOurs: 1 });
    expect(outcome.errors).toHaveLength(1);
    expect(outcome.errors[0]).toMatchObject({
      title: 'libraryMove.report.relinkFailedTitle {"name":"alpha","tool":"Amp folder"}',
      repair: { verb: "retry", tool: "amp" },
      subject: { skillId: "alpha", tool: "amp" },
    });
    expect(outcome.warnings.map((entry) => [entry.title, entry.subject])).toEqual([
      [
        'libraryMove.report.notOursTitle {"name":"alpha","tool":"Agents folder"}',
        { skillId: "alpha", skillName: "alpha", tool: "agents_skills", projectId: "project-app" },
      ],
    ]);
  });

  it("a clean move is a success whose old folder is a note, not a skip", () => {
    const outcome = libraryMoveOutcome(
      report({ skills: [moved("a")], deployments: [deployment("a", { status: "relinked", mode: "symlink" })] }),
      ctx,
    );
    expect(outcome.toast).toMatchObject({
      tone: "success",
      message: 'outcome.done.moved {"count":1} · outcome.done.relinked {"count":1}',
    });
    expect(outcome.errors).toEqual([]);
    expect(outcome.warnings).toEqual([]);
  });

  it("a resumed move's headline says it finished an interrupted move", () => {
    const outcome = libraryMoveOutcome(
      report({ skills: [moved("a")], deployments: [deployment("a", { status: "relinked", mode: "symlink" })] }),
      ctx,
      { resumed: true },
    );
    expect(outcome.toast).toMatchObject({
      tone: "success",
      message: `libraryMove.report.resumed ${JSON.stringify({
        summary: 'outcome.done.moved {"count":1} · outcome.done.relinked {"count":1}',
      })}`,
    });
  });

  it("an old copy that could not be removed is a failure", () => {
    const refused = { code: "REMOVAL_REFUSED", path: "/old/a", resolved: "/x", reason: "resolvesElsewhere" } as const;
    const outcome = libraryMoveOutcome(
      report({
        skills: [moved("a")],
        left_behind: [{ path: "/old/a", reason: "removal_failed", skill_id: "a", error: refused }],
      }),
      ctx,
    );
    expect(outcome.toast?.tone).toBe("failure");
    expect(outcome.errors[0]).toMatchObject({ title: 'libraryMove.report.removalFailedTitle {"name":"a"}', error: refused });
  });

  it("a skill kept in place is a skip that says why, and so is an old copy a skill still records", () => {
    const kept = (id: string, reason: SkillKept): LibraryMoveReport["skills"][number] => ({
      skill_id: id,
      name: id,
      status: { status: "kept", reason },
    });
    const r = report({
      skills: [
        moved("a"),
        kept("link", { kind: "unrelocatable", entry: "/old/link/abs", cause: "link_into_old_library" }),
        kept("esc", { kind: "unrelocatable", entry: "/old/esc/up", cause: "link_escapes" }),
        kept("fifo", { kind: "unrelocatable", entry: "/old/fifo/pipe", cause: "special_file" }),
        kept("x", { kind: "shared_copy", with_skill_id: "y" }),
        kept("y", { kind: "shared_copy", with_skill_id: "x" }),
      ],
      left_behind: [
        { path: "/old", reason: "old_library", skill_id: null, error: null },
        { path: "/old/a", reason: "still_recorded", skill_id: "a", error: null },
      ],
    });

    expect(libraryMoveCounts(r)).toMatchObject({ moved: 1, missing: 0, kept: 5, leftBehind: 1 });
    const outcome = libraryMoveOutcome(r, ctx);
    expect(outcome.toast).toMatchObject({
      tone: "skipped",
      message: [
        'outcome.done.moved {"count":1}',
        'outcome.skipped {"count":5}',
        'libraryMove.report.leftBehind {"count":1}',
      ].join(" · "),
    });
    expect(outcome.errors).toEqual([]);
    expect(outcome.warnings.map((entry) => [entry.title, entry.message])).toEqual([
      ['libraryMove.report.keptTitle {"name":"link"}', 'libraryMove.report.kept.link_into_old_library {"entry":"/old/link/abs"}'],
      ['libraryMove.report.keptTitle {"name":"esc"}', 'libraryMove.report.kept.link_escapes {"entry":"/old/esc/up"}'],
      ['libraryMove.report.keptTitle {"name":"fifo"}', 'libraryMove.report.kept.special_file {"entry":"/old/fifo/pipe"}'],
      ['libraryMove.report.keptTitle {"name":"x"}', 'libraryMove.report.kept.shared_copy {"other":"y"}'],
      ['libraryMove.report.keptTitle {"name":"y"}', 'libraryMove.report.kept.shared_copy {"other":"x"}'],
      ['libraryMove.report.stillRecordedTitle {"name":"a"}', 'libraryMove.report.stillRecordedMessage {"path":"/old/a"}'],
    ]);
    expect(outcome.warnings[0].subject).toEqual({ skillId: "link", skillName: "link" });
  });

  it("every copy key it uses exists in English", () => {
    const en = i18next.createInstance();
    void en.init({ lng: "en", resources });
    const outcome = libraryMoveOutcome(
      report({
        skills: [moved("a"), { skill_id: "g", name: "g", status: { status: "skipped", reason: "missing" } }],
        deployments: [deployment("a", { status: "skipped", reason: "not_ours" })],
      }),
      { t: en.t.bind(en) as unknown as typeof t },
    );
    expect(outcome.toast?.message).toBe("1 moved · 2 skipped");
    expect(outcome.warnings[0].title).toBe("g was not moved");
  });

  it("every kept reason has English copy", () => {
    const en = i18next.createInstance();
    void en.init({ lng: "en", resources });
    const causes = [
      "link_into_old_library",
      "link_escapes",
      "special_file",
      "hardlink_outside",
      "unverifiable_metadata",
    ] as const;
    const outcome = libraryMoveOutcome(
      report({
        skills: [
          ...causes.map((cause) => ({
            skill_id: cause,
            name: cause,
            status: { status: "kept", reason: { kind: "unrelocatable", entry: "/old/e", cause } } as const,
          })),
          { skill_id: "x", name: "x", status: { status: "kept", reason: { kind: "shared_copy", with_skill_id: "y" } } },
        ],
        left_behind: KEPT_REASONS.map((reason) => ({ path: `/old/${reason}`, reason, skill_id: "a", error: null })),
      }),
      { t: en.t.bind(en) as unknown as typeof t },
    );
    for (const entry of outcome.warnings) {
      expect(entry.message).not.toMatch(/libraryMove\./);
      expect(entry.title).not.toMatch(/libraryMove\./);
    }
    expect(outcome.warnings).toHaveLength(causes.length + 1 + KEPT_REASONS.length);
  });

  it("an old copy the clean kept on purpose is a skip with its own warning, never a failure", () => {
    const r = report({
      skills: [moved("a")],
      left_behind: [
        { path: "/old", reason: "old_library", skill_id: null, error: null },
        ...KEPT_REASONS.map((reason) => ({ path: `/old/${reason}`, reason, skill_id: "a", error: null })),
      ],
    });

    expect(libraryMoveCounts(r)).toMatchObject({ moved: 1, leftBehind: KEPT_REASONS.length });
    const outcome = libraryMoveOutcome(r, ctx);
    expect(outcome.toast).toMatchObject({
      tone: "skipped",
      message: ['outcome.done.moved {"count":1}', `libraryMove.report.leftBehind {"count":${KEPT_REASONS.length}}`].join(
        " · ",
      ),
    });
    expect(outcome.errors).toEqual([]);
    expect(outcome.warnings.map((entry) => [entry.title, entry.message])).toEqual([
      ['libraryMove.report.stillLinkedTitle {"name":"a"}', 'libraryMove.report.stillLinkedMessage {"path":"/old/still_linked"}'],
      [
        'libraryMove.report.stillRecordedTitle {"name":"a"}',
        'libraryMove.report.stillRecordedMessage {"path":"/old/still_recorded"}',
      ],
      [
        'libraryMove.report.outsideLibraryFolderTitle {"name":"a"}',
        'libraryMove.report.outsideLibraryFolderMessage {"path":"/old/outside_library_folder"}',
      ],
      [
        'libraryMove.report.changedSinceCopyTitle {"name":"a"}',
        'libraryMove.report.changedSinceCopyMessage {"path":"/old/changed_since_copy"}',
      ],
      [
        'libraryMove.report.destinationMissingTitle {"name":"a"}',
        'libraryMove.report.destinationMissingMessage {"path":"/old/destination_missing"}',
      ],
      ['libraryMove.report.replacedTitle {"name":"a"}', 'libraryMove.report.replacedMessage {"path":"/old/replaced"}'],
      [
        'libraryMove.report.staleJournalTitle {"name":"a"}',
        'libraryMove.report.staleJournalMessage {"path":"/old/stale_journal"}',
      ],
      ['libraryMove.report.setAsideTitle {"name":"a"}', 'libraryMove.report.setAsideMessage {"path":"/old/set_aside"}'],
    ]);
    expect(outcome.warnings.every((entry) => entry.subject?.skillId === "a")).toBe(true);
  });
});

describe("gitCacheCleanOutcome (Clean Now)", () => {
  const english = async () => {
    const i18n = i18next.createInstance();
    await i18n.init({ resources, lng: "en", interpolation: { escapeValue: false } });
    return i18n.t.bind(i18n) as unknown as typeof t;
  };

  it("names the cleared repos and the ones in use, skipped", async () => {
    const out = gitCacheCleanOutcome({ cleared: 4, in_use: 2 }, { t: await english() });
    expect(out.toast).toEqual({
      kind: "warning",
      tone: "skipped",
      message: "Cleared 4 cached repos; 2 in use, skipped",
    });
    expect(out.errors).toEqual([]);
    expect(out.warnings).toEqual([]);
    expect(out.completion).toEqual({ reload: false, closeModal: false, conflict: false });
  });

  it("omits the in-use clause at zero and agrees in number", async () => {
    const en = await english();
    expect(gitCacheCleanOutcome({ cleared: 1, in_use: 0 }, { t: en }).toast).toEqual({
      kind: "success",
      tone: "success",
      message: "Cleared 1 cached repo",
    });
    expect(gitCacheCleanOutcome({ cleared: 0, in_use: 1 }, { t: en }).toast?.message).toBe(
      "Cleared 0 cached repos; 1 in use, skipped",
    );
  });
});

// Round 20 t09 review finding 4: rows a pre-1.4.0 app wrote once per
// shared-dir sibling keep their own keys for settlement, but each names the
// Deployment folder it describes (`folder_key`, the backend's answer). The
// fold counts — and names failures by — one Deployment per (skill, folder),
// per project for an assignment; it never reads a path.
describe("removal reports count one Deployment per folder", () => {
  const amp = { scope: "global_target", id: "t-amp", skill_id: "s1", tool: "amp", folder_key: "amp" } as const;
  const kimi = { scope: "global_target", id: "t-kimi", skill_id: "s1", tool: "kimi_cli", folder_key: "amp" } as const;
  const trae = (project: string) =>
    ({ scope: "assignment", id: `a-trae-${project}`, project_id: project, skill_id: "s1", tool: "trae", folder_key: "trae" }) as const;
  const traeCn = (project: string) =>
    ({ scope: "assignment", id: `a-trae_cn-${project}`, project_id: project, skill_id: "s1", tool: "trae_cn", folder_key: "trae" }) as const;
  const report = (parts: Partial<RemovalReport>): RemovalReport => ({
    targets: [],
    central_removed: false,
    record_deleted: false,
    cleared_markers: [],
    disabled_markers: [],
    ...parts,
  });

  it("a removed artifact with two sibling rows is one Deployment disabled", () => {
    const out = removalOutcome(report({ targets: [{ path: "/shared/alpha", rows: [amp, kimi], status: { status: "removed" } }] }), {
      ...ctx,
      action: "all",
    });
    expect(out.toast?.message).toBe(t("outcome.done.disabled", { count: 1 }));
  });

  it("a kept artifact with two sibling rows is one failure, named by its folder", () => {
    const kept = report({
      targets: [{ path: "/shared/alpha", rows: [amp, kimi], status: { status: "failed", error: { code: "OTHER", message: "busy" } } }],
    });
    const out = removalOutcome(kept, { ...ctx, action: "all" });
    expect(words(out.errors)).toEqual([{ title: t("errors.unsyncFailedTitle", { tool: "amp" }), message: "busy" }]);
    expect(out.errors[0].subject).toEqual({ skillId: "s1", skillName: "", tool: "amp" });
    expect(out.toast?.message).toBe(t("outcome.failed", { count: 1 }));
    const deleted = deleteOutcome([{ skillId: "s1", name: "alpha", report: kept }], ctx);
    expect(deleted.toast?.message).toBe(t("status.skillDeleteKept", { failed: 1 }));
  });

  it("never-synced sibling markers are one Deployment cleared, per project", () => {
    const out = removalOutcome(report({ cleared_markers: [trae("p1"), traeCn("p1"), trae("p2")] }), {
      ...ctx,
      action: "bulkUnassign",
    });
    expect(out.toast?.message).toBe(t("projectsSurface.done.unassigned", { count: 2 }));
  });

  it("a kept row beside its sibling's cleared marker counts once, as failed", () => {
    const out = projectRemovalOutcome(
      report({
        targets: [
          { path: "/p1/.trae/skills/alpha", rows: [trae("p1")], status: { status: "failed", error: { code: "OTHER", message: "busy" } } },
        ],
        cleared_markers: [traeCn("p1")],
      }),
      { ...ctx, action: "configureTools" },
    );
    expect(words(out.errors)).toEqual([{ title: t("errors.projectRemovalFailedTitle", { tool: "trae" }), message: "busy" }]);
    expect(out.toast?.message).toBe(t("outcome.failed", { count: 1 }));
  });

  it("the Delete report names a shared folder once", () => {
    const rows = deleteReportRows(
      [{ skillId: "s1", name: "alpha", report: report({ record_deleted: true, targets: [{ path: "/shared/alpha", rows: [amp, kimi], status: { status: "removed" } }] }) }],
      ctx,
    );
    expect(rows[0].removedFrom).toEqual(["amp"]);
  });
});
