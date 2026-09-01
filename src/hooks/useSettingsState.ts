import { useCallback, useEffect, useState } from "react";
import type {
  AppSettings,
  SettingUpdate,
  SettingsBounds,
} from "../components/skills/types";
import { invokeTauri, isTauri } from "../lib/tauri";
import type { StatusReporter, TranslateFn } from "./useStatusReporter";

const themeStorageKey = "skills-theme";
const ZOOM_PRESETS = [0.75, 1, 1.1, 1.25, 1.5, 1.75, 2];

export type SettingsStateDeps = {
  t: TranslateFn;
  reporter: Pick<
    StatusReporter,
    "setError" | "setSuccessToastMessage" | "formatError"
  >;
  /** Called after the central repo moves so the skill list reloads. */
  onManagedSkillsChanged: () => Promise<void>;
};

/** Clamp into an inclusive `{min,max}` bound; passes through when unknown. */
function clampTo(
  value: number,
  bound: { min: number; max: number } | undefined,
): number {
  if (!bound) return value;
  return Math.max(bound.min, Math.min(value, bound.max));
}

/**
 * Settings world: theme, zoom (including the Cmd/Ctrl +/- hotkeys), central
 * repo storage path, git cache knobs, and the GitHub token. Loads the whole
 * backend settings snapshot once on mount (`get_settings`) and writes each
 * change through `update_setting`, adopting the effective values (and
 * bounds) the backend returns rather than the requested ones.
 */
export function useSettingsState({
  t,
  reporter,
  onManagedSkillsChanged,
}: SettingsStateDeps) {
  const { setError, setSuccessToastMessage, formatError } = reporter;
  const [themePreference, setThemePreference] = useState<
    "system" | "light" | "dark"
  >(() => {
    if (typeof window === "undefined") return "system";
    const stored = window.localStorage.getItem(themeStorageKey);
    if (stored === "light" || stored === "dark" || stored === "system")
      return stored;
    return "system";
  });
  const [systemTheme, setSystemTheme] = useState<"light" | "dark">("light");
  const [zoomLevel, setZoomLevel] = useState(1.0);
  const [storagePath, setStoragePath] = useState<string>(t("notAvailable"));
  const [gitCacheCleanupDays, setGitCacheCleanupDays] = useState<number>(30);
  const [gitCacheTtlSecs, setGitCacheTtlSecs] = useState<number>(60);
  const [githubToken, setGithubToken] = useState<string>("");
  // Clamp bounds come from the backend snapshot; null until loaded.
  const [bounds, setBounds] = useState<SettingsBounds | null>(null);

  const adoptSettings = useCallback((next: AppSettings) => {
    setStoragePath(next.central_repo_path);
    setGitCacheCleanupDays(next.git_cache_cleanup_days);
    setGitCacheTtlSecs(next.git_cache_ttl_secs);
    setGithubToken(next.github_token);
    setZoomLevel(next.ui_zoom_level);
    setBounds(next.bounds);
  }, []);

  const updateSetting = useCallback(
    (update: SettingUpdate) =>
      invokeTauri<AppSettings>("update_setting", { update }),
    [],
  );

  useEffect(() => {
    if (typeof window === "undefined") return;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const update = () => {
      setSystemTheme(media.matches ? "dark" : "light");
    };
    update();
    if (media.addEventListener) {
      media.addEventListener("change", update);
    } else {
      media.addListener(update);
    }
    return () => {
      if (media.removeEventListener) {
        media.removeEventListener("change", update);
      } else {
        media.removeListener(update);
      }
    };
  }, []);

  useEffect(() => {
    if (typeof document === "undefined") return;
    const resolvedTheme =
      themePreference === "system" ? systemTheme : themePreference;
    document.documentElement.dataset.theme = resolvedTheme;
    document.documentElement.style.colorScheme = resolvedTheme;
    try {
      window.localStorage.setItem(themeStorageKey, themePreference);
    } catch {
      // ignore storage failures
    }
  }, [systemTheme, themePreference]);

  useEffect(() => {
    if (!isTauri) return;
    invokeTauri<AppSettings>("get_settings")
      .then(adoptSettings)
      .catch((err) => {
        setError(formatError(err));
      });
  }, [adoptSettings, formatError, setError]);

  useEffect(() => {
    if (!isTauri) return;
    const handler = (e: KeyboardEvent) => {
      const zoomIn =
        (e.ctrlKey || e.metaKey) && (e.key === "=" || e.key === "+");
      const zoomOut = (e.ctrlKey || e.metaKey) && e.key === "-";
      if (!zoomIn && !zoomOut) return;
      e.preventDefault();
      setZoomLevel((prev) => {
        const idx = ZOOM_PRESETS.indexOf(prev);
        const curIdx =
          idx >= 0 ? idx : ZOOM_PRESETS.findIndex((p) => p >= prev);
        const nextIdx = zoomIn
          ? Math.min((curIdx >= 0 ? curIdx : 0) + 1, ZOOM_PRESETS.length - 1)
          : Math.max((curIdx >= 0 ? curIdx : ZOOM_PRESETS.length - 1) - 1, 0);
        const next = ZOOM_PRESETS[nextIdx];
        import("@tauri-apps/api/webview").then(({ getCurrentWebview }) => {
          getCurrentWebview()
            .setZoom(next)
            .catch(() => {});
        });
        updateSetting({ key: "ui_zoom_level", value: next }).catch(() => {});
        return next;
      });
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [updateSetting]);

  const handlePickStoragePath = useCallback(async () => {
    try {
      if (!isTauri) {
        throw new Error(t("errors.notTauri"));
      }
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        directory: true,
        multiple: false,
        title: t("selectStoragePath"),
      });
      if (!selected || Array.isArray(selected)) return;
      adoptSettings(
        await updateSetting({ key: "central_repo_path", value: selected }),
      );
      await onManagedSkillsChanged();
    } catch (err) {
      setError(formatError(err));
    }
  }, [
    adoptSettings,
    formatError,
    onManagedSkillsChanged,
    setError,
    t,
    updateSetting,
  ]);

  const handleGitCacheCleanupDaysChange = useCallback(
    async (nextDays: number) => {
      const normalized = clampTo(nextDays, bounds?.git_cache_cleanup_days);
      setGitCacheCleanupDays(normalized);
      if (!isTauri) return;
      try {
        adoptSettings(
          await updateSetting({
            key: "git_cache_cleanup_days",
            value: normalized,
          }),
        );
      } catch (err) {
        setError(formatError(err));
      }
    },
    [adoptSettings, bounds, formatError, setError, updateSetting],
  );

  const handleGitCacheTtlSecsChange = useCallback(
    async (nextSecs: number) => {
      const normalized = clampTo(nextSecs, bounds?.git_cache_ttl_secs);
      setGitCacheTtlSecs(normalized);
      if (!isTauri) return;
      try {
        adoptSettings(
          await updateSetting({ key: "git_cache_ttl_secs", value: normalized }),
        );
      } catch (err) {
        setError(formatError(err));
      }
    },
    [adoptSettings, bounds, formatError, setError, updateSetting],
  );

  const handleGithubTokenChange = useCallback(
    async (nextToken: string) => {
      setGithubToken(nextToken);
      if (!isTauri) return;
      try {
        adoptSettings(
          await updateSetting({ key: "github_token", value: nextToken }),
        );
      } catch (err) {
        setError(formatError(err));
      }
    },
    [adoptSettings, formatError, setError, updateSetting],
  );

  const handleClearGitCacheNow = useCallback(async () => {
    if (!isTauri) {
      setError(t("errors.notTauri"));
      return;
    }
    try {
      const removed = await invokeTauri<number>("clear_git_cache_now");
      setSuccessToastMessage(t("status.gitCacheCleared", { count: removed }));
    } catch (err) {
      setError(formatError(err));
    }
  }, [formatError, setError, setSuccessToastMessage, t]);

  const handleThemeChange = useCallback(
    (nextTheme: "system" | "light" | "dark") => {
      setThemePreference(nextTheme);
    },
    [],
  );

  const handleZoomLevelChange = useCallback(
    async (nextLevel: number) => {
      const normalized = clampTo(nextLevel, bounds?.ui_zoom_level);
      setZoomLevel(normalized);
      if (!isTauri) return;
      try {
        const { getCurrentWebview } = await import("@tauri-apps/api/webview");
        await getCurrentWebview().setZoom(normalized);
        await updateSetting({ key: "ui_zoom_level", value: normalized });
      } catch {
        /* ignore -- zoom is best-effort */
      }
    },
    [bounds, updateSetting],
  );

  return {
    themePreference,
    zoomLevel,
    storagePath,
    gitCacheCleanupDays,
    gitCacheTtlSecs,
    githubToken,
    bounds,
    handlePickStoragePath,
    handleGitCacheCleanupDaysChange,
    handleGitCacheTtlSecsChange,
    handleGithubTokenChange,
    handleClearGitCacheNow,
    handleThemeChange,
    handleZoomLevelChange,
  };
}
