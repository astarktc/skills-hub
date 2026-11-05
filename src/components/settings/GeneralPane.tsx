import { useId } from "react";
import type { TFunction } from "i18next";
import { Button } from "@/components/ui/button";
import { Field, FieldDescription, FieldLabel, Input, Label, Select } from "@/components/ui/field";
import { Switch } from "@/components/ui/switch";
import type { SettingsState } from "../../hooks/useSettingsState";
import { PaneHeader, SettingGroup, SettingRow } from "./PaneDialog";

const ZOOM_OPTIONS = [
  ["0.75", "zoomOptions.75"],
  ["1", "zoomOptions.100"],
  ["1.1", "zoomOptions.110"],
  ["1.25", "zoomOptions.125"],
  ["1.5", "zoomOptions.150"],
  ["1.75", "zoomOptions.175"],
  ["2", "zoomOptions.200"],
] as const;

type GeneralPaneProps = {
  settings: Pick<
    SettingsState,
    | "zoomLevel"
    | "storagePath"
    | "gitCacheCleanupDays"
    | "gitCacheTtlSecs"
    | "bounds"
    | "handleZoomLevelChange"
    | "handlePickStoragePath"
    | "handleGitCacheCleanupDaysChange"
    | "handleGitCacheTtlSecsChange"
    | "handleClearGitCacheNow"
  >;
  autoDeploy: { enabled: boolean; onChange: (enabled: boolean) => void };
  /**
   * Why the mutation controls — Auto-deploy, Browse… (relocation) and Clean
   * Now — are held while an action runs (D19), or null. Zoom and the cache
   * numbers are preferences that act on nothing an action touches.
   */
  blockedReason: string | null;
  t: TFunction;
};

/**
 * Settings → General: how new skills arrive (Auto-deploy, with D49's line on
 * where it acts), zoom, the Library folder and the Git cache. Every control
 * saves as it changes.
 */
export function GeneralPane({ settings, autoDeploy, blockedReason, t }: GeneralPaneProps) {
  const autoDeployLabel = useId();
  const autoDeployHint = useId();
  const { bounds } = settings;
  return (
    <>
      <PaneHeader
        title={t("settingsDialog.panes.general")}
        description={t("settingsDialog.general.description")}
        note={t("settingsDialog.autoSaves")}
      />
      <SettingGroup>
        <SettingRow
          control={
            <Switch
              checked={autoDeploy.enabled}
              onCheckedChange={autoDeploy.onChange}
              disabled={blockedReason !== null}
              focusableWhenDisabled
              title={blockedReason ?? undefined}
              aria-labelledby={autoDeployLabel}
              aria-describedby={autoDeployHint}
            />
          }
        >
          <Label id={autoDeployLabel}>{t("settingsAutoDeploy")}</Label>
          <p id={autoDeployHint} className="m-0 text-xs text-muted-foreground">
            {t("settingsDialog.general.autoDeployHint")}
          </p>
        </SettingRow>
        <Field className="gap-0">
          <SettingRow
            control={
              <Select
                className="w-40"
                value={String(settings.zoomLevel)}
                onChange={(event) => {
                  const next = parseFloat(event.target.value);
                  if (!Number.isNaN(next)) void settings.handleZoomLevelChange(next);
                }}
              >
                {ZOOM_OPTIONS.map(([value, key]) => (
                  <option key={value} value={value}>
                    {t(key)}
                  </option>
                ))}
              </Select>
            }
          >
            <FieldLabel>{t("defaultZoomLevel")}</FieldLabel>
            <FieldDescription>{t("defaultZoomLevelHint")}</FieldDescription>
          </SettingRow>
        </Field>
        <Field className="gap-0">
          <SettingRow
            stacked
            control={
              <div className="flex items-center gap-2">
                <Input className="flex-1" value={settings.storagePath} readOnly />
                <Button
                  onClick={() => void settings.handlePickStoragePath()}
                  disabled={blockedReason !== null}
                  focusableWhenDisabled
                  title={blockedReason ?? undefined}
                >
                  {t("settingsDialog.general.browse")}
                </Button>
              </div>
            }
          >
            <FieldLabel>{t("libraryFolder")}</FieldLabel>
            <FieldDescription>{t("libraryFolderHint")}</FieldDescription>
          </SettingRow>
        </Field>
      </SettingGroup>

      <SettingGroup title={t("settingsDialog.general.gitCache")}>
        <Field className="gap-0">
          <SettingRow
            control={
              <>
                <Input
                  className="w-24"
                  type="number"
                  min={bounds?.git_cache_cleanup_days.min}
                  max={bounds?.git_cache_cleanup_days.max}
                  step={1}
                  value={settings.gitCacheCleanupDays}
                  onChange={(event) => {
                    const next = Number(event.target.value);
                    if (!Number.isNaN(next)) void settings.handleGitCacheCleanupDaysChange(next);
                  }}
                />
                <Button
                  onClick={() => void settings.handleClearGitCacheNow()}
                  disabled={blockedReason !== null}
                  focusableWhenDisabled
                  title={blockedReason ?? undefined}
                >
                  {t("cleanNow")}
                </Button>
              </>
            }
          >
            <FieldLabel>{t("gitCacheCleanupDays")}</FieldLabel>
            <FieldDescription>{t("gitCacheCleanupHint")}</FieldDescription>
          </SettingRow>
        </Field>
        <Field className="gap-0">
          <SettingRow
            control={
              <Input
                className="w-24"
                type="number"
                min={bounds?.git_cache_ttl_secs.min}
                max={bounds?.git_cache_ttl_secs.max}
                step={1}
                value={settings.gitCacheTtlSecs}
                onChange={(event) => {
                  const next = Number(event.target.value);
                  if (!Number.isNaN(next)) void settings.handleGitCacheTtlSecsChange(next);
                }}
              />
            }
          >
            <FieldLabel>{t("gitCacheTtlSecs")}</FieldLabel>
            <FieldDescription>{t("gitCacheTtlHint")}</FieldDescription>
          </SettingRow>
        </Field>
      </SettingGroup>
    </>
  );
}
