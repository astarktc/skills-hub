import { Maximize2, Minimize2 } from "lucide-react";
import type { TFunction } from "i18next";
import { Button } from "@/components/ui/button";
import { useSkillDocument } from "../../../hooks/useSkillDocument";
import SkillFiles from "../SkillFiles";
import type { ManagedSkill } from "../types";
import InspectorSection from "./InspectorSection";

type FilesSectionProps = {
  skill: ManagedSkill;
  /** Expanded: the section takes the whole My Skills width in place (D3). */
  expanded: boolean;
  onExpandedChange: (expanded: boolean) => void;
  t: TFunction;
};

/**
 * The skill's files — tree and Markdown/code reader over its central copy
 * (`useSkillDocument`). Expand grows it to the full width in place and
 * collapses back; the list, its scroll and the focused row stay underneath.
 */
export default function FilesSection({ skill, expanded, onExpandedChange, t }: FilesSectionProps) {
  const centralMissing = skill.unlocatable === "central_missing";
  const doc = useSkillDocument(centralMissing ? null : { root: skill.central_path, version: skill.updated_at });
  const toggle = centralMissing ? null : (
    <Button
      variant="ghost"
      size="sm"
      onClick={() => onExpandedChange(!expanded)}
      data-files-toggle=""
    >
      {expanded ? <Minimize2 aria-hidden="true" /> : <Maximize2 aria-hidden="true" />}
      {expanded ? t("inspector.files.collapse") : t("inspector.files.expand")}
    </Button>
  );
  return (
    <InspectorSection title={t("inspector.sections.files")} actions={toggle} grow={expanded}>
      {centralMissing ? (
        <p className="m-0 text-sm text-muted-foreground">{t("inspector.files.centralMissing")}</p>
      ) : (
        <SkillFiles
          doc={doc}
          label={t("inspector.files.tree", { name: skill.name })}
          layout={expanded ? "split" : "stacked"}
          t={t}
        />
      )}
    </InspectorSection>
  );
}
