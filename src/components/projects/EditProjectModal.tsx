import { memo, useEffect, useState, type ReactNode } from "react";
import type { TFunction } from "i18next";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldLabel, FieldLegend, FieldSet, Input } from "@/components/ui/field";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type {
  GitignoreStatusDto,
  IgnoreUpdateOptions,
  ProjectDto,
} from "./types";

type EditProjectModalProps = {
  open: boolean;
  project: ProjectDto | null;
  onSave: (
    projectId: string,
    gitignore: IgnoreUpdateOptions,
  ) => Promise<void>;
  /** Backend access goes through the hook (useProjectState.getGitignoreStatus). */
  loadStatus: (projectId: string) => Promise<GitignoreStatusDto>;
  /** Why Save is disabled: another action is running (D19); null = it may run. */
  blockedReason: string | null;
  onRequestClose: () => void;
  /** The header progress at the dialog's foot while its save runs. */
  progress?: ReactNode;
  t: TFunction;
};

const EditProjectModalInner = ({
  project,
  onSave,
  loadStatus,
  blockedReason,
  onRequestClose,
  t,
}: Omit<EditProjectModalProps, "open" | "progress"> & { project: ProjectDto }) => {
  const [addToGitignore, setAddToGitignore] = useState(false);
  const [addToExclude, setAddToExclude] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let cancelled = false;
    loadStatus(project.id)
      .then((status) => {
        if (cancelled) return;
        setAddToGitignore(status.in_gitignore);
        setAddToExclude(status.in_exclude);
      })
      .catch(() => {
        setAddToGitignore(false);
        setAddToExclude(false);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [project.id, loadStatus]);

  const handleSave = async () => {
    setSaving(true);
    try {
      await onSave(project.id, {
        add_to_gitignore: addToGitignore,
        add_to_exclude: addToExclude,
      });
    } finally {
      setSaving(false);
    }
  };

  return (
    <>
      <DialogHeader>
        <DialogTitle>{t("projectsSurface.edit.title")}</DialogTitle>
      </DialogHeader>
      <DialogBody className="flex flex-col gap-4">
        <Field>
          <FieldLabel>{t("projects.pathLabel")}</FieldLabel>
          <Input type="text" value={project.path} readOnly />
        </Field>

        <FieldSet aria-busy={loading || undefined}>
          <FieldLegend>{t("projectsSurface.add.ignoreLegend")}</FieldLegend>
          {loading ? (
            <div aria-hidden="true" className="h-5 w-3/5 rounded-sm bg-raised" />
          ) : (
            <>
              <Field className="flex-row items-center gap-2">
                <Checkbox checked={addToGitignore} onCheckedChange={(checked) => setAddToGitignore(checked)} />
                <FieldLabel>{t("projects.gitignoreShared")}</FieldLabel>
              </Field>
              <Field className="flex-row items-center gap-2">
                <Checkbox checked={addToExclude} onCheckedChange={(checked) => setAddToExclude(checked)} />
                <FieldLabel>{t("projects.gitignorePrivate")}</FieldLabel>
              </Field>
            </>
          )}
        </FieldSet>
      </DialogBody>
      <DialogFooter>
        <Button onClick={onRequestClose}>{t("cancel")}</Button>
        <Button
          variant="solid"
          tone="accent"
          onClick={() => void handleSave()}
          disabled={loading || blockedReason !== null}
          pending={saving}
          focusableWhenDisabled={blockedReason !== null}
          title={blockedReason ?? undefined}
          aria-description={blockedReason ?? undefined}
        >
          {t("projects.save")}
        </Button>
      </DialogFooter>
    </>
  );
};

/**
 * Project settings: the folder (read-only) and the ignore choices. The shell
 * stays mounted for the app's life — only the body is keyed per project —
 * so closing returns focus to the control that opened it.
 */
const EditProjectModal = ({
  open,
  project,
  progress,
  ...rest
}: EditProjectModalProps) => (
  <Dialog
    open={open && project !== null}
    onOpenChange={(next) => {
      if (!next) rest.onRequestClose();
    }}
  >
    <DialogContent>
      {project ? <EditProjectModalInner key={project.id} project={project} {...rest} /> : null}
      {progress}
    </DialogContent>
  </Dialog>
);

export default memo(EditProjectModal);
