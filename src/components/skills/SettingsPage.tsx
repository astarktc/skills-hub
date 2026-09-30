import { memo, useEffect, useMemo } from "react";
import { ArrowLeft } from "lucide-react";
import type { TFunction } from "i18next";
import type { AppUpdate } from "../../hooks/useAppUpdate";
import type { SettingsBounds } from "./types";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";

type SettingsPageProps = {
  isTauri: boolean;
  /**
   * The one update module (`useAppUpdate`): a release found at startup is
   * the one shown here, and a check from here is the one the notice installs.
   */
  update: Pick<
    AppUpdate,
    "status" | "version" | "error" | "currentVersion" | "check" | "install"
  >;
  storagePath: string;
  gitCacheCleanupDays: number;
  gitCacheTtlSecs: number;
  /** Clamp bounds from the backend settings snapshot; null until loaded. */
  bounds: SettingsBounds | null;
  themePreference: "system" | "light" | "dark";
  zoomLevel: number;
  /**
   * Whether a token is saved; `null` when the keychain could not be read.
   * The secret itself is never read back.
   */
  githubTokenSet: boolean | null;
  /** The write-only Save input (never prefilled from the backend). */
  githubTokenDraft: string;
  /** A Save/Remove is in flight: input and buttons are disabled. */
  githubTokenPending: boolean;
  onPickStoragePath: () => void;
  onThemeChange: (nextTheme: "system" | "light" | "dark") => void;
  onZoomLevelChange: (nextLevel: number) => void;
  onGitCacheCleanupDaysChange: (nextDays: number) => void;
  onGitCacheTtlSecsChange: (nextSecs: number) => void;
  onClearGitCacheNow: () => void;
  onOpenLogFolder: () => void;
  onGithubTokenDraftChange: (draft: string) => void;
  /** Save the draft; a no-op while a token mutation is pending. */
  onGithubTokenSave: () => Promise<void>;
  /** Remove the saved token; a no-op while a token mutation is pending. */
  onGithubTokenRemove: () => Promise<void>;
  /** Auto-sync (moved here from the My Skills toolbar; its pane is ticket 20's). */
  autoSyncEnabled: boolean;
  onAutoSyncChange: (enabled: boolean) => void;
  /** Why the toggle is disabled while an action runs (D19), or null. */
  autoSyncBlockedReason: string | null;
  /**
   * The pane the address names (`#/settings/github`): the Notifications
   * panel's "Open Settings → GitHub" lands on the token field. The panes
   * themselves are ticket 20's.
   */
  focusPane: string | null;
  onBack: () => void;
  t: TFunction;
};

const SettingsPage = ({
  isTauri,
  update,
  storagePath,
  gitCacheCleanupDays,
  gitCacheTtlSecs,
  bounds,
  themePreference,
  zoomLevel,
  onPickStoragePath,
  onThemeChange,
  onZoomLevelChange,
  onGitCacheCleanupDaysChange,
  onGitCacheTtlSecsChange,
  onClearGitCacheNow,
  onOpenLogFolder,
  githubTokenSet,
  githubTokenDraft,
  githubTokenPending,
  onGithubTokenDraftChange,
  onGithubTokenSave,
  onGithubTokenRemove,
  autoSyncEnabled,
  onAutoSyncChange,
  autoSyncBlockedReason,
  focusPane,
  onBack,
  t,
}: SettingsPageProps) => {
  useEffect(() => {
    if (focusPane !== "github") return;
    const field = document.getElementById("settings-github-token");
    field?.scrollIntoView({ block: "center" });
    field?.focus();
  }, [focusPane]);
  const {
    status: updateStatus,
    version: updateVersion,
    error: updateError,
    currentVersion: appVersion,
    check: handleCheckUpdate,
    install: handleInstallUpdate,
  } = update;
  const versionText = useMemo(() => {
    if (!isTauri) return t("notAvailable");
    if (!appVersion) return t("unknown");
    return `v${appVersion}`;
  }, [appVersion, isTauri, t]);

  return (
    <div className="settings-page">
      {/* The former detail header (its App.css rules left with the detail,
          ticket 12); the Settings panes are ticket 20's. */}
      <div className="mx-auto flex w-full max-w-[560px] flex-col items-start gap-1.5 pt-6">
        <Button variant="ghost" size="sm" onClick={onBack}>
          <ArrowLeft aria-hidden="true" />
          {t("detail.back")}
        </Button>
        <h1 className="m-0 text-lg font-semibold text-foreground">{t("settings")}</h1>
      </div>
      <div className="settings-page-body">
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
          <label className="settings-label" id="settings-auto-sync-label">
            {t("settingsAutoSync")}
          </label>
          <div className="flex items-center gap-3">
            <Switch
              checked={autoSyncEnabled}
              onCheckedChange={onAutoSyncChange}
              disabled={autoSyncBlockedReason !== null}
              title={autoSyncBlockedReason ?? undefined}
              aria-labelledby="settings-auto-sync-label"
              aria-describedby="settings-auto-sync-hint"
            />
          </div>
          <div className="settings-helper" id="settings-auto-sync-hint">
            {t("settingsAutoSyncHint")}
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
              min={bounds?.git_cache_cleanup_days.min}
              max={bounds?.git_cache_cleanup_days.max}
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
              min={bounds?.git_cache_ttl_secs.min}
              max={bounds?.git_cache_ttl_secs.max}
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
              autoComplete="off"
              placeholder={t("githubTokenPlaceholder")}
              value={githubTokenDraft}
              disabled={githubTokenPending}
              onChange={(e) => onGithubTokenDraftChange(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void onGithubTokenSave();
              }}
            />
            <button
              className="btn btn-secondary btn-sm"
              type="button"
              disabled={githubTokenPending || githubTokenDraft.trim() === ""}
              onClick={() => void onGithubTokenSave()}
            >
              {t("githubTokenSave")}
            </button>
            {/* Unknown presence keeps Remove: it surfaces the typed error
                if the keychain is still unavailable. */}
            {githubTokenSet !== false && (
              <button
                className="btn btn-secondary btn-sm"
                type="button"
                disabled={githubTokenPending}
                onClick={() => void onGithubTokenRemove()}
              >
                {t("githubTokenClear")}
              </button>
            )}
          </div>
          <div className="settings-helper">
            {githubTokenSet === null
              ? t("githubTokenUnknown")
              : githubTokenSet
                ? t("githubTokenSet")
                : t("githubTokenUnset")}
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
                onClick={() => void handleCheckUpdate()}
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
                onClick={() => void handleInstallUpdate()}
              >
                {t("downloadAndInstall")}
              </button>
            </div>
          )}
          {updateStatus === "available" && updateError && (
            <div className="settings-update-error">
              <span>{updateError}</span>
            </div>
          )}
          {updateStatus === "installing" && (
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
                onClick={() => void handleCheckUpdate()}
              >
                {t("checkForUpdates")}
              </button>
            </div>
          )}
          <div className="settings-helper">{t("updateHint")}</div>
        </div>

        <div className="settings-field">
          <label className="settings-label">{t("diagnostics")}</label>
          <div className="settings-input-row">
            <button
              className="btn btn-secondary btn-sm"
              type="button"
              disabled={!isTauri}
              onClick={onOpenLogFolder}
            >
              {t("openLogFolder")}
            </button>
          </div>
          <div className="settings-helper">{t("openLogFolderHint")}</div>
        </div>
      </div>
    </div>
  );
};

export default memo(SettingsPage);
