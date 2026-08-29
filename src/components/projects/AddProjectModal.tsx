import { memo, useState } from "react";
import type { TFunction } from "i18next";
import Modal from "../shared/Modal";
import type { ProjectDto } from "./types";

type AddProjectModalProps = {
  open: boolean;
  loading: boolean;
  projects: ProjectDto[];
  onRegister: (
    path: string,
    gitignoreOptions: { addToGitignore: boolean; addToExclude: boolean },
  ) => Promise<void>;
  onRequestClose: () => void;
  t: TFunction;
};

const AddProjectModal = ({
  open,
  loading,
  projects,
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
    const { open: pick } = await import("@tauri-apps/plugin-dialog");
    const selected = await pick({
      directory: true,
      multiple: false,
      title: t("projects.pathLabel"),
    });
    if (selected && !Array.isArray(selected)) setPath(selected);
  };

  const handleSubmit = async () => {
    try {
      await onRegister(path, { addToGitignore, addToExclude });
      setPath("");
      setAddToGitignore(false);
      setAddToExclude(false);
    } catch {
      // Parent handles the error toast; preserve input state
    }
  };

  return (
    <Modal
      open={open}
      title={t("projects.addProjectTitle")}
      onRequestClose={onRequestClose}
      footer={
        <>
          <button className="btn btn-secondary" onClick={onRequestClose}>
            {t("cancel")}
          </button>
          <button
            className="btn btn-primary"
            onClick={handleSubmit}
            disabled={!path.trim() || isDuplicate || loading}
          >
            {t("projects.register")}
          </button>
        </>
      }
    >
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
    </Modal>
  );
};

export default memo(AddProjectModal);
