import { ArrowLeft, Clock, GitBranch } from "lucide-react";
import type { TFunction } from "i18next";
import { Button } from "@/components/ui/button";
import { useSkillDocument } from "../../hooks/useSkillDocument";
import type { ExplorePreview } from "../../hooks/useSurfaceState";
import { formatRelativeTime, skillSourceLine } from "../../lib/skillPresentation";
import SkillFiles from "./SkillFiles";

type ExplorePreviewViewProps = {
  preview: ExplorePreview;
  onBack: () => void;
  onInstall: () => void;
  t: TFunction;
};

/**
 * An Explore candidate cloned for preview (never managed): its name,
 * summary and source, Install, and its files through the same document
 * hook and file view the inspector uses — fed the clone, not a forged
 * Managed skill. Round 19b redesigns Explore's preview (D25).
 */
export default function ExplorePreviewView({ preview, onBack, onInstall, t }: ExplorePreviewViewProps) {
  const doc = useSkillDocument({ root: preview.cachePath, version: preview.fetchedAt });
  const source = skillSourceLine({ source_type: "git", source_ref: preview.sourceUrl, central_path: preview.cachePath }, t);
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex shrink-0 flex-col gap-2 border-b border-border px-6 py-4">
        <div className="flex items-center justify-between gap-2">
          <Button variant="ghost" size="sm" onClick={onBack}>
            <ArrowLeft aria-hidden="true" />
            {t("detail.back")}
          </Button>
          <Button variant="solid" tone="accent" onClick={onInstall}>
            {t("install")}
          </Button>
        </div>
        <h2 className="m-0 text-lg font-semibold text-foreground">{preview.name}</h2>
        {preview.summary ? <p className="m-0 text-sm text-muted-foreground">{preview.summary}</p> : null}
        <p className="m-0 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-faint">
          <span className="inline-flex items-center gap-1.5">
            <GitBranch className="size-3.5" aria-hidden="true" />
            {source.label}
          </span>
          <span className="inline-flex items-center gap-1.5">
            <Clock className="size-3.5" aria-hidden="true" />
            {formatRelativeTime(preview.fetchedAt, t)}
          </span>
        </p>
      </div>
      <div className="flex min-h-0 flex-1 px-6 py-4">
        <SkillFiles doc={doc} label={t("inspector.files.tree", { name: preview.name })} layout="split" t={t} />
      </div>
    </div>
  );
}
