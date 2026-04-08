import { memo } from "react";
import { TriangleAlert } from "lucide-react";
import type { TFunction } from "i18next";

type RemoveProjectModalProps = {
  open: boolean;
  loading: boolean;
  projectName: string | null;
  onConfirm: () => Promise<void>;
  onRequestClose: () => void;
  t: TFunction;
};

const RemoveProjectModal = ({
  open,
  loading,
  projectName,
  onConfirm,
  onRequestClose,
  t,
}: RemoveProjectModalProps) => {
  if (!open) return null;

  return (
    <div className="modal-backdrop" onClick={onRequestClose}>
      <div
        className="modal modal-delete"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
      >
        <div className="modal-body delete-body">
          <div className="delete-title">
            <TriangleAlert size={20} />
            {t("projects.removeTitle")}
          </div>
          <div className="delete-desc">
            {t("projects.removeBody", { name: projectName ?? "" })}
          </div>
          <div className="delete-warning">
            <ul>
              <li>{t("projects.removeWarning1")}</li>
              <li>{t("projects.removeWarning2")}</li>
            </ul>
          </div>
        </div>
        <div className="modal-footer space-between">
          <button
            className="btn btn-secondary"
            onClick={onRequestClose}
            disabled={loading}
          >
            {t("cancel")}
          </button>
          <button
            className="btn btn-danger-solid"
            onClick={onConfirm}
            disabled={loading}
          >
            {t("projects.removeConfirm")}
          </button>
        </div>
      </div>
    </div>
  );
};

export default memo(RemoveProjectModal);
