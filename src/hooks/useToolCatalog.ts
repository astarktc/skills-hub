import { useCallback, useEffect, useMemo, useState } from "react";
import type { ToolOption, ToolStatusDto } from "../components/skills/types";
import { invokeTauri, isTauri } from "../lib/tauri";
import type { TranslateFn } from "./useStatusReporter";

/**
 * The Tool catalog (CONTEXT term): which AI tools Skills Hub knows, which
 * are installed, what each is called, and which share a skills dir. Read by
 * sync orchestration, the skill library, the Add flow and the project world.
 * It holds no target set and no sync policy — those are the sync world's.
 *
 * Two lists, as the backend defines them: the **global** tool status
 * (`getToolStatus`, loaded once at startup — the call also records
 * detection, so its `newly_installed` is answered only once) and the
 * **project** tool status (`getProjectToolStatus`, the project-scoped tool
 * entries), loaded on demand and re-read whenever the project world asks,
 * so a tool installed meanwhile shows up in its configuration dialog.
 */
export function useToolCatalog({ t }: { t: TranslateFn }) {
  const [toolStatus, setToolStatus] = useState<ToolStatusDto | null>(null);
  const [projectToolStatus, setProjectToolStatus] =
    useState<ToolStatusDto | null>(null);

  useEffect(() => {
    if (!isTauri) return;
    void (async () => {
      try {
        setToolStatus(await invokeTauri("getToolStatus"));
      } catch (err) {
        // Non-fatal; the app still works without detection.
        console.warn(err);
      }
    })();
  }, []);

  const toolInfos = useMemo(() => toolStatus?.tools ?? [], [toolStatus]);

  /** Every known tool, labelled (i18n label when present, else the backend's). */
  const tools: ToolOption[] = useMemo(
    () =>
      toolInfos.map((info) => ({
        id: info.key,
        label: t(`tools.${info.key}`, { defaultValue: info.label }),
      })),
    [t, toolInfos],
  );

  /** Tool key → display label, over the global and the project entries. */
  const toolLabelById = useMemo(() => {
    const out: Record<string, string> = {};
    for (const info of projectToolStatus?.tools ?? [])
      out[info.key] = t(`tools.${info.key}`, { defaultValue: info.label });
    for (const tool of tools) out[tool.id] = tool.label;
    return out;
  }, [projectToolStatus, t, tools]);

  const installedToolIds = useMemo(
    () => toolStatus?.installed ?? [],
    [toolStatus],
  );
  const installedTools = useMemo(
    () => tools.filter((tool) => installedToolIds.includes(tool.id)),
    [tools, installedToolIds],
  );

  /**
   * toolId → every toolId sharing its skills dir (groups of two or more).
   * The backend owns the grouping (`ToolInfoDto.shared_with`); never
   * re-derive it from `skills_dir`.
   */
  const sharedToolIdsByToolId = useMemo(() => {
    const out: Record<string, string[]> = {};
    for (const info of toolInfos) {
      if (info.shared_with.length > 1) out[info.key] = info.shared_with;
    }
    return out;
  }, [toolInfos]);

  /** Re-read the project tool entries; rejects when the read fails. */
  const loadProjectToolStatus = useCallback(async () => {
    const status = await invokeTauri("getProjectToolStatus");
    setProjectToolStatus(status);
    return status;
  }, []);

  return useMemo(
    () => ({
      toolStatus,
      tools,
      toolLabelById,
      installedToolIds,
      installedTools,
      sharedToolIdsByToolId,
      projectToolStatus,
      loadProjectToolStatus,
    }),
    [
      toolStatus,
      tools,
      toolLabelById,
      installedToolIds,
      installedTools,
      sharedToolIdsByToolId,
      projectToolStatus,
      loadProjectToolStatus,
    ],
  );
}

export type ToolCatalog = ReturnType<typeof useToolCatalog>;
