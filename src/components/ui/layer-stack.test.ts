// The layer stack's focus-return question (ticket 12): a closing dialog
// decides where focus goes while it is still open — when everything behind
// it carries the stack's own `inert` — so that inert must not disqualify
// the opener. Only inert that outlives the layer does.

import { afterEach, describe, expect, it } from "vitest";
import { applyInert, closeLayer, openLayer, setLayerPopup, staysInertAfterClose } from "./layer-stack";

const byId = (id: string) => document.getElementById(id)!;

afterEach(() => {
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
