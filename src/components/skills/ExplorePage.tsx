import { memo, useMemo } from "react";
import { Download, Eye, EyeOff, Plus, Search, Star } from "lucide-react";
import type { TFunction } from "i18next";
import type { FeaturedSkillDto, ManagedSkill, OnlineSkillDto } from "./types";

type ExplorePageProps = {
  featuredSkills: FeaturedSkillDto[];
  featuredLoading: boolean;
  exploreFilter: string;
  searchResults: OnlineSkillDto[];
  searchLoading: boolean;
  managedSkills: ManagedSkill[];
  loading: boolean;
  hiddenSkills: Set<string>;
  showHidden: boolean;
  onShowHiddenChange: (value: boolean) => void;
  onHideSkill: (sourceUrl: string) => void;
  onUnhideSkill: (sourceUrl: string) => void;
  onExploreFilterChange: (value: string) => void;
  onInstallSkill: (sourceUrl: string, skillName?: string) => void;
  onViewSkill: (sourceUrl: string, skillName: string, summary?: string) => void;
  onOpenManualAdd: () => void;
  t: TFunction;
};

function formatCount(n: number): string {
  if (n >= 1000000) return `${(n / 1000000).toFixed(1)}M`;
  if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
  return String(n);
}

const ExplorePage = ({
  featuredSkills,
  featuredLoading,
  exploreFilter,
  searchResults,
  searchLoading,
  managedSkills,
  loading,
  hiddenSkills,
  showHidden,
  onShowHiddenChange,
  onHideSkill,
  onUnhideSkill,
  onExploreFilterChange,
  onInstallSkill,
  onViewSkill,
  onOpenManualAdd,
  t,
}: ExplorePageProps) => {
  const filteredSkills = useMemo(() => {
    if (!exploreFilter.trim()) return featuredSkills;
    const lower = exploreFilter.toLowerCase();
    return featuredSkills.filter(
      (s) =>
        s.name.toLowerCase().includes(lower) ||
        s.summary.toLowerCase().includes(lower),
    );
  }, [featuredSkills, exploreFilter]);

  const deduplicatedResults = useMemo(() => {
    const featuredNames = new Set(
      filteredSkills.map((s) => s.name.toLowerCase()),
    );
    return searchResults.filter(
      (s) => !featuredNames.has(s.name.toLowerCase()),
    );
  }, [searchResults, filteredSkills]);

  const visibleFeatured = useMemo(() => {
    if (showHidden) return filteredSkills;
    return filteredSkills.filter((s) => !hiddenSkills.has(s.source_url));
  }, [filteredSkills, hiddenSkills, showHidden]);

  const visibleSearchResults = useMemo(() => {
    if (showHidden) return deduplicatedResults;
    return deduplicatedResults.filter((s) => !hiddenSkills.has(s.source_url));
  }, [deduplicatedResults, hiddenSkills, showHidden]);

  const isSearchActive = exploreFilter.trim().length >= 2;

  // Check if a skill is already installed by matching name + source (case-insensitive)
  const installedSkillKeys = useMemo(() => {
    const keys = new Set<string>();
    for (const skill of managedSkills) {
      const source = (skill.source_ref ?? "")
        .replace("https://github.com/", "")
        .replace(/\.git$/, "")
        .split("/tree/")[0]
        .toLowerCase();
      keys.add(`${skill.name.toLowerCase()}|${source}`);
    }
    return keys;
  }, [managedSkills]);

  const isInstalled = (skillName: string, source: string) => {
    const normalizedSource = source
      .replace("https://github.com/", "")
      .replace(/\.git$/, "")
      .split("/tree/")[0]
      .toLowerCase();
    return installedSkillKeys.has(
      `${skillName.toLowerCase()}|${normalizedSource}`,
    );
  };

  return (
    <div className="explore-page">
      <div className="explore-hero">
        <div className="explore-search-row">
          <div className="explore-search-wrap">
            <Search size={16} className="explore-search-icon" />
            <input
              className="explore-search-input"
              placeholder={t("exploreFilterPlaceholder")}
              value={exploreFilter}
              onChange={(e) => onExploreFilterChange(e.target.value)}
            />
          </div>
          <button
            className="btn btn-secondary explore-manual-btn"
            type="button"
            onClick={onOpenManualAdd}
            disabled={loading}
          >
            <Plus size={15} />
            {t("manualAdd")}
          </button>
        </div>
        <label className="explore-show-hidden">
          <input
            type="checkbox"
            checked={showHidden}
            onChange={(e) => onShowHiddenChange(e.target.checked)}
          />
          {t("exploreShowHidden")}
          {hiddenSkills.size > 0 && (
            <span className="explore-hidden-count">({hiddenSkills.size})</span>
          )}
        </label>
        <div className="explore-source-label">{t("exploreSourceHint")}</div>
      </div>

      <div className="explore-scroll">
        {/* Featured section */}
        {featuredLoading ? (
          <div className="explore-loading">{t("exploreLoading")}</div>
        ) : (
          <>
            {isSearchActive && visibleFeatured.length > 0 && (
              <div className="explore-section-title">
                {t("exploreFeaturedTitle")}
              </div>
            )}
            {visibleFeatured.length > 0 ? (
              <div className="explore-grid">
                {visibleFeatured.map((skill) => {
                  const installed = isInstalled(skill.name, skill.source_url);
                  return (
                    <div key={skill.slug} className="explore-card">
                      <div className="explore-card-top">
                        <div className="explore-card-info">
                          <div className="explore-card-name">{skill.name}</div>
                          <div className="explore-card-author">
                            {
                              skill.source_url
                                .replace("https://github.com/", "")
                                .split("/tree/")[0]
                            }
                          </div>
                        </div>
                        {installed ? (
                          <span className="explore-btn-installed">
                            {t("status.installed")}
                          </span>
                        ) : (
                          <button
                            className="explore-btn-install"
                            type="button"
                            disabled={loading}
                            onClick={() => onInstallSkill(skill.source_url)}
                          >
                            {t("install")}
                          </button>
                        )}
                      </div>
                      <div className="explore-card-desc">{skill.summary}</div>
                      <div className="explore-card-bottom">
                        <div className="explore-card-stats">
                          {skill.downloads > 0 && (
                            <span className="explore-stat">
                              <Download size={12} />
                              {formatCount(skill.downloads)}
                            </span>
                          )}
                          {skill.stars > 0 && (
                            <span className="explore-stat">
                              <Star size={12} />
                              {formatCount(skill.stars)}
                            </span>
                          )}
                        </div>
                        <button
                          className="explore-btn-view"
                          type="button"
                          disabled={loading}
                          onClick={() =>
                            onViewSkill(
                              skill.source_url,
                              skill.name,
                              skill.summary,
                            )
                          }
                        >
                          <Eye size={12} />
                          {t("exploreView")}
                        </button>
                        <button
                          className="explore-btn-hide"
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            if (hiddenSkills.has(skill.source_url)) {
                              onUnhideSkill(skill.source_url);
                            } else {
                              onHideSkill(skill.source_url);
                            }
                          }}
                        >
                          {hiddenSkills.has(skill.source_url) ? (
                            <Eye size={12} />
                          ) : (
                            <EyeOff size={12} />
                          )}
                          {hiddenSkills.has(skill.source_url)
                            ? t("exploreUnhide")
                            : t("exploreHide")}
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            ) : !isSearchActive ? (
              <div className="explore-empty">{t("exploreEmpty")}</div>
            ) : null}

            {/* Online search results */}
            {isSearchActive && (
              <>
                <div className="explore-section-title">
                  {t("exploreOnlineTitle")}
                </div>
                {searchLoading ? (
                  <div className="explore-loading">{t("searchLoading")}</div>
                ) : visibleSearchResults.length > 0 ? (
                  <div className="explore-grid">
                    {visibleSearchResults.map((skill) => {
                      const installed = isInstalled(
                        skill.name,
                        skill.source_url,
                      );
                      return (
                        <div key={skill.source} className="explore-card">
                          <div className="explore-card-top">
                            <div className="explore-card-info">
                              <div className="explore-card-name">
                                {skill.name}
                              </div>
                              <div className="explore-card-author">
                                {skill.source}
                              </div>
                            </div>
                            {installed ? (
                              <span className="explore-btn-installed">
                                {t("status.installed")}
                              </span>
                            ) : (
                              <button
                                className="explore-btn-install"
                                type="button"
                                disabled={loading}
                                onClick={() =>
                                  onInstallSkill(skill.source_url, skill.name)
                                }
                              >
                                {t("install")}
                              </button>
                            )}
                          </div>
                          <div className="explore-card-bottom">
                            <div className="explore-card-stats">
                              <span className="explore-stat">
                                {formatCount(skill.installs)} installs
                              </span>
                            </div>
                            <button
                              className="explore-btn-view"
                              type="button"
                              disabled={loading}
                              onClick={() =>
                                onViewSkill(skill.source_url, skill.name)
                              }
                            >
                              <Eye size={12} />
                              {t("exploreView")}
                            </button>
                            <button
                              className="explore-btn-hide"
                              type="button"
                              onClick={(e) => {
                                e.stopPropagation();
                                if (hiddenSkills.has(skill.source_url)) {
                                  onUnhideSkill(skill.source_url);
                                } else {
                                  onHideSkill(skill.source_url);
                                }
                              }}
                            >
                              {hiddenSkills.has(skill.source_url) ? (
                                <Eye size={12} />
                              ) : (
                                <EyeOff size={12} />
                              )}
                              {hiddenSkills.has(skill.source_url)
                                ? t("exploreUnhide")
                                : t("exploreHide")}
                            </button>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                ) : (
                  <div className="explore-empty">{t("searchEmpty")}</div>
                )}
              </>
            )}
          </>
        )}
      </div>
    </div>
  );
};

export default memo(ExplorePage);
