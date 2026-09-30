import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  appVersion,
  checkForUpdate,
  type AppUpdateHandle,
} from "../lib/platform";
import { ignoredUpdateVersionPreference } from "../lib/preferences";
import type { StatusReporter } from "./useStatusReporter";
import type { DialogDoor } from "./useSurfaceState";

/**
 * Where the one updater lifecycle stands:
 * `idle` (nothing asked yet, or a silent startup check found nothing it may
 * say) → `checking` → `up-to-date` | `available` → `installing` → `done`;
 * `error` when a check the operator asked for, or an install, failed.
 */
export type AppUpdateStatus =
  | "idle"
  | "checking"
  | "up-to-date"
  | "available"
  | "installing"
  | "done"
  | "error";

/**
 * App-update world — the one updater lifecycle, read by Settings, the
 * passive header item (`UpdateAffordance`, D8) and the update dialog it
 * opens (`UpdateDialog`). A version found at startup is the version Settings
 * shows, and a version found from Settings is the one the dialog installs.
 *
 * On mount it reads the running version and checks once, silently: a
 * failure leaves it `idle`, and a found release the operator ignored
 * ("Don't remind me") stays `idle` too. A found release that may be
 * announced is only *available* — a quiet item in the shell ("1.3.1
 * available") that opens the dialog (`open`); nothing is raised on startup.
 * Settings' `check()` is the operator's own ask: it reports every outcome,
 * the ignored version included, and never raises the dialog.
 *
 * It never touches the header progress (update failures must not mix with
 * skill-operation status), but an install failure is a notification like
 * any other, so it also lands in the history through the reporter.
 */
export function useAppUpdate({
  reporter,
  surface,
}: {
  reporter: Pick<StatusReporter, "notify" | "formatError">;
  surface: Pick<DialogDoor, "openDialog" | "closeDialog">;
}) {
  const { notify, formatError } = reporter;
  const { openDialog, closeDialog } = surface;
  const [status, setStatus] = useState<AppUpdateStatus>("idle");
  const [version, setVersion] = useState<string | null>(null);
  const [notes, setNotes] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [currentVersion, setCurrentVersion] = useState<string | null>(null);
  // "Don't remind me" this session: the passive item stops announcing it.
  const [ignoredVersion, setIgnoredVersion] = useState<string | null>(null);
  const handleRef = useRef<AppUpdateHandle | null>(null);
  // A check answers only if it is still the latest one asked.
  const checkSeq = useRef(0);

  /** Adopt a found release: its version, then its notes when they land. */
  const adopt = useCallback((handle: AppUpdateHandle) => {
    const known = handleRef.current?.version === handle.version;
    handleRef.current = handle;
    setVersion(handle.version);
    setStatus("available");
    if (known) return;
    setNotes(null);
    void handle.releaseNotes().then((body) => {
      if (handleRef.current === handle) setNotes(body);
    });
  }, []);

  const runCheck = useCallback(
    async (mode: "startup" | "operator") => {
      const seq = ++checkSeq.current;
      setStatus("checking");
      setError(null);
      try {
        const handle = await checkForUpdate();
        if (seq !== checkSeq.current) return;
        if (!handle) {
          setStatus(mode === "startup" ? "idle" : "up-to-date");
          return;
        }
        if (
          mode === "startup" &&
          handle.version === ignoredUpdateVersionPreference.read()
        ) {
          setStatus("idle");
          return;
        }
        adopt(handle);
      } catch (err) {
        if (seq !== checkSeq.current) return;
        if (mode === "startup") {
          setStatus("idle");
          return;
        }
        setError(formatError(err) ?? "");
        setStatus("error");
      }
    },
    [adopt, formatError],
  );

  useEffect(() => {
    void (async () => {
      try {
        setCurrentVersion(await appVersion());
      } catch {
        setCurrentVersion(null);
      }
    })();
    void (async () => {
      await runCheck("startup");
    })();
    // Mount-once: the startup check; `runCheck` is stable after mount.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /** The operator's "Check for updates". */
  const check = useCallback(() => runCheck("operator"), [runCheck]);

  /** Download and install the found release; the app restarts to finish. */
  const install = useCallback(async () => {
    const handle = handleRef.current;
    if (!handle) return;
    setStatus("installing");
    setError(null);
    try {
      await handle.downloadAndInstall();
      setStatus("done");
    } catch (err) {
      const message = formatError(err);
      setError(message ?? "");
      // The release is still there to retry.
      setStatus("available");
      if (message) notify("error", message);
    }
  }, [formatError, notify]);

  /** The passive item's press: the update dialog, through surface state. */
  const open = useCallback(() => {
    openDialog({ kind: "update" });
  }, [openDialog]);

  /** Close the dialog; the release stays known (Settings and the item still show it). */
  const dismiss = useCallback(() => {
    closeDialog("update");
  }, [closeDialog]);

  /** "Don't remind me": the item no longer shows this version (Settings still does). */
  const ignore = useCallback(() => {
    if (version) ignoredUpdateVersionPreference.write(version);
    setIgnoredVersion(version);
    closeDialog("update");
  }, [closeDialog, version]);

  /** Whether the passive item shows: a release to install (not ignored), installing, or installed. */
  const announced =
    (status === "available" && version !== null && version !== ignoredVersion) ||
    status === "installing" ||
    status === "done";

  return useMemo(
    () => ({
      status,
      announced,
      version,
      notes,
      error,
      currentVersion,
      open,
      check,
      install,
      dismiss,
      ignore,
    }),
    [status, announced, version, notes, error, currentVersion, open, check, install, dismiss, ignore],
  );
}

export type AppUpdate = ReturnType<typeof useAppUpdate>;
