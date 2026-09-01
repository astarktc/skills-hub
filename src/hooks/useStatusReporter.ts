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
 * An early end to an action body, returned (never thrown) so control flow
 * stays visible at the call site: `return action.handOff()` /
 * `return action.fail(msg)`. Flows receive one only through the
 * ActionHandle; the static factories exist for runAction and its test
 * doubles.
 */
export class ActionExit {
  readonly kind: "hand-off" | "failed";
  /** Failure copy; null keeps the silent-cancel contract of formatError. */
  readonly message: string | null;
  private constructor(kind: "hand-off" | "failed", message: string | null) {
    this.kind = kind;
    this.message = message;
  }
  static handOff(): ActionExit {
    return new ActionExit("hand-off", null);
  }
  static failed(message: string | null): ActionExit {
    return new ActionExit("failed", message);
  }
}

/** The controls an action body gets from runAction. */
export type ActionHandle = {
  /**
   * End the action without a success toast or an error: another surface
   * (e.g. a candidate picker modal) takes over from here. The loading surface
   * still resets, always.
   */
  handOff: () => ActionExit;
  /**
   * End the action with a user-facing failure (already localized). `null`
   * ends it silently, mirroring formatError's cancel contract.
   */
  fail: (message: string | null) => ActionExit;
};

export type RunActionOptions<T> = {
  /** Initial progress line for the loading overlay. */
  message?: string;
  /**
   * Toast shown only when the body completes (no throw, no exit); a function
   * receives the body's value for copy that depends on the result.
   */
  successToast?: string | ((value: T) => string);
};

/**
 * The one shared status surface: every world hook reports loading, progress
 * messages, and failures through this interface so the app keeps a single
 * spinner/toast UX. World hooks receive it as a dependency (never create
 * their own), which keeps them testable with a mock reporter.
 *
 * The loading surface (`loading`/`loadingStartAt`) is owned by `runAction`
 * alone; there are deliberately no raw setters for it, so no flow can leave
 * the overlay stuck. The remaining setters exist for what happens outside an
 * action or inside one without ending it.
 */
export type StatusReporter = {
  loading: boolean;
  loadingStartAt: number | null;
  actionMessage: string | null;
  /**
   * Run one user action under the loading overlay. Owns the whole lifecycle:
   * begin (loading on, stale error cleared, `message` shown) → body →
   * success toast on completion / `formatError` → error toast on throw /
   * the body's explicit `handOff` or `fail` exit → loading surface reset,
   * always. Resolves to the body's value on completion, else `undefined`.
   */
  runAction: <T>(
    opts: RunActionOptions<T>,
    fn: (action: ActionHandle) => Promise<T | ActionExit>,
  ) => Promise<T | undefined>;
  /**
   * Progress line while an action runs (per-step loops, the sync progress
   * channel in useSyncOrchestration). Does not end the action.
   */
  setActionMessage: (value: string | null) => void;
  /**
   * One-shot error trigger: rendered as a toast, then auto-cleared. For
   * failures outside an action (input validation before one starts, hooks
   * that never show the overlay) or non-fatal warnings inside one; a failure
   * that ends an action goes through `action.fail` or a throw instead.
   */
  setError: (value: string | null) => void;
  /**
   * One-shot success trigger: rendered as a toast, then auto-cleared. For
   * successes outside an action; an action's success toast is `successToast`.
   */
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
    void invokeTauri("cancelCurrentOperation").catch(() => {});
    setLoading(false);
    setLoadingStartAt(null);
    setActionMessage(null);
  }, []);

  const runAction = useCallback(
    async <T,>(
      opts: RunActionOptions<T>,
      fn: (action: ActionHandle) => Promise<T | ActionExit>,
    ): Promise<T | undefined> => {
      setLoading(true);
      setLoadingStartAt(Date.now());
      setError(null);
      setActionMessage(opts.message ?? null);
      try {
        const outcome = await fn({
          handOff: () => ActionExit.handOff(),
          fail: (message) => ActionExit.failed(message),
        });
        if (outcome instanceof ActionExit) {
          if (outcome.kind === "failed") setError(outcome.message);
          return undefined;
        }
        const { successToast } = opts;
        if (typeof successToast === "function") {
          setSuccessToastMessage(successToast(outcome));
        } else if (successToast) {
          setSuccessToastMessage(successToast);
        }
        return outcome;
      } catch (err) {
        setError(formatError(err));
        return undefined;
      } finally {
        setLoading(false);
        setLoadingStartAt(null);
        setActionMessage(null);
      }
    },
    [formatError],
  );

  return {
    loading,
    loadingStartAt,
    actionMessage,
    runAction,
    setActionMessage,
    setError,
    setSuccessToastMessage,
    formatError,
    showActionErrors,
    cancelLoading,
  };
}
