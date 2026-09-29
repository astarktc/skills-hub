import { useCallback, useRef, useState } from "react";
import type {
  FeaturedSkillDto,
  OnlineSkillDto,
} from "../components/skills/types";
import type { ExplorePreview } from "./useSurfaceState";
import { invokeTauri } from "../lib/tauri";
import type { StatusReporter, TranslateFn } from "./useStatusReporter";
import { showHiddenPreference } from "../lib/preferences";
import { usePersistedPreference } from "./usePersistedPreference";

export type ExploreStateDeps = {
  t: TranslateFn;
  reporter: Pick<
    StatusReporter,
    "runAction" | "notify" | "notifyError"
  >;
  /** Focus the cloned preview (the surface module shows it in the detail). */
  onOpenPreview: (preview: ExplorePreview) => void;
  /**
   * Whether Explore is still where the operator is. A preview that waited
   * for a running action (or cloned slowly) opens only if it is: an intent
   * the operator navigated away from never opens a surface.
   */
  isActive?: () => boolean;
};

/**
 * Explore world: featured skills, the debounced online search, the hidden
 * skills list, and opening an online skill as a local preview.
 */
export function useExploreState({
  t,
  reporter,
  onOpenPreview,
  isActive,
}: ExploreStateDeps) {
  const { runAction, notify, notifyError } = reporter;
  const [featuredSkills, setFeaturedSkills] = useState<FeaturedSkillDto[]>([]);
  const [featuredLoading, setFeaturedLoading] = useState(false);
  const [exploreFilter, setExploreFilter] = useState("");
  const [searchResults, setSearchResults] = useState<OnlineSkillDto[]>([]);
  const [searchLoading, setSearchLoading] = useState(false);
  const searchTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // The latest preview request: an earlier one still waiting is superseded.
  const previewGenerationRef = useRef(0);
  const [hiddenSkills, setHiddenSkills] = useState<Set<string>>(new Set());
  const [showHidden, setShowHidden] =
    usePersistedPreference(showHiddenPreference);

  const loadFeaturedSkills = useCallback(async () => {
    if (featuredSkills.length > 0) return;
    setFeaturedLoading(true);
    try {
      const result = await invokeTauri("getFeaturedSkills");
      setFeaturedSkills(result);
    } catch {
      // silent — explore tab will show empty state
    } finally {
      setFeaturedLoading(false);
    }
  }, [featuredSkills.length]);

  const loadHiddenSkills = useCallback(async () => {
    try {
      const urls = await invokeTauri("getHiddenExploreSkills");
      setHiddenSkills(new Set(urls));
    } catch {
      // silent
    }
  }, []);

  const handleHideSkill = useCallback(
    async (sourceUrl: string) => {
      try {
        await invokeTauri("hideExploreSkill", sourceUrl);
        setHiddenSkills((prev) => new Set([...prev, sourceUrl]));
      } catch (err) {
        notifyError(err);
      }
    },
    [notifyError],
  );

  const handleUnhideSkill = useCallback(
    async (sourceUrl: string) => {
      try {
        await invokeTauri("unhideExploreSkill", sourceUrl);
        setHiddenSkills((prev) => {
          const next = new Set(prev);
          next.delete(sourceUrl);
          return next;
        });
      } catch (err) {
        notifyError(err);
      }
    },
    [notifyError],
  );

  const handleExploreFilterChange = useCallback(
    (value: string) => {
      setExploreFilter(value);
      if (searchTimerRef.current) {
        clearTimeout(searchTimerRef.current);
        searchTimerRef.current = null;
      }
      if (value.trim().length < 2) {
        setSearchResults([]);
        setSearchLoading(false);
        return;
      }
      setSearchLoading(true);
      searchTimerRef.current = setTimeout(async () => {
        try {
          const results = await invokeTauri(
            "searchSkillsOnline",
            value.trim(),
            50,
          );
          setSearchResults(results);
        } catch {
          notify("error", t("searchError"));
          setSearchResults([]);
        } finally {
          setSearchLoading(false);
        }
      }, 500);
    },
    [notify, t],
  );

  const handleOpenExploreDetail = useCallback(
    async (sourceUrl: string, skillName: string, summary?: string) => {
      // A preview changes nothing (a read: it waits its turn, never refused)
      // and its clone honours the cancel token. It is still wanted while it
      // is the latest request and Explore is still showing — checked when
      // its turn comes and again before it opens.
      const generation = ++previewGenerationRef.current;
      const stillWanted = () =>
        previewGenerationRef.current === generation && (isActive?.() ?? true);
      await runAction({ read: true, cancel: "cloneExploreSkill", stillWanted }, async () => {
        const cachePath = await invokeTauri(
          "cloneExploreSkill",
          sourceUrl,
          skillName,
        );
        if (!stillWanted()) return;
        // A preview is not a Managed skill: it carries only what the
        // detail shows (no id, no targets, nothing to act on but Install).
        onOpenPreview({
          sourceUrl,
          name: skillName,
          summary: summary ?? null,
          cachePath,
          fetchedAt: Date.now(),
        });
      });
    },
    [isActive, onOpenPreview, runAction],
  );

  return {
    featuredSkills,
    featuredLoading,
    exploreFilter,
    searchResults,
    searchLoading,
    hiddenSkills,
    showHidden,
    setShowHidden,
    loadFeaturedSkills,
    loadHiddenSkills,
    handleHideSkill,
    handleUnhideSkill,
    handleExploreFilterChange,
    handleOpenExploreDetail,
  };
}
