import { memo, useState } from "react";
import {
  AlertTriangle,
  Box,
  Copy,
  Folder,
  Link,
  MapPin,
  RefreshCw,
  Trash2,
  Unlink,
} from "lucide-react";
import { SiGithub } from "@icons-pack/react-simple-icons";
import type { TFunction } from "i18next";
import type { CopyToClipboardFn } from "../../hooks/useStatusReporter";
import InvocationModeBadge from "./InvocationModeBadge";
import type { ManagedSkill, ToolOption } from "./types";
import {
  formatRelativeTime,
  importedSourceLine,
  repoInfo,
  repointDoor,
  skillSourceLabel,
  sourceKind,
  unlocatableRepairs,
  UNLOCATABLE_STATE_KEY,
  UNLOCATABLE_TOOLTIP_KEY,
  UNLOCATABLE_REPAIR_KEY,
  type UnlocatableRepair,
} from "../../lib/skillPresentation";

type SkillCardProps = {
  skill: ManagedSkill;
  installedTools: ToolOption[];
  loading: boolean;
  onUpdate: (skill: ManagedSkill) => void;
  /** Unlocatable-skill repairs (see the badge row); Remove is `onDelete`. */
  onRepoint: (skill: ManagedSkill) => void;
  onDetach: (skill: ManagedSkill) => void;
  onRestore: (skill: ManagedSkill) => void;
  onDelete: (skillId: string) => void;
  onToggleTool: (skill: ManagedSkill, toolId: string) => void;
  onUnsync: (skillId: string) => void;
  onSyncToAllTools: (skill: ManagedSkill) => void;
  onOpenDetail: (skill: ManagedSkill) => void;
  onInvocationClick: (skillId: string) => void;
  /** The reporter's clipboard helper, handed down by the list. */
  copyToClipboard: CopyToClipboardFn;
  t: TFunction;
};

const MAX_VISIBLE_BADGES = 5;

const SkillCard = ({
  skill,
  installedTools,
  loading,
  onUpdate,
  onRepoint,
  onDetach,
  onRestore,
  onDelete,
  onToggleTool,
  onUnsync,
  onSyncToAllTools,
  onOpenDetail,
  onInvocationClick,
  copyToClipboard,
  t,
}: SkillCardProps) => {
  const kind = sourceKind(skill);
  const iconNode =
    kind === "git" ? (
      <SiGithub size={20} />
    ) : kind === "local" ? (
      <Folder size={20} />
    ) : (
      <Box size={20} />
    );
  const github = repoInfo(skill.source_ref);
  const copyValue = (github?.href ?? skill.source_ref ?? "").trim();
  // An imported skill's found-in Tool is display-only history: shown, never
  // treated as a source.
  const imported = importedSourceLine(skill, t);

  const handleCopy = () => {
    if (!copyValue) return;
    void copyToClipboard(copyValue);
  };

  // The Unlocatable state and the affordance bits are the backend's answers
  // (computed at list time); `unlocatableRepairs` pairs them, the card only
  // wires each repair to its handler. Remove is always offered.
  const unlocatable = skill.unlocatable;
  const repairHandlers: Record<UnlocatableRepair, () => void> = {
    repoint: () => onRepoint(skill),
    detach: () => onDetach(skill),
    restore: () => onRestore(skill),
  };
  const repairs = unlocatableRepairs(skill).map((repair) => ({
    key: repair,
    label: t(UNLOCATABLE_REPAIR_KEY[repair]),
    onClick: repairHandlers[repair],
  }));

  // Split tools into synced and remaining for badge display
  const syncedTools: { tool: ToolOption; target: (typeof skill.targets)[0] }[] =
    [];
  const unsyncedTools: ToolOption[] = [];
  for (const tool of installedTools) {
    const target = skill.targets.find((tgt) => tgt.tool === tool.id);
    if (target) {
      syncedTools.push({ tool, target });
    } else {
      unsyncedTools.push(tool);
    }
  }

  const [expanded, setExpanded] = useState(false);
  const needsCollapse = syncedTools.length > MAX_VISIBLE_BADGES;
  const visibleSynced = expanded
    ? syncedTools
    : syncedTools.slice(0, MAX_VISIBLE_BADGES);
  const remainingCount = syncedTools.length - MAX_VISIBLE_BADGES;

  return (
    <div className="skill-card">
      <div className="skill-icon">{iconNode}</div>
      <div className="skill-main">
        <div className="skill-header-row">
          <button
            type="button"
            className="skill-name clickable"
            onClick={() => onOpenDetail(skill)}
          >
            {skill.name}
          </button>
          <InvocationModeBadge
            mode={skill.invocation_mode}
            override={skill.invocation_override}
            disabled={loading || skill.unlocatable === "central_missing"}
            onClick={() => onInvocationClick(skill.id)}
            t={t}
          />
        </div>
        {skill.description ? (
          <div className="skill-desc">{skill.description}</div>
        ) : null}
        <div className="skill-meta-row">
          {kind === "imported" ? (
            <div
              className="skill-source"
              title={t("provenance.managedHereTooltip", {
                tool: imported.tool,
              })}
            >
              <span className="repo-pill">{imported.managedHere}</span>
              <span className="dot">•</span>
              {imported.importedFrom}
            </div>
          ) : github ? (
            <div className="skill-source">
              <button
                className="repo-pill copyable"
                type="button"
                title={t("copy")}
                aria-label={t("copy")}
                onClick={handleCopy}
                disabled={!copyValue}
              >
                {github.label}
                <span className="copy-icon" aria-hidden="true">
                  <Copy size={12} />
                </span>
              </button>
            </div>
          ) : (
            <div className="skill-source">
              <button
                className="repo-pill copyable"
                type="button"
                title={t("copy")}
                aria-label={t("copy")}
                onClick={handleCopy}
                disabled={!copyValue}
              >
                <span className="mono">{skillSourceLabel(skill)}</span>
                <span className="copy-icon" aria-hidden="true">
                  <Copy size={12} />
                </span>
              </button>
            </div>
          )}
          <div className="skill-source time">
            <span className="dot">•</span>
            {formatRelativeTime(skill.updated_at, t)}
          </div>
        </div>
        {unlocatable ? (
          <div
            className="skill-unlocatable"
            role="status"
            title={t(UNLOCATABLE_TOOLTIP_KEY[unlocatable])}
          >
            <span className="unlocatable-badge">
              <AlertTriangle size={12} aria-hidden="true" />
              {t(UNLOCATABLE_STATE_KEY[unlocatable])}
            </span>
            {repairs.map((repair) => (
              <button
                key={repair.key}
                type="button"
                className="unlocatable-action"
                onClick={repair.onClick}
                disabled={loading}
              >
                {repair.label}
              </button>
            ))}
            <button
              type="button"
              className="unlocatable-action danger"
              onClick={() => onDelete(skill.id)}
              disabled={loading}
            >
              {t("remove")}
            </button>
          </div>
        ) : null}
        <div
          className={`tool-matrix${!expanded && needsCollapse ? " collapsed" : ""}`}
        >
          {visibleSynced.map(({ tool, target }) => {
            const isError = target.status === "error";
            return (
              <button
                key={`${skill.id}-${tool.id}`}
                type="button"
                className={`tool-pill active${isError ? " error" : ""}`}
                title={
                  isError
                    ? t("syncTarget.errorTitle", { tool: tool.label })
                    : `${tool.label} (${target.mode ?? t("unknown")})`
                }
                onClick={() => void onToggleTool(skill, tool.id)}
              >
                <span className="status-badge" />
                {tool.label}
              </button>
            );
          })}
          {needsCollapse && !expanded ? (
            <button
              type="button"
              className="tool-pill more-badge"
              onClick={() => setExpanded(true)}
            >
              {t("moreTools", { count: remainingCount })}
            </button>
          ) : null}
          {expanded &&
            unsyncedTools.map((tool) => (
              <button
                key={`${skill.id}-${tool.id}`}
                type="button"
                className="tool-pill inactive"
                title={tool.label}
                onClick={() => void onToggleTool(skill, tool.id)}
              >
                {tool.label}
              </button>
            ))}
        </div>
      </div>
      <div className="skill-actions-col">
        {skill.refreshable && !unlocatable ? (
          <button
            className="card-btn primary-action"
            type="button"
            onClick={() => onUpdate(skill)}
            disabled={loading}
            aria-label={t("update")}
          >
            <RefreshCw size={16} />
          </button>
        ) : null}
        {repointDoor(skill) === "git" ? (
          <button
            className="card-btn secondary-action"
            type="button"
            onClick={() => onRepoint(skill)}
            disabled={loading}
            aria-label={t("gitRepoint.action")}
            title={t("gitRepoint.action")}
          >
            <MapPin size={16} />
          </button>
        ) : null}
        <button
          className="card-btn secondary-action"
          type="button"
          onClick={() =>
            skill.targets.length > 0
              ? onUnsync(skill.id)
              : onSyncToAllTools(skill)
          }
          disabled={loading}
          aria-label={
            skill.targets.length > 0
              ? t("unsyncSkillTooltip")
              : t("syncSkillTooltip")
          }
          title={
            skill.targets.length > 0
              ? t("unsyncSkillTooltip")
              : t("syncSkillTooltip")
          }
        >
          {skill.targets.length > 0 ? <Link size={16} /> : <Unlink size={16} />}
        </button>
        <button
          className="card-btn danger-action"
          type="button"
          onClick={() => onDelete(skill.id)}
          disabled={loading}
          aria-label={t("remove")}
        >
          <Trash2 size={16} />
        </button>
      </div>
    </div>
  );
};

export default memo(SkillCard);
