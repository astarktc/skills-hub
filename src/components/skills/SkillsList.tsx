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
  viewMode: "list" | "auto-grid" | "dense-grid";
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
  getGithubInfo,
  getSkillSourceLabel,
  formatRelative,
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
    const map = new Map<string, ManagedSkill[]>();
    for (const skill of visibleSkills) {
      const ref = skill.source_ref ?? "";
      const isGitUrl =
        ref.startsWith("git+") ||
        ref.startsWith("https://") ||
        ref.startsWith("http://") ||
        ref.includes("github.com");
      const ghInfo = isGitUrl && ref ? getGithubInfo(ref) : null;
      const key = isGitUrl ? (ghInfo?.label ?? ref) : "__local__";
      const list = map.get(key);
      if (list) {
        list.push(skill);
      } else {
        map.set(key, [skill]);
      }
    }
    const entries = [...map.keys()].map((key) => {
      const isRepo = key !== "__local__" && key.includes("/");
      return {
        key,
        label: key === "__local__" ? t("localGroup") : key || t("ungrouped"),
        href: isRepo ? `https://github.com/${key}` : null,
        skills: map.get(key)!,
      };
    });
    entries.sort((a, b) => {
      if (a.key === "__local__" && b.key !== "__local__") return 1;
      if (b.key === "__local__" && a.key !== "__local__") return -1;
      return a.label.toLowerCase().localeCompare(b.label.toLowerCase());
    });
    return entries;
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
