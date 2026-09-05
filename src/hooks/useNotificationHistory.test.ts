// Tests at the notification-history seam: the reporter's building block that
// owns the session ring (bounded, newest first), the unread watermark and
// clearing. It renders nothing and calls no side channel, so nothing is
// mocked: `record` is the only writer, the returned values the only readers.

import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import {
  NOTIFICATION_HISTORY_LIMIT,
  useNotificationHistory,
} from "./useNotificationHistory";

describe("useNotificationHistory", () => {
  it("records entries newest first with a monotonically increasing id and a timestamp", () => {
    const before = Date.now();
    const { result } = renderHook(() => useNotificationHistory());

    act(() => {
      result.current.record("success", "Installed", "skill-a");
      result.current.record("error", "Install failed", "disk full");
    });

    expect(result.current.notifications).toHaveLength(2);
    const [newest, oldest] = result.current.notifications;
    expect(newest).toMatchObject({
      kind: "error",
      title: "Install failed",
      message: "disk full",
    });
    expect(oldest).toMatchObject({ kind: "success", title: "Installed" });
    expect(newest.id).toBeGreaterThan(oldest.id);
    expect(newest.at).toBeGreaterThanOrEqual(before);
  });

  it("keeps only the last 100 entries, dropping the oldest", () => {
    const { result } = renderHook(() => useNotificationHistory());

    act(() => {
      for (let i = 1; i <= NOTIFICATION_HISTORY_LIMIT + 1; i++) {
        result.current.record("info", `n${i}`);
      }
    });

    expect(NOTIFICATION_HISTORY_LIMIT).toBe(100);
    expect(result.current.notifications).toHaveLength(100);
    expect(result.current.notifications[0].title).toBe("n101");
    expect(result.current.notifications[99].title).toBe("n2");
    expect(
      result.current.notifications.some((n) => n.title === "n1"),
    ).toBe(false);
  });

  it("counts only errors and warnings as unread", () => {
    const { result } = renderHook(() => useNotificationHistory());
    expect(result.current.unreadCount).toBe(0);

    act(() => {
      result.current.record("error", "e");
      result.current.record("warning", "w");
      result.current.record("success", "s");
      result.current.record("info", "i");
    });

    expect(result.current.unreadCount).toBe(2);
  });

  it("markAllRead zeroes the unread count without discarding; later entries count again", () => {
    const { result } = renderHook(() => useNotificationHistory());

    act(() => {
      result.current.record("error", "e1");
      result.current.record("warning", "w1");
    });
    act(() => {
      result.current.markAllRead();
    });

    expect(result.current.unreadCount).toBe(0);
    expect(result.current.notifications).toHaveLength(2);

    act(() => {
      result.current.record("error", "e2");
    });

    expect(result.current.unreadCount).toBe(1);
  });

  it("clear empties the history and the unread count", () => {
    const { result } = renderHook(() => useNotificationHistory());

    act(() => {
      result.current.record("error", "e1");
      result.current.record("success", "s1");
    });
    act(() => {
      result.current.clear();
    });

    expect(result.current.notifications).toEqual([]);
    expect(result.current.unreadCount).toBe(0);
  });
});
