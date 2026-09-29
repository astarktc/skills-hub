import { memo, useState } from "react";
import {
  AlertTriangle,
  Box,
  Copy,
  Folder,
  GitBranch,
  Link,
  MapPin,
  RefreshCw,
  Trash2,
  Unlink,
} from "lucide-react";
import { SiGithub } from "@icons-pack/react-simple-icons";
import type { TFunction } from "i18next";
import type { CopyToClipboardFn } from "../../hooks/useStatusReporter";
import type { SkillActions } from "../../hooks/useSkillActions";
import { describeCommandError } from "../../commandError";
import { cn } from "@/lib/utils";
import { isPendingSkill, type PendingAction } from "../../lib/actionLifecycle";
import InvocationModeBadge from "./InvocationModeBadge";
import type { ManagedSkill, SkillTargetDto, ToolOption } from "./types";
import {
  deployAction,
  formatRelativeTime,
  HEALTH_REPAIR_LABEL_KEY,
  HEALTH_ROW_LABEL_KEY,
  skillHealth,
  skillToolChips,
  UNLOCATABLE_TOOLTIP_KEY,
  type HealthRow,
  type SkillToolChip,
} from "../../lib/skillPresentation";

type SyncedChip = SkillToolChip & { target: SkillTargetDto };

type SkillCardProps = {
  skill: ManagedSkill;
  installedTools: ToolOption[];
  /** Every registry tool, detected or not — labels the skill's orphaned rows. */
  allTools: ToolOption[];
  /**
   * The action running now: this card's own shows inline (`aria-busy`); any
   * running action disables the card's verbs (one mutation at a time, D19).
   */
  pending: PendingAction | null;
  /** Why the verbs are disabled while something runs (the refusal in words). */
  blockedReason: string | null;
  /** The skill action door: every verb on this card goes through it. */
  actions: Pick<SkillActions, "run" | "repair">;
  onOpenDetail: (skill: ManagedSkill) => void;
  /** The reporter's clipboard helper, handed down by the list. */
  copyToClipboard: CopyToClipboardFn;
  t: TFunction;
};

const MAX_VISIBLE_BADGES = 5;

const SkillCard = ({
  skill,
  installedTools,
  allTools,
  pending,
  blockedReason,
  actions,
  onOpenDetail,
  copyToClipboard,
  t,
}: SkillCardProps) => {
  const { run, repair } = actions;
  const busy = pending !== null;
  const busyHere = isPendingSkill(pending, skill.id);
  // One fold answers health and the source line (card, inspector, banner).
  const health = skillHealth(skill, { t });
  const { source } = health;
  const iconNode =
    source.icon === "github" ? (
      <SiGithub size={20} />
    ) : source.icon === "git" ? (
      <GitBranch size={20} />
    ) : source.icon === "imported" ? (
      <Box size={20} />
    ) : (
      <Folder size={20} />
    );

  const handleCopy = () => {
    if (!source.copyValue) return;
    void copyToClipboard(source.copyValue);
  };

  // The Unlocatable row (computed at list time) and the stored last outcome
  // (Skill health) — skill-level rows of the fold, each with its repairs.
  // Target-level rows read on the pills until ticket 11's strip.
  const unlocatable = health.rows.find(
    (row) => row.kind === "source_missing" || row.kind === "central_missing",
  );
  const storedRow = skill.health
    ? health.rows.find((row) => row.kind === skill.health?.kind)
    : undefined;
  const repairButtons = (row: HealthRow) =>
    row.repairs.map((key) => (
      <button
        key={key}
        type="button"
        className={cn("unlocatable-action", key === "delete" && "danger")}
        onClick={() => void repair(skill, row, key)}
        disabled={busy}
        title={busy ? (blockedReason ?? undefined) : undefined}
      >
        {t(key === "delete" && row.kind !== "delete_incomplete" ? "remove" : HEALTH_REPAIR_LABEL_KEY[key])}
      </button>
    ));

  // Chips are the union of the rows this skill has and the installed tools,
  // so a target for an undetected Tool stays reachable (it renders muted and
  // keeps its unsync affordance) instead of being invisible.
  const chips = skillToolChips(skill.targets, installedTools, allTools);
  const syncedTools = chips.filter(
    (chip): chip is SyncedChip => chip.target !== null,
  );
  const unsyncedTools = chips.filter((chip) => chip.target === null);
  // Deployed somewhere: any live target (a failed first sync is not one). A
  // skill disabled at either grain offers the deploy action as an Enable,
  // labelled with the pairs it turns back on; one with no live target
  // (never deployed, or only failed first syncs) offers a sync.
  const deploy = deployAction(skill);
  const deployed = deploy.kind === "deployed";
  const deployLabel = deployed
    ? t("unsyncSkillTooltip")
    : deploy.kind === "enable" && deploy.disabledTargets > 0
      ? t("disabledSkips.deployDisabledPairs", { count: deploy.disabledTargets })
      : t("syncSkillTooltip");
  const handleDeploy = () =>
    void (deploy.kind === "deployed"
      ? run("disable", [skill])
      : deploy.kind === "enable"
        ? run("enable", [skill])
        : run("syncToTargets", [skill]));
  // Three states per pill: a disabled target enables, any other target
  // disables (the durable "not on this Tool"), no target syncs.
  const handleToggleTool = (chip: SkillToolChip) => {
    const status = chip.target?.status;
    void (status === "disabled"
      ? run("enable", [skill], { tool: chip.id })
      : chip.target
        ? run("disable", [skill], { tool: chip.id })
        : run("syncToTargets", [skill], { tool: chip.id }));
  };
  const blockedTitle = (title: string) => (busy && blockedReason ? blockedReason : title);

  const [expanded, setExpanded] = useState(false);
  const needsCollapse = syncedTools.length > MAX_VISIBLE_BADGES;
  const visibleSynced = expanded
    ? syncedTools
    : syncedTools.slice(0, MAX_VISIBLE_BADGES);
  const remainingCount = syncedTools.length - MAX_VISIBLE_BADGES;

  return (
    <div className={cn("skill-card", busyHere && "opacity-70")} aria-busy={busyHere || undefined}>
      <div className="skill-icon">{iconNode}</div>
      <div className="skill-main">
        <div className="skill-header-row">
          <button
            type="button"
            className="skill-name clickable"
            data-skill-row={skill.id}
            onClick={() => onOpenDetail(skill)}
          >
            {skill.name}
          </button>
          <InvocationModeBadge
            mode={skill.invocation_mode}
            override={skill.invocation_override}
            disabled={busy || skill.unlocatable === "central_missing"}
            onClick={() => void run("editInvocation", [skill])}
            t={t}
          />
        </div>
        {skill.description ? (
          <div className="skill-desc">{skill.description}</div>
        ) : null}
        <div className="skill-meta-row">
          {source.kind === "imported" ? (
            <div className="skill-source" title={source.title ?? undefined}>
              <span className="repo-pill">{source.label}</span>
              <span className="dot">•</span>
              {source.detail}
            </div>
          ) : (
            <div className="skill-source">
              <button
                className="repo-pill copyable"
                type="button"
                title={t("copy")}
                aria-label={t("copy")}
                onClick={handleCopy}
                disabled={!source.copyValue}
              >
                {source.icon === "github" ? source.label : <span className="mono">{source.label}</span>}
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
        {unlocatable && skill.unlocatable ? (
          <div
            className="skill-unlocatable"
            role="status"
            tabIndex={-1}
            data-skill-health={skill.id}
            title={t(UNLOCATABLE_TOOLTIP_KEY[skill.unlocatable])}
          >
            <span className="unlocatable-badge">
              <AlertTriangle size={12} aria-hidden="true" />
              {t(HEALTH_ROW_LABEL_KEY[unlocatable.kind])}
            </span>
            {unlocatable.kind === "source_missing" && skill.source_ref ? (
              <span className="mono" title={skill.source_ref}>
                {t("repointSource.missingFolder", { path: skill.source_ref })}
              </span>
            ) : null}
            {repairButtons(unlocatable)}
          </div>
        ) : null}
        {storedRow && storedRow.at !== null ? (
          <div
            className="skill-meta-row"
            role="status"
            tabIndex={-1}
            data-skill-health={unlocatable ? undefined : skill.id}
            title={storedRow.error ? (describeCommandError(storedRow.error, t) ?? undefined) : undefined}
          >
            <div className="skill-source">
              {t("skillHealth.line", {
                outcome: t(HEALTH_ROW_LABEL_KEY[storedRow.kind]),
                time: formatRelativeTime(storedRow.at, t),
              })}
            </div>
            {repairButtons(storedRow)}
          </div>
        ) : null}
        <div
          className={`tool-matrix${!expanded && needsCollapse ? " collapsed" : ""}`}
        >
          {visibleSynced.map((chip) => {
            const isError = chip.target.status === "error";
            // Disabled is the operator's choice, not a failure: it rides the
            // error path's labelled title without the red (ticket 11 gives the
            // strip its own mark).
            const isDisabled = chip.target.status === "disabled";
            return (
              <button
                key={`${skill.id}-${chip.id}`}
                type="button"
                className={`tool-pill ${isDisabled ? "inactive" : "active"}${
                  isError ? " error" : ""
                }${chip.detected ? "" : " undetected"}`}
                disabled={busy}
                title={blockedTitle(
                  !chip.detected
                    ? t("toolNotDetectedTooltip", { tool: chip.label })
                    : isDisabled
                      ? t("syncTarget.disabledTitle", { tool: chip.label })
                      : isError
                        ? t("syncTarget.errorTitle", { tool: chip.label })
                        : `${chip.label} (${chip.target.mode ?? t("unknown")})`,
                )}
                onClick={() => handleToggleTool(chip)}
              >
                {isDisabled ? null : <span className="status-badge" />}
                {chip.label}
                {isDisabled ? (
                  <span className="pick-item-badge muted">
                    ({t("syncTarget.disabled")})
                  </span>
                ) : null}
                {chip.detected ? null : (
                  <span className="pick-item-badge muted">
                    ({t("toolNotDetected")})
                  </span>
                )}
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
            unsyncedTools.map((chip) => (
              <button
                key={`${skill.id}-${chip.id}`}
                type="button"
                className="tool-pill inactive"
                disabled={busy}
                title={blockedTitle(chip.label)}
                onClick={() => handleToggleTool(chip)}
              >
                {chip.label}
              </button>
            ))}
        </div>
      </div>
      <div className="skill-actions-col">
        {skill.refreshable && !skill.unlocatable ? (
          <button
            className="card-btn primary-action"
            type="button"
            onClick={() => void run("update", [skill])}
            disabled={busy}
            aria-label={t("update")}
            title={busy ? (blockedReason ?? undefined) : undefined}
          >
            <RefreshCw size={16} />
          </button>
        ) : null}
        <button
          className="card-btn secondary-action"
          type="button"
          onClick={() => void run("repoint", [skill])}
          disabled={busy}
          aria-label={t("changeSource.action")}
          title={blockedTitle(t("changeSource.action"))}
        >
          <MapPin size={16} />
        </button>
        <button
          className="card-btn secondary-action"
          type="button"
          onClick={handleDeploy}
          disabled={busy}
          aria-label={deployLabel}
          title={blockedTitle(deployLabel)}
        >
          {deployed ? <Link size={16} /> : <Unlink size={16} />}
        </button>
        <button
          className="card-btn danger-action"
          type="button"
          onClick={() => void run("delete", [skill])}
          disabled={busy}
          aria-label={t("remove")}
          title={busy ? (blockedReason ?? undefined) : undefined}
        >
          <Trash2 size={16} />
        </button>
      </div>
    </div>
  );
};

export default memo(SkillCard);
