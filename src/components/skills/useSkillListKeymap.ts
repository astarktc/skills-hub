import { useCallback, useEffect, useLayoutEffect, useRef } from "react";
import type { SurfaceState } from "../../hooks/useSurfaceState";

/** Typing into these is never a shortcut. */
const isEditable = (target: EventTarget | null) =>
  target instanceof HTMLElement &&
  (target.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName));

/** A layer above the list (a dialog, the sheet, a popover, a menu) owns the keyboard. */
const layerOpen = () =>
  typeof document !== "undefined" &&
  document.querySelector("[role='dialog'], [role='alertdialog'], [role='menu']") !== null;

/**
 * A key another handler already consumed is not ours — except on a control
 * that is disabled but keeps focus (a pending or inapplicable button, e.g.
 * the rail's Disable after it ran): Base UI prevents the default there only
 * to block activation.
 */
const consumed = (event: KeyboardEvent) =>
  event.defaultPrevented &&
  !(event.target instanceof Element && event.target.closest("[aria-disabled='true'], [data-disabled]"));

const rowButtons = () => [...document.querySelectorAll<HTMLElement>("[data-skill-list] [data-skill-row]")];
const rowOf = (element: Element | null) =>
  element?.closest<HTMLElement>("[data-skill-item]")?.dataset.skillItem ?? null;

export type SkillListKeymapDeps = {
  /** My Skills is showing (the keymap is off everywhere else). */
  enabled: boolean;
  /** The lens's visible rows, in display order: ⌘A and a shift-click range read it. */
  order: readonly string[];
  surface: Pick<SurfaceState, "selection" | "select" | "toggle" | "clearSelection">;
  /** The skill the inspector shows, for `x` / `j` when focus is in no row. */
  focusedSkillId: string | null;
  /** Escape with nothing selected: the surface's own next step (collapse Files, close the inspector). */
  onEscape: () => void;
};

/**
 * My Skills' one keymap (ticket 13; the contract's j/k · x · Enter · Esc):
 *
 * - `j` / `k` move focus to the next / previous row (from outside a row: the
 *   inspector's skill, else the first row); Enter on a row opens it (the
 *   row's own button);
 * - `x` selects or deselects the row holding focus (else the inspector's skill);
 * - ⌘A / Ctrl+A selects every row the lens shows (the visible subset);
 * - `/` focuses the search;
 * - Escape clears the selection first, then takes the surface's next step.
 *
 * It never fires while typing, with a modifier it does not own, or while a
 * layer above the list holds the keyboard. Pointer selection shares its
 * anchor: a shift-click selects the range from the last row toggled
 * (`selectFromRow`); a group header selects its source (`selectGroup`).
 */
export function useSkillListKeymap({ enabled, order, surface, focusedSkillId, onEscape }: SkillListKeymapDeps) {
  const { selection, select, toggle, clearSelection } = surface;
  const selected = selection.skills;
  const anchor = useRef<string | null>(null);

  // The handler reads this render's values; it is registered once.
  const latest = useRef({ order, selected, focusedSkillId, onEscape });
  useLayoutEffect(() => {
    latest.current = { order, selected, focusedSkillId, onEscape };
  });

  /** Toggle one row; with `range`, select every visible row from the anchor to it. */
  const selectFromRow = useCallback(
    (skillId: string, range: boolean) => {
      const { order: rows, selected: current } = latest.current;
      const from = anchor.current ? rows.indexOf(anchor.current) : -1;
      const to = rows.indexOf(skillId);
      if (range && from >= 0 && to >= 0) {
        const [start, end] = from <= to ? [from, to] : [to, from];
        select("skills", new Set([...current, ...rows.slice(start, end + 1)]));
      } else {
        toggle("skills", skillId);
      }
      anchor.current = skillId;
    },
    [select, toggle],
  );

  /**
   * Clear the selection (Escape, the rail's ×). The rail goes with it, so
   * focus inside the rail moves first — to the row last toggled, else the
   * inspector's skill, else the first row — never to `<body>`.
   */
  const clear = useCallback(() => {
    const inRail = document.activeElement?.closest("[data-bulk-rail]") != null;
    const { focusedSkillId: inspected } = latest.current;
    const back = anchor.current;
    clearSelection("skills");
    anchor.current = null;
    if (!inRail) return;
    const buttons = rowButtons();
    const target =
      buttons.find((button) => button.dataset.skillRow === back) ??
      buttons.find((button) => button.dataset.skillRow === inspected) ??
      buttons[0] ??
      document.querySelector<HTMLElement>("[data-surface-heading='skills']");
    target?.focus();
  }, [clearSelection]);

  /** A group header's checkbox: every skill of the source in, or out. */
  const selectGroup = useCallback(
    (skillIds: readonly string[], on: boolean) => {
      const next = new Set(latest.current.selected);
      for (const id of skillIds) {
        if (on) next.add(id);
        else next.delete(id);
      }
      select("skills", next);
    },
    [select],
  );

  useEffect(() => {
    if (!enabled) return;
    const onKey = (event: KeyboardEvent) => {
      if (consumed(event) || event.altKey || isEditable(event.target) || layerOpen()) return;
      const { order: rows, selected: current, focusedSkillId: inspected, onEscape: next } = latest.current;
      const command = event.metaKey || event.ctrlKey;
      if (command) {
        if (event.key.toLowerCase() !== "a" || event.shiftKey) return;
        event.preventDefault();
        select("skills", new Set(rows));
        return;
      }
      if (event.key === "Escape") {
        if (current.size > 0) {
          event.preventDefault();
          clear();
          return;
        }
        next();
        return;
      }
      if (event.shiftKey) return;
      switch (event.key) {
        case "/": {
          const search = document.querySelector<HTMLInputElement>("[data-skill-search]");
          if (!search) return;
          event.preventDefault();
          search.focus();
          search.select();
          return;
        }
        case "j":
        case "k": {
          const buttons = rowButtons();
          if (buttons.length === 0) return;
          event.preventDefault();
          const here = rowOf(document.activeElement);
          const from = here ?? inspected;
          const index = from ? buttons.findIndex((button) => button.dataset.skillRow === from) : -1;
          const target =
            index < 0
              ? buttons[0]
              : here === null
                ? buttons[index]
                : buttons[Math.min(buttons.length - 1, Math.max(0, index + (event.key === "j" ? 1 : -1)))];
          target.focus();
          target.scrollIntoView?.({ block: "nearest" });
          return;
        }
        case "x": {
          const id = rowOf(document.activeElement) ?? inspected;
          if (!id || !rows.includes(id)) return;
          event.preventDefault();
          toggle("skills", id);
          anchor.current = id;
          return;
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [clear, enabled, select, toggle]);

  return { selectFromRow, selectGroup, clear };
}
