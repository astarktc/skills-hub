import { memo } from "react";
import { TriangleAlert } from "lucide-react";
import type { TFunction } from "i18next";
import Modal from "../shared/Modal";

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
  return (
    <Modal
      open={open}
      onRequestClose={onRequestClose}
      className="modal-delete"
      bodyClassName="delete-body"
      footerClassName="space-between"
      footer={
        <>
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
        </>
      }
    >
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
    </Modal>
  );
};

export default memo(RemoveProjectModal);
