import { memo, useEffect, useState } from "react";
import type { TFunction } from "i18next";
import type { ToolStatusDto } from "../skills/types";
import type { ProjectToolDto } from "./types";

type ToolConfigModalProps = {
  open: boolean;
  loading: boolean;
  toolStatus: ToolStatusDto | null;
  currentTools: ProjectToolDto[];
  onConfirm: (selectedTools: string[]) => Promise<void>;
  onRequestClose: () => void;
  t: TFunction;
};

const ToolConfigModal = ({
  open,
  loading,
  toolStatus,
  currentTools,
  onConfirm,
  onRequestClose,
  t,
}: ToolConfigModalProps) => {
  const [selectedTools, setSelectedTools] = useState<Set<string>>(new Set());

  useEffect(() => {
    if (open) {
      const initial = new Set<string>();
      if (toolStatus) {
        for (const key of toolStatus.installed) {
          initial.add(key);
        }
      }
      for (const ct of currentTools) {
        initial.add(ct.tool);
      }
      setSelectedTools(initial);
    }
  }, [open, toolStatus, currentTools]);

  if (!open) return null;

  const tools = toolStatus?.tools ?? [];

  const handleToggle = (key: string) => {
    setSelectedTools((prev) => {
      const next = new Set(prev);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
  };

  const handleConfirm = async () => {
    await onConfirm(Array.from(selectedTools));
  };

  return (
    <div className="modal-backdrop" onClick={onRequestClose}>
      <div
        className="modal"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
      >
        <div className="modal-header">
          <div className="modal-title">{t("projects.toolConfigTitle")}</div>
          <button
            className="modal-close"
            type="button"
            onClick={onRequestClose}
            aria-label={t("close")}
          >
            &#10005;
          </button>
        </div>
        <div className="modal-body">
          <p className="helper-text">{t("projects.toolConfigDesc")}</p>
          <div className="tool-pick-list">
            {tools.map((tool) => (
              <div key={tool.key} className="pick-item">
                <label className="pick-item-label">
                  <input
                    className="pick-item-checkbox"
                    type="checkbox"
                    checked={selectedTools.has(tool.key)}
                    onChange={() => handleToggle(tool.key)}
                  />
                  <span>{tool.label}</span>
                  {toolStatus?.installed.includes(tool.key) && (
                    <span className="pick-item-badge"> (installed)</span>
                  )}
                </label>
              </div>
            ))}
          </div>
        </div>
        <div className="modal-footer">
          <button className="btn btn-secondary" onClick={onRequestClose}>
            {t("cancel")}
          </button>
          <button
            className="btn btn-primary"
            onClick={handleConfirm}
            disabled={loading}
          >
            {t("projects.toolConfigConfirm")}
          </button>
        </div>
      </div>
    </div>
  );
};

export default memo(ToolConfigModal);
