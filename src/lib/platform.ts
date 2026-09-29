// The one seam between the frontend and the native platform — everything
// that crosses into Tauri that is not an app command (those go through
// `invokeTauri`, `src/lib/tauri.ts`): the folder picker, progress channels
// for streamed commands, the app updater, webview zoom and the app version.
//
// Hooks and components call these functions and never import a Tauri plugin
// or `@tauri-apps/api/*` themselves; tests mock this module. The `isTauri`
// guard lives here: outside a Tauri webview every call rejects with
// `PlatformUnavailableError` (or, for zoom, does nothing — zoom is
// best-effort), instead of hanging on a missing IPC bridge.
//
// Two adapters sit behind the seam: the Tauri adapter below (plugins imported
// lazily, so none of them loads before it is used) and the fixture's
// (`src/fixtures/index.ts`, dev-only), installed through
// `installPlatformAdapter` before the first render.

import type { Channel } from "@tauri-apps/api/core";
import { isTauri } from "./tauri";

/** A release newer than the running app, as the updater found it. */
export type AppUpdateHandle = {
  version: string;
  /** Download the release and install it; the app restarts to finish. */
  downloadAndInstall: () => Promise<void>;
  /**
   * The release notes (Markdown): the full GitHub release body when it can
   * be fetched, else the updater manifest's notes, else null. Never rejects.
   */
  releaseNotes: () => Promise<string | null>;
};

/** What each adapter supplies; the exported functions add the one rule. */
export type PlatformAdapter = {
  /** The raw picker: selected folder paths, `[]` when cancelled. */
  pickFolder: (options: { title: string; multiple: boolean }) => Promise<string[]>;
  createProgressChannel: <T>(onMessage: (message: T) => void) => Promise<Channel<T>>;
  checkForUpdate: () => Promise<AppUpdateHandle | null>;
  setZoom: (factor: number) => Promise<void>;
  appVersion: () => Promise<string>;
};

/** A platform call made outside a Tauri webview (plain browser dev server). */
export class PlatformUnavailableError extends Error {
  constructor() {
    super("Tauri API is not available");
    this.name = "PlatformUnavailableError";
  }
}

const RELEASES_API = "https://api.github.com/repos/astarktc/skills-hub/releases/tags";

const tauriAdapter: PlatformAdapter = {
  async pickFolder({ title, multiple }) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({ directory: true, multiple, title });
    if (!selected) return [];
    return Array.isArray(selected) ? selected : [selected];
  },
  async createProgressChannel<T>(onMessage: (message: T) => void) {
    const { Channel } = await import("@tauri-apps/api/core");
    const channel = new Channel<T>();
    channel.onmessage = onMessage;
    return channel;
  },
  async checkForUpdate() {
    const { check } = await import("@tauri-apps/plugin-updater");
    const update = await check();
    if (!update) return null;
    const manifestNotes = update.body ?? null;
    return {
      version: update.version,
      downloadAndInstall: () => update.downloadAndInstall(),
      releaseNotes: async () => {
        try {
          const res = await fetch(`${RELEASES_API}/v${update.version}`);
          if (!res.ok) return manifestNotes;
          const data = (await res.json()) as { body?: string | null };
          return data.body ?? manifestNotes;
        } catch {
          return manifestNotes;
        }
      },
    };
  },
  async setZoom(factor) {
    const { getCurrentWebview } = await import("@tauri-apps/api/webview");
    await getCurrentWebview().setZoom(factor);
  },
  async appVersion() {
    const { getVersion } = await import("@tauri-apps/api/app");
    return getVersion();
  },
};

let adapter: PlatformAdapter = tauriAdapter;

/** Replace the Tauri adapter (the fixture does, once, before the first render). */
export function installPlatformAdapter(next: PlatformAdapter): void {
  adapter = next;
}

const unavailable = () => Promise.reject(new PlatformUnavailableError());

/**
 * The folder picker — the one rule for every "Browse…" button: a directory
 * dialog titled `title`, answering the picked paths (`[]` when the operator
 * cancelled; at most one unless `multiple`). A picker failure rejects; every
 * caller catches it onto its error surface.
 */
export function pickFolder({
  title,
  multiple = false,
}: {
  title: string;
  multiple?: boolean;
}): Promise<string[]> {
  if (!isTauri) return unavailable();
  return adapter
    .pickFolder({ title, multiple })
    .then((paths) => (multiple ? paths : paths.slice(0, 1)));
}

/**
 * A fresh progress channel for one streamed command invocation, delivering
 * each message to `onMessage`. Build one per call; never reuse it.
 */
export function createProgressChannel<T>(
  onMessage: (message: T) => void,
): Promise<Channel<T>> {
  if (!isTauri) return unavailable();
  return adapter.createProgressChannel(onMessage);
}

/** Ask the updater for a newer release: its handle, or null when current. */
export function checkForUpdate(): Promise<AppUpdateHandle | null> {
  if (!isTauri) return unavailable();
  return adapter.checkForUpdate();
}

/** Set the webview zoom factor. Best-effort: a no-op outside Tauri. */
export function setZoom(factor: number): Promise<void> {
  if (!isTauri) return Promise.resolve();
  return adapter.setZoom(factor);
}

/** The running app's version (e.g. `1.2.18`). */
export function appVersion(): Promise<string> {
  if (!isTauri) return unavailable();
  return adapter.appVersion();
}
