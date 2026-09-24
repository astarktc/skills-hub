import { useCallback, useEffect, useRef, useState } from "react";

/** One occupied sync target the operator may authorise overwriting. */
export type OverwriteRow = {
  skillId: string;
  skillName: string;
  toolKey: string;
  toolLabel: string;
  /** The occupied target path, as the backend reported it. */
  path: string;
};

export type OverwritePending = {
  rows: OverwriteRow[];
  resolve: (confirmed: boolean) => void;
};

/**
 * Overwrite confirmation: one building-block hook owning the single
 * pending-confirmation value the sync seam raises when a batch settled
 * `TARGET_EXISTS` rows — a target that exists with *different* content
 * (identical content is replaced silently by the batch's same-content rule
 * and never reaches here).
 *
 * `request` resolves with the operator's answer once the modal is answered
 * (or `cancel` is called, which resolves false). Ownership is explicit: the
 * current request lives in a ref, every request settles exactly once, a
 * newer request settles the one it displaces as declined (so no action can
 * wait forever behind a modal that no longer shows it), and unmounting
 * settles whatever is pending as declined. Unlike the shared-dir
 * confirmation this one is raised mid-action, under the loading overlay,
 * so its modal layers above the overlay and does not disable on `loading`.
 */
export function useOverwriteConfirmation() {
  const [pending, setPending] = useState<OverwritePending | null>(null);
  const current = useRef<OverwritePending | null>(null);

  const request = useCallback(
    (rows: OverwriteRow[]): Promise<boolean> =>
      new Promise<boolean>((resolve) => {
        // A newer ask displaces the older one: the older action continues
        // as declined rather than dangling behind a modal it no longer owns.
        current.current?.resolve(false);
        let settled = false;
        const entry: OverwritePending = {
          rows,
          resolve: (confirmed) => {
            if (settled) return;
            settled = true;
            if (current.current === entry) {
              current.current = null;
              setPending(null);
            }
            resolve(confirmed);
          },
        };
        current.current = entry;
        setPending(entry);
      }),
    [],
  );

  const cancel = useCallback(() => {
    current.current?.resolve(false);
  }, []);

  useEffect(
    () => () => {
      current.current?.resolve(false);
    },
    [],
  );

  return { pending, request, cancel };
}

export type OverwriteConfirmation = ReturnType<typeof useOverwriteConfirmation>;
