// The layer stack's focus-return question (ticket 12): a closing dialog
// decides where focus goes while it is still open — when everything behind
// it carries the stack's own `inert` — so that inert must not disqualify
// the opener. Only inert that outlives the layer does.

import { afterEach, describe, expect, it } from "vitest";
import {
  LIVE_HOST_ATTRIBUTE,
  applyInert,
  claimDismissal,
  closeLayer,
  hasLayerAbove,
  openLayer,
  setLayerPopup,
  staysInertAfterClose,
} from "./layer-stack";

const byId = (id: string) => document.getElementById(id)!;

afterEach(() => {
  closeLayer("c");
  closeLayer("b");
  closeLayer("a");
  document.body.innerHTML = "";
});

describe("staysInertAfterClose", () => {
  it("does not count the inert the closing layer applied", () => {
    document.body.innerHTML = `<div id="app"><button id="opener">Re-point</button></div><div data-base-ui-portal><div id="popup-a"></div></div>`;
    openLayer("a");
    setLayerPopup("a", byId("popup-a"));
    applyInert();
    expect(byId("opener").closest("[inert]")).not.toBeNull();
    expect(staysInertAfterClose("a", byId("opener"))).toBe(false);
  });

  it("counts inert that outlives the layer: the app under a remaining layer, someone else's inert", () => {
    document.body.innerHTML = `<div id="app"><button id="row">docx</button><div inert id="foreign"><button id="inside">x</button></div></div><div data-base-ui-portal><div id="popup-a"><button id="in-a">Restore</button></div><div id="popup-b"></div></div>`;
    openLayer("a");
    setLayerPopup("a", byId("popup-a"));
    openLayer("b");
    setLayerPopup("b", byId("popup-b"));
    applyInert();
    // Closing b leaves a on top: its popup is live again, the app is not.
    expect(staysInertAfterClose("b", byId("in-a"))).toBe(false);
    expect(staysInertAfterClose("b", byId("row"))).toBe(true);
    // An inert the stack did not apply stays whatever closes.
    closeLayer("b");
    expect(staysInertAfterClose("a", byId("inside"))).toBe(true);
  });
});

describe("floating layers (ticket 16 A1: a popover and the ask one of its controls raised)", () => {
  const html = `<div id="app"><button id="opener">Add skill</button></div><div data-base-ui-portal><div id="popup-a"><button id="trigger">Edit Tools</button></div></div><div data-base-ui-portal><div id="popup-b"><button id="amp">Amp</button></div></div><div data-base-ui-portal><div id="popup-c"></div></div>`;
  const floating = { opener: null, fallback: null };

  it("a floating layer alone makes nothing inert", () => {
    document.body.innerHTML = html;
    openLayer("b", floating, "floating");
    setLayerPopup("b", byId("popup-b"));
    applyInert();
    expect(document.querySelector("[inert]")).toBeNull();
  });

  it("is live above the top modal layer and inert beneath one", () => {
    document.body.innerHTML = html;
    openLayer("a");
    setLayerPopup("a", byId("popup-a"));
    openLayer("b", floating, "floating");
    setLayerPopup("b", byId("popup-b"));
    applyInert();
    expect(byId("app").hasAttribute("inert")).toBe(true);
    expect(byId("popup-a").hasAttribute("inert")).toBe(false);
    expect(byId("popup-b").hasAttribute("inert")).toBe(false);

    openLayer("c");
    setLayerPopup("c", byId("popup-c"));
    applyInert();
    expect(byId("popup-a").hasAttribute("inert")).toBe(true);
    expect(byId("popup-b").hasAttribute("inert")).toBe(true);
    // Closing the ask makes the popover's checkbox live again; the app stays inert under Add.
    expect(staysInertAfterClose("c", byId("amp"))).toBe(false);
    expect(staysInertAfterClose("c", byId("trigger"))).toBe(false);
    expect(staysInertAfterClose("c", byId("opener"))).toBe(true);
  });

  it("hands over focus return only to a modal layer above", () => {
    openLayer("a");
    openLayer("b", floating, "floating");
    expect(hasLayerAbove("a")).toBe(false);
    openLayer("c");
    expect(hasLayerAbove("a")).toBe(true);
    expect(hasLayerAbove("b")).toBe(true);
  });

  it("one event dismisses one layer: only the top one, and only once", () => {
    openLayer("a");
    openLayer("b", floating, "floating");
    const escape = new KeyboardEvent("keydown", { key: "Escape" });
    expect(claimDismissal("a", escape)).toBe(false);
    expect(claimDismissal("b", escape)).toBe(true);
    closeLayer("b");
    // The popover has left the stack; the dialog beneath still ignores the same Escape.
    expect(claimDismissal("a", escape)).toBe(false);
    expect(claimDismissal("a", new KeyboardEvent("keydown", { key: "Escape" }))).toBe(true);
  });
});

describe("the live host (ticket 29: a toast raised under a modal is announced)", () => {
  const html = `<div ${LIVE_HOST_ATTRIBUTE} id="live"><section aria-live="polite" id="region"><button id="view">View report</button></section></div><div id="app"><button id="opener">Delete</button></div><div data-base-ui-portal><div id="popup-a"></div></div><div data-base-ui-portal><div id="popup-b"></div></div><div id="stray"></div>`;

  it("is never inerted while a modal layer stands; every other body child still is", () => {
    document.body.innerHTML = html;
    openLayer("a");
    setLayerPopup("a", byId("popup-a"));
    applyInert();
    expect(byId("live").hasAttribute("inert")).toBe(false);
    expect(byId("region").closest("[inert]")).toBeNull();
    expect(byId("view").closest("[inert]")).toBeNull();
    expect(byId("app").hasAttribute("inert")).toBe(true);
    expect(byId("stray").hasAttribute("inert")).toBe(true);
    expect(byId("popup-a").closest("[inert]")).toBeNull();
  });

  it("stays live under nested modal layers, and the layer beneath still goes inert", () => {
    document.body.innerHTML = html;
    openLayer("a");
    setLayerPopup("a", byId("popup-a"));
    openLayer("b");
    setLayerPopup("b", byId("popup-b"));
    applyInert();
    expect(byId("live").closest("[inert]")).toBeNull();
    expect(byId("popup-a").hasAttribute("inert")).toBe(true);
    expect(byId("app").hasAttribute("inert")).toBe(true);
    closeLayer("b");
    closeLayer("a");
    expect(document.querySelector("[inert]")).toBeNull();
  });

  it("is not a layer: it claims no event, so Escape still dismisses the top layer", () => {
    document.body.innerHTML = html;
    openLayer("a");
    const escape = new KeyboardEvent("keydown", { key: "Escape" });
    byId("view").dispatchEvent(escape);
    expect(claimDismissal("a", escape)).toBe(true);
  });
});
