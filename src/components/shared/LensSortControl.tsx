import { memo } from "react";
import { ArrowUpDown } from "lucide-react";
import type { TFunction } from "i18next";
import type { SortMode } from "../../lib/skillPresentation";

type LensSortControlProps = {
  sort: SortMode;
  onSortChange: (sort: SortMode) => void;
  /** The matrix toolbar's smaller button. */
  compact?: boolean;
  t: TFunction;
};

const SORT_LABEL_KEY: Record<SortMode, string> = {
  name: "sortName",
  updated: "sortUpdated",
  added: "sortAdded",
};

/**
 * The list lens's sort control — one control for every lens surface (My
 * Skills, the assignment matrix), so the sort vocabulary lives once.
 */
const LensSortControl = ({ sort, onSortChange, compact = false, t }: LensSortControlProps) => (
  <button
    className={compact ? "btn btn-secondary btn-sm sort-btn" : "btn btn-secondary sort-btn"}
    type="button"
  >
    <span className="sort-label">{t("filterSort")}:</span>
    {t(SORT_LABEL_KEY[sort])}
    <ArrowUpDown size={12} />
    <select
      aria-label={t("filterSort")}
      value={sort}
      onChange={(event) => onSortChange(event.target.value as SortMode)}
    >
      <option value="name">{t("sortName")}</option>
      <option value="updated">{t("sortUpdated")}</option>
      <option value="added">{t("sortAdded")}</option>
    </select>
  </button>
);

export default memo(LensSortControl);
