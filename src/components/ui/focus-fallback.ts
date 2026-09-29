/**
 * The app-owned last resort for returning focus when a layer closes and the
 * element that opened it is gone (a deleted row, a removed project): the
 * surface's main landmark. It is made programmatically focusable
 * (`tabindex="-1"`, never in the Tab order), so focus lands on the surface
 * the operator is working in, never on `<body>` — where a screen reader
 * restarts from the top and the next Tab starts from the document start.
 *
 * Dialog uses it after `fallbackFocus` and the layer beneath; removal dialogs
 * pass it as their `fallbackFocus` to say "my opener disappears". Ticket 09's
 * surface lifecycle is to supply the row-level target (the next row, then the
 * list heading) in front of it.
 */
export function mainLandmarkFocus(): HTMLElement | null {
  if (typeof document === "undefined") return null;
  const main = document.querySelector<HTMLElement>("main, [role='main']");
  if (!main || !main.isConnected || main.closest("[inert]")) return null;
  if (!main.hasAttribute("tabindex")) main.setAttribute("tabindex", "-1");
  return main;
}
