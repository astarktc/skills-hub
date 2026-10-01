import { useCallback, useLayoutEffect, useRef, type FocusEvent, type RefObject } from "react";
import { isFocusLost } from "../../lib/actionLifecycle";
import { matrixFocusAfterLeave } from "../../lib/projectPresentation";

/** Where focus was inside the matrix: a row, and the cell's Tool column (null on a row verb). */
type HeldFocus = { skillId: string; column: string | null };

/**
 * Post-filter focus reconciliation for the assignment matrix (N2): the
 * matrix — not a cell — remembers which row and column held focus, and when
 * a render takes that row out of the shown rows (Sync this on a row's only
 * issue under Issues, Unassign from all or the last cell under Assigned)
 * while focus is lost with it, focus moves to the same column's checkbox in
 * the next surviving row, else the previous, else the active filter chip
 * (`matrixFocusAfterLeave`). Focus the operator placed elsewhere is never
 * taken. The DOM contract: rows carry `data-matrix-row`, cells
 * `data-matrix-col`, the pressed chip `data-matrix-filter` + `aria-pressed`.
 */
export function useMatrixFocusReconciliation(
  rootRef: RefObject<HTMLElement | null>,
  displayed: readonly string[],
) {
  const held = useRef<HeldFocus | null>(null);
  const previous = useRef(displayed);

  useLayoutEffect(() => {
    const before = previous.current;
    previous.current = displayed;
    const last = held.current;
    const root = rootRef.current;
    if (!last || !root || !isFocusLost(document.activeElement)) return;
    const target = matrixFocusAfterLeave(before, displayed, last.skillId);
    if (!target) return;
    const chip = root.querySelector<HTMLElement>("[data-matrix-filter][aria-pressed='true']");
    if (target.kind === "row") {
      const row = root.querySelector<HTMLElement>(`[data-matrix-row="${CSS.escape(target.skillId)}"]`);
      const cell =
        (last.column &&
          row?.querySelector<HTMLElement>(`[data-matrix-col="${CSS.escape(last.column)}"] [role='checkbox']`)) ||
        row?.querySelector<HTMLElement>("[role='checkbox']");
      if (cell) {
        cell.focus();
        if (document.activeElement === cell) {
          held.current = { skillId: target.skillId, column: last.column };
          return;
        }
      }
    }
    held.current = null;
    chip?.focus();
  }, [displayed, rootRef]);

  const onFocus = useCallback((event: FocusEvent<HTMLElement>) => {
    const target = event.target as Element;
    const row = target.closest<HTMLElement>("[data-matrix-row]");
    held.current = row
      ? {
          skillId: row.dataset.matrixRow ?? "",
          column: target.closest<HTMLElement>("[data-matrix-col]")?.dataset.matrixCol ?? null,
        }
      : null;
  }, []);

  const onBlur = useCallback((event: FocusEvent<HTMLElement>) => {
    // Focus moved somewhere else on the page: the matrix holds nothing. A
    // blur from the focused control's own removal keeps the record.
    const next = event.relatedTarget as Element | null;
    if (next && next.isConnected && !event.currentTarget.contains(next)) held.current = null;
  }, []);

  return { onFocus, onBlur };
}
