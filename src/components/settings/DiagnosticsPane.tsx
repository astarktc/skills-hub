import type { TFunction } from "i18next";
import { Button } from "@/components/ui/button";
import { Field, FieldDescription, FieldLabel, Input, Label } from "@/components/ui/field";
import type { SettingsState } from "../../hooks/useSettingsState";
import { SETTINGS_FIELD_ID } from "../../lib/settingsPanes";
import { PaneHeader, SettingGroup, SettingRow } from "./PaneDialog";

type DiagnosticsPaneProps = {
  settings: Pick<
    SettingsState,
    | "storagePath"
    | "githubTokenSet"
    | "githubTokenDraft"
    | "githubTokenPending"
    | "handleGithubTokenDraftChange"
    | "handleGithubTokenSave"
    | "handleGithubTokenRemove"
    | "handleOpenLogFolder"
  >;
  versionText: string;
  isTauri: boolean;
  t: TFunction;
};

/**
 * Settings → Diagnostics: the version, the log folder (the existing
 * `open_log_folder` command reveals it), the Library folder read-only (General
 * owns its picker — one editor per setting) and the GitHub token: presence
 * only, never read back, saved to the system keychain with an explicit Save
 * (the one setting here that does not save as it changes).
 */
export function DiagnosticsPane({ settings, versionText, isTauri, t }: DiagnosticsPaneProps) {
  const { githubTokenSet, githubTokenDraft, githubTokenPending } = settings;
  return (
    <>
      <PaneHeader
        title={t("settingsDialog.panes.diagnostics")}
        description={t("settingsDialog.diagnostics.description")}
      />
      <SettingGroup>
        <SettingRow control={<span className="font-mono text-sm text-foreground">{versionText}</span>}>
          <Label>{t("settingsDialog.diagnostics.version")}</Label>
        </SettingRow>
        <SettingRow
          control={
            <Button disabled={!isTauri} onClick={() => void settings.handleOpenLogFolder()}>
              {t("openLogFolder")}
            </Button>
          }
        >
          <Label>{t("settingsDialog.diagnostics.log")}</Label>
          <p className="m-0 text-xs text-muted-foreground">{t("openLogFolderHint")}</p>
        </SettingRow>
        <Field className="gap-0">
          <SettingRow stacked control={<Input value={settings.storagePath} readOnly />}>
            <FieldLabel>{t("libraryFolder")}</FieldLabel>
            <FieldDescription>{t("settingsDialog.diagnostics.libraryFolderHint")}</FieldDescription>
          </SettingRow>
        </Field>
        <Field className="gap-0">
          <SettingRow
            stacked
            control={
              <div className="flex items-center gap-2">
                <Input
                  id={SETTINGS_FIELD_ID.githubToken}
                  className="flex-1"
                  type="password"
                  autoComplete="off"
                  spellCheck={false}
                  placeholder={t("githubTokenPlaceholder")}
                  value={githubTokenDraft}
                  disabled={githubTokenPending}
                  onChange={(event) => settings.handleGithubTokenDraftChange(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") void settings.handleGithubTokenSave();
                  }}
                />
                <Button
                  disabled={githubTokenPending || githubTokenDraft.trim() === ""}
                  onClick={() => void settings.handleGithubTokenSave()}
                >
                  {t("githubTokenSave")}
                </Button>
                {/* Unknown presence keeps Remove: it surfaces the typed error
                    if the keychain is still unavailable. */}
                {githubTokenSet !== false ? (
                  <Button disabled={githubTokenPending} onClick={() => void settings.handleGithubTokenRemove()}>
                    {t("githubTokenClear")}
                  </Button>
                ) : null}
              </div>
            }
          >
            <FieldLabel>{t("githubToken")}</FieldLabel>
            <FieldDescription>
              {githubTokenSet === null
                ? t("githubTokenUnknown")
                : githubTokenSet
                  ? t("githubTokenSet")
                  : t("githubTokenUnset")}{" "}
              {t("githubTokenHint")} {t("settingsDialog.diagnostics.tokenSaves")}
            </FieldDescription>
          </SettingRow>
        </Field>
      </SettingGroup>
    </>
  );
}
