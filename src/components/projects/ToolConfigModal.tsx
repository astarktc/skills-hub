import { memo, useState } from "react";
import type { TFunction } from "i18next";
import type { ToolStatusDto } from "../skills/types";
import type { ProjectToolDto } from "./types";

function buildInitialSelection(
  toolStatus: ToolStatusDto | null,
  currentTools: ProjectToolDto[],
): Set<string> {
  // If project already has tools configured, use those as baseline
  if (currentTools.length > 0) {
    return new Set(currentTools.map((ct) => ct.tool));
  }
  // For new projects with no tools yet, pre-select installed tools
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
  currentTools: ProjectToolDto[];
  onConfirm: (selectedTools: string[]) => Promise<void>;
  onRequestClose: () => void;
  t: TFunction;
};

const ToolConfigModalInner = ({
  loading,
  toolStatus,
  currentTools,
  onConfirm,
  onRequestClose,
  t,
}: Omit<ToolConfigModalProps, "open">) => {
  const [selectedTools, setSelectedTools] = useState<Set<string>>(() =>
    buildInitialSelection(toolStatus, currentTools),
  );
  const [detectedOnly, setDetectedOnly] = useState(true);

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
          <label className="tool-filter-toggle">
            <input
              type="checkbox"
              checked={detectedOnly}
              onChange={() => setDetectedOnly((v) => !v)}
            />
            {t("projects.toolConfigDetectedOnly")}
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
                  {toolStatus?.installed.includes(tool.key) && (
                    <span className="pick-item-badge"> (installed)</span>
                  )}
                </label>
                {tool.key === "agents_skills" && (
                  <span className="pick-item-subtitle">
                    Cursor, Codex, Amp, Kimi Code CLI, Antigravity, Cline,
                    Gemini CLI, GitHub Copilot, OpenCode
                  </span>
                )}
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

const ToolConfigModal = ({ open, ...rest }: ToolConfigModalProps) => {
  if (!open) return null;
  return <ToolConfigModalInner {...rest} />;
};

export default memo(ToolConfigModal);
