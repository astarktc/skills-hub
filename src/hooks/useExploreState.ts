import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import type {
  FeaturedSkillDto,
  ManagedSkill,
  OnlineSkillDto,
} from "../components/skills/types";
import { invokeTauri } from "../lib/tauri";
import type { StatusReporter, TranslateFn } from "./useStatusReporter";

const showHiddenStorageKey = "explore-showHidden";

export type ExploreStateDeps = {
  t: TranslateFn;
  reporter: Pick<StatusReporter, "runAction" | "formatError">;
  /** Navigate to the explore-detail view with the cloned preview skill. */
  onOpenExploreDetail: (skill: ManagedSkill) => void;
};

/**
 * Explore world: featured skills, the debounced online search, the hidden
 * skills list, and opening an online skill as a local preview.
 */
export function useExploreState({
  t,
  reporter,
  onOpenExploreDetail,
}: ExploreStateDeps) {
  const { runAction, formatError } = reporter;
  const [featuredSkills, setFeaturedSkills] = useState<FeaturedSkillDto[]>([]);
  const [featuredLoading, setFeaturedLoading] = useState(false);
  const [exploreFilter, setExploreFilter] = useState("");
  const [searchResults, setSearchResults] = useState<OnlineSkillDto[]>([]);
  const [searchLoading, setSearchLoading] = useState(false);
  const searchTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [hiddenSkills, setHiddenSkills] = useState<Set<string>>(new Set());
  const [showHidden, setShowHidden] = useState(() => {
    try {
      return window.localStorage.getItem(showHiddenStorageKey) === "true";
    } catch {
      return false;
    }
  });

  useEffect(() => {
    if (typeof window === "undefined") return;
    try {
      window.localStorage.setItem(showHiddenStorageKey, String(showHidden));
    } catch {
      // ignore
    }
  }, [showHidden]);

  const loadFeaturedSkills = useCallback(async () => {
    if (featuredSkills.length > 0) return;
    setFeaturedLoading(true);
    try {
      const result = await invokeTauri<FeaturedSkillDto[]>(
        "get_featured_skills",
      );
      setFeaturedSkills(result);
    } catch {
      // silent — explore tab will show empty state
    } finally {
      setFeaturedLoading(false);
    }
  }, [featuredSkills.length]);

  const loadHiddenSkills = useCallback(async () => {
    try {
      const urls = await invokeTauri<string[]>("get_hidden_explore_skills");
      setHiddenSkills(new Set(urls));
    } catch {
      // silent
    }
  }, []);

  const handleHideSkill = useCallback(
    async (sourceUrl: string) => {
      try {
        await invokeTauri("hide_explore_skill", { sourceUrl });
        setHiddenSkills((prev) => new Set([...prev, sourceUrl]));
      } catch (err) {
        const msg = formatError(err);
        if (msg) toast.error(msg);
      }
    },
    [formatError],
  );

  const handleUnhideSkill = useCallback(
    async (sourceUrl: string) => {
      try {
        await invokeTauri("unhide_explore_skill", { sourceUrl });
        setHiddenSkills((prev) => {
          const next = new Set(prev);
          next.delete(sourceUrl);
          return next;
        });
      } catch (err) {
        const msg = formatError(err);
        if (msg) toast.error(msg);
      }
    },
    [formatError],
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
          const results = await invokeTauri<OnlineSkillDto[]>(
            "search_skills_online",
            { query: value.trim(), limit: 50 },
          );
          setSearchResults(results);
        } catch {
          toast.error(t("searchError"));
          setSearchResults([]);
        } finally {
          setSearchLoading(false);
        }
      }, 500);
    },
    [t],
  );

  const handleOpenExploreDetail = useCallback(
    async (sourceUrl: string, skillName: string, summary?: string) => {
      await runAction({}, async () => {
        const cachePath = await invokeTauri<string>("clone_explore_skill", {
          sourceUrl,
          skillName,
        });
        const exploreManagedSkill: ManagedSkill = {
          id: "",
          name: skillName,
          description: summary ?? null,
          source_type: "github",
          source_ref: sourceUrl,
          central_path: cachePath,
          created_at: 0,
          updated_at: Date.now(),
          last_sync_at: null,
          status: "",
          targets: [],
        };
        onOpenExploreDetail(exploreManagedSkill);
      });
    },
    [onOpenExploreDetail, runAction],
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
