import { memo, useState, type ReactNode } from "react";
import { Activity, ArrowUpCircle, Blocks, Palette, SlidersHorizontal } from "lucide-react";
import type { TFunction } from "i18next";
import type { AppUpdate } from "../../hooks/useAppUpdate";
import type { SettingsState } from "../../hooks/useSettingsState";
import type { LayerOf } from "../../hooks/useSurfaceState";
import { SETTINGS_FIELD_ID, SETTINGS_PANES, updatesPaneHasControls, type SettingsPane } from "../../lib/settingsPanes";
import type { ToolStatusDto } from "../skills/types";
import { AppearancePane } from "./AppearancePane";
import { DiagnosticsPane } from "./DiagnosticsPane";
import { GeneralPane } from "./GeneralPane";
import { HarnessesPane } from "./HarnessesPane";
import { PaneDialog, type PaneSpec } from "./PaneDialog";
import { UpdatesPane } from "./UpdatesPane";

const PANE_ICON: Record<SettingsPane, ReactNode> = {
  general: <SlidersHorizontal aria-hidden="true" />,
  harnesses: <Blocks aria-hidden="true" />,
  appearance: <Palette aria-hidden="true" />,
  updates: <ArrowUpCircle aria-hidden="true" />,
  diagnostics: <Activity aria-hidden="true" />,
};

type SettingsDialogProps = {
  /** The surface module's `settings` layer (`surface.layer("settings")`), or null while closed. */
  layer: LayerOf<"settings"> | null;
  /** Show another pane (dialog state, never the hash). */
  onPaneChange: (pane: SettingsPane) => void;
  onClose: () => void;
  /** The settings world. */
  settings: SettingsState;
  /** The one update module. */
  update: Pick<AppUpdate, "status" | "version" | "notes" | "error" | "currentVersion" | "check" | "install" | "ignore">;
  /** The sync world's policy and selection, as the panes show and change them. */
  deploy: {
    autoDeployEnabled: boolean;
    onAutoDeployChange: (enabled: boolean) => void;
    /** The Effective global target set. */
    selected: readonly string[];
    scanSelectedOnly: boolean;
    onSelectedChange: (toolId: string, selected: boolean) => void;
    /** Save "scan only the selected harnesses" for Import. */
    onScanSelectedOnlyChange: (scanSelectedOnly: boolean) => void;
  };
  /** The Tool catalog's global list and labels. */
  toolStatus: ToolStatusDto | null;
  labelOf: (toolId: string) => string;
  /** The lifecycle's refusal in words (D19), or null. */
  blockedReason: string | null;
  /** The header progress, at the foot while a deploy this dialog started runs. */
  progress: ReactNode;
  isTauri: boolean;
  t: TFunction;
};

/**
 * Settings (round-19 ticket 20, D37 / D47–D50): the pane frame over the
 * settings, update and sync worlds — General · Harnesses · Appearance ·
 * Updates · Diagnostics. Mounted once by the binder and kept mounted (focus
 * returns to its opener); the shown pane is the surface layer's.
 */
const SettingsDialog = ({
  layer,
  onPaneChange,
  onClose,
  settings,
  update,
  deploy,
  toolStatus,
  labelOf,
  blockedReason,
  progress,
  isTauri,
  t,
}: SettingsDialogProps) => {
  // The pane survives the close, so the frame never flashes General as it leaves.
  const [pane, setPane] = useState<SettingsPane>(layer?.pane ?? "general");
  if (layer && layer.pane !== pane) setPane(layer.pane);
  const field = layer?.field ?? null;

  const versionText = !isTauri ? t("notAvailable") : update.currentVersion ? `v${update.currentVersion}` : t("unknown");

  const content: Record<SettingsPane, ReactNode> = {
    general: (
      <GeneralPane
        settings={settings}
        autoDeploy={{ enabled: deploy.autoDeployEnabled, onChange: deploy.onAutoDeployChange }}
        blockedReason={blockedReason}
        t={t}
      />
    ),
    harnesses: (
      <HarnessesPane
        toolStatus={toolStatus}
        labelOf={labelOf}
        selected={deploy.selected}
        scanSelectedOnly={deploy.scanSelectedOnly}
        autoDeployEnabled={deploy.autoDeployEnabled}
        onSelectedChange={deploy.onSelectedChange}
        onScanSelectedOnlyChange={deploy.onScanSelectedOnlyChange}
        onOpenGeneral={() => onPaneChange("general")}
        blockedReason={blockedReason}
        t={t}
      />
    ),
    appearance: <AppearancePane settings={settings} t={t} />,
    updates: <UpdatesPane update={update} versionText={versionText} isTauri={isTauri} t={t} />,
    diagnostics: <DiagnosticsPane settings={settings} versionText={versionText} isTauri={isTauri} t={t} />,
  };
  const panes: PaneSpec<SettingsPane>[] = SETTINGS_PANES.map((id) => ({
    id,
    label: t(`settingsDialog.panes.${id}`),
    icon: PANE_ICON[id],
    content: content[id],
    // Updates mid-check / mid-install (or outside the app) has nothing to operate.
    static: id === "updates" && !updatesPaneHasControls(update.status, isTauri),
  }));

  return (
    <PaneDialog
      open={layer !== null}
      title={t("settings")}
      paneListLabel={t("settingsDialog.paneList")}
      panes={panes}
      pane={pane}
      onPaneChange={onPaneChange}
      onClose={onClose}
      initialFocus={field ? () => document.getElementById(SETTINGS_FIELD_ID[field]) : undefined}
      progress={progress}
    />
  );
};

export default memo(SettingsDialog);
