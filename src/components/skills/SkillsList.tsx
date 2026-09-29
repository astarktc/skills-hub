import { memo } from "react";
import { GitBranch, MessageCircle } from "lucide-react";
import type { TFunction } from "i18next";
import type { ManagedSkill, OnboardingPlan, ToolOption } from "./types";
import type { CopyToClipboardFn } from "../../hooks/useStatusReporter";
import type { SkillActions } from "../../hooks/useSkillActions";
import type { PendingAction } from "../../lib/actionLifecycle";
import SkillCard from "./SkillCard";
import type { ListLens } from "../../hooks/useListLens";

type SkillsListProps = {
  plan: OnboardingPlan | null;
  /** My Skills' list lens: the visible skills, their groups (null = Flat), the view mode. */
  lens: ListLens<ManagedSkill>;
  installedTools: ToolOption[];
  /** Every registry tool, detected or not — labels a card's orphaned rows. */
  allTools: ToolOption[];
  /** The action running now (the reporter's `pending`); cards show their own inline. */
  pending: PendingAction | null;
  /** Why verbs are disabled while something runs, in words. */
  blockedReason: string | null;
  onReviewImport: () => void;
  /** The skill action door's handle: every per-skill verb goes through it. */
  actions: Pick<SkillActions, "run" | "repair">;
  onOpenDetail: (skill: ManagedSkill) => void;
  /** The reporter's clipboard helper, handed down by the binder. */
  copyToClipboard: CopyToClipboardFn;
  t: TFunction;
};

const SkillsList = ({
  plan,
  lens,
  installedTools,
  allTools,
  pending,
  blockedReason,
  onReviewImport,
  actions,
  onOpenDetail,
  copyToClipboard,
  t,
}: SkillsListProps) => {
  // `displayed` is the one on-screen order (a grouped list walks its groups
  // below); the focus rule and selection read the same sequence.
  const { displayed, groups, viewMode } = lens;

  const renderSkill = (skill: ManagedSkill) => (
    <SkillCard
      key={skill.id}
      skill={skill}
      installedTools={installedTools}
      allTools={allTools}
      pending={pending}
      blockedReason={blockedReason}
      actions={actions}
      onOpenDetail={onOpenDetail}
      copyToClipboard={copyToClipboard}
      t={t}
    />
  );

  const gridClass =
    viewMode !== "list" ? `skills-grid skills-grid--${viewMode}` : "";

  return (
    <div className="skills-list">
      {plan && plan.group_count > 0 ? (
        <div className="discovered-banner">
          <div className="banner-left">
            <div className="banner-icon">
              <MessageCircle size={18} />
            </div>
            <div className="banner-content">
              <div className="banner-title">{t("discoveredTitle")}</div>
              <div className="banner-subtitle">
                {t("discoveredCount", { count: plan.group_count })}
              </div>
            </div>
          </div>
          <button
            className="btn btn-warning"
            type="button"
            onClick={onReviewImport}
            disabled={pending !== null}
            title={blockedReason ?? undefined}
          >
            {t("reviewImport")}
          </button>
        </div>
      ) : null}

      {displayed.length === 0 ? (
        <div className="empty">{t("skillsEmpty")}</div>
      ) : groups ? (
        <>
          {groups.map((group) => (
            <div key={group.key} className="repo-group">
              <div className="repo-group-header">
                <GitBranch size={14} className="repo-group-icon" />
                {group.href ? (
                  <a
                    href={group.href}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="repo-group-link"
                  >
                    {group.label}
                  </a>
                ) : (
                  <span>{group.label}</span>
                )}
                <span className="repo-count">{group.skills.length}</span>
              </div>
              <div
                className={
                  viewMode !== "list"
                    ? `skills-grid skills-grid--${viewMode}`
                    : "skills-group-list"
                }
              >
                {group.skills.map(renderSkill)}
              </div>
            </div>
          ))}
        </>
      ) : (
        <div className={gridClass}>{displayed.map(renderSkill)}</div>
      )}
    </div>
  );
};

export default memo(SkillsList);
