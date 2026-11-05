/**
 * The Settings dialog's panes and the Keyboard shortcuts dialog's (round-19
 * ticket 20, D37 / D50): pure vocabulary shared by the surface module (the
 * dialog state, the old-link redirect), the ⌘K palette and the dialogs.
 *
 * Settings is a dialog, not an area: a pane is dialog state, never an
 * address. `#/settings/<pane>` left the hash contract; an old link opens the
 * dialog on the pane it meant, once, and the address is cleaned
 * (`legacySettingsLink`).
 */

import type { AppUpdateStatus } from "../hooks/useAppUpdate";

/** The Settings panes, in pane-list order. */
export const SETTINGS_PANES = ["general", "harnesses", "appearance", "updates", "diagnostics"] as const;
export type SettingsPane = (typeof SETTINGS_PANES)[number];

/** A control the dialog focuses as it opens (Notifications' "Open Settings → GitHub"). */
export type SettingsField = "githubToken";

/** The pane each field lives on. */
export const SETTINGS_FIELD_PANE: Readonly<Record<SettingsField, SettingsPane>> = {
  githubToken: "diagnostics",
};

/** Each field's control id: the dialog's opening focus (and Notifications' focus return) find it by this. */
export const SETTINGS_FIELD_ID: Readonly<Record<SettingsField, string>> = {
  githubToken: "settings-github-token",
};

/** The Keyboard shortcuts panes (the same frame as Settings). */
export const SHORTCUT_PANES = ["general", "skills"] as const;
export type ShortcutPane = (typeof SHORTCUT_PANES)[number];

export const isSettingsPane = (value: string): value is SettingsPane =>
  (SETTINGS_PANES as readonly string[]).includes(value);

/** Pane names an old `#/settings/<pane>` link may carry that are not today's panes. */
const LEGACY_PANES: Readonly<Record<string, { pane: SettingsPane; field: SettingsField | null }>> = {
  github: { pane: "diagnostics", field: "githubToken" },
  tools: { pane: "harnesses", field: null },
  sync: { pane: "general", field: null },
};

/**
 * What an old Settings address (`#/settings`, `#/settings/<pane>`) asks for,
 * or null when the hash is not one: a known pane opens on itself, the 1.3
 * names on their successors (`github` → Diagnostics' token field), anything
 * else on General.
 */
export function legacySettingsLink(hash: string): { pane: SettingsPane; field: SettingsField | null } | null {
  const match = /^#\/settings(?:\/([^/]+))?\/?$/.exec(hash);
  if (!match) return null;
  let raw = match[1] ?? "";
  try {
    raw = decodeURIComponent(raw);
  } catch {
    raw = "";
  }
  if (isSettingsPane(raw)) return { pane: raw, field: null };
  return LEGACY_PANES[raw] ?? { pane: "general", field: null };
}

/** Whether Settings → Updates offers Check: a native build with no check or install under way. */
export function updateCheckOffered(status: AppUpdateStatus, isTauri: boolean): boolean {
  return isTauri && (status === "idle" || status === "up-to-date" || status === "error");
}

/**
 * Whether Settings → Updates holds any control now (Check, or a found
 * release's Download & Install / Don't remind again); without one the frame
 * makes the pane itself the Tab stop (`PaneSpec.static`).
 */
export function updatesPaneHasControls(status: AppUpdateStatus, isTauri: boolean): boolean {
  return updateCheckOffered(status, isTauri) || status === "available";
}
