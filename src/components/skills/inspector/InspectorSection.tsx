import { useId, type ReactNode } from "react";
import { cn } from "@/lib/utils";

type InspectorSectionProps = {
  title: string;
  /** Controls beside the heading (a view toggle, Expand). */
  actions?: ReactNode;
  /** Extra attributes for a focus target (`data-inspector-health`). */
  sectionProps?: Record<`data-${string}`, string> & { tabIndex?: number };
  /** Layout only: a section that fills the remaining height (Files expanded). */
  grow?: boolean;
  children: ReactNode;
};

/**
 * One inspector section: a named `<section>` (its `h3` labels it) with an
 * optional control row, separated from the next by a hairline.
 */
export default function InspectorSection({ title, actions, sectionProps, grow = false, children }: InspectorSectionProps) {
  const headingId = useId();
  return (
    <section
      aria-labelledby={headingId}
      {...sectionProps}
      className={cn(
        "flex flex-col gap-2 border-b border-border px-4 py-3 last:border-b-0",
        grow && "min-h-0 flex-1",
      )}
    >
      <div className="flex min-h-6 items-center gap-2">
        <h3 id={headingId} className="m-0 text-sm font-semibold text-foreground">
          {title}
        </h3>
        {actions ? <div className="ml-auto flex items-center gap-1">{actions}</div> : null}
      </div>
      {children}
    </section>
  );
}
