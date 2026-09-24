// Ownership of the one pending overwrite ask: exactly-once settlement, a
// newer request displacing an older one as declined, cancel reading the
// current request, and unmount settling whatever is pending.

import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useOverwriteConfirmation, type OverwriteRow } from "./useOverwriteConfirmation";

const row = (skillId: string): OverwriteRow => ({
  skillId,
  skillName: skillId,
  toolKey: "claude",
  toolLabel: "CLAUDE",
  path: `/t/${skillId}`,
});

describe("useOverwriteConfirmation", () => {
  it("resolves the answer once and clears the pending ask", async () => {
    const { result } = renderHook(() => useOverwriteConfirmation());
    let answer!: Promise<boolean>;
    act(() => {
      answer = result.current.request([row("s1")]);
    });
    expect(result.current.pending?.rows).toEqual([row("s1")]);

    act(() => {
      result.current.pending!.resolve(true);
      // A second resolve of the same ask is a no-op, not a second settlement.
      result.current.pending?.resolve(false);
    });
    await expect(answer).resolves.toBe(true);
    expect(result.current.pending).toBeNull();
  });

  it("cancel declines the current ask", async () => {
    const { result } = renderHook(() => useOverwriteConfirmation());
    let answer!: Promise<boolean>;
    act(() => {
      answer = result.current.request([row("s1")]);
    });
    act(() => {
      result.current.cancel();
    });
    await expect(answer).resolves.toBe(false);
    expect(result.current.pending).toBeNull();
  });

  it("a newer request settles the displaced one as declined and owns the modal", async () => {
    const { result } = renderHook(() => useOverwriteConfirmation());
    let first!: Promise<boolean>;
    let second!: Promise<boolean>;
    act(() => {
      first = result.current.request([row("s1")]);
    });
    const displaced = result.current.pending!;
    act(() => {
      second = result.current.request([row("s2")]);
    });
    await expect(first).resolves.toBe(false);
    expect(result.current.pending?.rows).toEqual([row("s2")]);

    // The displaced ask's stale resolver can no longer touch the newer one.
    act(() => {
      displaced.resolve(true);
    });
    expect(result.current.pending?.rows).toEqual([row("s2")]);

    act(() => {
      result.current.pending!.resolve(true);
    });
    await expect(second).resolves.toBe(true);
    expect(result.current.pending).toBeNull();
  });

  it("unmounting settles a pending ask as declined", async () => {
    const { result, unmount } = renderHook(() => useOverwriteConfirmation());
    let answer!: Promise<boolean>;
    act(() => {
      answer = result.current.request([row("s1")]);
    });
    unmount();
    await expect(answer).resolves.toBe(false);
  });
});
