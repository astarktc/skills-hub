import { memo, useMemo } from "react";
import { GitBranch, MessageCircle } from "lucide-react";
import type { TFunction } from "i18next";
import type { ManagedSkill, OnboardingPlan, ToolOption } from "./types";
import SkillCard from "./SkillCard";

type GithubInfo = {
  label: string;
  href: string;
};

type SkillsListProps = {
  plan: OnboardingPlan | null;
  visibleSkills: ManagedSkill[];
  groupByRepo: boolean;
  installedTools: ToolOption[];
  loading: boolean;
  getGithubInfo: (url: string | null | undefined) => GithubInfo | null;
  getSkillSourceLabel: (skill: ManagedSkill) => string;
  formatRelative: (ms: number | null | undefined) => string;
  onReviewImport: () => void;
  onUpdateSkill: (skill: ManagedSkill) => void;
  onDeleteSkill: (skillId: string) => void;
  onToggleTool: (skill: ManagedSkill, toolId: string) => void;
  onUnsyncSkill: (skillId: string) => void;
  onOpenDetail: (skill: ManagedSkill) => void;
  t: TFunction;
};

const SkillsList = ({
  plan,
  visibleSkills,
  groupByRepo,
  installedTools,
  loading,
  getGithubInfo,
  getSkillSourceLabel,
  formatRelative,
  onReviewImport,
  onUpdateSkill,
  onDeleteSkill,
  onToggleTool,
  onUnsyncSkill,
  onOpenDetail,
  t,
}: SkillsListProps) => {
  const groups = useMemo(() => {
    if (!groupByRepo) return null;
    const map = new Map<string, ManagedSkill[]>();
    for (const skill of visibleSkills) {
      const key = skill.source_ref ?? "";
      const list = map.get(key);
      if (list) {
        list.push(skill);
      } else {
        map.set(key, [skill]);
      }
    }
    const keys = [...map.keys()].sort((a, b) => {
      if (a === "" && b !== "") return 1;
      if (b === "" && a !== "") return -1;
      return a.localeCompare(b);
    });
    return keys.map((key) => {
      const ghInfo = key ? getGithubInfo(key) : null;
      return {
        key,
        label: ghInfo ? ghInfo.label : key || t("ungrouped"),
        href: ghInfo?.href ?? null,
        skills: map.get(key)!,
      };
    });
  }, [groupByRepo, visibleSkills, t, getGithubInfo]);

  const renderSkill = (skill: ManagedSkill) => (
    <SkillCard
      key={skill.id}
      skill={skill}
      installedTools={installedTools}
      loading={loading}
      getGithubInfo={getGithubInfo}
      getSkillSourceLabel={getSkillSourceLabel}
      formatRelative={formatRelative}
      onUpdate={onUpdateSkill}
      onDelete={onDeleteSkill}
      onToggleTool={onToggleTool}
      onUnsync={onUnsyncSkill}
      onOpenDetail={onOpenDetail}
      t={t}
    />
  );

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
              {group.skills.map(renderSkill)}
            </div>
          ))}
        </>
      ) : (
        <>{visibleSkills.map(renderSkill)}</>
      )}
    </div>
  );
};

export default memo(SkillsList);
