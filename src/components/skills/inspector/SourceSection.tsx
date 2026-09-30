import { Box, Copy, ExternalLink, Folder, GitBranch } from "lucide-react";
import type { TFunction } from "i18next";
import { Button } from "@/components/ui/button";
import type { SkillActions } from "../../../hooks/useSkillActions";
import type { CopyToClipboardFn } from "../../../hooks/useStatusReporter";
import { formatRelativeTime, importedSourceLine, type SkillHealthFold } from "../../../lib/skillPresentation";
import type { ManagedSkill } from "../types";
import InspectorSection from "./InspectorSection";

type SourceSectionProps = {
  skill: ManagedSkill;
  health: SkillHealthFold;
  actions: Pick<SkillActions, "run">;
  copyToClipboard: CopyToClipboardFn;
  blockedReason: string | null;
  t: TFunction;
};

const ICON = { github: GitBranch, git: GitBranch, folder: Folder, imported: Box } as const;

/**
 * Where the skill comes from (the "Upstream" step): the fold's source line
 * — label, link, Copy (a local skill shows its recorded folder, D15) — the
 * provenance in words (an imported skill: "Imported from Cursor · no
 * upstream — Refresh skips it", D18), when it last changed, and the source
 * verbs: Update, Change source…, Detach.
 */
export default function SourceSection({ skill, health, actions, copyToClipboard, blockedReason, t }: SourceSectionProps) {
  const { source } = health;
  const busy = blockedReason !== null;
  const Icon = ICON[source.icon];
  const folderMissing = skill.unlocatable === "source_missing";
  const provenance =
    source.kind === "imported"
      ? skill.imported_from_tool
        ? t("inspector.source.imported", { tool: importedSourceLine(skill, t).tool })
        : t("inspector.source.importedUnknown")
      : source.kind === "local"
        ? folderMissing
          ? t("inspector.source.localMissing")
          : t("inspector.source.local")
        : t("inspector.source.git");

  return (
    <InspectorSection title={t("inspector.sections.source")}>
      <div className="flex min-w-0 items-center gap-2">
        <Icon aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
        {source.href ? (
          <a
            href={source.href}
            target="_blank"
            rel="noreferrer"
            className="inline-flex min-w-0 items-center gap-1 rounded-sm text-sm text-foreground underline decoration-border-strong underline-offset-3 hover:decoration-foreground"
          >
            <span className="truncate">{source.label}</span>
            <ExternalLink aria-hidden="true" className="size-3 shrink-0 text-faint" />
          </a>
        ) : (
          <span className="min-w-0 truncate font-mono text-xs text-foreground" title={source.title ?? source.label}>
            {source.label}
          </span>
        )}
        {source.copyValue ? (
          <Button
            variant="ghost"
            size="icon-sm"
            className="ml-auto"
            aria-label={t("inspector.source.copyLabel", { source: source.copyValue })}
            title={t("inspector.source.copy")}
            onClick={() => void copyToClipboard(source.copyValue ?? "")}
          >
            <Copy />
          </Button>
        ) : null}
      </div>
      <p className={folderMissing ? "m-0 text-xs text-stale" : "m-0 text-xs text-muted-foreground"}>{provenance}</p>
      <p className="m-0 text-xs text-faint">
        {t("inspector.source.lastUpdated", { time: formatRelativeTime(skill.updated_at, t) })}
      </p>
      <div className="flex flex-wrap gap-2">
        {skill.refreshable && !skill.unlocatable ? (
          <Button
            size="sm"
            disabled={busy}
            title={blockedReason ?? undefined}
            onClick={() => void actions.run("update", [skill])}
          >
            {t("update")}
          </Button>
        ) : null}
        <Button
          size="sm"
          disabled={busy}
          title={blockedReason ?? undefined}
          aria-haspopup="dialog"
          onClick={() => void actions.run("repoint", [skill])}
        >
          {t("changeSource.action")}
        </Button>
        {skill.detachable ? (
          <Button
            size="sm"
            variant="ghost"
            disabled={busy}
            title={blockedReason ?? undefined}
            aria-haspopup="dialog"
            onClick={() => void actions.run("detach", [skill])}
          >
            {t("unlocatable.detach")}
          </Button>
        ) : null}
      </div>
    </InspectorSection>
  );
}
