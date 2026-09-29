import * as React from "react";
import { Tabs as TabsPrimitive } from "@base-ui/react/tabs";
import { cn } from "@/lib/utils";
import { disabledState, focusRing, iconSlot } from "./styles";

/**
 * Tabs — `role="tablist"` / `tab` / `tabpanel`, arrow keys move between
 * tabs (Base UI). Two looks on `TabsList`:
 *   - `line`: an underline marks the selected tab (section switching);
 *   - `segmented`: a filled segment marks it (a compact switch of views).
 * For an option set that does not switch panels, use `Segmented`.
 */

type TabsVariant = "line" | "segmented";
const TabsVariantContext = React.createContext<TabsVariant>("line");

function Tabs({
  className,
  ...props
}: Omit<TabsPrimitive.Root.Props, "className"> & { className?: string }) {
  return (
    <TabsPrimitive.Root
      data-slot="tabs"
      className={cn("flex flex-col gap-3", className)}
      {...props}
    />
  );
}

function TabsList({
  variant = "line",
  className,
  ...props
}: Omit<TabsPrimitive.List.Props, "className"> & {
  variant?: TabsVariant;
  /** Layout only. */
  className?: string;
}) {
  return (
    <TabsVariantContext.Provider value={variant}>
      <TabsPrimitive.List
        data-slot="tabs-list"
        data-variant={variant}
        className={cn(
          "inline-flex w-fit items-center",
          variant === "line"
            ? "gap-1 border-b border-border"
            : "h-8 gap-0.5 rounded-md border border-border-strong p-0.5",
          className,
        )}
        {...props}
      />
    </TabsVariantContext.Provider>
  );
}

function TabsTrigger({
  className,
  ...props
}: Omit<TabsPrimitive.Tab.Props, "className"> & { className?: string }) {
  const variant = React.useContext(TabsVariantContext);
  return (
    <TabsPrimitive.Tab
      data-slot="tabs-trigger"
      className={cn(
        "relative inline-flex cursor-default items-center justify-center gap-1.5 text-sm font-medium whitespace-nowrap text-muted-foreground select-none hover:text-foreground data-active:text-foreground",
        variant === "line"
          ? "-mb-px h-8 border-b-2 border-transparent px-2 data-active:border-ring"
          : "h-6 rounded-sm px-2.5 data-active:bg-raised data-active:ring-1 data-active:ring-ring data-active:ring-inset",
        focusRing,
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
  ...props
}: Omit<TabsPrimitive.Panel.Props, "className"> & { className?: string }) {
  return (
    <TabsPrimitive.Panel
      data-slot="tabs-content"
      className={cn("flex-1", focusRing, className)}
      {...props}
    />
  );
}

export { Tabs, TabsContent, TabsList, TabsTrigger };
