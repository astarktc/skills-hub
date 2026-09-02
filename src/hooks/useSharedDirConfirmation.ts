import { useCallback, useMemo, useState } from "react";

/**
 * The minimal per-tool facts the confirmation needs: the tool key, its
 * display label, and the backend-owned shared-skills-dir group it belongs
 * to (`ToolInfoDto.shared_with`, never re-derived from `skills_dir`).
 */
export type SharedDirTool = {
  id: string;
  label: string;
  sharedWith: string[];
};

export type SharedDirPending = {
  toolKey: string;
  toolLabel: string;
  /** The other group members' labels, in the group's order. */
  labels: string[];
  resolve: (confirmed: boolean) => void;
};

/**
 * Shared-skills-dir confirmation: one building-block hook owning the
 * decision ("does changing this Tool affect others?"), the label
 * arithmetic, and the single pending-confirmation value both the per-skill
 * toggle and the sync-target change drive through the Modal shell.
 *
 * `request` resolves true immediately when no confirmation is needed, and
 * otherwise resolves with the operator's answer once the modal is answered
 * (or `cancel` is called, which resolves false).
 */
export function useSharedDirConfirmation(tools: SharedDirTool[]) {
  const [pending, setPending] = useState<SharedDirPending | null>(null);

  const byId = useMemo(() => {
    const out: Record<string, SharedDirTool> = {};
    for (const tool of tools) out[tool.id] = tool;
    return out;
  }, [tools]);

  const otherMembers = useCallback(
    (toolKey: string) =>
      (byId[toolKey]?.sharedWith ?? []).filter((id) => id !== toolKey),
    [byId],
  );

  const needsConfirmation = useCallback(
    (toolKey: string) => otherMembers(toolKey).length > 0,
    [otherMembers],
  );

  const sharedLabels = useCallback(
    (toolKey: string) =>
      otherMembers(toolKey).map((id) => byId[id]?.label ?? id),
    [byId, otherMembers],
  );

  const request = useCallback(
    (toolKey: string): Promise<boolean> => {
      if (!needsConfirmation(toolKey)) return Promise.resolve(true);
      return new Promise<boolean>((resolve) => {
        setPending({
          toolKey,
          toolLabel: byId[toolKey]?.label ?? toolKey,
          labels: sharedLabels(toolKey),
          resolve: (confirmed) => {
            setPending(null);
            resolve(confirmed);
          },
        });
      });
    },
    [byId, needsConfirmation, sharedLabels],
  );

  const cancel = useCallback(() => {
    pending?.resolve(false);
  }, [pending]);

  return { needsConfirmation, sharedLabels, pending, request, cancel };
}

export type SharedDirConfirmation = ReturnType<typeof useSharedDirConfirmation>;
