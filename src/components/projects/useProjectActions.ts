import { useCallback, useMemo } from "react";
import { pickFolder } from "../../lib/platform";
import {
  projectRemovalOutcome,
  projectSyncOutcome,
  removalOutcome,
} from "../../lib/reportOutcome";
import {
  describeRefusal,
  pendingCellKeys,
  refusalFor,
} from "../../lib/actionLifecycle";
import type { ProjectState } from "./useProjectState";
import type { IgnoreUpdateOptions } from "./types";
import type { ToolCatalog } from "../../hooks/useToolCatalog";
import type { StatusReporter, TranslateFn } from "../../hooks/useStatusReporter";
import type { SurfaceState } from "../../hooks/useSurfaceState";

/** A project-wide mutation's pending subject (register, resync, configure, remove, …). */
const batch = { kind: "batch" } as const;

export type ProjectActionsDeps = {
  t: TranslateFn;
  /** The project world (`useProjectState`). */
  state: ProjectState;
  catalog: Pick<ToolCatalog, "toolLabelById" | "loadProjectToolStatus">;
  /** The reporter's lifecycle: one mutation at a time (D19), every report through `applyOutcome`. */
  reporter: Pick<StatusReporter, "pending" | "runAction" | "applyOutcome" | "notify" | "notifyError">;
  surface: Pick<SurfaceState, "focus" | "layer" | "openDialog" | "dismissDialog">;
};

/**
 * The project view's operator actions. **Every** mutation — registration,
 * the Edit dialog's ignore write, a path update, tool configuration,
 * removal, resync, the matrix's cells and rows — runs under the reporter's
 * `runAction`, so while one runs every other is refused and never sends a
 * command (D19). A folder pick stays outside (it changes nothing) but is not
 * opened while refused; the mutation it leads to is gated. `blockedReason`
 * is the refusal in words: the list, matrix and dialogs (Add, Edit, Tool
 * configuration, Remove) disable their mutation controls with it as the
 * title while anything runs; `pendingCells` marks only the cells the
 * running action is about.
 */
export function useProjectActions({ t, state, catalog, reporter, surface }: ProjectActionsDeps) {
  const { toolLabelById, loadProjectToolStatus } = catalog;
  const { pending, runAction, applyOutcome, notifyError } = reporter;
  const { focus, openDialog, dismissDialog } = surface;

  const pendingCells = useMemo(() => pendingCellKeys(pending), [pending]);
  const refusal = refusalFor(pending);
  const blockedReason = refusal ? describeRefusal(refusal, t) : null;
  const label = t("lifecycle.verbs.projects");
  const cellLabel = t("lifecycle.verbs.assign");

  const handleAddProject = useCallback(
    async (path: string, gitignore: IgnoreUpdateOptions) => {
      // The ignore intent rides along with registration; the hook hands it
      // to the backend once the tool set is confirmed.
      const project = await runAction({ subject: batch, label: t("lifecycle.verbs.registerProject"), activity: t("activity.addingProject") }, async () => {
        const registered = await state.registerProject(path, gitignore);
        await state.selectProject(registered.id);
        return registered;
      });
      if (!project) return;
      focus({ kind: "project", id: project.id });
      openDialog({ kind: "projectToolConfig" });
      try {
        await loadProjectToolStatus();
      } catch (err) {
        notifyError(err);
      }
    },
    [focus, loadProjectToolStatus, notifyError, openDialog, runAction, state, t],
  );

  const handleToolConfigConfirm = useCallback(
    async (selectedTools: string[]) => {
      await runAction({ subject: batch, label }, async () => {
        const report = await state.configureTools(selectedTools);
        if (!report) return;
        await applyOutcome(
          projectRemovalOutcome(report, { t, toolLabelById, action: "configureTools" }),
          { closeModal: () => dismissDialog("projectToolConfig") },
        );
      });
    },
    [applyOutcome, dismissDialog, label, runAction, state, t, toolLabelById],
  );

  const removeDialog = surface.layer("projectRemove");
  const handleRemoveProject = useCallback(async () => {
    if (!removeDialog) return;
    await runAction({ subject: batch, label, activity: t("activity.removingProject") }, async () => {
      const report = await state.removeProject(removeDialog.projectId);
      await applyOutcome(
        projectRemovalOutcome(report, { t, toolLabelById, action: "removeProject" }),
        { closeModal: () => dismissDialog("projectRemove") },
      );
    });
  }, [applyOutcome, dismissDialog, label, removeDialog, runAction, state, t, toolLabelById]);

  const handleEditSave = useCallback(
    async (projectId: string, gitignore: IgnoreUpdateOptions) => {
      await runAction(
        {
          subject: batch,
          label: t("lifecycle.verbs.editProject"),
          successToast: t("projects.configureProject"),
        },
        async () => {
          await state.updateGitignore(projectId, gitignore);
          dismissDialog("projectEdit");
        },
      );
    },
    [dismissDialog, runAction, state, t],
  );

  const handleUpdatePath = useCallback(
    async (projectId: string) => {
      // Refused before the picker opens: a path picked now could not be applied.
      if (refusalFor(reporter.pending)) {
        await runAction({ subject: batch }, async () => undefined);
        return;
      }
      let newPath: string | undefined;
      try {
        [newPath] = await pickFolder({ title: t("projects.updatePath") });
      } catch (err) {
        notifyError(err);
        return;
      }
      if (!newPath) return;
      const picked = newPath;
      await runAction(
        {
          subject: batch,
          label: t("lifecycle.verbs.updateProjectPath"),
          successToast: t("projects.updatePathSuccess"),
        },
        () => state.updateProjectPath(projectId, picked),
      );
    },
    [notifyError, reporter.pending, runAction, state, t],
  );

  const handleResyncProject = useCallback(async () => {
    await runAction({ subject: batch, label, activity: t("activity.syncingProjects") }, async () => {
      const report = await state.resyncProject();
      await applyOutcome(projectSyncOutcome(report, { t, toolLabelById, action: "resync" }));
    });
  }, [applyOutcome, label, runAction, state, t, toolLabelById]);

  const handleResyncAll = useCallback(async () => {
    await runAction({ subject: batch, label, activity: t("activity.syncingProjects") }, async () => {
      const { report, viewRefreshed } = await state.resyncAll();
      const projectLabelById = Object.fromEntries(
        state.projects.map((project) => [project.id, project.name]),
      );
      await applyOutcome(
        projectSyncOutcome(report, { t, toolLabelById, projectLabelById, action: "resyncAll" }),
      );
      if (!viewRefreshed) reporter.notify("warning", t("projects.viewRefreshFailed"));
    });
  }, [applyOutcome, label, reporter, runAction, state, t, toolLabelById]);

  const handleToggleAssignment = useCallback(
    async (skillId: string, tool: string) => {
      await runAction({ subject: { kind: "cell", ids: [`${skillId}:${tool}`] }, label: cellLabel }, async () => {
        const result = await state.toggleAssignment(skillId, tool);
        if (!result) return;
        await applyOutcome(
          result.kind === "assigned"
            ? projectSyncOutcome(result.report, { t, toolLabelById, action: "toggleOn" })
            : removalOutcome(result.report, { t, toolLabelById, action: "toggle" }),
        );
      });
    },
    [applyOutcome, cellLabel, runAction, state, t, toolLabelById],
  );

  // A row's cells pend together: every Tool for assign, the assigned ones
  // for unassign.
  const rowCells = useCallback(
    (skillId: string, assignedOnly: boolean) =>
      (assignedOnly
        ? state.assignments.filter((a) => a.skill_id === skillId).map((a) => a.tool)
        : state.tools.map((tool) => tool.tool)
      ).map((tool) => `${skillId}:${tool}`),
    [state.assignments, state.tools],
  );

  const handleBulkAssign = useCallback(
    async (skillId: string) => {
      await runAction({ subject: { kind: "cell", ids: rowCells(skillId, false) }, label: cellLabel }, async () => {
        const report = await state.bulkAssign(skillId);
        if (report)
          await applyOutcome(projectSyncOutcome(report, { t, toolLabelById, action: "bulkAssign" }));
      });
    },
    [applyOutcome, cellLabel, rowCells, runAction, state, t, toolLabelById],
  );

  // No confirmation, symmetric with bulk assign: a removed link is
  // re-assignable, not data loss (round 16 D2).
  const handleBulkUnassign = useCallback(
    async (skillId: string) => {
      await runAction({ subject: { kind: "cell", ids: rowCells(skillId, true) }, label: cellLabel }, async () => {
        const report = await state.bulkUnassign(skillId);
        if (report)
          await applyOutcome(removalOutcome(report, { t, toolLabelById, action: "bulkUnassign" }));
      });
    },
    [applyOutcome, cellLabel, rowCells, runAction, state, t, toolLabelById],
  );

  const handleConfigureToolsFromToolbar = useCallback(async () => {
    try {
      await loadProjectToolStatus();
      openDialog({ kind: "projectToolConfig" });
    } catch (err) {
      notifyError(err);
    }
  }, [loadProjectToolStatus, notifyError, openDialog]);

  /**
   * Add project's Browse: the picked folder, or null (cancelled / failed /
   * refused). Like a path update, refused before the picker opens while
   * another action runs — the registration it leads to could not run either.
   */
  const handlePickProjectFolder = useCallback(async () => {
    if (refusalFor(reporter.pending)) {
      await runAction({ subject: batch }, async () => undefined);
      return null;
    }
    try {
      const [picked] = await pickFolder({ title: t("projects.pathLabel") });
      return picked ?? null;
    } catch (err) {
      notifyError(err);
      return null;
    }
  }, [notifyError, reporter.pending, runAction, t]);

  return {
    pendingCells,
    blockedReason,
    handleAddProject,
    handleToolConfigConfirm,
    handleRemoveProject,
    handleEditSave,
    handleUpdatePath,
    handleResyncProject,
    handleResyncAll,
    handleToggleAssignment,
    handleBulkAssign,
    handleBulkUnassign,
    handleConfigureToolsFromToolbar,
    handlePickProjectFolder,
  };
}
