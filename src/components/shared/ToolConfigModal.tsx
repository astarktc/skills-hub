import { memo, useState } from "react";
import type { TFunction } from "i18next";
import Modal from "./Modal";
import type { ToolStatusDto } from "../skills/types";
import { visibleToolChoices } from "../../lib/skillPresentation";

function buildInitialSelection(
  toolStatus: ToolStatusDto | null,
  savedSelection: string[] | null,
): Set<string> {
  // If a selection is already saved, use it as the baseline.
  if (savedSelection) {
    return new Set(savedSelection);
  }
  // Otherwise pre-select installed tools.
  const initial = new Set<string>();
  if (toolStatus) {
    for (const key of toolStatus.installed) {
      initial.add(key);
    }
  }
  return initial;
}

export type ToolConfigModalLabels = {
  title: string;
  description: string;
  confirmLabel: string;
  /** Label for the scan-only checkbox; required when scanSelectedOnly is set. */
  scanToggleLabel?: string;
};

type ToolConfigModalProps = {
  open: boolean;
  loading: boolean;
  toolStatus: ToolStatusDto | null;
  /** Saved selection baseline; null = nothing saved yet, default to installed tools. */
  savedSelection: string[] | null;
  /**
   * When provided, renders the "scan selected only" checkbox initialized to
   * this value; the draft value is passed as onConfirm's second argument.
   */
  scanSelectedOnly?: boolean;
  labels: ToolConfigModalLabels;
  onConfirm: (
    selectedTools: string[],
    scanSelectedOnly?: boolean,
  ) => Promise<void>;
  onRequestClose: () => void;
  t: TFunction;
};

const ToolConfigModalInner = ({
  loading,
  toolStatus,
  savedSelection,
  scanSelectedOnly: savedScanSelectedOnly,
  labels,
  onConfirm,
  onRequestClose,
  t,
}: Omit<ToolConfigModalProps, "open">) => {
  const [selectedTools, setSelectedTools] = useState<Set<string>>(() =>
    buildInitialSelection(toolStatus, savedSelection),
  );
  const [detectedOnly, setDetectedOnly] = useState(true);
  const [scanSelectedOnly, setScanSelectedOnly] = useState(
    savedScanSelectedOnly ?? false,
  );
  const hasScanToggle = savedScanSelectedOnly !== undefined;

  const allTools = toolStatus?.tools ?? [];
  const installed = toolStatus?.installed ?? [];
  // A tool the operator has already selected renders whatever the filter
  // says: the filter hides undetected tools the operator has no relationship
  // with, never their own state. An invisible selected entry could not be
  // unticked yet was re-persisted on every save.
  const tools = visibleToolChoices(
    allTools,
    installed,
    selectedTools,
    detectedOnly,
  );

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
    await onConfirm(
      Array.from(selectedTools),
      hasScanToggle ? scanSelectedOnly : undefined,
    );
  };

  return (
    <Modal
      open
      title={labels.title}
      onRequestClose={onRequestClose}
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
            className="btn btn-primary"
            onClick={handleConfirm}
            disabled={loading}
          >
            {labels.confirmLabel}
          </button>
        </>
  }
>
      <p className="helper-text">{labels.description}</p>
      <label className="tool-filter-toggle">
        <input
          type="checkbox"
          checked={detectedOnly}
          onChange={() => setDetectedOnly((v) => !v)}
        />
        {t("toolConfigDetectedOnly")}
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
              {installed.includes(tool.key) ? (
                <span className="pick-item-badge">
                  {" "}
                  ({t("status.installed")})
                </span>
              ) : (
                <span className="pick-item-badge muted">
                  {" "}
                  ({t("toolNotDetected")})
                </span>
              )}
            </label>
            {tool.constituents.length > 0 && (
              <span className="pick-item-subtitle">
                {tool.constituents.join(", ")}
              </span>
            )}
          </div>
        ))}
      </div>
      {hasScanToggle && (
        <label className="tool-filter-toggle">
          <input
            type="checkbox"
            checked={scanSelectedOnly}
            onChange={() => setScanSelectedOnly((v) => !v)}
          />
          {labels.scanToggleLabel}
        </label>
          )}
    </Modal>
  );
};

const ToolConfigModal = ({ open, ...rest }: ToolConfigModalProps) => {
  if (!open) return null;
  return <ToolConfigModalInner {...rest} />;
};

export default memo(ToolConfigModal);
