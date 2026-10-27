// The Toaster primitive's place on the layer stack (ticket 29): it renders into
// its own host — the first `body` child, marked LIVE_HOST_ATTRIBUTE — so the
// inert a modal layer puts on the app root never reaches its polite region.
// A primitive contract test (ADR-0006 §7): where it mounts and what stays in
// the accessibility tree, never pixels or copy.

import { act, render, waitFor } from "@testing-library/react";
import { toast } from "sonner";
import { afterEach, beforeAll, describe, expect, it } from "vitest";
import { LIVE_HOST_ATTRIBUTE, applyInert, closeLayer, openLayer, setLayerPopup } from "./layer-stack";
import { Toaster } from "./sonner";

beforeAll(() => {
  // sonner reads the colour scheme; jsdom has no matchMedia.
  window.matchMedia ??= ((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addEventListener: () => {},
    removeEventListener: () => {},
    addListener: () => {},
    removeListener: () => {},
    dispatchEvent: () => false,
  })) as unknown as typeof window.matchMedia;
});

afterEach(() => {
  act(() => toast.dismiss());
  closeLayer("modal");
  document.body.innerHTML = "";
});

/** The app root the binder renders into, with the Toaster inside it as App mounts it. */
function mountApp() {
  const root = document.createElement("div");
  root.id = "root";
  document.body.append(root);
  const portal = document.createElement("div");
  portal.setAttribute("data-base-ui-portal", "");
  const popup = document.createElement("div");
  portal.append(popup);
  const view = render(<Toaster position="top-right" />, { container: root });
  document.body.append(portal);
  return { root, popup, view };
}

const host = () => document.querySelector<HTMLElement>(`[${LIVE_HOST_ATTRIBUTE}]`);
const region = () => document.querySelector<HTMLElement>("[aria-live='polite']");

describe("Toaster host", () => {
  it("is the first body child, outside the app root, and holds the one polite region", () => {
    const { root } = mountApp();
    expect(document.body.firstElementChild).toBe(host());
    expect(root.contains(host())).toBe(false);
    expect(document.querySelectorAll("[aria-live]")).toHaveLength(1);
    expect(host()!.contains(region())).toBe(true);
  });

  it("keeps a toast raised under a modal layer out of the inert background", async () => {
    const { root, popup } = mountApp();
    openLayer("modal");
    setLayerPopup("modal", popup);
    applyInert();
    act(() => {
      toast.info("Disable is running — wait for it to finish", { description: "Nothing was changed." });
    });
    await waitFor(() => expect(document.querySelector("[data-sonner-toast]")).not.toBeNull());
    expect(root.hasAttribute("inert")).toBe(true);
    expect(region()!.closest("[inert]")).toBeNull();
    expect(region()!.textContent).toContain("Nothing was changed.");
  });

  it("leaves with the Toaster", () => {
    const { view } = mountApp();
    view.unmount();
    expect(host()).toBeNull();
  });
});
