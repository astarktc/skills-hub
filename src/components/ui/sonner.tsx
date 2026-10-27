import * as React from "react";
import { createPortal } from "react-dom";
// The one importer of the toast *component*: the binder mounts this Toaster
// once; useStatusReporter makes every toast() call and owns toast lifetime.
import { Toaster as Sonner, type ToasterProps } from "sonner";
import { LIVE_HOST_ATTRIBUTE } from "./layer-stack";

/**
 * Toaster — sonner, themed by tokens (not by sonner's own light/dark
 * palettes): every colour sonner draws is one of our variables, so a toast
 * reads the same pair in whichever theme the app shows, and no JS theme
 * value is needed. State toasts: the state colour as text on an 8 % tint of
 * it over the popover surface (measured ≥ 4.5:1 in both themes, DESIGN.md).
 * Sonner honours prefers-reduced-motion itself (and the base layer's
 * reduced-motion rule covers it too).
 *
 * Motion is the world's (DESIGN.md § Motion): sonner's own 400 ms slide,
 * fade and height glide become a 120 ms (`--motion-fast`) transform + opacity
 * on `ease-out` — the toast still enters from and leaves through the edge it
 * sits on (spatial consistency), it no longer animates its height (layout),
 * and a swipe-out, the stack's lift and its content fade share the same
 * 120 ms. Inline styles outrank sonner's injected (unlayered) CSS; the content
 * fade is on the toast's children, so it takes an important child utility.
 * Under reduced motion sonner's `transition: none !important` and the base
 * layer both still win.
 *
 * Where it lives (ticket 29): not inside the app root, which a modal layer
 * makes inert, but in its own host — the first `body` child, marked
 * `LIVE_HOST_ATTRIBUTE`, which the layer stack never inerts. So a toast raised
 * while a Dialog, ConfirmDialog or the inspector sheet stands open is announced
 * by sonner's one polite region and its action (View report) takes a pointer.
 * First in the body keeps its place in the Tab order (it was the app's first
 * child). The host is created with the Toaster and removed with it; the
 * interface (sonner's props) is unchanged.
 */
const tint = (token: string) =>
  `color-mix(in oklab, var(${token}) 8%, var(--popover))`;
const edge = (token: string) =>
  `color-mix(in oklab, var(${token}) 35%, var(--popover))`;

const tokenStyle = {
  "--normal-bg": "var(--popover)",
  "--normal-text": "var(--popover-foreground)",
  "--normal-border": "var(--border-strong)",
  "--normal-bg-hover": "var(--raised)",
  "--normal-border-hover": "var(--border-strong)",
  "--success-bg": tint("--ok"),
  "--success-text": "var(--ok)",
  "--success-border": edge("--ok"),
  "--warning-bg": tint("--stale"),
  "--warning-text": "var(--stale)",
  "--warning-border": edge("--stale"),
  "--error-bg": tint("--failed"),
  "--error-text": "var(--failed)",
  "--error-border": edge("--failed"),
  "--info-bg": "var(--popover)",
  "--info-text": "var(--popover-foreground)",
  "--info-border": "var(--border-strong)",
  "--border-radius": "var(--radius-md)",
} as React.CSSProperties;

const toasterMotion = {
  transitionDuration: "var(--motion-fast)",
  transitionTimingFunction: "var(--ease-out)",
} as React.CSSProperties;

const toastMotion = {
  transitionProperty: "transform, opacity",
  transitionDuration: "var(--motion-fast)",
  transitionTimingFunction: "var(--ease-out)",
  animationDuration: "var(--motion-fast)",
} as React.CSSProperties;

const toastContentMotion = "*:duration-(--motion-fast)! *:ease-out!";

function Toaster({ toastOptions, ...props }: ToasterProps) {
  // Created detached during render (no DOM mutation there); attached by the
  // layout effect before the first paint and before any toast can show.
  const [host] = React.useState(() => {
    const element = document.createElement("div");
    element.setAttribute(LIVE_HOST_ATTRIBUTE, "");
    return element;
  });
  React.useLayoutEffect(() => {
    document.body.prepend(host);
    return () => host.remove();
  }, [host]);
  return createPortal(
    <Sonner
      style={{ ...tokenStyle, ...toasterMotion }}
      toastOptions={{
        ...toastOptions,
        style: { ...toastMotion, ...toastOptions?.style },
        className: [toastContentMotion, toastOptions?.className].filter(Boolean).join(" "),
      }}
      {...props}
    />,
    host,
  );
}

export { Toaster };
