import { memo, useCallback, useEffect, useRef, useState } from "react";
import {
  Bell,
  Check,
  CircleAlert,
  CircleCheck,
  Copy,
  Info,
  TriangleAlert,
} from "lucide-react";
import type { TFunction } from "i18next";
import Modal from "./Modal";
import type {
  Notification,
  NotificationKind,
  NotifyFn,
} from "../../hooks/useStatusReporter";
import { formatRelativeTime } from "../../lib/skillPresentation";

type NotificationsModalProps = {
  open: boolean;
  /** Newest first — the reporter's own order. */
  notifications: Notification[];
  onRequestClose: () => void;
  onClear: () => void;
  /** Copy failures are reported like any other outcome. */
  notify: NotifyFn;
  t: TFunction;
};

const KIND_ICON: Record<NotificationKind, typeof CircleAlert> = {
  error: CircleAlert,
  warning: TriangleAlert,
  success: CircleCheck,
  info: Info,
};

/** How long the copy button shows its "done" state. */
const COPIED_FEEDBACK_MS = 1500;

/** What a single entry copies: the title, then the message beneath it. */
function entryText(n: Notification): string {
  return n.message ? `${n.title}\n${n.message}` : n.title;
}

/**
 * What "Copy all" copies: the visible list, newest first, one entry per
 * line with an absolute timestamp so it can be pasted into a bug report.
 */
function listText(notifications: Notification[]): string {
  return notifications
    .map((n) => {
      const head = `${new Date(n.at).toISOString()} [${n.kind}] ${n.title}`;
      return n.message ? `${head}\n    ${n.message}` : head;
    })
    .join("\n");
}

type CopyTarget = number | "all";

const NotificationsModal = ({
  open,
  notifications,
  onRequestClose,
  onClear,
  notify,
  t,
}: NotificationsModalProps) => {
  const [copied, setCopied] = useState<CopyTarget | null>(null);
  const copiedTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (copiedTimer.current) clearTimeout(copiedTimer.current);
    };
  }, []);

  const copy = useCallback(
    async (target: CopyTarget, text: string) => {
      try {
        await navigator.clipboard.writeText(text);
        setCopied(target);
        if (copiedTimer.current) clearTimeout(copiedTimer.current);
        copiedTimer.current = setTimeout(() => {
          setCopied(null);
          copiedTimer.current = null;
        }, COPIED_FEEDBACK_MS);
      } catch {
        notify("error", t("copyFailed"));
      }
    },
    [notify, t],
  );

  const isEmpty = notifications.length === 0;

  return (
    <Modal
      open={open}
      title={t("notifications.title")}
      onRequestClose={onRequestClose}
      className="modal-notifications"
      bodyClassName="notif-body"
      footerClassName="space-between"
      footer={
        <>
          <button
            className="btn btn-secondary"
            type="button"
            onClick={onClear}
            disabled={isEmpty}
          >
            {t("notifications.clear")}
          </button>
          <button
            className="btn btn-secondary"
            type="button"
            onClick={() => void copy("all", listText(notifications))}
            disabled={isEmpty}
          >
            {copied === "all" ? <Check size={14} /> : <Copy size={14} />}
            {copied === "all" ? t("copied") : t("notifications.copyAll")}
          </button>
        </>
      }
    >
      {isEmpty ? (
        <div className="notif-empty">
          <Bell size={28} className="notif-empty-icon" aria-hidden="true" />
          <div className="notif-empty-title">{t("notifications.empty")}</div>
          <div className="notif-empty-hint">{t("notifications.emptyHint")}</div>
        </div>
      ) : (
        <ul className="notif-list">
          {notifications.map((n) => {
            const Icon = KIND_ICON[n.kind];
            const at = new Date(n.at);
            return (
              <li key={n.id} className={`notif-row notif-${n.kind}`}>
                <span
                  className="notif-glyph"
                  role="img"
                  aria-label={t(`notifications.kind.${n.kind}`)}
                >
                  <Icon size={16} />
                </span>
                <div className="notif-text">
                  <div className="notif-title">{n.title}</div>
                  {n.message ? (
                    <div className="notif-message">{n.message}</div>
                  ) : null}
                </div>
                <time
                  className="notif-time"
                  dateTime={at.toISOString()}
                  title={at.toLocaleString()}
                >
                  {formatRelativeTime(n.at, t)}
                </time>
                <button
                  className="card-btn notif-copy"
                  type="button"
                  aria-label={t("notifications.copyEntry")}
                  title={t("notifications.copyEntry")}
                  onClick={() => void copy(n.id, entryText(n))}
                >
                  {copied === n.id ? <Check size={14} /> : <Copy size={14} />}
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </Modal>
  );
};

export default memo(NotificationsModal);
