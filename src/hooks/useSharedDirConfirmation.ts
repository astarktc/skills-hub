import { useCallback, useMemo } from "react";
import type { AskAnswer, SurfaceState } from "./useSurfaceState";

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

/** The ask as its modal renders it (`surface.pendingAsk("sharedDir")`). */
export type SharedDirPending = {
  toolKey: string;
  toolLabel: string;
  /** The other group members' labels, in the group's order. */
  labels: string[];
  answer: (answer: AskAnswer) => void;
};

/**
 * Shared-skills-dir confirmation: one building-block hook owning the
 * decision ("does changing this Tool affect others?") and the label
 * arithmetic both the per-skill toggle and the sync-target change use.
 *
 * `request` resolves true immediately when no confirmation is needed, and
 * otherwise raises the ask on the surface stack (`asks.ask`) and resolves
 * true only for an explicit Continue. Its "no" records nothing durable (the
 * toggle simply does not happen), so the explicit Cancel and a dismissal
 * (Escape, displaced, dropped) are the same answer here: false. The
 * three-way `AskAnswer` matters where a "no" is itself recorded — the
 * overwrite ask's Keep.
 */
export function useSharedDirConfirmation(
  tools: SharedDirTool[],
  asks: Pick<SurfaceState, "ask">,
) {
  const { ask } = asks;

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
      return ask({
        kind: "sharedDir",
        toolKey,
        toolLabel: byId[toolKey]?.label ?? toolKey,
        labels: sharedLabels(toolKey),
      }).then((answer) => answer === "confirmed");
    },
    [ask, byId, needsConfirmation, sharedLabels],
  );

  return { needsConfirmation, sharedLabels, request };
}

export type SharedDirConfirmation = ReturnType<typeof useSharedDirConfirmation>;
