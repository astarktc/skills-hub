import { memo, useState, type ReactNode } from "react";
import { Keyboard, Layers } from "lucide-react";
import type { TFunction } from "i18next";
import type { LayerOf } from "../../hooks/useSurfaceState";
import { SHORTCUT_PANES, type ShortcutPane } from "../../lib/settingsPanes";
import { PaneDialog, PaneHeader, type PaneSpec } from "./PaneDialog";

type ShortcutsDialogProps = {
  /** The surface module's `shortcuts` layer, or null while closed. */
  layer: LayerOf<"shortcuts"> | null;
  onPaneChange: (pane: ShortcutPane) => void;
  onClose: () => void;
  t: TFunction;
};

function ShortcutList({ shortcuts }: { shortcuts: readonly (readonly [string, string])[] }) {
  return (
    <dl className="m-0 grid grid-cols-[minmax(6rem,auto)_1fr] items-center gap-x-4 gap-y-2">
      {shortcuts.map(([key, action]) => (
        <div key={action} className="contents">
          <dt>
            <kbd className="inline-flex h-6 min-w-6 items-center justify-center rounded-sm border border-border-strong px-1.5 font-mono text-xs text-foreground">
              {key}
            </kbd>
          </dt>
          <dd className="m-0 text-sm text-muted-foreground">{action}</dd>
        </div>
      ))}
    </dl>
  );
}

/**
 * Keyboard shortcuts in the Settings frame (D37): its own `shortcuts` dialog
 * of the surface module, the same `PaneDialog` — General (anywhere) · My
 * Skills (the list keymap, `useSkillListKeymap`) — with the same focus,
 * Escape and return rules. Opened from the sidebar and the ⌘K palette.
 */
const ShortcutsDialog = ({ layer, onPaneChange, onClose, t }: ShortcutsDialogProps) => {
  const [pane, setPane] = useState<ShortcutPane>(layer?.pane ?? "general");
  if (layer && layer.pane !== pane) setPane(layer.pane);

  const lists: Record<ShortcutPane, readonly (readonly [string, string])[]> = {
    general: [
      [t("palette.shortcutKey"), t("palette.shortcutOpen")],
      [t("shell.shortcutEscape"), t("shortcutsDialog.escape")],
      [t("shortcutsDialog.arrowsKey"), t("shortcutsDialog.panesMove")],
      [t("shortcutsDialog.zoomKey"), t("shortcutsDialog.zoom")],
    ],
    // My Skills' keymap (`useSkillListKeymap`), in the order one uses it.
    skills: [
      ["/", t("shell.shortcutSearch")],
      [t("bulk.shortcuts.jk"), t("bulk.shortcuts.move")],
      [t("shell.shortcutEnter"), t("shell.shortcutOpen")],
      [t("bulk.shortcuts.x"), t("bulk.shortcuts.select")],
      [t("bulk.shortcuts.shiftClick"), t("bulk.shortcuts.range")],
      [t("bulk.shortcuts.modA"), t("bulk.shortcuts.selectAll")],
      [t("shell.shortcutEscape"), t("bulk.shortcuts.clear")],
    ],
  };
  const icons: Record<ShortcutPane, ReactNode> = {
    general: <Keyboard aria-hidden="true" />,
    skills: <Layers aria-hidden="true" />,
  };
  const panes: PaneSpec<ShortcutPane>[] = SHORTCUT_PANES.map((id) => ({
    id,
    label: t(`shortcutsDialog.panes.${id}`),
    icon: icons[id],
    // Reference lists: nothing to operate, so the panel is the Tab stop.
    static: true,
    content: (
      <>
        <PaneHeader title={t(`shortcutsDialog.panes.${id}`)} description={t(`shortcutsDialog.description.${id}`)} />
        <ShortcutList shortcuts={lists[id]} />
      </>
    ),
  }));

  return (
    <PaneDialog
      open={layer !== null}
      title={t("shell.shortcuts")}
      paneListLabel={t("shortcutsDialog.paneList")}
      panes={panes}
      pane={pane}
      onPaneChange={onPaneChange}
      onClose={onClose}
    />
  );
};

export default memo(ShortcutsDialog);
