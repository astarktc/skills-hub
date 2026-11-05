import * as React from "react";
import { Tabs as TabsPrimitive } from "@base-ui/react/tabs";
import { cn } from "@/lib/utils";
import { disabledState, focusRing, iconSlot, insetFocusRing } from "./styles";

/**
 * Tabs — `role="tablist"` / `tab` / `tabpanel`, arrow keys move between
 * tabs (Base UI). Three looks:
 *   - `line`: an underline marks the selected tab (section switching);
 *   - `segmented`: a filled segment marks it (a compact switch of views);
 *   - `pane`: the pane frame's vertical pane list (Settings, Keyboard
 *     shortcuts; set on `Tabs`). ↑ / ↓ move between panes and show each
 *     (roving focus, activation follows focus); Tab leaves the list for the
 *     shown pane's first control — or the panel itself, for a `staticPane`. The pane list is a fixed 192 px column on
 *     the dialog's ground with a hairline to its right; each pane panel
 *     fills the rest and scrolls on its own.
 * For an option set that does not switch panels, use `Segmented` (or
 * `RadioGroup` for a setting).
 */

type TabsVariant = "line" | "segmented" | "pane";
const TabsVariantContext = React.createContext<TabsVariant>("line");

function Tabs({
  variant,
  className,
  orientation,
  ...props
}: Omit<TabsPrimitive.Root.Props, "className"> & {
  /** `pane` lays the list and panels side by side (vertical); `line` / `segmented` may also be set on `TabsList`. */
  variant?: TabsVariant;
  className?: string;
}) {
  const root = (
    <TabsPrimitive.Root
      data-slot="tabs"
      data-variant={variant}
      orientation={variant === "pane" ? "vertical" : orientation}
      className={cn(variant === "pane" ? "flex min-h-0 flex-row" : "flex flex-col gap-3", className)}
      {...props}
    />
  );
  return variant ? <TabsVariantContext.Provider value={variant}>{root}</TabsVariantContext.Provider> : root;
}

const listClass: Record<TabsVariant, string> = {
  line: "inline-flex w-fit items-center gap-1 border-b border-border",
  segmented: "inline-flex h-8 w-fit items-center gap-0.5 rounded-md border border-border-strong p-0.5",
  pane: "flex w-48 shrink-0 flex-col items-stretch gap-0.5 overflow-y-auto border-r border-border p-2",
};

function TabsList({
  variant: listVariant,
  className,
  activateOnFocus,
  ...props
}: Omit<TabsPrimitive.List.Props, "className"> & {
  variant?: TabsVariant;
  /** Layout only. */
  className?: string;
}) {
  const inherited = React.useContext(TabsVariantContext);
  const variant = listVariant ?? inherited;
  return (
    <TabsVariantContext.Provider value={variant}>
      <TabsPrimitive.List
        data-slot="tabs-list"
        data-variant={variant}
        activateOnFocus={activateOnFocus ?? variant === "pane"}
        className={cn(listClass[variant], className)}
        {...props}
      />
    </TabsVariantContext.Provider>
  );
}

const triggerClass: Record<TabsVariant, string> = {
  line: "-mb-px h-8 justify-center border-b-2 border-transparent px-2 data-active:border-ring",
  segmented:
    "h-6 justify-center rounded-sm px-2.5 data-active:bg-raised data-active:ring-1 data-active:ring-ring data-active:ring-inset",
  // Selected: the `raised` ground and a 2 px iris bar on its leading edge —
  // the bar is the ≥ 3:1 state cue (the ground alone is not).
  pane: [
    "h-8 w-full shrink-0 justify-start gap-2 rounded-md px-2.5 text-left",
    "before:absolute before:inset-y-1.5 before:left-0 before:w-0.5 before:rounded-full",
    "data-active:bg-raised data-active:before:bg-ring data-active:[&_svg]:text-ring",
  ].join(" "),
};

function TabsTrigger({
  className,
  ...props
}: Omit<TabsPrimitive.Tab.Props, "className"> & { className?: string }) {
  const variant = React.useContext(TabsVariantContext);
  return (
    <TabsPrimitive.Tab
      data-slot="tabs-trigger"
      className={cn(
        "relative inline-flex cursor-default items-center gap-1.5 text-sm font-medium whitespace-nowrap text-muted-foreground select-none hover:text-foreground data-active:text-foreground",
        triggerClass[variant],
        variant === "pane" ? insetFocusRing : focusRing,
        disabledState,
        "data-disabled:opacity-50",
        iconSlot,
        className,
      )}
      {...props}
    />
  );
}

function TabsContent({
  className,
  tabIndex,
  staticPane = false,
  ...props
}: Omit<TabsPrimitive.Panel.Props, "className"> & {
  className?: string;
  /**
   * `pane` only: the pane holds no focusable control (a reference list, a
   * status with no action), so the panel itself is the Tab stop after the
   * pane list — Tab never skips the pane to the dialog's Close.
   */
  staticPane?: boolean;
}) {
  const variant = React.useContext(TabsVariantContext);
  const pane = variant === "pane";
  return (
    <TabsPrimitive.Panel
      data-slot="tabs-content"
      // A pane with controls: Tab goes from the pane list straight to its
      // first one. A static pane: the panel is the stop (WAI-ARIA: a panel
      // is a Tab stop exactly when it holds no focusable content).
      tabIndex={tabIndex ?? (pane ? (staticPane ? 0 : -1) : undefined)}
      className={cn(pane ? "min-h-0 min-w-0 flex-1 overflow-y-auto" : "flex-1", pane ? insetFocusRing : focusRing, className)}
      {...props}
    />
  );
}

export { Tabs, TabsContent, TabsList, TabsTrigger };
