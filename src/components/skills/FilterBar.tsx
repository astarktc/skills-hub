import { memo } from "react";
import { ArrowUpDown, RefreshCw, Search } from "lucide-react";
import type { TFunction } from "i18next";

type FilterBarProps = {
  sortBy: "updated" | "name";
  searchQuery: string;
  loading: boolean;
  onSortChange: (value: "updated" | "name") => void;
  onSearchChange: (value: string) => void;
  onRefresh: () => void;
  autoSyncEnabled: boolean;
  onAutoSyncChange: (enabled: boolean) => void;
  onUnsyncAll: () => void;
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
  t,
}: FilterBarProps) => {
  return (
    <div className="filter-bar">
      <div className="filter-title">{t("allSkills")}</div>
      <div className="filter-actions">
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
        <button className="btn btn-secondary sort-btn" type="button">
          <span className="sort-label">{t("filterSort")}:</span>
          {sortBy === "updated" ? t("sortUpdated") : t("sortName")}
          <ArrowUpDown size={12} />
          <select
            aria-label={t("filterSort")}
            value={sortBy}
            onChange={(event) =>
              onSortChange(event.target.value as "updated" | "name")
            }
          >
            <option value="updated">{t("sortUpdated")}</option>
            <option value="name">{t("sortName")}</option>
          </select>
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
