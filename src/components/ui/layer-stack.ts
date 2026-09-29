/**
 * The stack of open modal layers (Dialog, ConfirmDialog, the progress
 * overlay), most recently opened on top. It gives the Dialog primitive the
 * two guarantees Base UI cannot give sibling dialogs on its own:
 *
 *   - **Top layer only.** Escape and outside presses dismiss only the top
 *     layer; every lower layer ignores them (Base UI listens per dialog on
 *     the document, so two sibling dialogs would both close).
 *   - **Inert background.** While any layer is open, the app content (every
 *     body child that is not a Base UI portal) and every lower layer's popup
 *     carry `inert` — not only `aria-hidden` — so nothing behind the top
 *     layer can be focused, clicked or found.
 *
 * Module state, not React state: the stack is a property of the document,
 * and the dismissal check must read it synchronously inside a key event.
 */

type Layer = { id: string; popup: HTMLElement | null };

const stack: Layer[] = [];
/** Elements this module made inert, so it never clears someone else's. */
const inertedByUs = new Set<HTMLElement>();

export function openLayer(id: string): void {
  if (!stack.some((layer) => layer.id === id)) stack.push({ id, popup: null });
}

export function setLayerPopup(id: string, popup: HTMLElement | null): void {
  const layer = stack.find((entry) => entry.id === id);
  if (layer) layer.popup = popup;
}

export function closeLayer(id: string): void {
  const index = stack.findIndex((layer) => layer.id === id);
  if (index >= 0) stack.splice(index, 1);
  applyInert();
}

export function isTopLayer(id: string): boolean {
  return stack.length > 0 && stack[stack.length - 1].id === id;
}

/** The popup of the layer directly beneath `id`, for focus fallback. */
export function popupBelow(id: string): HTMLElement | null {
  const index = stack.findIndex((layer) => layer.id === id);
  return index > 0 ? stack[index - 1].popup : null;
}

export function openLayerCount(): number {
  return stack.length;
}

/** Re-derive `inert` from the stack. Idempotent. */
export function applyInert(): void {
  if (typeof document === "undefined") return;
  for (const element of inertedByUs) element.removeAttribute("inert");
  inertedByUs.clear();
  if (stack.length === 0) return;

  const mark = (element: HTMLElement | null) => {
    if (!element || element.hasAttribute("inert")) return;
    element.setAttribute("inert", "");
    inertedByUs.add(element);
  };

  for (const child of Array.from(document.body.children)) {
    if (!(child instanceof HTMLElement)) continue;
    if (child.hasAttribute("data-base-ui-portal")) continue;
    if (child.tagName === "SCRIPT" || child.tagName === "STYLE") continue;
    mark(child);
  }
  // Lower layers: their popups only (a nested dialog's portal can live
  // inside its parent's portal node, so never inert a whole portal).
  for (const layer of stack.slice(0, -1)) mark(layer.popup);
}
