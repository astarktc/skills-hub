/**
 * The stack of open layers, most recently opened on top: **modal** layers
 * (Dialog, ConfirmDialog, the overwrite and shared-dir asks) and **floating**
 * ones (Popover). It gives the primitives the guarantees Base UI cannot give
 * layers that live in sibling portals on its own:
 *
 *   - **Top layer only.** Escape, outside presses — and, for a floating
 *     layer, focus leaving it — dismiss only the top layer; every lower layer
 *     ignores them, and one event dismisses one layer (`claimDismissal`).
 *     Base UI listens per layer on the document, so two sibling dialogs —
 *     or a popover and the confirmation one of its checkboxes raised (Add's
 *     Tool scope → the shared-folder ask) — would both close.
 *   - **Inert background.** While any modal layer is open, the app content
 *     (every body child that is not a Base UI portal or the live host) and every layer's popup
 *     beneath the top modal layer carry `inert` — not only `aria-hidden` — so
 *     nothing behind it can be focused, clicked or found. A floating layer
 *     makes nothing inert: it is non-modal; above the top modal layer it is
 *     live, beneath one it is inert like a lower dialog.
 *   - **The live host is never inert.** The toaster renders into its own
 *     `body` child marked `LIVE_HOST_ATTRIBUTE` (first in the body, so its
 *     place in the Tab order is the one it had as the app's first child).
 *     It is not a layer: it stays live above every layer — a toast raised
 *     under a modal is announced by its polite region and its action takes a
 *     pointer — dismisses nothing and claims no event. The modal's focus trap
 *     still holds: Tab cycles inside the top layer's popup, and Escape still
 *     reaches the top layer (sonner only collapses its expanded stack).
 *
 * It also carries each layer's **return chain** — the element that opened it
 * and its caller's fallback — so a layer closing *beneath* another (a count
 * confirmation dismissed while the overwrite ask its verb raised is up) hands its
 * chain to the layer it opened instead of aiming focus into an inert page.
 *
 * Module state, not React state: the stack is a property of the document,
 * and the dismissal check must read it synchronously inside a key event.
 */

/**
 * A caller's focus target for when a layer's opener is gone. It receives the
 * closing layer's `usable` test (connected, and not inert once the layer has
 * closed) so it can walk its own chain to the first target that will take
 * focus; whatever it answers is tested again.
 */
export type FocusFallback = (usable: (element: Element) => boolean) => HTMLElement | null;

/** Where a layer returns focus: its opener, then its caller's fallback. */
export type ReturnChain = { opener: HTMLElement | null; fallback: FocusFallback | null };

/** `modal` (a Dialog) or `floating` (a Popover: non-modal, inert only beneath a modal layer). */
export type LayerKind = "modal" | "floating";

type Layer = {
  id: string;
  kind: LayerKind;
  popup: HTMLElement | null;
  own: ReturnChain;
  /** Chains handed over by layers that closed beneath this one (nearest first). */
  inherited: ReturnChain[];
};

/**
 * The mark on the toaster's host (`ui/sonner.tsx`): a `body` child that
 * `applyInert` never makes inert, the way it skips Base UI's portals.
 */
export const LIVE_HOST_ATTRIBUTE = "data-skillshub-live";

const stack: Layer[] = [];
/** Elements this module made inert, so it never clears someone else's. */
const inertedByUs = new Set<HTMLElement>();
/** Events a layer has already been dismissed by: a lower layer never acts on the same one. */
const claimed = new WeakSet<Event>();

export function openLayer(
  id: string,
  own: ReturnChain = { opener: null, fallback: null },
  kind: LayerKind = "modal",
): void {
  if (!stack.some((layer) => layer.id === id)) stack.push({ id, kind, popup: null, own, inherited: [] });
}

/** Whether `id` is open (registered) at all. */
export function isOpenLayer(id: string): boolean {
  return stack.some((layer) => layer.id === id);
}

/**
 * Whether `id` is open with a **modal** layer above it — one that takes over
 * its focus return. A floating layer above it (its own popover) does not:
 * it goes with it.
 */
export function hasLayerAbove(id: string): boolean {
  const index = stack.findIndex((layer) => layer.id === id);
  return index >= 0 && stack.slice(index + 1).some((layer) => layer.kind === "modal");
}

/**
 * May `id` be dismissed by `event` (an Escape, an outside press, focus
 * leaving a floating layer)? Only the top layer may, and only once per
 * event: the answer `true` claims the event, so a layer beneath whose
 * listener runs later — after the top one has already left the stack —
 * still ignores it. A missing event (an imperative close) claims nothing.
 */
export function claimDismissal(id: string, event: Event | undefined): boolean {
  if (!isTopLayer(id)) return false;
  if (event) {
    if (claimed.has(event)) return false;
    claimed.add(event);
  }
  return true;
}

/**
 * Layer `id` is closing beneath another: every layer above it that it
 * opened — its opener lies inside this layer's popup, or is already gone —
 * inherits this layer's return chain (and what it had inherited), so the
 * layer that closes last returns focus where this one would have.
 */
export function handOverReturn(id: string): void {
  const index = stack.findIndex((layer) => layer.id === id);
  if (index < 0) return;
  const closing = stack[index];
  const chains = [closing.own, ...closing.inherited];
  for (const above of stack.slice(index + 1)) {
    const opener = above.own.opener;
    const openedHere = !opener || !opener.isConnected || (closing.popup?.contains(opener) ?? false);
    if (openedHere) above.inherited = [...chains, ...above.inherited];
  }
}

/** A layer's return chains, in the order it tries them: its own, then the inherited ones. */
export function returnChains(id: string): ReturnChain[] {
  const layer = stack.find((entry) => entry.id === id);
  return layer ? [layer.own, ...layer.inherited] : [];
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

/**
 * Whether `element` will still be inert once layer `id` has closed — the
 * question a closing dialog asks of its focus target. The dialog decides
 * where focus returns *while it is still open*, when everything behind it
 * carries this module's `inert`; that inert goes with the layer. What stays
 * inert: someone else's `inert` ancestor, and — while a modal layer remains
 * — anything outside the popups of the top remaining modal layer and of the
 * floating layers above it (a checkbox in the popover that raised the
 * closing ask is live again).
 */
export function staysInertAfterClose(id: string, element: Element): boolean {
  for (let node: Element | null = element; node; node = node.parentElement) {
    if (node.hasAttribute("inert") && !inertedByUs.has(node as HTMLElement)) return true;
  }
  const remaining = stack.filter((layer) => layer.id !== id);
  const topModal = lastModalIndex(remaining);
  if (topModal < 0) return false;
  return !remaining.slice(topModal).some((layer) => layer.popup?.contains(element) ?? false);
}

function lastModalIndex(layers: Layer[]): number {
  for (let index = layers.length - 1; index >= 0; index -= 1) {
    if (layers[index].kind === "modal") return index;
  }
  return -1;
}

export function openLayerCount(): number {
  return stack.length;
}

/** Re-derive `inert` from the stack. Idempotent. */
export function applyInert(): void {
  if (typeof document === "undefined") return;
  for (const element of inertedByUs) element.removeAttribute("inert");
  inertedByUs.clear();
  const topModal = lastModalIndex(stack);
  if (topModal < 0) return;

  const mark = (element: HTMLElement | null) => {
    if (!element || element.hasAttribute("inert")) return;
    element.setAttribute("inert", "");
    inertedByUs.add(element);
  };

  for (const child of Array.from(document.body.children)) {
    if (!(child instanceof HTMLElement)) continue;
    if (child.hasAttribute("data-base-ui-portal")) continue;
    if (child.hasAttribute(LIVE_HOST_ATTRIBUTE)) continue;
    if (child.tagName === "SCRIPT" || child.tagName === "STYLE") continue;
    mark(child);
  }
  // Layers beneath the top modal one: their popups only (a nested dialog's
  // portal can live inside its parent's portal node, so never inert a whole
  // portal). Floating layers above it stay live.
  for (const layer of stack.slice(0, topModal)) mark(layer.popup);
}
