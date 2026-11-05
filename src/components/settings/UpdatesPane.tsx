import type { TFunction } from "i18next";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/field";
import type { AppUpdate, AppUpdateStatus } from "../../hooks/useAppUpdate";
import { updateCheckOffered } from "../../lib/settingsPanes";
import { PaneHeader, SettingGroup, SettingRow } from "./PaneDialog";

/** The status line for each lifecycle state (an error adds its own line below). */
const STATUS_LINE: Record<AppUpdateStatus, string> = {
  idle: "updateHint",
  checking: "checkingUpdates",
  "up-to-date": "updateNotAvailable",
  available: "updateAvailableWithVersion",
  installing: "installingUpdate",
  done: "updateInstalledRestart",
  error: "updateHint",
};

type UpdatesPaneProps = {
  /**
   * The one update module (`useAppUpdate`): the release the sidebar item
   * announces is the one shown here, and a check from here is the one the
   * update dialog installs.
   */
  update: Pick<AppUpdate, "status" | "version" | "notes" | "error" | "check" | "install" | "ignore">;
  /** The running version as shown ("v1.3.0", or why there is none). */
  versionText: string;
  isTauri: boolean;
  t: TFunction;
};

/**
 * Settings → Updates: the running version, Check, and a found release with
 * its notes, Download & Install and Don't remind again — the update module's
 * state, so the sidebar's quiet item and its dialog always agree with it.
 */
export function UpdatesPane({ update, versionText, isTauri, t }: UpdatesPaneProps) {
  const { status, version, notes, error, check, install, ignore } = update;
  const canCheck = updateCheckOffered(status, isTauri);
  return (
    <>
      <PaneHeader title={t("settingsDialog.panes.updates")} description={t("settingsDialog.updates.description")} />
      <SettingGroup>
        <SettingRow
          control={
            canCheck ? (
              <Button onClick={() => void check()}>{t("checkForUpdates")}</Button>
            ) : null
          }
        >
          <Label>{t("settingsDialog.updates.version", { version: versionText })}</Label>
          <p className="m-0 text-xs text-muted-foreground" role="status">
            {t(STATUS_LINE[status], { version })}
          </p>
          {error ? <p className="m-0 text-xs text-destructive">{error}</p> : null}
        </SettingRow>
        {status === "available" ? (
          <SettingRow
            stacked
            control={
              <div className="flex items-center gap-2">
                <Button variant="solid" tone="accent" onClick={() => void install()}>
                  {t("downloadAndInstall")}
                </Button>
                <Button onClick={ignore}>{t("updateBannerDismiss")}</Button>
              </div>
            }
          >
            <Label>{t("settingsDialog.updates.notes")}</Label>
            {notes ? (
              <div className="prose max-h-64 overflow-y-auto rounded-md border border-border px-4 py-3">
                <Markdown remarkPlugins={[remarkGfm]}>{notes}</Markdown>
              </div>
            ) : null}
            <p className="m-0 text-xs text-muted-foreground">{t("settingsDialog.updates.ignoreHint")}</p>
          </SettingRow>
        ) : null}
      </SettingGroup>
    </>
  );
}
