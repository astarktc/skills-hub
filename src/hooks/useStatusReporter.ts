import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import { describeCommandError } from "../commandError";
import { invokeTauri } from "../lib/tauri";
import {
  useNotificationHistory,
  type Notification,
  type NotificationKind,
} from "./useNotificationHistory";

// The toast library has two importers by design: this module makes every
// call (and owns each toast's lifetime, below), and the binder (`App.tsx`)
// mounts its `<Toaster>` once.

// The history's shape is the reporter's public vocabulary; the ring itself
// lives in the building block (`useNotificationHistory`).
export type { Notification, NotificationKind };

export type ActionErrorEntry = { title: string; message: string };

/**
 * How long each kind stays on screen. The single owner of toast lifetime:
 * an error stays until the operator closes it (it is the only record of a
 * failed install/refresh), a warning lingers long enough to read, a success
 * or info just confirms. No other module passes a `duration` to the toast
 * library and the app-level `<Toaster>` sets none (it only sets how many
 * toasts stay visible at once).
 */
const TOAST_DURATION_MS: Record<NotificationKind, number> = {
  error: Infinity,
  warning: 5000,
  success: 2000,
  info: 2000,
};

/** The one place the toast library is called. */
function showToast(kind: NotificationKind, title: string, message?: string) {
  const options = { description: message, duration: TOAST_DURATION_MS[kind] };
  switch (kind) {
    case "error":
      // An infinite toast needs an explicit way off the screen.
      toast.error(title, { ...options, closeButton: true });
      break;
    case "warning":
      toast.warning(title, options);
      break;
    case "success":
      toast.success(title, options);
      break;
    case "info":
      toast.info(title, options);
      break;
  }
}

export type TranslateFn = (
  key: string,
  opts?: Record<string, unknown>,
) => string;

/**
 * The reporter's notification entry point as a value components can receive
 * from the binder (they never import the hook themselves).
 */
export type NotifyFn = (
  kind: NotificationKind,
  title: string,
  message?: string,
) => void;

/**
 * The reporter's command-failure entry point as a value components can
 * receive from the binder: `notifyError(err)` in a catch block.
 */
export type NotifyErrorFn = (err: unknown) => void;

/** Localized copy for a command failure, or null for silent cancellation. */
export type FormatErrorFn = (err: unknown) => string | null;

/**
 * The reporter's clipboard entry point as a value components can receive
 * from the binder. Resolves to whether the text landed on the clipboard.
 */
export type CopyToClipboardFn = (text: string) => Promise<boolean>;

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

/**
 * What an action notifies when its body completes: a bare title, or a title
 * with a message (rendered as the toast's description and kept on the
 * history row, so explanatory lines stay readable instead of being folded
 * into the title). A batch that completed *with failures* sets `kind:
 * "warning"` — it lingers and counts as unread, where a success would flash
 * and be forgotten; one rule for every summary that carries a failure count.
 */
export type CompletionToast =
  | string
  | { kind?: "success" | "warning"; title: string; message?: string };

function completionToastParts(value: CompletionToast): {
  kind: "success" | "warning";
  title: string;
  message?: string;
} {
  if (typeof value === "string") return { kind: "success", title: value };
  return {
    kind: value.kind ?? "success",
    title: value.title,
    message: value.message,
  };
}

export type RunActionOptions<T> = {
  /** Initial progress line for the loading overlay. */
  message?: string;
  /**
   * Toast shown only when the body completes (no throw, no exit); a function
   * receives the body's value for copy that depends on the result.
   */
  successToast?: CompletionToast | ((value: T) => CompletionToast);
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
   * The single entry point for every user-visible notification: shows the
   * toast for `kind` with the lifetime the reporter owns and records the
   * entry in `notifications`. The setters below and runAction's
   * success/failure paths all end here.
   */
  notify: NotifyFn;
  /**
   * This session's notification history, newest first, bounded at
   * NOTIFICATION_HISTORY_LIMIT. In memory only; the backend log is partial
   * forensics for earlier runs (backend-logged events), not a record of
   * these.
   */
  notifications: Notification[];
  /** Errors and warnings raised since the last `markAllRead`. */
  unreadCount: number;
  /** Opening the history panel: everything listed counts as seen. */
  markAllRead: () => void;
  /** Empties the history (and with it the unread count). */
  clearNotifications: () => void;
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
  formatError: FormatErrorFn;
  /**
   * A caught command failure as an error Notification: `formatError`, then
   * `notify` unless it was a silent cancellation. The one place that rule
   * is written for callers outside runAction.
   */
  notifyError: NotifyErrorFn;
  /**
   * A batch's failures as error Notifications: one toast on screen (the
   * first entry, "+N more"), every entry its own history row. Entries with
   * an empty message are silenced failures (e.g. cancelled) and dropped.
   */
  showActionErrors: (errors: ActionErrorEntry[]) => void;
  /**
   * The same batching as warnings — for outcomes that are not failures but
   * must not be forgotten either (a Refresh that skipped skills it could
   * not locate): they linger and count as unread.
   */
  showActionWarnings: (warnings: ActionErrorEntry[]) => void;
  /**
   * The one clipboard helper. Copying is a courtesy, not an outcome: a
   * success flashes a toast without becoming a Notification (it is never
   * recorded), while a failure is an error Notification like any other.
   */
  copyToClipboard: CopyToClipboardFn;
  /** Cancel the in-flight backend operation and reset the loading surface. */
  cancelLoading: () => void;
};

export function useStatusReporter(t: TranslateFn): StatusReporter {
  const [loading, setLoading] = useState(false);
  const [loadingStartAt, setLoadingStartAt] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  // The one-shot completion trigger. Public callers set a string (see
  // `setSuccessToastMessage`); runAction may set the full shape.
  const [successToastMessage, setSuccessToastMessage] =
    useState<CompletionToast | null>(null);

  const formatError = useCallback(
    (err: unknown) => describeCommandError(err, t),
    [t],
  );

  // The session ring: every recorded entry passes through `record`,
  // toasted or not.
  const {
    notifications,
    unreadCount,
    record,
    markAllRead,
    clear: clearNotifications,
  } = useNotificationHistory();

  const notify = useCallback<NotifyFn>(
    (kind, title, message) => {
      showToast(kind, title, message);
      record(kind, title, message);
    },
    [record],
  );

  // One batch rule for both kinds: one toast (head + "+N more"), every
  // entry its own history row, so the N behind the suffix stay readable.
  const showActionBatch = useCallback(
    (kind: "error" | "warning", entries: ActionErrorEntry[]) => {
      // Entries with an empty message are silenced failures (e.g. cancelled).
      const visible = entries.filter((entry) => entry.message);
      if (visible.length === 0) return;
      const head = visible[0];
      const more =
        visible.length > 1
          ? t("errors.moreCount", { count: visible.length - 1 })
          : "";
      showToast(kind, head.title, `${head.message}${more}`);
      // The history lists newest first, so the batch is recorded last-to-
      // first: its first entry ends up at the top of its block and the
      // entries read top-down in the order they happened.
      for (let i = visible.length - 1; i >= 0; i--) {
        record(kind, visible[i].title, visible[i].message);
      }
    },
    [record, t],
  );

  const showActionErrors = useCallback(
    (errors: ActionErrorEntry[]) => showActionBatch("error", errors),
    [showActionBatch],
  );

  const showActionWarnings = useCallback(
    (warnings: ActionErrorEntry[]) => showActionBatch("warning", warnings),
    [showActionBatch],
  );

  const notifyError = useCallback<NotifyErrorFn>(
    (err) => {
      const message = formatError(err);
      if (message) notify("error", message);
    },
    [formatError, notify],
  );

  const copyToClipboard = useCallback<CopyToClipboardFn>(
    async (text) => {
      try {
        await navigator.clipboard.writeText(text);
      } catch {
        notify("error", t("copyFailed"));
        return false;
      }
      showToast("success", t("copied"));
      return true;
    },
    [notify, t],
  );

  useEffect(() => {
    if (!successToastMessage) return;
    const { kind, title, message } = completionToastParts(successToastMessage);
    notify(kind, title, message);
    // Clear the one-shot trigger in a microtask so the reset is not a
    // synchronous setState in the effect body (satisfies
    // react-hooks/set-state-in-effect). The flag is internal and only consumed
    // by this effect, so deferring it one microtask is behavior-preserving.
    void Promise.resolve().then(() => setSuccessToastMessage(null));
  }, [notify, successToastMessage]);

  useEffect(() => {
    if (!error) return;
    notify("error", error);
    // Reset the one-shot error/action triggers in a microtask (see the success
    // toast effect above) to keep the setState out of the synchronous effect
    // body. Behavior-preserving: these flags are only consumed here.
    void Promise.resolve().then(() => {
      setError(null);
      setActionMessage(null);
    });
  }, [error, notify]);

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
    notify,
    notifications,
    unreadCount,
    markAllRead,
    clearNotifications,
    setError,
    setSuccessToastMessage,
    formatError,
    notifyError,
    showActionErrors,
    showActionWarnings,
    copyToClipboard,
    cancelLoading,
  };
}
