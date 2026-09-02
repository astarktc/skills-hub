import { memo, useMemo } from "react";
import { GitBranch, MessageCircle } from "lucide-react";
import type { TFunction } from "i18next";
import type { ManagedSkill, OnboardingPlan, ToolOption } from "./types";
import SkillCard from "./SkillCard";
import { groupSkillsByRepo } from "../../lib/skillPresentation";

type SkillsListProps = {
  plan: OnboardingPlan | null;
  visibleSkills: ManagedSkill[];
  groupByRepo: boolean;
  viewMode: "list" | "auto-grid" | "dense-grid";
  installedTools: ToolOption[];
  loading: boolean;
  onReviewImport: () => void;
  onUpdateSkill: (skill: ManagedSkill) => void;
  onDeleteSkill: (skillId: string) => void;
  onToggleTool: (skill: ManagedSkill, toolId: string) => void;
  onUnsyncSkill: (skillId: string) => void;
  onSyncSkillToAllTools: (skill: ManagedSkill) => void;
  onOpenDetail: (skill: ManagedSkill) => void;
  t: TFunction;
};

const SkillsList = ({
  plan,
  visibleSkills,
  groupByRepo,
  viewMode,
  installedTools,
  loading,
  onReviewImport,
  onUpdateSkill,
  onDeleteSkill,
  onToggleTool,
  onUnsyncSkill,
  onSyncSkillToAllTools,
  onOpenDetail,
  t,
}: SkillsListProps) => {
  const groups = useMemo(() => {
    if (!groupByRepo) return null;
    return groupSkillsByRepo(visibleSkills, {
      local: t("localGroup"),
      ungrouped: t("ungrouped"),
    });
  }, [groupByRepo, visibleSkills, t]);

  const renderSkill = (skill: ManagedSkill) => (
    <SkillCard
      key={skill.id}
      skill={skill}
      installedTools={installedTools}
      loading={loading}
      onUpdate={onUpdateSkill}
      onDelete={onDeleteSkill}
      onToggleTool={onToggleTool}
      onUnsync={onUnsyncSkill}
      onSyncToAllTools={onSyncSkillToAllTools}
      onOpenDetail={onOpenDetail}
      t={t}
    />
  );

  const gridClass =
    viewMode !== "list" ? `skills-grid skills-grid--${viewMode}` : "";

  return (
    <div className="skills-list">
      {plan && plan.total_skills_found > 0 ? (
        <div className="discovered-banner">
          <div className="banner-left">
            <div className="banner-icon">
              <MessageCircle size={18} />
            </div>
            <div className="banner-content">
              <div className="banner-title">{t("discoveredTitle")}</div>
              <div className="banner-subtitle">
                {t("discoveredCount", { count: plan.total_skills_found })}
              </div>
            </div>
          </div>
          <button
            className="btn btn-warning"
            type="button"
            onClick={onReviewImport}
            disabled={loading}
          >
            {t("reviewImport")}
          </button>
        </div>
      ) : null}

      {visibleSkills.length === 0 ? (
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
        <div className={gridClass}>{visibleSkills.map(renderSkill)}</div>
      )}
    </div>
  );
};

export default memo(SkillsList);
