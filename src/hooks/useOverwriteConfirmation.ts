import { useCallback, useState } from "react";

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
 * (or `cancel` is called, which resolves false). Unlike the shared-dir
 * confirmation this one is raised mid-action, under the loading overlay,
 * so its modal layers above the overlay and does not disable on `loading`.
 */
export function useOverwriteConfirmation() {
  const [pending, setPending] = useState<OverwritePending | null>(null);

  const request = useCallback(
    (rows: OverwriteRow[]): Promise<boolean> =>
      new Promise<boolean>((resolve) => {
        setPending({
          rows,
          resolve: (confirmed) => {
            setPending(null);
            resolve(confirmed);
          },
        });
      }),
    [],
  );

  const cancel = useCallback(() => {
    pending?.resolve(false);
  }, [pending]);

  return { pending, request, cancel };
}

export type OverwriteConfirmation = ReturnType<typeof useOverwriteConfirmation>;
