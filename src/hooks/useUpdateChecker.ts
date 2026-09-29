import { useCallback, useEffect, useRef, useState } from "react";
import type { MutableRefObject } from "react";
import type { Update } from "@tauri-apps/plugin-updater";
import { isTauri } from "../lib/tauri";
import { ignoredUpdateVersionPreference } from "../lib/preferences";
import type { StatusReporter } from "./useStatusReporter";
import type { DialogDoor } from "./useSurfaceState";

/**
 * App-update world: checks for a newer release on mount (unless the user
 * ignored that version), fetches its release notes, and drives the
 * download-and-install flow. It never touches the loading overlay (update
 * failures must not mix with skill-operation status), but an install
 * failure is a notification like any other, so it goes through the
 * reporter's `notify` and lands in the history.
 *
 * The found release is data; whether its dialog shows is the surface
 * module's (`{ kind: "update" }`), offered so it waits for any dialog
 * already up (the new-tools announcement raised at the same startup).
 */
export function useUpdateChecker({
  reporter,
  surface,
}: {
  reporter: Pick<StatusReporter, "notify" | "formatError">;
  surface: Pick<DialogDoor, "offerDialog" | "closeDialog">;
}) {
  const { notify, formatError } = reporter;
  const { offerDialog, closeDialog } = surface;
  const [updateAvailableVersion, setUpdateAvailableVersion] = useState<
    string | null
  >(null);
  const [updateBody, setUpdateBody] = useState<string | null>(null);
  const [updateInstalling, setUpdateInstalling] = useState(false);
  const [updateDone, setUpdateDone] = useState(false);
  const updateObjRef = useRef<Update | null>(
    null,
  ) as MutableRefObject<Update | null>;

  useEffect(() => {
    if (!isTauri) return;
    const ignoredVersion = ignoredUpdateVersionPreference.read();
    import("@tauri-apps/plugin-updater")
      .then(({ check }) => check())
      .then(async (update) => {
        if (update && update.version !== ignoredVersion) {
          updateObjRef.current = update;
          setUpdateAvailableVersion(update.version);
          offerDialog({ kind: "update" });
          // Fetch full release notes from GitHub API
          try {
            const res = await fetch(
              `https://api.github.com/repos/astarktc/skills-hub/releases/tags/v${update.version}`,
            );
            if (res.ok) {
              const data = await res.json();
              setUpdateBody(data.body ?? update.body ?? null);
            } else {
              setUpdateBody(update.body ?? null);
            }
          } catch {
            setUpdateBody(update.body ?? null);
          }
        }
      })
      .catch(() => {});
    // Mount-once check; `offerDialog` is a stable surface action.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const dismissUpdate = useCallback(() => {
    closeDialog("update");
  }, [closeDialog]);

  const dismissUpdateForever = useCallback(() => {
    if (updateAvailableVersion) {
      ignoredUpdateVersionPreference.write(updateAvailableVersion);
    }
    closeDialog("update");
  }, [closeDialog, updateAvailableVersion]);

  const updateNow = useCallback(async () => {
    const update = updateObjRef.current;
    if (!update) return;
    setUpdateInstalling(true);
    try {
      await update.downloadAndInstall();
      setUpdateInstalling(false);
      setUpdateDone(true);
    } catch (err) {
      setUpdateInstalling(false);
      const msg = formatError(err);
      if (msg) notify("error", msg);
    }
  }, [formatError, notify]);

  return {
    updateAvailableVersion,
    updateBody,
    updateInstalling,
    updateDone,
    dismissUpdate,
    dismissUpdateForever,
    updateNow,
  };
}
