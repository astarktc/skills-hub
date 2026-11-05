import type { ReactNode } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

export type PaneSpec<P extends string> = {
  id: P;
  label: string;
  icon: ReactNode;
  /** The pane's body; rendered only while it is the shown pane. */
  content: ReactNode;
  /**
   * The pane holds no focusable control (Keyboard shortcuts' lists, Updates
   * with no action to offer): its panel becomes the Tab stop after the pane
   * list, so Tab reaches the content instead of skipping to Close.
   */
  static?: boolean;
};

type PaneDialogProps<P extends string> = {
  /** Whether the surface module shows this dialog. Keep the frame mounted and toggle this (focus return). */
  open: boolean;
  title: string;
  /** The pane list's accessible name. */
  paneListLabel: string;
  panes: readonly PaneSpec<P>[];
  pane: P;
  onPaneChange: (pane: P) => void;
  onClose: () => void;
  /**
   * Where focus goes as it opens: by default the shown pane's tab (↑ / ↓
   * walk the panes from there); a function for a named control.
   */
  initialFocus?: () => HTMLElement | null;
  /** The header progress at the dialog's foot while an action this dialog started runs. */
  progress?: ReactNode;
};

/**
 * The pane frame (round-19 ticket 20, D37 / D50): Settings and Keyboard
 * shortcuts. One Dialog on the layer stack, `size="settings"` (880 × 640,
 * capped at 90 vw / 85 vh, not resizable), its title in the header, a
 * vertical pane list on the left (`Tabs variant="pane"`: tablist / tab /
 * tabpanel; ↑ / ↓ move between panes and show them, Tab enters the pane —
 * its first control, or a static pane's panel) and
 * the shown pane on the right, which alone scrolls. Escape and the ✕ close
 * it; focus returns to its opener (the layer stack). The pane is the
 * caller's state — the surface module's dialog layer, never the hash.
 */
export function PaneDialog<P extends string>({
  open,
  title,
  paneListLabel,
  panes,
  pane,
  onPaneChange,
  onClose,
  initialFocus,
  progress,
}: PaneDialogProps<P>) {
  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) onClose();
      }}
    >
      <DialogContent size="settings" initialFocus={initialFocus ? () => initialFocus() ?? true : undefined}>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>
        <Tabs
          variant="pane"
          value={pane}
          onValueChange={(next) => onPaneChange(next as P)}
          className="min-h-0 flex-1"
        >
          <TabsList aria-label={paneListLabel}>
            {panes.map((spec) => (
              <TabsTrigger key={spec.id} value={spec.id} data-pane-tab={spec.id}>
                {spec.icon}
                {spec.label}
              </TabsTrigger>
            ))}
          </TabsList>
          {panes.map((spec) => (
            <TabsContent key={spec.id} value={spec.id} staticPane={spec.static === true}>
              <div className="flex flex-col gap-5 px-6 py-5">{spec.content}</div>
            </TabsContent>
          ))}
        </Tabs>
        {progress}
      </DialogContent>
    </Dialog>
  );
}

/** A pane's heading: its name (`h3`, under the dialog's `h2`), what it holds, and how it saves. */
export function PaneHeader({ title, description, note }: { title: string; description: string; note?: string }) {
  return (
    <div className="flex flex-col gap-1">
      <h3 className="m-0 text-base font-semibold text-foreground">{title}</h3>
      <p className="m-0 text-sm text-muted-foreground">{description}</p>
      {note ? <p className="m-0 text-xs text-faint">{note}</p> : null}
    </div>
  );
}

/**
 * A group of settings: an optional `h4` and its rows, each row divided from
 * the next by a hairline (label and helper on the left, the control on the
 * right; `SettingRow`).
 */
export function SettingGroup({ title, children }: { title?: string; children: ReactNode }) {
  return (
    <section className="flex flex-col">
      {title ? <h4 className="m-0 pb-1 text-xs font-medium text-faint">{title}</h4> : null}
      <div className="flex flex-col divide-y divide-border border-y border-border">{children}</div>
    </section>
  );
}

/** One setting: what it is and what it does on the left, its control on the right (`stacked`: below). */
export function SettingRow({ children, control, stacked = false }: { children: ReactNode; control: ReactNode; stacked?: boolean }) {
  return stacked ? (
    <div className="flex flex-col gap-2 py-3">
      {children}
      {control}
    </div>
  ) : (
    <div className="flex items-start justify-between gap-6 py-3">
      <div className="flex min-w-0 flex-1 flex-col gap-1">{children}</div>
      <div className="flex shrink-0 items-center gap-2">{control}</div>
    </div>
  );
}
