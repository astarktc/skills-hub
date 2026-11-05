import { useId } from "react";
import { Monitor, Moon, Sun } from "lucide-react";
import type { TFunction } from "i18next";
import { Label } from "@/components/ui/field";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import type { SettingsState } from "../../hooks/useSettingsState";
import { PaneHeader, SettingGroup, SettingRow } from "./PaneDialog";

type ThemePreference = SettingsState["themePreference"];

const THEMES: readonly [ThemePreference, typeof Monitor][] = [
  ["system", Monitor],
  ["light", Sun],
  ["dark", Moon],
];

type AppearancePaneProps = {
  settings: Pick<SettingsState, "themePreference" | "handleThemeChange">;
  t: TFunction;
};

/**
 * Settings → Appearance: the theme as a radio group (System · Light · Dark;
 * arrow keys move and select, the choice applies at once). The sidebar's
 * Theme menu stays as the quick toggle over the same preference (operator
 * 2026-11-04).
 */
export function AppearancePane({ settings, t }: AppearancePaneProps) {
  const labelId = useId();
  const hintId = useId();
  return (
    <>
      <PaneHeader
        title={t("settingsDialog.panes.appearance")}
        description={t("settingsDialog.appearance.description")}
        note={t("settingsDialog.autoSaves")}
      />
      <SettingGroup>
        <SettingRow
          control={
            <RadioGroup
              aria-labelledby={labelId}
              aria-describedby={hintId}
              value={settings.themePreference}
              onValueChange={(next) => settings.handleThemeChange(next as ThemePreference)}
            >
              {THEMES.map(([value, Icon]) => (
                <RadioGroupItem key={value} value={value}>
                  <Icon aria-hidden="true" />
                  {t(`themeOptions.${value}`)}
                </RadioGroupItem>
              ))}
            </RadioGroup>
          }
        >
          <Label id={labelId}>{t("shell.themeMenu")}</Label>
          <p id={hintId} className="m-0 text-xs text-muted-foreground">
            {t("settingsDialog.appearance.themeHint")}
          </p>
        </SettingRow>
      </SettingGroup>
    </>
  );
}
