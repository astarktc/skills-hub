import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ArrowLeft } from "lucide-react";
import type { TFunction } from "i18next";
import type { Update } from "@tauri-apps/plugin-updater";

type UpdateStatus =
  | "idle"
  | "checking"
  | "up-to-date"
  | "available"
  | "downloading"
  | "done"
  | "error";

type SettingsPageProps = {
  isTauri: boolean;
  language: string;
  storagePath: string;
  gitCacheCleanupDays: number;
  gitCacheTtlSecs: number;
  themePreference: "system" | "light" | "dark";
  zoomLevel: number;
  githubToken: string;
  onPickStoragePath: () => void;
  onToggleLanguage: () => void;
  onThemeChange: (nextTheme: "system" | "light" | "dark") => void;
  onZoomLevelChange: (nextLevel: number) => void;
  onGitCacheCleanupDaysChange: (nextDays: number) => void;
  onGitCacheTtlSecsChange: (nextSecs: number) => void;
  onClearGitCacheNow: () => void;
  onGithubTokenChange: (token: string) => void;
  onBack: () => void;
  t: TFunction;
};

const SettingsPage = ({
  isTauri,
  language,
  storagePath,
  gitCacheCleanupDays,
  gitCacheTtlSecs,
  themePreference,
  zoomLevel,
  onPickStoragePath,
  onToggleLanguage,
  onThemeChange,
  onZoomLevelChange,
  onGitCacheCleanupDaysChange,
  onGitCacheTtlSecsChange,
  onClearGitCacheNow,
  githubToken,
  onGithubTokenChange,
  onBack,
  t,
}: SettingsPageProps) => {
  const [localToken, setLocalToken] = useState(githubToken);
  useEffect(() => {
    setLocalToken(githubToken);
  }, [githubToken]);

  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>("idle");
  const [updateVersion, setUpdateVersion] = useState<string | null>(null);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const updateRef = useRef<Update | null>(null);

  const handleCheckUpdate = useCallback(async () => {
    if (!isTauri) return;
    setUpdateStatus("checking");
    setUpdateError(null);
    try {
      const { check } = await import("@tauri-apps/plugin-updater");
      const update = await check();
      if (update) {
        updateRef.current = update;
        setUpdateVersion(update.version);
        setUpdateStatus("available");
      } else {
        setUpdateStatus("up-to-date");
      }
    } catch (err) {
      setUpdateError(err instanceof Error ? err.message : String(err));
      setUpdateStatus("error");
    }
  }, [isTauri]);

  const handleInstallUpdate = useCallback(async () => {
    const update = updateRef.current;
    if (!update) return;
    setUpdateStatus("downloading");
    setUpdateError(null);
    try {
      await update.downloadAndInstall();
      setUpdateStatus("done");
    } catch (err) {
      setUpdateError(err instanceof Error ? err.message : String(err));
      setUpdateStatus("error");
    }
  }, []);

  const [appVersion, setAppVersion] = useState<string | null>(null);
  const versionText = useMemo(() => {
    if (!isTauri) return t("notAvailable");
    if (!appVersion) return t("unknown");
    return `v${appVersion}`;
  }, [appVersion, isTauri, t]);

  const loadAppVersion = useCallback(async () => {
    if (!isTauri) {
      setAppVersion(null);
      return;
    }
    try {
      const { getVersion } = await import("@tauri-apps/api/app");
      const v = await getVersion();
      setAppVersion(v);
    } catch {
      setAppVersion(null);
    }
  }, [isTauri]);

  useEffect(() => {
    void loadAppVersion();
    return () => {
      updateRef.current = null;
    };
  }, [loadAppVersion]);

  return (
    <div className="settings-page">
      <div className="detail-header">
        <button className="detail-back-btn" type="button" onClick={onBack}>
          <ArrowLeft size={16} />
          {t("detail.back")}
        </button>
        <div className="detail-skill-name">{t("settings")}</div>
      </div>
      <div className="settings-page-body">
        <div className="settings-field">
          <label className="settings-label" htmlFor="settings-language">
            {t("interfaceLanguage")}
          </label>
          <div className="settings-select-wrap">
            <select
              id="settings-language"
              className="settings-select"
              value={language}
              onChange={(event) => {
                if (event.target.value !== language) {
                  onToggleLanguage();
                }
              }}
            >
              <option value="en">{t("languageOptions.en")}</option>
              <option value="zh">{t("languageOptions.zh")}</option>
            </select>
            <svg
              className="settings-select-caret"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <path d="M6 9l6 6 6-6" />
            </svg>
          </div>
        </div>

        <div className="settings-field">
          <label className="settings-label" id="settings-theme-label">
            {t("themeMode")}
          </label>
          <div
            className="settings-theme-options"
            role="group"
            aria-labelledby="settings-theme-label"
          >
            <button
              type="button"
              className={`settings-theme-btn ${
                themePreference === "system" ? "active" : ""
              }`}
              aria-pressed={themePreference === "system"}
              onClick={() => onThemeChange("system")}
            >
              {t("themeOptions.system")}
            </button>
            <button
              type="button"
              className={`settings-theme-btn ${
                themePreference === "light" ? "active" : ""
              }`}
              aria-pressed={themePreference === "light"}
              onClick={() => onThemeChange("light")}
            >
              {t("themeOptions.light")}
            </button>
            <button
              type="button"
              className={`settings-theme-btn ${
                themePreference === "dark" ? "active" : ""
              }`}
              aria-pressed={themePreference === "dark"}
              onClick={() => onThemeChange("dark")}
            >
              {t("themeOptions.dark")}
            </button>
          </div>
        </div>

        <div className="settings-field">
          <label className="settings-label" htmlFor="settings-zoom">
            {t("defaultZoomLevel")}
          </label>
          <div className="settings-select-wrap">
            <select
              id="settings-zoom"
              className="settings-select"
              value={String(zoomLevel)}
              onChange={(event) => {
                const next = parseFloat(event.target.value);
                if (!Number.isNaN(next)) {
                  onZoomLevelChange(next);
                }
              }}
            >
              <option value="0.75">{t("zoomOptions.75")}</option>
              <option value="1">{t("zoomOptions.100")}</option>
              <option value="1.1">{t("zoomOptions.110")}</option>
              <option value="1.25">{t("zoomOptions.125")}</option>
              <option value="1.5">{t("zoomOptions.150")}</option>
              <option value="1.75">{t("zoomOptions.175")}</option>
              <option value="2">{t("zoomOptions.200")}</option>
            </select>
            <svg
              className="settings-select-caret"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <path d="M6 9l6 6 6-6" />
            </svg>
          </div>
          <div className="settings-helper">{t("defaultZoomLevelHint")}</div>
        </div>

        <div className="settings-field">
          <label className="settings-label" htmlFor="settings-storage">
            {t("skillsStoragePath")}
          </label>
          <div className="settings-input-row">
            <input
              id="settings-storage"
              className="settings-input mono"
              value={storagePath}
              readOnly
            />
            <button
              className="btn btn-secondary settings-browse"
              type="button"
              onClick={onPickStoragePath}
            >
              {t("browse")}
            </button>
          </div>
          <div className="settings-helper">{t("skillsStorageHint")}</div>
        </div>

        <div className="settings-field">
          <label className="settings-label" htmlFor="settings-git-cache-days">
            {t("gitCacheCleanupDays")}
          </label>
          <div className="settings-input-row">
            <input
              id="settings-git-cache-days"
              className="settings-input"
              type="number"
              min={0}
              max={3650}
              step={1}
              value={gitCacheCleanupDays}
              onChange={(event) => {
                const next = Number(event.target.value);
                if (!Number.isNaN(next)) {
                  onGitCacheCleanupDaysChange(next);
                }
              }}
            />
            <button
              className="btn btn-secondary settings-browse"
              type="button"
              onClick={onClearGitCacheNow}
            >
              {t("cleanNow")}
            </button>
          </div>
          <div className="settings-helper">{t("gitCacheCleanupHint")}</div>
        </div>

        <div className="settings-field">
          <label className="settings-label" htmlFor="settings-git-cache-ttl">
            {t("gitCacheTtlSecs")}
          </label>
          <div className="settings-input-row">
            <input
              id="settings-git-cache-ttl"
              className="settings-input"
              type="number"
              min={0}
              max={3600}
              step={1}
              value={gitCacheTtlSecs}
              onChange={(event) => {
                const next = Number(event.target.value);
                if (!Number.isNaN(next)) {
                  onGitCacheTtlSecsChange(next);
                }
              }}
            />
          </div>
          <div className="settings-helper">{t("gitCacheTtlHint")}</div>
        </div>

        <div className="settings-field">
          <label className="settings-label" htmlFor="settings-github-token">
            {t("githubToken")}
          </label>
          <div className="settings-input-row">
            <input
              id="settings-github-token"
              className="settings-input mono"
              type="password"
              placeholder={t("githubTokenPlaceholder")}
              value={localToken}
              onChange={(e) => setLocalToken(e.target.value)}
              onBlur={() => {
                if (localToken !== githubToken) {
                  onGithubTokenChange(localToken);
                }
              }}
            />
          </div>
          <div className="settings-helper">{t("githubTokenHint")}</div>
        </div>

        <div className="settings-field settings-update-section">
          <label className="settings-label">{t("appUpdates")}</label>
          <div className="settings-version-row">
            <span className="settings-version-text">
              {t("appName")} {versionText}
            </span>
            {isTauri && updateStatus === "idle" && (
              <button
                className="btn btn-secondary btn-sm"
                type="button"
                onClick={handleCheckUpdate}
              >
                {t("checkForUpdates")}
              </button>
            )}
            {updateStatus === "checking" && (
              <span className="settings-update-status">
                {t("checkingUpdates")}
              </span>
            )}
            {updateStatus === "up-to-date" && (
              <span className="settings-update-status settings-update-ok">
                {t("updateNotAvailable")}
              </span>
            )}
          </div>
          {updateStatus === "available" && (
            <div className="settings-update-available">
              <span>
                {t("updateAvailableWithVersion", { version: updateVersion })}
              </span>
              <button
                className="btn btn-primary btn-sm"
                type="button"
                onClick={handleInstallUpdate}
              >
                {t("downloadAndInstall")}
              </button>
            </div>
          )}
          {updateStatus === "downloading" && (
            <div className="settings-update-status">
              {t("installingUpdate")}
            </div>
          )}
          {updateStatus === "done" && (
            <div className="settings-update-ok">
              {t("updateInstalledRestart")}
            </div>
          )}
          {updateStatus === "error" && (
            <div className="settings-update-error">
              <span>{updateError}</span>
              <button
                className="btn btn-secondary btn-sm"
                type="button"
                onClick={handleCheckUpdate}
              >
                {t("checkForUpdates")}
              </button>
            </div>
          )}
          <div className="settings-helper">{t("updateHint")}</div>
        </div>
      </div>
    </div>
  );
};

export default memo(SettingsPage);
