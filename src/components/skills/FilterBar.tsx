import { memo } from "react";
import {
  LayoutList,
  Plus,
  RefreshCw,
  Search,
  Wrench,
} from "lucide-react";
import type { TFunction } from "i18next";
import type { ListLens, ViewMode } from "../../hooks/useListLens";
import LensSortControl from "../shared/LensSortControl";
import type { SkillActions } from "../../hooks/useSkillActions";

type FilterBarProps = {
  /** My Skills' list lens: query, sort, Grouped/Flat and view mode. */
  lens: ListLens;
  /** Why the toolbar's verbs are disabled while an action runs (one at a time, D19), or null. */
  blockedReason: string | null;
  /** Opens the Local / Git add modal — the same flow Explore's Manual button uses. */
  onAddSkill: () => void;
  autoSyncEnabled: boolean;
  onAutoSyncChange: (enabled: boolean) => void;
  /** The skill action door: Refresh and library-wide Disable go through it. */
  actions: Pick<SkillActions, "run">;
  onConfigureTools: () => void;
  t: TFunction;
};

const FilterBar = ({
  lens,
  blockedReason,
  onAddSkill,
  autoSyncEnabled,
  onAutoSyncChange,
  actions,
  onConfigureTools,
  t,
}: FilterBarProps) => {
  const { viewMode, setViewMode } = lens;
  const blocked = blockedReason !== null;
  const titled = (title: string) => blockedReason ?? title;
  return (
    <div className="filter-bar">
      <div className="filter-bar-left">
        <LensSortControl sort={lens.sort} onSortChange={lens.setSort} t={t} />
        <label className="group-by-repo-toggle" title={t("groupByRepo")}>
          <input
            type="checkbox"
            checked={lens.groupByRepo}
            onChange={(e) => lens.setGroupByRepo(e.target.checked)}
          />
          <span className="group-by-repo-label">{t("groupByRepo")}</span>
        </label>
        <button className="btn btn-secondary sort-btn" type="button">
          <span className="sort-label">{t("viewMode")}:</span>
          {viewMode === "list"
            ? t("viewList")
            : viewMode === "auto-grid"
              ? t("viewAutoGrid")
              : t("viewDenseGrid")}
          <LayoutList size={12} />
          <select
            aria-label={t("viewMode")}
            value={viewMode}
            onChange={(event) => setViewMode(event.target.value as ViewMode)}
          >
            <option value="list">{t("viewList")}</option>
            <option value="auto-grid">{t("viewAutoGrid")}</option>
            <option value="dense-grid">{t("viewDenseGrid")}</option>
          </select>
        </button>
      </div>
      <div className="filter-bar-right">
        <button
          className="btn btn-primary"
          type="button"
          onClick={onAddSkill}
          disabled={blocked}
          title={blockedReason ?? undefined}
        >
          <Plus size={14} />
          {t("addSkillButton")}
        </button>
        <label className="auto-sync-toggle" title={t("autoSyncToggle")}>
          <input
            type="checkbox"
            checked={autoSyncEnabled}
            onChange={(e) => onAutoSyncChange(e.target.checked)}
            disabled={blocked}
          />
          <span className="auto-sync-label">{t("autoSyncToggle")}</span>
        </label>
        <button
          className="btn btn-secondary"
          type="button"
          onClick={onConfigureTools}
          disabled={blocked}
          title={titled(t("configureTools"))}
        >
          <Wrench size={14} />
          {t("configureTools")}
        </button>
        <button
          className="btn btn-secondary unsync-all-btn"
          type="button"
          onClick={() => void actions.run("disable", "all")}
          disabled={blocked}
          title={titled(t("unsyncAll"))}
        >
          {t("unsyncAll")}
        </button>
        <div className="search-container">
          <Search size={16} className="search-icon-abs" />
          <input
            className="search-input"
            value={lens.query}
            onChange={(event) => lens.setQuery(event.target.value)}
            placeholder={t("searchPlaceholder")}
          />
        </div>
        <button
          className="btn btn-secondary"
          type="button"
          onClick={() => void actions.run("update", "all")}
          disabled={blocked}
          title={blockedReason ?? undefined}
        >
          <RefreshCw size={14} />
          {t("refresh")}
        </button>
      </div>
    </div>
  );
};

export default memo(FilterBar);
