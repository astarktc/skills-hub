import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { describeCommandError } from "../commandError";
import { invokeTauri } from "../lib/tauri";
import {
  describeRefusal,
  refusalFor,
  type CancellableCommand,
  type MutationRefusal,
  type PendingAction,
  type PendingProgress,
  type PendingSubject,
} from "../lib/actionLifecycle";
import type { Outcome, OutcomeEntry } from "../lib/reportOutcome";
import {
  useNotificationHistory,
  type Notification,
  type NotificationAction,
  type NotificationKind,
  type RunRow,
} from "./useNotificationHistory";

// The toast library has two importers by design: this module makes every
// call (and owns each toast's lifetime, below), and the binder (`App.tsx`)
// mounts its `<Toaster>` once.

// The history's shape is the reporter's public vocabulary; the ring itself
// lives in the building block (`useNotificationHistory`).
export type { Notification, NotificationKind };

export type ActionErrorEntry = {
  title: string;
  message: string;
  action?: NotificationAction;
};

/**
 * The single owner of toast lifetime: errors stay until closed; warnings
 * allow 5 s of reading time; success/info briefly confirm for 2 s. Sonner
 * pauses finite timers while the toaster is hovered/interacting or the
 * document is hidden, so these are not wall-clock deadlines. Errors and
 * warnings have close buttons so longer-lived outcomes can be dismissed
 * without waiting for a timer. Notifications also remain in session history.
 * No other module passes a `duration`, including the app-level `<Toaster>`.
 */
const TOAST_DURATION_MS: Record<NotificationKind, number> = {
  error: Infinity,
  warning: 5000,
  success: 2000,
  info: 2000,
};

/** The one place the toast library is called. */
function showToast(
  kind: NotificationKind,
  title: string,
  message?: string,
  action?: NotificationAction,
) {
  const options = {
    description: message,
    duration: TOAST_DURATION_MS[kind],
    ...(action ? { action } : {}),
  };
  switch (kind) {
    case "error":
      // An infinite toast needs an explicit way off the screen.
      toast.error(title, { ...options, closeButton: true });
      break;
    case "warning":
      toast.warning(title, { ...options, closeButton: true });
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
  /**
   * Withdraw Cancel for one phase of a cancellable action — a follow-up that
   * runs no token-taking command (the overwrite ask's retry), so Cancel is
   * never offered while nothing would stop it (D20). The action's declared
   * `cancel` is offered again when `run` settles; resolves `run`'s value.
   */
  uncancellable: <R>(run: () => Promise<R>) => Promise<R>;
  /**
   * Whether the operator asked this action to stop. Cancel reaches the
   * backend only through a token-taking command, so a body checks this at
   * each boundary before starting one: a Cancel pressed while an earlier,
   * token-less step ran (a listing) must still stop the work that follows.
   */
  cancelRequested: () => boolean;
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
   * What the action is about (default: a batch). A skill or cell action
   * shows inline pending on its subject; a batch (or a streamed/cancellable
   * action) shows the progress surface.
   */
  subject?: PendingSubject;
  /** What is running, in the operator's words — a refused action names it. */
  label?: string;
  /**
   * The token-taking command this action runs (D20): the only way an action
   * becomes cancellable. Omitted = no Cancel, because nothing would stop it.
   */
  cancel?: CancellableCommand;
  /**
   * A read that runs under the lifecycle (its overlay, its error toast) but
   * changes nothing — the onboarding plan scan, an Explore preview. A read is
   * never refused: it waits for the running action to settle, then runs.
   * Only mutations are refused (D19).
   */
  read?: boolean;
  /**
   * A queued read's validity, checked when its turn comes (before its body
   * runs): a navigational read (an Explore preview, the import review) whose
   * intent went stale while it waited — the operator navigated away, asked
   * for another preview — is dropped instead of opening a surface.
   */
  stillWanted?: () => boolean;
  /**
   * Toast shown only when the body completes (no throw, no exit); a function
   * receives the body's value for copy that depends on the result.
   */
  successToast?: CompletionToast | ((value: T) => CompletionToast);
};

/** A streamed step: `done` of `total`, with the line the progress surface shows. */
export type ProgressTick = PendingProgress & { message?: string | null };

/** How an Outcome's completion is carried out by the caller that knows how. */
export type OutcomeHandlers = {
  /** Re-read the catalog (`completion.reload`), awaited before anything is shown. */
  reload?: () => Promise<unknown>;
  /** Close the dialog the action ran from (`completion.closeModal`). */
  closeModal?: () => void;
  /** Resolve an entry's declarative action (by skill id) when it is clicked. */
  onAction?: (action: NonNullable<OutcomeEntry["action"]>) => void;
};

/** The completion an executed Outcome answers with. */
export type OutcomeCompletion = Outcome["completion"];

/** The reporter's Outcome executor as a value (`applyOutcome(outcome, handlers)`). */
export type ApplyOutcomeFn = (
  outcome: Outcome<OutcomeEntry> | Outcome<Omit<OutcomeEntry, "action">>,
  handlers?: OutcomeHandlers,
) => Promise<OutcomeCompletion>;

/**
 * The one shared status surface: every world hook reports its action
 * lifecycle, progress and failures through this interface so the app keeps
 * one pending/toast UX. World hooks receive it as a dependency (never create
 * their own), which keeps them testable with a mock reporter.
 *
 * The lifecycle is owned by `runAction` alone:
 * - **One mutation at a time** (D19): while an action is pending, another
 *   `runAction` is refused — it never runs, answers `undefined`, and the
 *   operator reads why (`refusal`, `describeRefusal`). Nothing is queued.
 * - **Per-subject pending**: `pending` says what is running (a skill, matrix
 *   cells, a batch), its label, streamed progress and whether it can be
 *   cancelled — only a declared token-taking command can (D20).
 * - **Outcome execution**: `applyOutcome` turns a fold's answer into the
 *   toast, the run's history block and the completion, so no caller unpacks
 *   an Outcome by hand.
 */
export type StatusReporter = {
  /** The action running now, or null. */
  pending: PendingAction | null;
  /** Whether an action is running (`pending !== null`): dialogs stay put, submits wait. */
  loading: boolean;
  /** What a new action would be refused with now (null = it may start). */
  refusal: MutationRefusal | null;
  /** The progress surface's current line (a step of a streamed action). */
  actionMessage: string | null;
  /**
   * Run one user action. Owns the whole lifecycle: refusal while another
   * runs → begin (`pending` set, `message` shown) → body → success toast on
   * completion / `formatError` → error toast on throw / the body's explicit
   * `handOff` or `fail` exit → pending cleared, always. Resolves to the
   * body's value on completion, else `undefined`.
   */
  runAction: <T>(
    opts: RunActionOptions<T>,
    fn: (action: ActionHandle) => Promise<T | ActionExit>,
  ) => Promise<T | undefined>;
  /**
   * Progress line while an action runs (the sync progress channel in
   * useSyncOrchestration). Does not end the action.
   */
  setActionMessage: (value: string | null) => void;
  /** A streamed step: the count the progress surface shows, and its line. */
  reportProgress: (tick: ProgressTick) => void;
  /**
   * Execute a fold's Outcome: reload first when the completion asks (so the
   * toast describes the catalog on screen), then the entries' toasts, the
   * headline toast, one history block for the run (`recordRun`: headline +
   * entries + quiet notes), and `closeModal`. Resolves to the completion.
   */
  applyOutcome: ApplyOutcomeFn;
  /**
   * The single entry point for a standalone notification: shows the toast
   * for `kind` with the lifetime the reporter owns and records the entry in
   * `notifications`.
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
   * Legacy one-shot error trigger (a toast, then auto-cleared). The failure
   * dialect is `notifyError(err)` / `notify("error", …)`; this remains only
   * for the sync and Add worlds until they migrate.
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
   * The one failure dialect outside runAction: a caught command failure as
   * an error Notification (`formatError`, then `notify` unless it was a
   * silent cancellation).
   */
  notifyError: NotifyErrorFn;
  /**
   * A batch's failures as error Notifications: one toast on screen (the
   * first entry, "+N more"), every entry its own history row. Superseded by
   * `applyOutcome`; kept only until the Add world switches.
   */
  showActionErrors: (errors: ActionErrorEntry[]) => void;
  /** The same batching as warnings (see `showActionErrors`). */
  showActionWarnings: (warnings: ActionErrorEntry[]) => void;
  /**
   * The one clipboard helper. Copying is a courtesy, not an outcome: a
   * success flashes a toast without becoming a Notification (it is never
   * recorded), while a failure is an error Notification like any other.
   */
  copyToClipboard: CopyToClipboardFn;
  /**
   * Ask the running action's command to stop (D20). Only a cancellable action
   * can be cancelled; the action keeps `pending` until its command returns
   * (its report says what was cancelled) — nothing is hidden that did not stop.
   */
  cancelPending: () => void;
};

export function useStatusReporter(t: TranslateFn): StatusReporter {
  const [pending, setPendingState] = useState<PendingAction | null>(null);
  // The one-at-a-time rule reads the ref: two clicks in one tick must not
  // both see "nothing running".
  const pendingRef = useRef<PendingAction | null>(null);
  // Reads waiting for the slot, woken when the running action settles.
  const idleWaitersRef = useRef<Array<() => void>>([]);
  // Unmounted: a waiting read wakes and ends without running its body.
  const disposedRef = useRef(false);
  useEffect(() => {
    // The queue is one mutable array owned by this ref for the hook's life.
    const waiting = idleWaitersRef.current;
    disposedRef.current = false;
    return () => {
      disposedRef.current = true;
      for (const wake of waiting.splice(0)) wake();
    };
  }, []);
  const setPending = useCallback(
    (update: (current: PendingAction | null) => PendingAction | null) => {
      pendingRef.current = update(pendingRef.current);
      setPendingState(pendingRef.current);
    },
    [],
  );
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
    recordRun,
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

  // One toast per batch kind: the head entry plus "+N more". Entries with
  // an empty message are silenced failures (e.g. cancelled) and dropped by
  // the callers before this.
  const toastBatch = useCallback(
    (kind: "error" | "warning", visible: ActionErrorEntry[]) => {
      if (visible.length === 0) return;
      const head = visible[0];
      const more =
        visible.length > 1
          ? t("errors.moreCount", { count: visible.length - 1 })
          : "";
      showToast(kind, head.title, `${head.message}${more}`, head.action);
    },
    [t],
  );

  // One batch rule for both kinds: one toast (head + "+N more"), every
  // entry its own history row, so the N behind the suffix stay readable.
  const showActionBatch = useCallback(
    (kind: "error" | "warning", entries: ActionErrorEntry[]) => {
      const visible = entries.filter((entry) => entry.message);
      if (visible.length === 0) return;
      toastBatch(kind, visible);
      // The history lists newest first, so the batch is recorded last-to-
      // first: its first entry ends up at the top of its block.
      for (let i = visible.length - 1; i >= 0; i--) {
        record(kind, visible[i].title, visible[i].message, visible[i].action);
      }
    },
    [record, toastBatch],
  );

  const showActionErrors = useCallback(
    (errors: ActionErrorEntry[]) => showActionBatch("error", errors),
    [showActionBatch],
  );

  const showActionWarnings = useCallback(
    (warnings: ActionErrorEntry[]) => showActionBatch("warning", warnings),
    [showActionBatch],
  );

  const applyOutcome = useCallback<ApplyOutcomeFn>(
    async (outcome, handlers = {}) => {
      if (outcome.completion.reload && handlers.reload) await handlers.reload();
      const toEntry = (entry: OutcomeEntry): ActionErrorEntry => {
        const { action, ...rest } = entry;
        const onAction = handlers.onAction;
        return action && onAction
          ? { ...rest, action: { label: action.label, onClick: () => onAction(action) } }
          : rest;
      };
      const errors = (outcome.errors as OutcomeEntry[]).map(toEntry).filter((e) => e.message);
      const warnings = (outcome.warnings as OutcomeEntry[]).map(toEntry).filter((e) => e.message);
      toastBatch("error", errors);
      toastBatch("warning", warnings);
      const { toast: headline } = outcome;
      if (headline) showToast(headline.kind, headline.message, headline.detail);
      // One history block per run: the headline over its entries; quiet
      // notes are their own rows only when no headline carries them.
      const rows: RunRow[] = [
        ...errors.map((entry) => ({ kind: "error" as const, ...entry })),
        ...warnings.map((entry) => ({ kind: "warning" as const, ...entry })),
        ...(headline ? [] : outcome.notes.map((note) => ({ kind: "info" as const, title: note }))),
      ];
      if (headline || rows.length > 0) {
        recordRun(
          headline
            ? { kind: headline.kind, title: headline.message, message: headline.detail }
            : null,
          rows,
        );
      }
      if (outcome.completion.closeModal) handlers.closeModal?.();
      return outcome.completion;
    },
    [recordRun, toastBatch],
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
    // toast effect above) to keep the setState out of the synchronous effect body.
    void Promise.resolve().then(() => {
      setError(null);
      setActionMessage(null);
    });
  }, [error, notify]);

  const reportProgress = useCallback(
    ({ done, total, message }: ProgressTick) => {
      setPending((current) => (current ? { ...current, progress: { done, total } } : current));
      if (message !== undefined) setActionMessage(message);
    },
    [setPending],
  );

  const cancelPending = useCallback(() => {
    const current = pendingRef.current;
    if (!current?.cancellable || current.cancelling) return;
    void invokeTauri("cancelCurrentOperation").catch(() => {});
    // The "Cancelling…" line is derived from `cancelling` (below), so no
    // progress tick can replace it before the command returns.
    setPending((running) => (running ? { ...running, cancelling: true } : running));
  }, [setPending]);

  const runAction = useCallback(
    async <T,>(
      opts: RunActionOptions<T>,
      fn: (action: ActionHandle) => Promise<T | ActionExit>,
    ): Promise<T | undefined> => {
      // A read waits its turn; a mutation is refused (never queued).
      if (opts.read) {
        while (pendingRef.current) {
          await new Promise<void>((resolve) => idleWaitersRef.current.push(resolve));
          if (disposedRef.current) return undefined;
        }
        if (disposedRef.current || (opts.stillWanted && !opts.stillWanted())) return undefined;
      }
      const refusal = refusalFor(pendingRef.current);
      if (refusal) {
        // Refused, not queued: say why, record nothing (it never ran).
        showToast("info", describeRefusal(refusal, t));
        return undefined;
      }
      setPending(() => ({
        subject: opts.subject ?? { kind: "batch" },
        label: opts.label ?? null,
        startedAt: Date.now(),
        cancellable: opts.cancel !== undefined,
        cancelling: false,
        progress: null,
      }));
      setActionMessage(opts.message ?? null);
      try {
        const declared = opts.cancel !== undefined;
        const outcome = await fn({
          handOff: () => ActionExit.handOff(),
          fail: (message) => ActionExit.failed(message),
          uncancellable: async (run) => {
            setPending((current) => (current ? { ...current, cancellable: false } : current));
            try {
              return await run();
            } finally {
              setPending((current) => (current ? { ...current, cancellable: declared } : current));
            }
          },
          cancelRequested: () => pendingRef.current?.cancelling === true,
        });
        if (outcome instanceof ActionExit) {
          if (outcome.kind === "failed" && outcome.message) notify("error", outcome.message);
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
        const message = formatError(err);
        if (message) notify("error", message);
        return undefined;
      } finally {
        setPending(() => null);
        setActionMessage(null);
        const waiters = idleWaitersRef.current.splice(0);
        for (const wake of waiters) wake();
      }
    },
    [formatError, notify, setPending, t],
  );

  const refusal = useMemo(() => refusalFor(pending), [pending]);

  return {
    pending,
    loading: pending !== null,
    refusal,
    actionMessage: pending?.cancelling ? t("lifecycle.cancelling") : actionMessage,
    runAction,
    setActionMessage,
    reportProgress,
    applyOutcome,
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
    cancelPending,
  };
}
