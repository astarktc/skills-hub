import { memo } from "react";
import { ArrowUpDown, LayoutList, RefreshCw, Search } from "lucide-react";
import type { TFunction } from "i18next";

type FilterBarProps = {
  sortBy: "name" | "updated" | "added";
  searchQuery: string;
  loading: boolean;
  onSortChange: (value: "name" | "updated" | "added") => void;
  onSearchChange: (value: string) => void;
  onRefresh: () => void;
  autoSyncEnabled: boolean;
  onAutoSyncChange: (enabled: boolean) => void;
  onUnsyncAll: () => void;
  groupByRepo: boolean;
  onGroupByRepoChange: (value: boolean) => void;
  viewMode: "list" | "auto-grid" | "dense-grid";
  onViewModeChange: (value: "list" | "auto-grid" | "dense-grid") => void;
  t: TFunction;
};

const FilterBar = ({
  sortBy,
  searchQuery,
  loading,
  onSortChange,
  onSearchChange,
  onRefresh,
  autoSyncEnabled,
  onAutoSyncChange,
  onUnsyncAll,
  groupByRepo,
  onGroupByRepoChange,
  viewMode,
  onViewModeChange,
  t,
}: FilterBarProps) => {
  return (
    <div className="filter-bar">
      <div className="filter-bar-left">
        <button className="btn btn-secondary sort-btn" type="button">
          <span className="sort-label">{t("filterSort")}:</span>
          {sortBy === "name"
            ? t("sortName")
            : sortBy === "added"
              ? t("sortAdded")
              : t("sortUpdated")}
          <ArrowUpDown size={12} />
          <select
            aria-label={t("filterSort")}
            value={sortBy}
            onChange={(event) =>
              onSortChange(event.target.value as "name" | "updated" | "added")
            }
          >
            <option value="name">{t("sortName")}</option>
            <option value="updated">{t("sortUpdated")}</option>
            <option value="added">{t("sortAdded")}</option>
          </select>
        </button>
        <label className="group-by-repo-toggle" title={t("groupByRepo")}>
          <input
            type="checkbox"
            checked={groupByRepo}
            onChange={(e) => onGroupByRepoChange(e.target.checked)}
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
            onChange={(event) =>
              onViewModeChange(
                event.target.value as "list" | "auto-grid" | "dense-grid",
              )
            }
          >
            <option value="list">{t("viewList")}</option>
            <option value="auto-grid">{t("viewAutoGrid")}</option>
            <option value="dense-grid">{t("viewDenseGrid")}</option>
          </select>
        </button>
      </div>
      <div className="filter-bar-right">
        <label className="auto-sync-toggle" title={t("autoSyncToggle")}>
          <input
            type="checkbox"
            checked={autoSyncEnabled}
            onChange={(e) => onAutoSyncChange(e.target.checked)}
            disabled={loading}
          />
          <span className="auto-sync-label">{t("autoSyncToggle")}</span>
        </label>
        <button
          className="btn btn-secondary unsync-all-btn"
          type="button"
          onClick={onUnsyncAll}
          disabled={loading}
          title={t("unsyncAll")}
        >
          {t("unsyncAll")}
        </button>
        <div className="search-container">
          <Search size={16} className="search-icon-abs" />
          <input
            className="search-input"
            value={searchQuery}
            onChange={(event) => onSearchChange(event.target.value)}
            placeholder={t("searchPlaceholder")}
          />
        </div>
        <button
          className="btn btn-secondary"
          type="button"
          onClick={onRefresh}
          disabled={loading}
        >
          <RefreshCw size={14} />
          {t("refresh")}
        </button>
      </div>
    </div>
  );
};

export default memo(FilterBar);
