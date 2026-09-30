import { memo } from "react";
import { Search, SlidersHorizontal } from "lucide-react";
import type { TFunction } from "i18next";
import { Button } from "@/components/ui/button";
import { Chip } from "@/components/ui/badge";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Field, FieldLabel, Input } from "@/components/ui/field";
import { Segmented, SegmentedItem } from "@/components/ui/segmented";
import type { ListLens, ViewMode } from "../../hooks/useListLens";
import type { SkillFilter, SortMode } from "../../lib/skillPresentation";

type FilterBarProps = {
  /** My Skills' list lens: query, filter chips, sort, Grouped/Flat and view mode. */
  lens: ListLens;
  t: TFunction;
};

const FILTERS: readonly SkillFilter[] = ["all", "issues", "edited", "disabled"];

const SORT_LABEL_KEY: Record<SortMode, string> = {
  name: "sortName",
  updated: "sortUpdated",
  added: "sortAdded",
};

const VIEW_LABEL_KEY: Record<ViewMode, string> = {
  list: "viewList",
  "auto-grid": "viewAutoGrid",
  "dense-grid": "viewDenseGrid",
};

/**
 * The My Skills filter bar, all through the list lens: search (`/` focuses
 * it — My Skills' keymap, `useSkillListKeymap`, finds it by
 * `data-skill-search`), the chips All · Issues · Edited · Disabled, Grouped / Flat, and one
 * Sort and view menu. Policy (auto-sync) and library-wide verbs live
 * elsewhere (Settings; the bulk rail).
 */
const FilterBar = ({ lens, t }: FilterBarProps) => {
  const { query, setQuery, filter, setFilter, filterCounts } = lens;

  return (
    <div className="flex min-h-10 shrink-0 flex-wrap items-center gap-x-4 gap-y-2 border-b border-border px-4 py-2">
      <Field className="flex-row items-center gap-2">
        <FieldLabel>
          <Search className="size-4 text-muted-foreground" aria-hidden="true" />
          <span className="sr-only">{t("library.searchLabel")}</span>
        </FieldLabel>
        <Input
          data-skill-search=""
          type="search"
          size="sm"
          className="w-52"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Escape" && query) {
              event.preventDefault();
              setQuery("");
            }
          }}
          placeholder={t("library.searchPlaceholder")}
          aria-keyshortcuts="/"
        />
      </Field>

      <div role="group" aria-label={t("library.filterGroup")} className="flex items-center gap-1.5">
        {FILTERS.map((key) => (
          <Chip key={key} pressed={filter === key} onPressedChange={() => setFilter(key)}>
            {t(`library.filters.${key}`)}{" "}
            <span className="text-faint tabular-nums">{filterCounts[key] ?? 0}</span>
          </Chip>
        ))}
      </div>

      <div className="ml-auto flex items-center gap-2">
        <Segmented
          aria-label={t("library.grouping")}
          value={lens.groupByRepo ? "grouped" : "flat"}
          onValueChange={(value) => lens.setGroupByRepo(value === "grouped")}
        >
          <SegmentedItem value="grouped">{t("library.grouped")}</SegmentedItem>
          <SegmentedItem value="flat">{t("library.flat")}</SegmentedItem>
        </Segmented>

        <DropdownMenu>
          <DropdownMenuTrigger
            render={
              <Button
                variant="ghost"
                aria-label={`${t("library.display")}: ${t(SORT_LABEL_KEY[lens.sort])}`}
              />
            }
          >
            <SlidersHorizontal />
            {t(SORT_LABEL_KEY[lens.sort])}
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="min-w-44">
            <DropdownMenuGroup>
              <DropdownMenuLabel>{t("library.sortBy")}</DropdownMenuLabel>
              <DropdownMenuRadioGroup value={lens.sort} onValueChange={(value) => lens.setSort(value as SortMode)}>
                {(Object.keys(SORT_LABEL_KEY) as SortMode[]).map((mode) => (
                  <DropdownMenuRadioItem key={mode} value={mode}>
                    {t(SORT_LABEL_KEY[mode])}
                  </DropdownMenuRadioItem>
                ))}
              </DropdownMenuRadioGroup>
            </DropdownMenuGroup>
            <DropdownMenuSeparator />
            <DropdownMenuGroup>
              <DropdownMenuLabel>{t("library.view")}</DropdownMenuLabel>
              <DropdownMenuRadioGroup
                value={lens.viewMode}
                onValueChange={(value) => lens.setViewMode(value as ViewMode)}
              >
                {(Object.keys(VIEW_LABEL_KEY) as ViewMode[]).map((mode) => (
                  <DropdownMenuRadioItem key={mode} value={mode}>
                    {t(VIEW_LABEL_KEY[mode])}
                  </DropdownMenuRadioItem>
                ))}
              </DropdownMenuRadioGroup>
            </DropdownMenuGroup>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  );
};

export default memo(FilterBar);
