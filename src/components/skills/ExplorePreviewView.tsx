import { useEffect, useId, useRef, type ReactNode, type RefObject } from "react";
import { Check, Clock, GitBranch, X } from "lucide-react";
import type { TFunction } from "i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { useSkillDocument } from "../../hooks/useSkillDocument";
import type { ExplorePreview } from "../../hooks/useSurfaceState";
import { isFocusLost } from "../../lib/actionLifecycle";
import { formatRelativeTime, skillSourceLine } from "../../lib/skillPresentation";
import type { InspectorPlacement } from "./inspector/SkillInspector";
import SkillFiles from "./SkillFiles";

type ExplorePreviewViewProps = {
  /** The focused candidate's preview; none closes it (the sheet's Dialog stays mounted and closes). */
  preview: ExplorePreview | null;
  /** Beside the list (≥ 1200 px), or a sheet over it (D23) — the inspector's slot. */
  placement: InspectorPlacement;
  /** The Managed skill this candidate already is: it offers the inspector, never a second Install. */
  installedId: string | null;
  /** Add's deploy scope, said beside Install (D25). */
  scopeTools: readonly { id: string; label: string }[];
  deploys: boolean;
  onClose: () => void;
  onInstall: () => void;
  onOpenInstalled: (skillId: string) => void;
  /** Where closing returns focus: its row, else the next one on screen, else the search (ExplorePage's resolver). */
  returnFocus: () => HTMLElement | null;
  t: TFunction;
};

/**
 * An Explore candidate cloned for preview (never managed), in the
 * inspector's slot: its name (the arrival focus), summary, **Install** with
 * the deploy scope it will use, the source and when the preview was
 * fetched, and its files through the same document hook and file view the
 * inspector uses — fed the clone, not a forged Managed skill. An installed
 * candidate offers its Managed skill instead (D25).
 */
export default function ExplorePreviewView({
  preview,
  placement,
  onClose,
  returnFocus,
  ...panel
}: ExplorePreviewViewProps) {
  const headingId = useId();
  const headingRef = useRef<HTMLHeadingElement>(null);
  const content = preview ? (
    <PreviewPanel
      key={`${preview.cachePath}:${preview.fetchedAt}`}
      {...panel}
      preview={preview}
      placement={placement}
      onClose={onClose}
      headingId={headingId}
      headingRef={headingRef}
    />
  ) : null;

  if (placement === "sheet") {
    // The shell stays mounted and toggles `open` (the primitive's contract):
    // a Dialog unmounted with its preview never returns focus.
    return (
      <Dialog
        open={preview !== null}
        onOpenChange={(open) => {
          if (!open) onClose();
        }}
      >
        <DialogContent
          placement="sheet"
          showCloseButton={false}
          aria-labelledby={headingId}
          data-explore-preview=""
          initialFocus={headingRef}
          finalFocus={() => returnFocus() ?? true}
        >
          {content}
        </DialogContent>
      </Dialog>
    );
  }

  return content ? (
    <aside
      aria-labelledby={headingId}
      data-explore-preview=""
      className="flex min-h-0 w-[max(22.5rem,33.333%)] shrink-0 flex-col border-l border-border bg-card"
    >
      {content}
    </aside>
  ) : null;
}

type PreviewPanelProps = Omit<ExplorePreviewViewProps, "preview" | "returnFocus"> & {
  preview: ExplorePreview;
  headingId: string;
  headingRef: RefObject<HTMLHeadingElement | null>;
};

/** One preview's header and body, mounted per acquisition inside the placement's shell. */
function PreviewPanel({
  preview,
  placement,
  installedId,
  scopeTools,
  deploys,
  onClose,
  onInstall,
  onOpenInstalled,
  headingId,
  headingRef,
  t,
}: PreviewPanelProps) {
  const scopeId = useId();
  const doc = useSkillDocument({ root: preview.cachePath, version: preview.fetchedAt });
  const source = skillSourceLine({ source_type: "git", source_ref: preview.sourceUrl, central_path: preview.cachePath }, t);

  // Beside the list the heading takes focus as the preview opens — from its
  // row, or with focus nowhere (a reparent) — never from a control the
  // operator moved to meanwhile. The sheet opens on it itself.
  useEffect(() => {
    if (placement !== "side") return;
    const active = document.activeElement;
    if (isFocusLost(active) || active?.closest("[data-explore-list]")) headingRef.current?.focus();
  }, [headingRef, placement]);

  const shown = scopeTools.slice(0, 3).map((tool) => tool.label);
  const more = scopeTools.length - shown.length;
  const scopeText = !deploys
    ? t("add.summary.autoSyncOff")
    : scopeTools.length === 0
      ? t("add.summary.noTools")
      : shown.join(t("common.listSeparator"));
  const scopeTitle = scopeTools.map((tool) => tool.label).join(t("common.listSeparator"));

  const header = (
    <div className="flex shrink-0 items-start gap-2 border-b border-border py-3 pr-2 pl-4">
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <h2
          ref={headingRef}
          id={headingId}
          tabIndex={-1}
          data-explore-preview-heading=""
          title={preview.name}
          className="m-0 truncate rounded-sm text-lg leading-8 font-semibold text-foreground"
        >
          {preview.name}
        </h2>
        {preview.summary ? <p className="m-0 line-clamp-3 text-sm text-muted-foreground">{preview.summary}</p> : null}
      </div>
      <Button variant="ghost" size="icon" aria-label={t("explore.preview.close")} title={t("explore.preview.close")} onClick={onClose}>
        <X />
      </Button>
    </div>
  );

  const action = installedId ? (
    <div className="flex flex-wrap items-center gap-3">
      <Button variant="solid" tone="accent" onClick={() => onOpenInstalled(installedId)}>
        {t("explore.preview.openInLibrary")}
      </Button>
      <span className="flex items-center gap-2 text-sm text-muted-foreground">
        <Badge>
          <Check aria-hidden="true" />
          {t("explore.row.installed")}
        </Badge>
        {t("explore.preview.installedBody")}
      </span>
    </div>
  ) : (
    <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-2">
      <Button variant="solid" tone="accent" aria-haspopup="dialog" aria-describedby={scopeId} onClick={onInstall}>
        {t("install")}
      </Button>
      <p id={scopeId} className="m-0 flex min-w-0 items-baseline gap-1.5 text-sm" title={scopeTitle || undefined}>
        <span className="shrink-0 text-muted-foreground">{t("add.summary.tools")}</span>
        <span className="min-w-0 truncate text-foreground">{scopeText}</span>
        {deploys && more > 0 ? (
          <span className="shrink-0 text-muted-foreground tabular-nums">{t("add.summary.more", { count: more })}</span>
        ) : null}
      </p>
    </div>
  );

  const body: ReactNode = (
    <>
      <div className="flex shrink-0 flex-col gap-3 border-b border-border px-4 py-3">
        {action}
        <p className="m-0 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-faint">
          <span className="inline-flex min-w-0 items-center gap-1.5">
            <GitBranch className="size-3.5 shrink-0" aria-hidden="true" />
            {source.href ? (
              <a
                href={source.href}
                target="_blank"
                rel="noopener noreferrer"
                className="truncate rounded-sm text-muted-foreground underline"
              >
                {source.label}
              </a>
            ) : (
              <span className="truncate">{source.label}</span>
            )}
          </span>
          <span className="inline-flex items-center gap-1.5">
            <Clock className="size-3.5" aria-hidden="true" />
            {t("explore.preview.fetched", { time: formatRelativeTime(preview.fetchedAt, t) })}
          </span>
        </p>
      </div>
      <div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-4 py-3">
        <SkillFiles doc={doc} label={t("inspector.files.tree", { name: preview.name })} layout="stacked" t={t} />
      </div>
    </>
  );

  return (
    <>
      {header}
      {body}
    </>
  );
}
