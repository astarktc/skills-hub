import { memo, useCallback, useEffect, useId, useLayoutEffect, useMemo, useRef, type KeyboardEvent, type MouseEvent, type ReactNode } from "react";
import { Check, Download, Ellipsis, Plus, Search, Star } from "lucide-react";
import type { TFunction } from "i18next";
import { cn } from "@/lib/utils";
import { Badge, Chip } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Field, FieldLabel, Input } from "@/components/ui/field";
import { useListLens } from "../../hooks/useListLens";
import type { ExplorePreview } from "../../hooks/useSurfaceState";
import { isFocusLost } from "../../lib/actionLifecycle";
import {
  EXPLORE_FILTERS,
  EXPLORE_FILTER_ORDER,
  EXPLORE_MIN_ONLINE_QUERY,
  exploreCandidates,
  exploreHideKey,
  exploreQueryFold,
  exploreSearchMode,
  formatPopularity,
  type ExploreCandidate,
  type ExploreFilter,
} from "../../lib/explorePresentation";
import ExplorePreviewView from "./ExplorePreviewView";
import type { InspectorPlacement } from "./inspector/SkillInspector";
import type { FeaturedSkillDto, ManagedSkill, OnlineSkillDto } from "./types";

type ExplorePageProps = {
  featuredSkills: FeaturedSkillDto[];
  featuredLoading: boolean;
  /** The query: the Explore world's, since it drives the online search. */
  query: string;
  onQueryChange: (value: string) => void;
  searchResults: OnlineSkillDto[];
  searchLoading: boolean;
  managedSkills: ManagedSkill[];
  hiddenSkills: ReadonlySet<string>;
  onHideSkill: (key: string) => Promise<void>;
  onUnhideSkill: (key: string) => Promise<void>;
  /** Clone the candidate and focus its preview (a read; it waits its turn). */
  onPreview: (sourceUrl: string, skillName: string, summary?: string) => void;
  /** The Add intent on the candidate's repository, the skill checked (ticket 16). */
  onInstall: (sourceUrl: string, skillName: string) => void;
  /** An installed candidate is its Managed skill: focus it in the inspector. */
  onOpenInstalled: (skillId: string) => void;
  onOpenAdd: () => void;
  /** The focused candidate's preview (surface state), shown in the inspector slot. */
  preview: ExplorePreview | null;
  /** Beside the list (≥ 1200 px) or a sheet (D23), as the inspector. */
  placement: InspectorPlacement;
  onClosePreview: () => void;
  /** Add's deploy scope (its sticky summary's), shown beside Install (D25). */
  scopeTools: readonly { id: string; label: string }[];
  deploys: boolean;
  t: TFunction;
};

const POPULARITY_ICON = { downloads: Download, stars: Star, installs: Download } as const;

/** A row's name control, found by its candidate's key (focus return, ↑/↓). */
function rowControl(key: string): HTMLElement | null {
  return document.querySelector<HTMLElement>(`[data-explore-row="${CSS.escape(key)}"]`);
}

function rowControls(): HTMLElement[] {
  return [...document.querySelectorAll<HTMLElement>("[data-explore-row]")];
}

/**
 * Explore as tracker rows (D25): search (labelled) and the chips Not in
 * library (default) · Installed · Hidden through one list lens; featured
 * skills and skills.sh results in two source cards, rows that read like My
 * Skills' (name · source · description · popularity · installed mark).
 * A row opens its candidate's preview in the inspector slot — an installed
 * one opens its Managed skill in My Skills instead; Hide is in the row's
 * overflow, Unhide on the Hidden chip's rows.
 */
const ExplorePage = ({
  featuredSkills,
  featuredLoading,
  query,
  onQueryChange,
  searchResults,
  searchLoading,
  managedSkills,
  hiddenSkills,
  onHideSkill,
  onUnhideSkill,
  onPreview,
  onInstall,
  onOpenInstalled,
  onOpenAdd,
  preview,
  placement,
  onClosePreview,
  scopeTools,
  deploys,
  t,
}: ExplorePageProps) => {
  const featuredId = useId();
  const onlineId = useId();
  const mode = exploreSearchMode(query);
  // Online results only for a query long enough to have searched, and not
  // while a newer search runs (its rows would be the previous query's).
  const online = mode === "online" && !searchLoading ? searchResults : null;
  const candidates = useMemo(
    () =>
      exploreCandidates({
        featured: featuredSkills,
        online: online ?? [],
        managed: managedSkills,
        hidden: hiddenSkills,
      }),
    [featuredSkills, hiddenSkills, managedSkills, online],
  );
  const controlledQuery = useMemo(() => ({ value: query, set: onQueryChange }), [onQueryChange, query]);
  const lens = useListLens({
    items: candidates,
    labels: { local: "", ungrouped: "", individual: "" },
    fold: exploreQueryFold,
    filters: EXPLORE_FILTERS,
    defaultFilter: "notInLibrary",
    controlledQuery,
  });
  const filter = lens.filter as ExploreFilter;
  const featuredRows = lens.displayed.filter((candidate) => candidate.origin === "featured");
  const onlineRows = lens.displayed.filter((candidate) => candidate.origin === "online");
  const onlineTotal = candidates.filter((candidate) => candidate.origin === "online").length;

  const previewKey = preview ? exploreHideKey(preview.name, preview.sourceUrl) : null;
  const previewInstalledId = previewKey
    ? (candidates.find((candidate) => candidate.key === previewKey)?.installedId ?? null)
    : null;

  const open = useCallback(
    (candidate: ExploreCandidate) => {
      if (candidate.installedId) onOpenInstalled(candidate.installedId);
      else onPreview(candidate.sourceUrl, candidate.name, candidate.description ?? undefined);
    },
    [onOpenInstalled, onPreview],
  );

  // Hide / Unhide take the row out of the chip it was under: focus goes to
  // the next row on screen, else the previous one, else the search (D21) —
  // in the commit that removes the row, unless the operator moved it.
  const leaving = useRef<{ key: string; neighbours: string[]; hidden: ReadonlySet<string> } | null>(null);
  useLayoutEffect(() => {
    const aside = leaving.current;
    // Settled once the hidden list changed (a failed change leaves it as is).
    if (!aside || aside.hidden === hiddenSkills) return;
    leaving.current = null;
    if (rowControl(aside.key) || !isFocusLost(document.activeElement)) return;
    const next = aside.neighbours.map(rowControl).find((element) => element !== null);
    (next ?? document.querySelector<HTMLElement>("[data-explore-search]"))?.focus();
  }, [hiddenSkills]);
  const moveAside = useCallback(
    (candidate: ExploreCandidate, change: (key: string) => Promise<void>) => {
      const order = lens.displayed.map((row) => row.key);
      const at = order.indexOf(candidate.key);
      leaving.current = {
        key: candidate.key,
        neighbours: [...order.slice(at + 1), ...order.slice(0, Math.max(0, at)).reverse()],
        hidden: hiddenSkills,
      };
      return change(candidate.key);
    },
    [hiddenSkills, lens.displayed],
  );

  // Until a Hide lands the row stays: its closing menu hands focus to the
  // row's name, not the trigger (which goes with the row).
  const hiding = useRef<string | null>(null);

  // Closing the preview returns focus to its row (the inspector's rule),
  // when nothing else took it meanwhile.
  const lastPreviewKey = useRef(previewKey);
  useEffect(() => {
    const left = lastPreviewKey.current;
    lastPreviewKey.current = previewKey;
    if (!left || previewKey) return;
    if (isFocusLost(document.activeElement)) rowControl(left)?.focus();
  }, [previewKey]);

  // Escape beside the list closes the preview (the sheet's is the Dialog's);
  // a menu or popover spends its own first.
  const onSurfaceKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "Escape" || !preview || placement !== "side" || event.defaultPrevented) return;
    const target = event.target as Element;
    if (target.closest("[role='menu'], [role='dialog'], [role='alertdialog'], [data-slot='popover-content']")) return;
    event.preventDefault();
    onClosePreview();
  };

  // ↓ from the search enters the rows; ↑ / ↓ walk them; ↑ from the first
  // returns to the search.
  const onRowsKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    const target = event.target as HTMLElement;
    if (!target.matches("[data-explore-row]")) return;
    const rows = rowControls();
    const at = rows.indexOf(target);
    event.preventDefault();
    if (event.key === "ArrowDown") rows[at + 1]?.focus();
    else if (at === 0) document.querySelector<HTMLElement>("[data-explore-search]")?.focus();
    else rows[at - 1]?.focus();
  };

  const renderRow = (candidate: ExploreCandidate) => {
    const installed = candidate.installedId !== null;
    const focused = candidate.key === previewKey;
    const popularity = candidate.popularity;
    const PopularityIcon = popularity ? POPULARITY_ICON[popularity.kind] : null;
    const count = popularity ? formatPopularity(popularity.count) : null;
    const popularityText =
      popularity && count ? t(`explore.row.popularity.${popularity.kind}`, { value: count.full }) : undefined;
    const handleRowClick = (event: MouseEvent<HTMLElement>) => {
      if ((event.target as HTMLElement).closest("button, a, [role='menu'], [role='menuitem']")) return;
      open(candidate);
    };
    return (
      <li
        key={candidate.id}
        data-focused={focused || undefined}
        aria-current={focused ? "true" : undefined}
        onClick={handleRowClick}
        className="grid h-row grid-cols-[minmax(0,1fr)_4.5rem_5.5rem_4.5rem] items-center gap-x-3 border-b border-border px-3 last:rounded-b-lg last:border-b-0 hover:bg-card data-focused:bg-raised data-focused:ring-1 data-focused:ring-ring data-focused:ring-inset"
      >
        <span className="flex min-w-0 items-center gap-3">
          <button
            type="button"
            data-explore-row={candidate.key}
            onClick={() => open(candidate)}
            title={candidate.name}
            aria-description={installed ? t("explore.row.openInstalled") : t("explore.row.openPreview")}
            className="h-6 min-w-6 max-w-[22rem] shrink-0 truncate rounded-sm text-left text-sm leading-6 font-medium text-foreground hover:underline"
          >
            {candidate.name}
          </button>
          <span className="max-w-[14rem] shrink truncate font-mono text-xs text-faint" title={candidate.sourceLabel}>
            {candidate.sourceLabel}
          </span>
          {candidate.description ? (
            <span
              className="min-w-0 flex-1 truncate text-sm text-muted-foreground @max-3xl:hidden"
              title={candidate.description}
            >
              {candidate.description}
            </span>
          ) : null}
        </span>
        <span className="flex items-center justify-end gap-1 text-xs text-faint tabular-nums" title={popularityText}>
          {PopularityIcon && count ? (
            <>
              <PopularityIcon className="size-3" aria-hidden="true" />
              <span aria-hidden="true">{count.short}</span>
              <span className="sr-only">{popularityText}</span>
            </>
          ) : null}
        </span>
        <span className="flex items-center justify-end">
          {installed ? (
            <Badge title={t("explore.row.openInstalled")}>
              <Check aria-hidden="true" />
              {t("explore.row.installed")}
            </Badge>
          ) : null}
        </span>
        <span className="flex items-center justify-end">
          {candidate.hidden ? (
            <Button
              variant="ghost"
              size="sm"
              aria-label={t("explore.row.unhideLabel", { name: candidate.name })}
              onClick={() => void moveAside(candidate, onUnhideSkill)}
            >
              {t("explore.row.unhide")}
            </Button>
          ) : (
            <DropdownMenu>
              <DropdownMenuTrigger
                render={
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label={t("explore.row.more", { name: candidate.name })}
                    title={t("explore.row.more", { name: candidate.name })}
                  />
                }
              >
                <Ellipsis />
              </DropdownMenuTrigger>
              <DropdownMenuContent
                align="end"
                className="min-w-48"
                finalFocus={() => (hiding.current === candidate.key ? (rowControl(candidate.key) ?? true) : true)}
              >
                <DropdownMenuItem onClick={() => open(candidate)}>
                  {installed ? t("explore.row.openInLibrary") : t("explore.row.preview")}
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  onClick={() => {
                    hiding.current = candidate.key;
                    void moveAside(candidate, onHideSkill).finally(() => {
                      if (hiding.current === candidate.key) hiding.current = null;
                    });
                  }}
                >
                  {t("explore.row.hide")}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          )}
        </span>
      </li>
    );
  };

  const section = (id: string, title: string, rows: ExploreCandidate[], body: ReactNode) => (
    <section aria-labelledby={id} className="rounded-lg border border-border">
      <div className="flex h-row items-center gap-3 rounded-t-lg border-b border-border bg-card px-3">
        <h2 id={id} className="m-0 text-sm font-semibold text-foreground">
          {title}
        </h2>
        <span className="text-xs text-faint tabular-nums">{t("library.group.count", { count: rows.length })}</span>
      </div>
      {rows.length > 0 ? (
        <ul className="m-0 list-none p-0" onKeyDown={onRowsKeyDown}>
          {rows.map(renderRow)}
        </ul>
      ) : (
        body
      )}
    </section>
  );

  const note = (text: string, status = false) => (
    <p role={status ? "status" : undefined} className="m-0 px-3 py-3 text-sm text-muted-foreground">
      {text}
    </p>
  );

  const emptyFeatured = featuredLoading
    ? note(t("explore.loading"), true)
    : featuredSkills.length === 0
      ? note(t("explore.noFeatured"))
      : (
          <div className="flex flex-wrap items-center gap-3 px-3 py-3">
            <p className="m-0 text-sm text-muted-foreground">
              {query.trim() ? t("explore.noMatches", { query: query.trim() }) : t(`explore.empty.${filter}`)}
            </p>
            {query.trim() ? (
              <Button size="sm" onClick={() => onQueryChange("")}>
                {t("explore.clearSearch")}
              </Button>
            ) : null}
          </div>
        );
  const emptyOnline = searchLoading
    ? note(t("explore.searching"), true)
    : onlineTotal === 0
      ? note(t("explore.noOnline"))
      : note(t(`explore.empty.${filter}`));

  const modeLine =
    mode === "online" ? (
      t("explore.mode.online", { query: query.trim() })
    ) : (
      <>
        {mode === "featured" ? t("explore.mode.featured") : t("explore.mode.short")}
        <span className="text-faint">
          {" · "}
          {t("explore.mode.minQueryHint", { count: EXPLORE_MIN_ONLINE_QUERY })}
        </span>
      </>
    );

  return (
    <div className="flex min-h-0 flex-1 flex-col" onKeyDown={onSurfaceKeyDown}>
      <div className="flex h-12 shrink-0 items-center gap-3 border-b border-border px-4">
        <h1 data-surface-heading="explore" tabIndex={-1} className="m-0 rounded-sm text-lg font-semibold text-foreground">
          {t("navExplore")}
        </h1>
        <Button className="ml-auto" onClick={onOpenAdd}>
          <Plus />
          {t("addSkillButton")}
        </Button>
      </div>

      <div className="flex shrink-0 flex-col gap-1.5 border-b border-border px-4 py-2">
        <div className="flex min-h-8 flex-wrap items-center gap-x-4 gap-y-2">
          <Field className="flex-row items-center gap-2">
            <FieldLabel>
              <Search className="size-4 text-muted-foreground" aria-hidden="true" />
              <span className="sr-only">{t("explore.searchLabel")}</span>
            </FieldLabel>
            <Input
              data-explore-search=""
              type="search"
              size="sm"
              className="w-64"
              value={query}
              onChange={(event) => onQueryChange(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Escape" && query) {
                  event.preventDefault();
                  onQueryChange("");
                } else if (event.key === "ArrowDown") {
                  const first = rowControls()[0];
                  if (first) {
                    event.preventDefault();
                    first.focus();
                  }
                }
              }}
              placeholder={t("explore.searchPlaceholder")}
            />
          </Field>
          <div role="group" aria-label={t("explore.filterGroup")} className="flex items-center gap-1.5">
            {EXPLORE_FILTER_ORDER.map((key) => (
              <Chip key={key} pressed={filter === key} onPressedChange={() => lens.setFilter(key)}>
                {t(`explore.filters.${key}`)}{" "}
                <span className="text-faint tabular-nums">{lens.filterCounts[key] ?? 0}</span>
              </Chip>
            ))}
          </div>
        </div>
        <p className="m-0 text-xs text-muted-foreground">{modeLine}</p>
      </div>

      <div className="flex min-h-0 flex-1">
        <div
          data-explore-list=""
          role="region"
          aria-label={t("explore.list")}
          className={cn("@container flex min-h-0 min-w-0 flex-1 flex-col gap-4 overflow-y-auto p-4")}
        >
          {mode !== "online" || featuredRows.length > 0
            ? section(featuredId, t("explore.sections.featured"), featuredRows, emptyFeatured)
            : null}
          {mode === "online" ? section(onlineId, t("explore.sections.online"), onlineRows, emptyOnline) : null}
        </div>
        {preview ? (
          <ExplorePreviewView
            key={`${preview.cachePath}:${preview.fetchedAt}:${placement}`}
            preview={preview}
            placement={placement}
            installedId={previewInstalledId}
            scopeTools={scopeTools}
            deploys={deploys}
            onClose={onClosePreview}
            onInstall={() => onInstall(preview.sourceUrl, preview.name)}
            onOpenInstalled={onOpenInstalled}
            rowControl={() => (previewKey ? rowControl(previewKey) : null)}
            t={t}
          />
        ) : null}
      </div>
    </div>
  );
};

export default memo(ExplorePage);
