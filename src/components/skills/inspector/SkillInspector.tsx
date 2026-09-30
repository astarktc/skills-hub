import { memo, useId, useMemo, type ReactNode } from "react";
import { X } from "lucide-react";
import type { TFunction } from "i18next";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import type { SkillPlacement } from "../../projects/useProjectState";
import type { SkillActions } from "../../../hooks/useSkillActions";
import type { CopyToClipboardFn } from "../../../hooks/useStatusReporter";
import { skillHealth, syncedStrip } from "../../../lib/skillPresentation";
import type { ManagedSkill } from "../types";
import EditsSection from "./EditsSection";
import FilesSection from "./FilesSection";
import HealthSection from "./HealthSection";
import ProjectsSection from "./ProjectsSection";
import SourceSection from "./SourceSection";
import TargetsSection from "./TargetsSection";

export type InspectorPlacement = "side" | "sheet";

type SkillInspectorProps = {
  skill: ManagedSkill;
  /** Beside the list (≥ 1200 px), or a sheet over it (D23). */
  placement: InspectorPlacement;
  /** The strip's Tools (effective target set, registry order, labelled) — the row's own. */
  stripTools: readonly { id: string; label: string }[];
  labelOf: (tool: string) => string;
  isDetected: (tool: string) => boolean;
  actions: Pick<SkillActions, "run" | "repair">;
  /** Why every verb is disabled while an action runs (null = they may run). */
  blockedReason: string | null;
  readSkillPlacement: (skillId: string) => Promise<SkillPlacement[]>;
  projectCount: number;
  copyToClipboard: CopyToClipboardFn;
  filesExpanded: boolean;
  onFilesExpandedChange: (expanded: boolean) => void;
  onClose: () => void;
  /** Where the sheet returns focus when it closes (D23: the focused row). */
  sheetFinalFocus: () => HTMLElement | boolean;
  t: TFunction;
};

/**
 * The inspector (D3, D22, D23): the focused skill — a selection, not a
 * route — beside the list, or a sheet over it below the shell breakpoint.
 * The operational question comes first: Health, Sync targets, Source,
 * Edits, Projects, Files, then Remove. Every verb goes through the door's
 * handle; health rows and the source line come only from the fold. The
 * landmark is labelled by the skill's name; its heading is the focus
 * target when the inspector is entered or reparented.
 */
function SkillInspector({
  skill,
  placement,
  stripTools,
  labelOf,
  isDetected,
  actions,
  blockedReason,
  readSkillPlacement,
  projectCount,
  copyToClipboard,
  filesExpanded,
  onFilesExpandedChange,
  onClose,
  sheetFinalFocus,
  t,
}: SkillInspectorProps) {
  const headingId = useId();
  const health = skillHealth(skill, { t });
  const strip = useMemo(
    () => syncedStrip(skill, stripTools, { labelOf, isDetected }),
    [isDetected, labelOf, skill, stripTools],
  );
  const busy = blockedReason !== null;

  const header = (
    <div className="flex shrink-0 items-start gap-2 border-b border-border py-3 pr-2 pl-4">
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <h2
          id={headingId}
          tabIndex={-1}
          data-inspector-heading={skill.id}
          title={skill.name}
          className="m-0 truncate rounded-sm text-lg leading-8 font-semibold text-foreground"
        >
          {skill.name}
        </h2>
        {skill.description ? (
          <p className="m-0 line-clamp-3 text-sm text-muted-foreground">{skill.description}</p>
        ) : null}
      </div>
      <Button variant="ghost" size="icon" aria-label={t("inspector.close")} title={t("inspector.close")} onClick={onClose}>
        <X />
      </Button>
    </div>
  );

  const files = (
    <FilesSection skill={skill} expanded={filesExpanded} onExpandedChange={onFilesExpandedChange} t={t} />
  );

  // One tree whether Files is expanded or not: the Files section keeps its
  // place (and its reader state, and focus on its toggle) while the other
  // sections step aside.
  const body: ReactNode = (
    <>
      <div className={cn("min-h-0 flex-1", filesExpanded ? "flex flex-col" : "overflow-y-auto")}>
        {filesExpanded ? null : (
          <>
            <HealthSection skill={skill} health={health} labelOf={labelOf} actions={actions} blockedReason={blockedReason} t={t} />
            <TargetsSection skill={skill} strip={strip} actions={actions} blockedReason={blockedReason} t={t} />
            <SourceSection
              skill={skill}
              health={health}
              actions={actions}
              copyToClipboard={copyToClipboard}
              blockedReason={blockedReason}
              t={t}
            />
            <EditsSection skill={skill} actions={actions} blockedReason={blockedReason} t={t} />
            <ProjectsSection
              skill={skill}
              readSkillPlacement={readSkillPlacement}
              projectCount={projectCount}
              labelOf={labelOf}
              actions={actions}
              blockedReason={blockedReason}
              t={t}
            />
          </>
        )}
        {files}
      </div>
      {filesExpanded ? null : (
        <div className="flex shrink-0 items-center border-t border-border px-4 py-3">
          <Button
            size="sm"
            variant="outline"
            tone="danger"
            disabled={busy}
            title={blockedReason ?? undefined}
            aria-haspopup="dialog"
            onClick={() => void actions.run("delete", [skill])}
          >
            {t("inspector.remove")}
          </Button>
        </div>
      )}
    </>
  );

  if (placement === "sheet") {
    return (
      <Dialog
        open
        onOpenChange={(open) => {
          if (!open) onClose();
        }}
      >
        <DialogContent
          placement="sheet"
          showCloseButton={false}
          aria-labelledby={headingId}
          data-inspector={skill.id}
          initialFocus={() => document.querySelector<HTMLElement>(`[data-inspector-heading="${CSS.escape(skill.id)}"]`)}
          finalFocus={sheetFinalFocus}
          className={cn(filesExpanded && "w-[calc(100vw-56px)]")}
        >
          {header}
          {body}
        </DialogContent>
      </Dialog>
    );
  }

  return (
    <aside
      aria-labelledby={headingId}
      data-inspector={skill.id}
      className={cn(
        "flex min-h-0 flex-col border-l border-border bg-card",
        filesExpanded ? "absolute inset-0 z-10 border-l-0" : "w-[max(22.5rem,33.333%)] shrink-0",
      )}
    >
      {header}
      {body}
    </aside>
  );
}

export default memo(SkillInspector);
