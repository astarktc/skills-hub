import { memo, useEffect, useState } from "react";
// Direct invoke import: projects subtree always runs inside Tauri context.
// App.tsx uses invokeTauri() for lazy import, but that pattern is a hook-local
// callback and not importable here. Acceptable for Tauri-only components.
import { invoke } from "@tauri-apps/api/core";
import type { TFunction } from "i18next";
import Modal from "../shared/Modal";
import type { GitignoreStatusDto, ProjectDto } from "./types";

type EditProjectModalProps = {
  open: boolean;
  project: ProjectDto | null;
  onSave: (
    projectId: string,
    gitignoreOptions: { addToGitignore: boolean; addToExclude: boolean },
  ) => Promise<void>;
  onRequestClose: () => void;
  t: TFunction;
};

const EditProjectModalInner = ({
  project,
  onSave,
  onRequestClose,
  t,
}: Omit<EditProjectModalProps, "open"> & { project: ProjectDto }) => {
  const [addToGitignore, setAddToGitignore] = useState(false);
  const [addToExclude, setAddToExclude] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let cancelled = false;
    invoke<GitignoreStatusDto>("get_project_gitignore_status", {
      projectId: project.id,
    })
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
  }, [project.id]);

  const handleSave = async () => {
    setSaving(true);
    try {
      await onSave(project.id, { addToGitignore, addToExclude });
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open
      title={t("projects.configureProject")}
      onRequestClose={onRequestClose}
      footer={
        <>
          <button className="btn btn-secondary" onClick={onRequestClose}>
            {t("cancel")}
          </button>
          <button
            className="btn btn-primary"
            onClick={handleSave}
            disabled={loading || saving}
          >
            {t("projects.save")}
          </button>
        </>
      }
    >
          <div className="form-group">
            <label className="label">{t("projects.pathLabel")}</label>
            <input
              className="input"
              type="text"
              value={project.path}
              disabled
            />
          </div>

          <div className="gitignore-section">
            <label>{t("projects.gitignoreLabel")}</label>
            {loading ? (
              <div
                className="skeleton-row"
                style={{ width: "60%", height: 20 }}
              />
            ) : (
              <>
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
              </>
            )}
          </div>
    </Modal>
  );
};

const EditProjectModal = ({
  open,
  project,
  ...rest
}: EditProjectModalProps) => {
  if (!open || !project) return null;
  return <EditProjectModalInner project={project} {...rest} />;
};

export default memo(EditProjectModal);
