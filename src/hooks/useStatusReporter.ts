import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import { describeCommandError } from "../commandError";
import { invokeTauri } from "../lib/tauri";

export type ActionErrorEntry = { title: string; message: string };

export type TranslateFn = (
  key: string,
  opts?: Record<string, unknown>,
) => string;

/**
 * The one shared status surface: every world hook reports loading, progress
 * messages, and failures through this interface so the app keeps a single
 * spinner/toast UX. World hooks receive it as a dependency (never create
 * their own), which keeps them testable with a mock reporter.
 */
export type StatusReporter = {
  loading: boolean;
  loadingStartAt: number | null;
  actionMessage: string | null;
  setLoading: (value: boolean) => void;
  setLoadingStartAt: (value: number | null) => void;
  setActionMessage: (value: string | null) => void;
  /** One-shot error trigger: rendered as a toast, then auto-cleared. */
  setError: (value: string | null) => void;
  /** One-shot success trigger: rendered as a toast, then auto-cleared. */
  setSuccessToastMessage: (value: string | null) => void;
  /**
   * Single narrow waist for command failures: localized copy (or null for
   * silent cancellation) comes from describeCommandError.
   */
  formatError: (err: unknown) => string | null;
  showActionErrors: (errors: ActionErrorEntry[]) => void;
  /** Cancel the in-flight backend operation and reset the loading surface. */
  cancelLoading: () => void;
};

export function useStatusReporter(t: TranslateFn): StatusReporter {
  const [loading, setLoading] = useState(false);
  const [loadingStartAt, setLoadingStartAt] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [successToastMessage, setSuccessToastMessage] = useState<string | null>(
    null,
  );

  const formatError = useCallback(
    (err: unknown) => describeCommandError(err, t),
    [t],
  );

  const showActionErrors = useCallback(
    (errors: ActionErrorEntry[]) => {
      // Entries with an empty message are silenced failures (e.g. cancelled).
      const visible = errors.filter((entry) => entry.message);
      if (visible.length === 0) return;
      const head = visible[0];
      const more =
        visible.length > 1
          ? t("errors.moreCount", { count: visible.length - 1 })
          : "";
      toast.error(`${head.title}\n${head.message}${more}`, { duration: 3200 });
    },
    [t],
  );

  useEffect(() => {
    if (!successToastMessage) return;
    toast.success(successToastMessage, { duration: 1800 });
    // Clear the one-shot trigger in a microtask so the reset is not a
    // synchronous setState in the effect body (satisfies
    // react-hooks/set-state-in-effect). The flag is internal and only consumed
    // by this effect, so deferring it one microtask is behavior-preserving.
    void Promise.resolve().then(() => setSuccessToastMessage(null));
  }, [successToastMessage]);

  useEffect(() => {
    if (!error) return;
    toast.error(error, { duration: 2600 });
    // Reset the one-shot error/action triggers in a microtask (see the success
    // toast effect above) to keep the setState out of the synchronous effect
    // body. Behavior-preserving: these flags are only consumed here.
    void Promise.resolve().then(() => {
      setError(null);
      setActionMessage(null);
    });
  }, [error]);

  const cancelLoading = useCallback(() => {
    void invokeTauri("cancel_current_operation").catch(() => {});
    setLoading(false);
    setLoadingStartAt(null);
    setActionMessage(null);
  }, []);

  return {
    loading,
    loadingStartAt,
    actionMessage,
    setLoading,
    setLoadingStartAt,
    setActionMessage,
    setError,
    setSuccessToastMessage,
    formatError,
    showActionErrors,
    cancelLoading,
  };
}
