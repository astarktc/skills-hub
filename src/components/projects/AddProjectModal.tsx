import { memo, useState, type ReactNode } from "react";
import type { TFunction } from "i18next";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Field,
  FieldDescription,
  FieldError,
  FieldLabel,
  FieldLegend,
  FieldSet,
  Input,
} from "@/components/ui/field";
import type { IgnoreUpdateOptions, ProjectDto } from "./types";

type AddProjectModalProps = {
  open: boolean;
  /** Why Browse and Register are disabled — another action is running — or null. */
  blockedReason: string | null;
  projects: ProjectDto[];
  /**
   * The Browse button: the picked folder, or null when cancelled (a picker
   * failure is reported by the caller, which also answers null).
   */
  onPickFolder: () => Promise<string | null>;
  onRegister: (
    path: string,
    gitignore: IgnoreUpdateOptions,
  ) => Promise<void>;
  onRequestClose: () => void;
  /** The header progress at the dialog's foot while its registration runs. */
  progress?: ReactNode;
  t: TFunction;
};

/**
 * Add project: the folder (Input + Browse…, the Add dialog's local-folder
 * pattern; a folder already registered is refused under the field) and
 * the two ignore choices. The shell stays mounted so Close returns focus.
 */
const AddProjectModal = ({
  open,
  blockedReason,
  projects,
  onPickFolder,
  onRegister,
  onRequestClose,
  progress,
  t,
}: AddProjectModalProps) => {
  const [path, setPath] = useState("");
  const [addToGitignore, setAddToGitignore] = useState(false);
  const [addToExclude, setAddToExclude] = useState(false);

  const normalizedPath = path.replace(/[/\\]+$/, "");
  const isDuplicate =
    normalizedPath.length > 0 &&
    projects.some((p) => p.path.replace(/[/\\]+$/, "") === normalizedPath);
  const blocked = blockedReason !== null;

  const handleBrowse = async () => {
    const selected = await onPickFolder();
    if (selected) setPath(selected);
  };

  const handleSubmit = async () => {
    try {
      await onRegister(path, {
        add_to_gitignore: addToGitignore,
        add_to_exclude: addToExclude,
      });
      setPath("");
      setAddToGitignore(false);
      setAddToExclude(false);
    } catch {
      // Parent handles the error; preserve input state
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) onRequestClose();
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("projects.addProjectTitle")}</DialogTitle>
        </DialogHeader>
        <DialogBody>
          <form
            className="flex flex-col gap-4"
            onSubmit={(event) => {
              event.preventDefault();
              if (path.trim() && !isDuplicate && !blocked) void handleSubmit();
            }}
          >
            <Field invalid={isDuplicate}>
              <FieldLabel>{t("projects.pathLabel")}</FieldLabel>
              <div className="flex gap-2">
                <Input
                  type="text"
                  value={path}
                  onChange={(e) => setPath(e.target.value)}
                  placeholder={t("add.source.pathPlaceholder")}
                  autoComplete="off"
                  spellCheck={false}
                />
                <Button
                  type="button"
                  onClick={() => void handleBrowse()}
                  disabled={blocked}
                  focusableWhenDisabled
                  title={blockedReason ?? undefined}
                  aria-description={blockedReason ?? undefined}
                >
                  {t("projectsSurface.add.browse")}
                </Button>
              </div>
              <FieldError>{isDuplicate ? t("projects.duplicateError") : null}</FieldError>
              <FieldDescription>{t("projectsSurface.add.pathHelp")}</FieldDescription>
            </Field>

            <FieldSet>
              <FieldLegend>{t("projectsSurface.add.ignoreLegend")}</FieldLegend>
              <Field className="flex-row items-center gap-2">
                <Checkbox checked={addToGitignore} onCheckedChange={(checked) => setAddToGitignore(checked)} />
                <FieldLabel>{t("projects.gitignoreShared")}</FieldLabel>
              </Field>
              <Field className="flex-row items-center gap-2">
                <Checkbox checked={addToExclude} onCheckedChange={(checked) => setAddToExclude(checked)} />
                <FieldLabel>{t("projects.gitignorePrivate")}</FieldLabel>
              </Field>
            </FieldSet>
          </form>
        </DialogBody>
        <DialogFooter>
          <Button onClick={onRequestClose}>{t("cancel")}</Button>
          <Button
            variant="solid"
            tone="accent"
            onClick={() => void handleSubmit()}
            disabled={!path.trim() || isDuplicate || blocked}
            focusableWhenDisabled={blocked}
            title={blockedReason ?? undefined}
            aria-description={blockedReason ?? undefined}
          >
            {t("projects.register")}
          </Button>
        </DialogFooter>
        {progress}
      </DialogContent>
    </Dialog>
  );
};

export default memo(AddProjectModal);
