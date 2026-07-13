import { memo, useState } from "react";
import type { TFunction } from "i18next";
import type { ToolStatusDto } from "../types";

function buildInitialSelection(
  toolStatus: ToolStatusDto | null,
  selectedTools: string[] | null,
): Set<string> {
  // If a global selection is already saved, use it as the baseline.
  if (selectedTools) {
    return new Set(selectedTools);
  }
  // Otherwise pre-select installed tools (matches the default sync targets).
  const initial = new Set<string>();
  if (toolStatus) {
    for (const key of toolStatus.installed) {
      initial.add(key);
    }
  }
  return initial;
}

type ToolConfigModalProps = {
  open: boolean;
  loading: boolean;
  toolStatus: ToolStatusDto | null;
  selectedTools: string[] | null;
  scanSelectedOnly: boolean;
  onConfirm: (
    selectedTools: string[],
    scanSelectedOnly: boolean,
  ) => Promise<void>;
  onRequestClose: () => void;
  t: TFunction;
};

const ToolConfigModalInner = ({
  loading,
  toolStatus,
  selectedTools: savedSelection,
  scanSelectedOnly: savedScanSelectedOnly,
  onConfirm,
  onRequestClose,
  t,
}: Omit<ToolConfigModalProps, "open">) => {
  const [selectedTools, setSelectedTools] = useState<Set<string>>(() =>
    buildInitialSelection(toolStatus, savedSelection),
  );
  const [detectedOnly, setDetectedOnly] = useState(true);
  const [scanSelectedOnly, setScanSelectedOnly] = useState(
    savedScanSelectedOnly,
  );

  const allTools = toolStatus?.tools ?? [];
  const installed = toolStatus?.installed ?? [];
  const tools = detectedOnly
    ? allTools.filter((tool) => installed.includes(tool.key))
    : allTools;

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
    await onConfirm(Array.from(selectedTools), scanSelectedOnly);
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
          <div className="modal-title">{t("globalToolConfigTitle")}</div>
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
          <p className="helper-text">{t("globalToolConfigDesc")}</p>
          <label className="tool-filter-toggle">
            <input
              type="checkbox"
              checked={detectedOnly}
              onChange={() => setDetectedOnly((v) => !v)}
            />
            {t("globalToolConfigDetectedOnly")}
          </label>
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
                  {installed.includes(tool.key) && (
                    <span className="pick-item-badge"> (installed)</span>
                  )}
                </label>
              </div>
            ))}
          </div>
          <label className="tool-filter-toggle">
            <input
              type="checkbox"
              checked={scanSelectedOnly}
              onChange={() => setScanSelectedOnly((v) => !v)}
            />
            {t("globalToolConfigScanSelectedOnly")}
          </label>
        </div>
        <div className="modal-footer">
          <button
            className="btn btn-secondary"
            onClick={onRequestClose}
            disabled={loading}
          >
            {t("cancel")}
          </button>
          <button
            className="btn btn-primary"
            onClick={handleConfirm}
            disabled={loading}
          >
            {t("globalToolConfigConfirm")}
          </button>
        </div>
      </div>
    </div>
  );
};

const ToolConfigModal = ({ open, ...rest }: ToolConfigModalProps) => {
  if (!open) return null;
  return <ToolConfigModalInner {...rest} />;
};

export default memo(ToolConfigModal);
