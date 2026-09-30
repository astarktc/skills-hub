import { useEffect } from "react";

const TABBABLE =
  "a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])";

/** An element's keyboard stops, in DOM order (visible, enabled, not inert). */
export function tabbablesIn(root: Element): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(TABBABLE)].filter(
    (element) => element.tabIndex >= 0 && !element.closest("[inert]") && element.getClientRects().length > 0,
  );
}

const escape = (value: string) => (typeof CSS !== "undefined" && CSS.escape ? CSS.escape(value) : value);

/**
 * "Tab enters after the row" (ticket 12): while the inspector sits beside
 * the list, it follows its row in the keyboard order — Tab from the row's
 * last control lands on the inspector's heading, Tab from the inspector's
 * last control continues with the next row, and Shift+Tab walks the same
 * path back. The DOM keeps list then inspector; only the two seams move.
 */
export function useInspectorTabOrder(skillId: string | null, enabled: boolean) {
  useEffect(() => {
    if (!skillId || !enabled) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Tab" || event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey) return;
      const active = document.activeElement;
      if (!(active instanceof HTMLElement)) return;
      const row = document.querySelector(`[data-skill-row="${escape(skillId)}"]`)?.closest("li");
      const inspector = document.querySelector<HTMLElement>(`aside[data-inspector="${escape(skillId)}"]`);
      if (!row || !inspector) return;
      const heading = inspector.querySelector<HTMLElement>("[data-inspector-heading]");
      const rowStops = tabbablesIn(row);
      const inspectorStops = tabbablesIn(inspector);
      // What natively follows the row's last control in the list (the next
      // row, or the next group's header) is where the inspector hands back.
      const list = row.closest("[data-skill-list]");
      const listStops = list ? tabbablesIn(list) : [];
      const lastRowStop = rowStops[rowStops.length - 1];
      const nextStop = lastRowStop ? (listStops[listStops.indexOf(lastRowStop) + 1] ?? null) : null;
      const move = (target: HTMLElement | null | undefined) => {
        if (!target) return;
        event.preventDefault();
        target.focus();
      };
      if (!event.shiftKey) {
        if (active === rowStops[rowStops.length - 1]) move(heading ?? inspectorStops[0]);
        else if (active === inspectorStops[inspectorStops.length - 1] && nextStop) move(nextStop);
      } else {
        if (active === heading || (active === inspectorStops[0] && !heading)) move(rowStops[rowStops.length - 1]);
        else if (active === inspectorStops[0] && heading) move(heading);
        else if (nextStop && active === nextStop) move(inspectorStops[inspectorStops.length - 1]);
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [enabled, skillId]);
}
