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

export type ProjectToolsModalLabels = {
  title: string;
  description: string;
  confirmLabel: string;
};

/**
 * A project's harnesses (Project → Configure harnesses…): the project world's
 * selection dialog. The global selection it once shared this shell with is
 * Settings → Harnesses (round-19 ticket 20, D47).
 */
type ProjectToolsModalProps = {
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
  labels: ProjectToolsModalLabels;
  onConfirm: (selectedTools: string[]) => Promise<void>;
  onRequestClose: () => void;
  /** The header progress at the dialog's foot while its own save runs. */
  progress?: ReactNode;
  t: TFunction;
};

/**
 * The dialog's body: mounted with the popup each time it opens, so the
 * selection starts from the saved baseline (or the installed tools) again.
 */
const ProjectToolsBody = ({
  loading,
  blockedReason = null,
  toolStatus,
  savedSelection,
  labels,
  onConfirm,
  onRequestClose,
  t,
}: Omit<ProjectToolsModalProps, "open" | "progress">) => {
  const [selectedTools, setSelectedTools] = useState<Set<string>>(() =>
    buildInitialSelection(toolStatus, savedSelection),
  );
  const [detectedOnly, setDetectedOnly] = useState(true);

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
    await onConfirm(Array.from(selectedTools));
  };

  return (
    <>
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
    </>
  );
};

/**
 * The shell stays mounted for the page's life and only `open` toggles
 * (#97): a conditionally mounted dialog unmounts before it can return focus
 * to the control that opened it.
 */
const ProjectToolsModal = ({ open, progress, ...rest }: ProjectToolsModalProps) => (
  <Dialog
    open={open}
    onOpenChange={(next) => {
      if (!next) rest.onRequestClose();
    }}
  >
    <DialogContent>
      <ProjectToolsBody {...rest} />
      {progress}
    </DialogContent>
  </Dialog>
);

export default memo(ProjectToolsModal);
