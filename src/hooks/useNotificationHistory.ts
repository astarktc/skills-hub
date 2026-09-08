import { useCallback, useMemo, useRef, useState } from "react";

/** The severity of one user-visible notification. */
export type NotificationKind = "error" | "warning" | "success" | "info";

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
  /** Wall-clock time the notification was raised (ms since epoch). */
  at: number;
};

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
) => void;

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
  /** The history's only writer. */
  record: RecordFn;
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

  const record = useCallback<RecordFn>((kind, title, message) => {
    const entry: Notification = {
      id: nextIdRef.current++,
      kind,
      title,
      message,
      at: Date.now(),
    };
    setNotifications((prev) =>
      [entry, ...prev].slice(0, NOTIFICATION_HISTORY_LIMIT),
    );
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

  return { notifications, unreadCount, record, markAllRead, clear };
}
