import { afterEach, describe, expect, it } from "vitest";
import { renderHook } from "@testing-library/react";
import type { FocusEvent } from "react";
import { useMatrixFocusReconciliation } from "./useMatrixFocusReconciliation";

/**
 * The matrix's post-filter focus rule (N2) against a real DOM: rows carry
 * `data-matrix-row`, cells `data-matrix-col`, the pressed chip
 * `data-matrix-filter`. The test plays React's part — it removes the row the
 * render dropped, then re-renders the hook with the new display order.
 */
function matrix(rows: readonly string[]) {
  const root = document.createElement("div");
  root.innerHTML = `
    <div role="group">
      <button data-matrix-filter="all" aria-pressed="false">All</button>
      <button data-matrix-filter="issues" aria-pressed="true">Issues</button>
    </div>
    <button data-outside>search</button>
    <table><tbody>${rows
      .map(
        (id) => `<tr data-matrix-row="${id}">
          <th>${id}</th>
          <td data-matrix-col="claude"><button role="checkbox" aria-label="${id} claude"></button><button data-sync="${id}">Sync this</button></td>
          <td data-matrix-col="pi"><button role="checkbox" aria-label="${id} pi"></button></td>
          <td><button data-verb="${id}">Unassign from all</button></td>
        </tr>`,
      )
      .join("")}</tbody></table>`;
  document.body.appendChild(root);
  return root;
}

const focusIn = (handlers: { onFocus: (e: FocusEvent<HTMLElement>) => void }, element: HTMLElement) => {
  element.focus();
  handlers.onFocus({ target: element } as unknown as FocusEvent<HTMLElement>);
};

function setup(rows: string[]) {
  const root = matrix(rows);
  const ref = { current: root };
  const rendered = renderHook(({ displayed }) => useMatrixFocusReconciliation(ref, displayed), {
    initialProps: { displayed: rows },
  });
  /** React's render: the row leaves the DOM, then the hook sees the new order. */
  const leave = (id: string, displayed: string[]) => {
    root.querySelector(`[data-matrix-row="${id}"]`)?.remove();
    rendered.rerender({ displayed });
  };
  return { root, rendered, leave };
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("useMatrixFocusReconciliation", () => {
  it("Sync this on a row's only issue under Issues: focus goes to the next row's cell in the same column", () => {
    const { root, rendered, leave } = setup(["a", "b", "c"]);
    focusIn(rendered.result.current, root.querySelector<HTMLElement>("[data-sync='b']")!);
    leave("b", ["a", "c"]);
    expect(document.activeElement).toBe(root.querySelector("[aria-label='c claude']"));
  });

  it("Unassign from all on the last row under Assigned: the previous row's first cell", () => {
    const { root, rendered, leave } = setup(["a", "b"]);
    focusIn(rendered.result.current, root.querySelector<HTMLElement>("[data-verb='b']")!);
    leave("b", ["a"]);
    expect(document.activeElement).toBe(root.querySelector("[aria-label='a claude']"));
  });

  it("the last cell unassigned under Assigned keeps its column", () => {
    const { root, rendered, leave } = setup(["a", "b"]);
    focusIn(rendered.result.current, root.querySelector<HTMLElement>("[aria-label='a pi']")!);
    leave("a", ["b"]);
    expect(document.activeElement).toBe(root.querySelector("[aria-label='b pi']"));
  });

  it("no row left: the active chip", () => {
    const { root, rendered, leave } = setup(["a"]);
    focusIn(rendered.result.current, root.querySelector<HTMLElement>("[data-sync='a']")!);
    leave("a", []);
    expect(document.activeElement).toBe(root.querySelector("[data-matrix-filter='issues']"));
  });

  it("successive repairs walk down the filtered rows", () => {
    const { root, rendered, leave } = setup(["a", "b", "c"]);
    focusIn(rendered.result.current, root.querySelector<HTMLElement>("[data-sync='a']")!);
    leave("a", ["b", "c"]);
    expect(document.activeElement).toBe(root.querySelector("[aria-label='b claude']"));
    leave("b", ["c"]);
    expect(document.activeElement).toBe(root.querySelector("[aria-label='c claude']"));
  });

  it("focus the operator placed elsewhere is never taken", () => {
    const { root, rendered, leave } = setup(["a", "b"]);
    focusIn(rendered.result.current, root.querySelector<HTMLElement>("[data-sync='a']")!);
    const outside = root.querySelector<HTMLElement>("[data-outside]")!;
    outside.focus();
    leave("a", ["b"]);
    expect(document.activeElement).toBe(outside);
  });

  it("a row still shown is left to the cell's own rule", () => {
    const { root, rendered } = setup(["a", "b"]);
    focusIn(rendered.result.current, root.querySelector<HTMLElement>("[data-sync='a']")!);
    root.querySelector("[data-sync='a']")!.remove(); // the cell healed; the row stays
    rendered.rerender({ displayed: ["a", "b"] });
    expect(document.activeElement).toBe(document.body);
  });

  it("focus on the toolbar (a chip, the search) holds no row", () => {
    const { root, rendered, leave } = setup(["a", "b"]);
    focusIn(rendered.result.current, root.querySelector<HTMLElement>("[data-sync='a']")!);
    focusIn(rendered.result.current, root.querySelector<HTMLElement>("[data-matrix-filter='all']")!);
    (document.activeElement as HTMLElement).blur();
    leave("a", ["b"]);
    expect(document.activeElement).toBe(document.body);
  });
});
