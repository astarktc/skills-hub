import { useCallback, useEffect, useMemo, useState } from "react";
import type { CommandError } from "../bindings";
import type { ManagedSkill, RepointPreview, RepointTarget } from "../components/skills/types";
import { toCommandError } from "../commandError";
import { pickFolder } from "../lib/platform";
import { invokeTauri, isTauri } from "../lib/tauri";
import type { DialogDoor } from "./useSurfaceState";
import type { StatusReporter, TranslateFn } from "./useStatusReporter";

export type SkillLibraryDeps = {
  t: TranslateFn;
  reporter: Pick<StatusReporter, "loading" | "notify" | "notifyError" | "runAction">;
  /** The surface module: Delete, Invocation edit and Change source are its dialogs. */
  surface: Pick<DialogDoor, "layer" | "closeDialog" | "dismissDialog">;
};

const NO_SKILLS: ManagedSkill[] = [];

/**
 * What a Re-point preview answered: the target resolved (D15), or the typed
 * refusal the dialog shows beside its field — never a toast over it.
 * `undefined` from `previewRepoint`: the read did not run (disposed / stale).
 */
export type RepointPreviewAnswer =
  | { kind: "preview"; preview: RepointPreview }
  | { kind: "error"; error: CommandError };

/**
 * Skill library world: the managed-skill catalog and which skill each of
 * its dialogs is about. Every verb on a skill — sync, Disable / Enable,
 * Update / Restore / Re-point, Detach, Delete, Edit, project assignment —
 * goes through the skill action door (`useSkillActions`), which replaces
 * this catalog from a mutation's `{ report, skills }` (`replaceCatalog`) or
 * reloads it (`loadManagedSkills`).
 */
export function useSkillLibrary({ t, reporter, surface }: SkillLibraryDeps) {
  const { loading, notify, notifyError, runAction } = reporter;
  const { layer, closeDialog, dismissDialog } = surface;

  // null until the first load lands: a deep-linked focus must survive the
  // empty first render (`managedSkillIds` is what reconciliation reads).
  const [catalog, setCatalog] = useState<ManagedSkill[] | null>(null);
  const managedSkills = catalog ?? NO_SKILLS;
  const managedSkillIds = useMemo(
    () => catalog?.map((skill) => skill.id) ?? null,
    [catalog],
  );
  const findSkill = (id: string | undefined) =>
    managedSkills.find((skill) => skill.id === id) ?? null;

  /** A mutation's returned catalog replaces the list whole. */
  const replaceCatalog = useCallback((skills: ManagedSkill[]) => {
    setCatalog(skills);
  }, []);

  const loadManagedSkills = useCallback(async () => {
    try {
      setCatalog(await invokeTauri("getManagedSkills"));
    } catch (err) {
      notifyError(err);
    }
  }, [notifyError]);

  useEffect(() => {
    if (!isTauri) return;
    // Fire-and-forget load on mount. Awaited inside an IIFE so the loader's
    // setState runs in an async continuation rather than synchronously in the
    // effect body (satisfies react-hooks/set-state-in-effect).
    void (async () => {
      await loadManagedSkills();
    })();
  }, [loadManagedSkills]);

  const isSkillNameTaken = useCallback(
    (name: string) =>
      managedSkills.some(
        (skill) => skill.name.toLowerCase() === name.toLowerCase(),
      ),
    [managedSkills],
  );

  // Which skill each of this world's dialogs is about lives in the surface
  // module's dialog value; the skill itself is resolved against this list.
  const invocationEditSkill = findSkill(layer("invocationEdit")?.skillId);
  const closeInvocationEdit = useCallback(() => {
    if (!loading) closeDialog("invocationEdit");
  }, [closeDialog, loading]);

  const pendingDeleteSkill = findSkill(layer("deleteSkill")?.skillId);
  const handleCloseDelete = useCallback(() => {
    if (!loading) closeDialog("deleteSkill");
  }, [closeDialog, loading]);

  // Change source: which skill the modal is open for and the kind it opens on.
  const repointSelection = layer("repoint");
  const pendingRepointSkill = findSkill(repointSelection?.skillId);
  const handleCloseRepoint = useCallback(() => {
    closeDialog("repoint");
  }, [closeDialog]);

  // Both the modal and historical notification actions select by id. A click
  // resolves against this render's list, not the list that produced the report:
  // a skill that is gone is reported, and its dialog withdrawn.
  const repointSkillGone = repointSelection !== null && pendingRepointSkill === null;
  useEffect(() => {
    if (repointSkillGone && repointSelection) {
      notify("warning", t("errors.skillGone", { name: repointSelection.name }));
      dismissDialog("repoint");
    }
  }, [dismissDialog, repointSelection, notify, repointSkillGone, t]);

  /**
   * Resolve a Re-point target without landing it (`preview_repoint_source`,
   * D15) — a read under the lifecycle: it waits its turn, blocks mutations
   * while it runs, and its Cancel stops the acquisition (a token-taking
   * command). A refusal is answered as a value for the dialog's field.
   */
  const previewRepoint = useCallback(
    (skillId: string, target: RepointTarget) =>
      runAction<RepointPreviewAnswer>(
        {
          read: true,
          subject: { kind: "skill", id: skillId },
          label: t("repoint.previewing"),
          cancel: "previewRepointSource",
        },
        async () => {
          try {
            return { kind: "preview", preview: await invokeTauri("previewRepointSource", skillId, target) };
          } catch (err) {
            return { kind: "error", error: toCommandError(err) };
          }
        },
      ),
    [runAction, t],
  );

  // Detach's consequence confirmation and the Assign-to-project picker.
  const pendingDetachSkill = findSkill(layer("detachSkill")?.skillId);
  const handleCloseDetach = useCallback(() => {
    if (!loading) closeDialog("detachSkill");
  }, [closeDialog, loading]);
  const assignProjectSkill = findSkill(layer("assignProject")?.skillId);
  const handleCloseAssignProject = useCallback(() => {
    closeDialog("assignProject");
  }, [closeDialog]);

  const repointName = repointSelection?.name ?? "";
  /**
   * The Change source modal's folder button. A cancelled picker answers
   * null — not an action at all; a picker failure is an error Notification.
   */
  const pickRepointFolder = useCallback(async (): Promise<string | null> => {
    try {
      const [selected] = await pickFolder({
        title: t("changeSource.selectFolderTitle", { name: repointName }),
      });
      return selected ?? null;
    } catch (err) {
      notifyError(err);
      return null;
    }
  }, [notifyError, repointName, t]);

  return {
    managedSkills,
    managedSkillIds,
    replaceCatalog,
    loadManagedSkills,
    isSkillNameTaken,
    invocationEditSkill,
    closeInvocationEdit,
    pendingDeleteSkill,
    handleCloseDelete,
    repointSelection,
    pendingRepointSkill,
    handleCloseRepoint,
    pickRepointFolder,
    previewRepoint,
    pendingDetachSkill,
    handleCloseDetach,
    assignProjectSkill,
    handleCloseAssignProject,
  };
}

export type SkillLibrary = ReturnType<typeof useSkillLibrary>;
