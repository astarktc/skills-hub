import { memo, useState, type ReactNode } from "react";
import type { TFunction } from "i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Field, FieldDescription, FieldLabel, FieldLegend, FieldSet } from "@/components/ui/field";
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
  /**
   * Why Confirm is disabled — another action is running — shown as its
   * title; null or absent when nothing blocks it.
   */
  blockedReason?: string | null;
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
  /** The header progress at the dialog's foot while its own save runs. */
  progress?: ReactNode;
  t: TFunction;
};

const ToolConfigModalInner = ({
  loading,
  blockedReason = null,
  toolStatus,
  savedSelection,
  scanSelectedOnly: savedScanSelectedOnly,
  labels,
  onConfirm,
  onRequestClose,
  progress,
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
    <Dialog
      open
      onOpenChange={(next) => {
        if (!next) onRequestClose();
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{labels.title}</DialogTitle>
          <DialogDescription>{labels.description}</DialogDescription>
        </DialogHeader>
        <DialogBody>
          <Field className="flex-row items-center gap-2">
            <Checkbox checked={detectedOnly} onCheckedChange={(checked) => setDetectedOnly(checked)} />
            <FieldLabel>{t("toolConfigDetectedOnly")}</FieldLabel>
          </Field>
          <FieldSet>
            <FieldLegend className="sr-only">{labels.title}</FieldLegend>
            <ul className="m-0 flex max-h-80 list-none flex-col gap-1 overflow-y-auto p-0">
              {tools.map((tool) => (
                <li key={tool.key}>
                  <Field className="flex-row items-start gap-2 py-1">
                    <Checkbox
                      className="mt-0.5"
                      checked={selectedTools.has(tool.key)}
                      onCheckedChange={() => handleToggle(tool.key)}
                    />
                    <div className="flex min-w-0 flex-col gap-0.5">
                      <span className="flex items-center gap-2">
                        <FieldLabel>{tool.label}</FieldLabel>
                        {installed.includes(tool.key) ? (
                          <Badge tone="ok">{t("status.installed")}</Badge>
                        ) : (
                          // A selected tool detection no longer sees: visible so it can be unticked.
                          <Badge>{t("toolNotDetected")}</Badge>
                        )}
                      </span>
                      {tool.constituents.length > 0 ? (
                        <FieldDescription>{tool.constituents.join(", ")}</FieldDescription>
                      ) : null}
                    </div>
                  </Field>
                </li>
              ))}
            </ul>
          </FieldSet>
          {hasScanToggle ? (
            <Field className="flex-row items-center gap-2">
              <Checkbox checked={scanSelectedOnly} onCheckedChange={(checked) => setScanSelectedOnly(checked)} />
              <FieldLabel>{labels.scanToggleLabel}</FieldLabel>
            </Field>
          ) : null}
        </DialogBody>
        <DialogFooter>
          <Button onClick={onRequestClose} disabled={loading}>
            {t("cancel")}
          </Button>
          <Button
            variant="solid"
            tone="accent"
            onClick={() => void handleConfirm()}
            disabled={loading || blockedReason !== null}
            focusableWhenDisabled
            title={blockedReason ?? undefined}
          >
            {labels.confirmLabel}
          </Button>
        </DialogFooter>
        {progress}
      </DialogContent>
    </Dialog>
  );
};

const ToolConfigModal = ({ open, ...rest }: ToolConfigModalProps) => {
  if (!open) return null;
  return <ToolConfigModalInner {...rest} />;
};

export default memo(ToolConfigModal);
