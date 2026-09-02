import { useCallback, useEffect, useRef, useState } from "react";
import type { MutableRefObject } from "react";
import type { Update } from "@tauri-apps/plugin-updater";
import { toast } from "sonner";
import { describeCommandError } from "../commandError";
import { isTauri } from "../lib/tauri";
import { ignoredUpdateVersionPreference } from "../lib/preferences";
import type { TranslateFn } from "./useStatusReporter";

/**
 * App-update world: checks for a newer release on mount (unless the user
 * ignored that version), fetches its release notes, and drives the
 * download-and-install flow. Self-contained — reports through its own toast,
 * not the shared status reporter, because update failures should never mix
 * with skill-operation status.
 */
export function useUpdateChecker(t: TranslateFn) {
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
  }, []);

  const dismissUpdate = useCallback(() => {
    setUpdateAvailableVersion(null);
    setUpdateBody(null);
  }, []);

  const dismissUpdateForever = useCallback(() => {
    if (updateAvailableVersion) {
      ignoredUpdateVersionPreference.write(updateAvailableVersion);
    }
    setUpdateAvailableVersion(null);
    setUpdateBody(null);
  }, [updateAvailableVersion]);

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
      const msg = describeCommandError(err, t);
      if (msg) toast.error(msg, { duration: 3200 });
    }
  }, [t]);

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
