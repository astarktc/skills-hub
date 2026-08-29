import { useCallback, useEffect, useState } from "react";
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

/**
 * Settings world: theme, zoom (including the Cmd/Ctrl +/- hotkeys), central
 * repo storage path, git cache knobs, and the GitHub token. Loads persisted
 * values on mount and writes changes straight back through commands.
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
    invokeTauri<string>("get_central_repo_path")
      .then((path) => setStoragePath(path))
      .catch((err) => {
        setError(formatError(err));
      });
  }, [formatError, setError]);

  useEffect(() => {
    if (!isTauri) return;
    invokeTauri<number>("get_git_cache_cleanup_days")
      .then((days) => setGitCacheCleanupDays(days))
      .catch((err) => {
        setError(formatError(err));
      });
  }, [formatError, setError]);

  useEffect(() => {
    if (!isTauri) return;
    invokeTauri<number>("get_git_cache_ttl_secs")
      .then((secs) => setGitCacheTtlSecs(secs))
      .catch((err) => {
        setError(formatError(err));
      });
  }, [formatError, setError]);

  useEffect(() => {
    if (!isTauri) return;
    invokeTauri<string>("get_github_token")
      .then((token) => setGithubToken(token))
      .catch(() => {});
  }, []);

  useEffect(() => {
    if (!isTauri) return;
    invokeTauri<number>("get_ui_zoom_level")
      .then((level) => setZoomLevel(level))
      .catch(() => {});
  }, []);

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
        invokeTauri("set_ui_zoom_level", { zoomLevel: next }).catch(() => {});
        return next;
      });
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

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
      const newPath = await invokeTauri<string>("set_central_repo_path", {
        path: selected,
      });
      setStoragePath(newPath);
      await onManagedSkillsChanged();
    } catch (err) {
      setError(formatError(err));
    }
  }, [formatError, onManagedSkillsChanged, setError, t]);

  const handleGitCacheCleanupDaysChange = useCallback(
    async (nextDays: number) => {
      const normalized = Math.max(0, Math.min(nextDays, 3650));
      setGitCacheCleanupDays(normalized);
      if (!isTauri) return;
      try {
        const updated = await invokeTauri<number>(
          "set_git_cache_cleanup_days",
          {
            days: normalized,
          },
        );
        setGitCacheCleanupDays(updated);
      } catch (err) {
        setError(formatError(err));
      }
    },
    [formatError, setError],
  );

  const handleGitCacheTtlSecsChange = useCallback(
    async (nextSecs: number) => {
      const normalized = Math.max(0, Math.min(nextSecs, 3600));
      setGitCacheTtlSecs(normalized);
      if (!isTauri) return;
      try {
        const updated = await invokeTauri<number>("set_git_cache_ttl_secs", {
          secs: normalized,
        });
        setGitCacheTtlSecs(updated);
      } catch (err) {
        setError(formatError(err));
      }
    },
    [formatError, setError],
  );

  const handleGithubTokenChange = useCallback(
    async (nextToken: string) => {
      setGithubToken(nextToken);
      if (!isTauri) return;
      try {
        await invokeTauri("set_github_token", { token: nextToken });
      } catch (err) {
        setError(formatError(err));
      }
    },
    [formatError, setError],
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

  const handleZoomLevelChange = useCallback(async (nextLevel: number) => {
    setZoomLevel(nextLevel);
    if (!isTauri) return;
    try {
      const { getCurrentWebview } = await import("@tauri-apps/api/webview");
      await getCurrentWebview().setZoom(nextLevel);
      await invokeTauri("set_ui_zoom_level", { zoomLevel: nextLevel });
    } catch {
      /* ignore -- zoom is best-effort */
    }
  }, []);

  return {
    themePreference,
    zoomLevel,
    storagePath,
    gitCacheCleanupDays,
    gitCacheTtlSecs,
    githubToken,
    handlePickStoragePath,
    handleGitCacheCleanupDaysChange,
    handleGitCacheTtlSecsChange,
    handleGithubTokenChange,
    handleClearGitCacheNow,
    handleThemeChange,
    handleZoomLevelChange,
  };
}
