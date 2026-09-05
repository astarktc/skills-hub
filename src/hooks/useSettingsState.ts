import { useCallback, useEffect, useState } from "react";
import type {
  AppSettings,
  SettingUpdate,
  SettingsBounds,
} from "../components/skills/types";
import { invokeTauri, isTauri } from "../lib/tauri";
import type { StatusReporter, TranslateFn } from "./useStatusReporter";
import { themePreference as storedThemePreference } from "../lib/preferences";

const ZOOM_PRESETS = [0.75, 1, 1.1, 1.25, 1.5, 1.75, 2];

/**
 * What the settings panel shows for the one render before `get_settings`
 * resolves. Deliberately a restatement of the backend defaults
 * (`core/settings.rs`) and not a second source of truth: the snapshot is
 * adopted wholesale on mount, so any drift here is visible for a single frame
 * and never reaches a write. Shipping the backend's defaults over the wire
 * instead would add a field the frontend must still have a placeholder for.
 */
const PRE_LOAD_PLACEHOLDERS: {
  zoomLevel: number;
  gitCacheCleanupDays: number;
  gitCacheTtlSecs: number;
} = {
  zoomLevel: 1.0,
  gitCacheCleanupDays: 30,
  gitCacheTtlSecs: 60,
};

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
  >(() => storedThemePreference.read());
  const [systemTheme, setSystemTheme] = useState<"light" | "dark">("light");
  const [zoomLevel, setZoomLevel] = useState(PRE_LOAD_PLACEHOLDERS.zoomLevel);
  const [storagePath, setStoragePath] = useState<string>(t("notAvailable"));
  const [gitCacheCleanupDays, setGitCacheCleanupDays] = useState<number>(
    PRE_LOAD_PLACEHOLDERS.gitCacheCleanupDays,
  );
  const [gitCacheTtlSecs, setGitCacheTtlSecs] = useState<number>(
    PRE_LOAD_PLACEHOLDERS.gitCacheTtlSecs,
  );
  const [githubToken, setGithubToken] = useState<string>("");
  // Clamp bounds come from the backend snapshot; null until loaded.
  const [bounds, setBounds] = useState<SettingsBounds | null>(null);

  /** Adopt a whole snapshot. Only the mount load may do this — see `writeSetting`. */
  const adoptSettings = useCallback((next: AppSettings) => {
    setStoragePath(next.central_repo_path);
    setGitCacheCleanupDays(next.git_cache_cleanup_days);
    setGitCacheTtlSecs(next.git_cache_ttl_secs);
    setGithubToken(next.github_token);
    setZoomLevel(next.ui_zoom_level);
    setBounds(next.bounds);
  }, []);

  /**
   * Write one setting and adopt the backend's echo of *that field only* (plus
   * the bounds, which are constants).
   *
   * Writes are independent and can overlap — the token field writes on every
   * keystroke while a slider is being dragged, and the zoom hotkey fires
   * unprompted. Adopting the whole snapshot here would replay this response's
   * now-stale values for the other fields over newer local edits. The echo is
   * still adopted rather than the requested value, because the backend clamps.
   */
  const writeSetting = useCallback(
    async (update: SettingUpdate) => {
      const next = await invokeTauri("updateSetting", update);
      setBounds(next.bounds);
      switch (update.key) {
        case "central_repo_path":
          setStoragePath(next.central_repo_path);
          break;
        case "git_cache_cleanup_days":
          setGitCacheCleanupDays(next.git_cache_cleanup_days);
          break;
        case "git_cache_ttl_secs":
          setGitCacheTtlSecs(next.git_cache_ttl_secs);
          break;
        case "github_token":
          setGithubToken(next.github_token);
          break;
        case "ui_zoom_level":
          setZoomLevel(next.ui_zoom_level);
          break;
        default:
          // auto_sync_enabled / global_tool_config are owned by other worlds.
          break;
      }
      return next;
    },
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
    storedThemePreference.write(themePreference);
  }, [systemTheme, themePreference]);

  useEffect(() => {
    if (!isTauri) return;
    invokeTauri("getSettings")
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
        writeSetting({ key: "ui_zoom_level", value: next }).catch(() => {});
        return next;
      });
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [writeSetting]);

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
      await writeSetting({ key: "central_repo_path", value: selected });
      await onManagedSkillsChanged();
    } catch (err) {
      setError(formatError(err));
    }
  }, [
    formatError,
    onManagedSkillsChanged,
    setError,
    t,
    writeSetting,
  ]);

  const handleGitCacheCleanupDaysChange = useCallback(
    async (nextDays: number) => {
      const normalized = clampTo(nextDays, bounds?.git_cache_cleanup_days);
      setGitCacheCleanupDays(normalized);
      if (!isTauri) return;
      try {
        await writeSetting({
          key: "git_cache_cleanup_days",
          value: normalized,
        });
      } catch (err) {
        setError(formatError(err));
      }
    },
    [bounds, formatError, setError, writeSetting],
  );

  const handleGitCacheTtlSecsChange = useCallback(
    async (nextSecs: number) => {
      const normalized = clampTo(nextSecs, bounds?.git_cache_ttl_secs);
      setGitCacheTtlSecs(normalized);
      if (!isTauri) return;
      try {
        await writeSetting({ key: "git_cache_ttl_secs", value: normalized });
      } catch (err) {
        setError(formatError(err));
      }
    },
    [bounds, formatError, setError, writeSetting],
  );

  const handleGithubTokenChange = useCallback(
    async (nextToken: string) => {
      setGithubToken(nextToken);
      if (!isTauri) return;
      try {
        await writeSetting({ key: "github_token", value: nextToken });
      } catch (err) {
        setError(formatError(err));
      }
    },
    [formatError, setError, writeSetting],
  );

  const handleClearGitCacheNow = useCallback(async () => {
    if (!isTauri) {
      setError(t("errors.notTauri"));
      return;
    }
    try {
      const removed = await invokeTauri("clearGitCacheNow");
      setSuccessToastMessage(t("status.gitCacheCleared", { count: removed }));
    } catch (err) {
      setError(formatError(err));
    }
  }, [formatError, setError, setSuccessToastMessage, t]);

  /** Reveal the backend log dir: the post-restart record of past failures. */
  const handleOpenLogFolder = useCallback(async () => {
    if (!isTauri) {
      setError(t("errors.notTauri"));
      return;
    }
    try {
      await invokeTauri("openLogFolder");
    } catch (err) {
      setError(formatError(err));
    }
  }, [formatError, setError, t]);

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
        await writeSetting({ key: "ui_zoom_level", value: normalized });
      } catch {
        /* ignore -- zoom is best-effort */
      }
    },
    [bounds, writeSetting],
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
    handleOpenLogFolder,
    handleThemeChange,
    handleZoomLevelChange,
  };
}
