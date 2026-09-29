import type * as React from "react";
// The one importer of the toast *component*: the binder mounts this Toaster
// once; useStatusReporter makes every toast() call and owns toast lifetime.
import { Toaster as Sonner, type ToasterProps } from "sonner";

/**
 * Toaster — sonner, themed by tokens (not by sonner's own light/dark
 * palettes): every colour sonner draws is one of our variables, so a toast
 * reads the same pair in whichever theme the app shows, and no JS theme
 * value is needed. State toasts: the state colour as text on an 8 % tint of
 * it over the popover surface (measured ≥ 4.5:1 in both themes, DESIGN.md).
 * Sonner honours prefers-reduced-motion itself (and the base layer's
 * reduced-motion rule covers it too).
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

function Toaster(props: ToasterProps) {
  return <Sonner style={tokenStyle} {...props} />;
}

export { Toaster };
