import { useCallback, useMemo, useRef, useState } from "react";
import type { CommandError } from "../bindings";
import type { EntryRepair, EntrySubject } from "../lib/reportOutcome";

/** The severity of one user-visible notification. */
export type NotificationKind = "error" | "warning" | "success" | "info";

/** A session-only action shared by the toast and history row. */
export type NotificationAction = { label: string; onClick: () => void };

/**
 * One user-visible outcome of an action: shown once as a toast and kept in
 * the session's history; opening the history marks it read.
 * `id` increases monotonically within the session, so "unread" is a watermark on it.
 */
export type Notification = {
  id: number;
  kind: NotificationKind;
  title: string;
  message?: string;
  action?: NotificationAction;
  /** Wall-clock time the notification was raised (ms since epoch). */
  at: number;
  /**
   * The run this row belongs to: every row `recordRun` writes for one fold
   * call shares it (D11 groups the panel by it — ticket 14). Absent for a
   * standalone notification.
   */
  runId?: number;
  /** Within a run: its composite headline, or one of its entries. */
  role?: "headline" | "entry";
  /** The action the run was ("Refresh", "Sync") — the panel's group row names it. */
  runLabel?: string | null;
} & RowFacts;

/**
 * What a report entry is about, kept as data on its row so the panel can
 * offer its actions (`notificationRuns.rowActions`) long after the run.
 */
export type RowFacts = {
  subject?: EntrySubject;
  error?: CommandError;
  repair?: EntryRepair;
  /** The "to review" kind: an unanswered collision, never a failure (D10). */
  review?: true;
};

/** One row of a run, before the ring stamps it. */
export type RunRow = {
  kind: NotificationKind;
  title: string;
  message?: string;
  action?: NotificationAction;
} & RowFacts;

/** The history keeps this many entries, newest first; older ones drop off. */
export const NOTIFICATION_HISTORY_LIMIT = 100;

/** Only these kinds count as unread: a success or info needs no follow-up. */
function isAttentionKind(kind: NotificationKind): boolean {
  return kind === "error" || kind === "warning";
}

/** Appends one entry to the history. */
export type RecordFn = (
  kind: NotificationKind,
  title: string,
  message?: string,
  action?: NotificationAction,
) => void;

/**
 * Appends one run — a fold's headline (when it has one) and its entries —
 * as one block under a fresh `runId`, which it returns. The headline sits on
 * top; the entries follow in report order. `label` names the action the run
 * was (the reporter's pending label), for the panel's group row.
 */
export type RecordRunFn = (
  headline: RunRow | null,
  entries: RunRow[],
  label?: string | null,
) => number;

export type NotificationHistory = {
  /**
   * This session's notification history, newest first, bounded at
   * NOTIFICATION_HISTORY_LIMIT. In memory only; the backend log is partial
   * forensics for earlier runs (backend-logged events), not a record of
   * these.
   */
  notifications: Notification[];
  /** Errors and warnings recorded since the last `markAllRead`. */
  unreadCount: number;
  /** The history's writer for a standalone notification. */
  record: RecordFn;
  /** The history's writer for one run of a batch action. */
  recordRun: RecordRunFn;
  /** Opening the history panel: everything listed counts as seen. */
  markAllRead: () => void;
  /** Empties the history (and with it the unread count). */
  clear: () => void;
};

/**
 * The notification ring: a building block owned by the reporter world and
 * composed by `useStatusReporter`, which decides what gets recorded (and
 * toasts it). This hook knows nothing about toasts — only the ring, the
 * unread watermark and clearing.
 */
export function useNotificationHistory(): NotificationHistory {
  const [notifications, setNotifications] = useState<Notification[]>([]);
  // Every entry with an id above this watermark is unread; ids only grow.
  const [lastReadId, setLastReadId] = useState(0);
  const nextIdRef = useRef(1);
  const nextRunIdRef = useRef(1);

  const record = useCallback<RecordFn>((kind, title, message, action) => {
    const entry: Notification = {
      id: nextIdRef.current++,
      kind,
      title,
      message,
      ...(action ? { action } : {}),
      at: Date.now(),
    };
    setNotifications((prev) =>
      [entry, ...prev].slice(0, NOTIFICATION_HISTORY_LIMIT),
    );
  }, []);

  const recordRun = useCallback<RecordRunFn>((headline, entries, label = null) => {
    const runId = nextRunIdRef.current++;
    const at = Date.now();
    const rows: Array<RunRow & { role: "headline" | "entry" }> = [
      ...(headline ? [{ ...headline, role: "headline" as const }] : []),
      ...entries.map((entry) => ({ ...entry, role: "entry" as const })),
    ];
    // Newest first: the block's last row gets the smallest id, so the
    // headline — recorded last — is on top and the entries read top-down.
    const stamped: Notification[] = [];
    for (let i = rows.length - 1; i >= 0; i--) {
      const { action, ...row } = rows[i];
      stamped.unshift({
        ...row,
        id: nextIdRef.current++,
        ...(action ? { action } : {}),
        at,
        runId,
        runLabel: label,
      });
    }
    setNotifications((prev) =>
      [...stamped, ...prev].slice(0, NOTIFICATION_HISTORY_LIMIT),
    );
    return runId;
  }, []);

  const unreadCount = useMemo(
    () =>
      notifications.filter((n) => n.id > lastReadId && isAttentionKind(n.kind))
        .length,
    [notifications, lastReadId],
  );

  const markAllRead = useCallback(() => {
    setLastReadId(nextIdRef.current - 1);
  }, []);

  const clear = useCallback(() => {
    setNotifications([]);
    setLastReadId(nextIdRef.current - 1);
  }, []);

  return { notifications, unreadCount, record, recordRun, markAllRead, clear };
}
