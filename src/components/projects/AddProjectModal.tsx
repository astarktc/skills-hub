import { memo, useState } from "react";
import type { TFunction } from "i18next";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
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
  t: TFunction;
};

const AddProjectModal = ({
  open,
  blockedReason,
  projects,
  onPickFolder,
  onRegister,
  onRequestClose,
  t,
}: AddProjectModalProps) => {
  const [path, setPath] = useState("");
  const [addToGitignore, setAddToGitignore] = useState(false);
  const [addToExclude, setAddToExclude] = useState(false);

  const normalizedPath = path.replace(/[/\\]+$/, "");
  const isDuplicate =
    normalizedPath.length > 0 &&
    projects.some((p) => p.path.replace(/[/\\]+$/, "") === normalizedPath);

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
      // Parent handles the error toast; preserve input state
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
          <div className="form-group">
            <label className="label">{t("projects.pathLabel")}</label>
            <div className="input-row">
              <input
                className="input"
                type="text"
                value={path}
                onChange={(e) => setPath(e.target.value)}
                placeholder={t("projects.pathHelper")}
              />
              <button
                className="btn btn-secondary input-action"
                type="button"
                onClick={handleBrowse}
                disabled={blockedReason !== null}
                title={blockedReason ?? undefined}
              >
                {t("projects.browse")}
              </button>
            </div>
            {isDuplicate && path.length > 0 && (
              <div className="field-error">{t("projects.duplicateError")}</div>
            )}
          </div>

          <div className="gitignore-section">
            <label>{t("projects.gitignoreLabel")}</label>
            <label className="gitignore-checkbox">
              <input
                type="checkbox"
                checked={addToGitignore}
                onChange={(e) => setAddToGitignore(e.target.checked)}
              />
              {t("projects.gitignoreShared")}
            </label>
            <label className="gitignore-checkbox">
              <input
                type="checkbox"
                checked={addToExclude}
                onChange={(e) => setAddToExclude(e.target.checked)}
              />
              {t("projects.gitignorePrivate")}
            </label>
          </div>
        </DialogBody>
        <DialogFooter>
          <button className="btn btn-secondary" onClick={onRequestClose}>
            {t("cancel")}
          </button>
          <button
            className="btn btn-primary"
            onClick={handleSubmit}
            disabled={!path.trim() || isDuplicate || blockedReason !== null}
            title={blockedReason ?? undefined}
          >
            {t("projects.register")}
          </button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default memo(AddProjectModal);
