import { useCallback, useLayoutEffect, useMemo, useRef } from "react";
import type {
  InvocationMode,
  ManagedSkill,
  RefreshProgressDto,
  RepointTarget,
  SkillMutationResultDto,
  SkillOutcomeKind,
} from "../components/skills/types";
import type { BatchSyncSkillDto, CommandError, DisableScope, RefreshReport } from "../bindings";
import { toCommandError } from "../commandError";
import {
  combineOutcomes,
  deleteOutcome,
  type DeleteSettlement,
  enableOutcome,
  invocationEditOutcome,
  projectSyncOutcome,
  refreshOutcome,
  removalOutcome,
  syncOutcome,
} from "../lib/reportOutcome";
import {
  focusAfter,
  isFocusLost,
  resolveFirst,
  type FocusTarget,
} from "../lib/actionLifecycle";
import { staleCollision, type StaleCollision } from "../lib/notificationRuns";
import { createProgressChannel } from "../lib/platform";
import { invokeTauri } from "../lib/tauri";
import {
  deployAction,
  repointKind,
  type HealthRepair,
  type HealthRow,
  type RepointKind,
} from "../lib/skillPresentation";
import type { ProjectState } from "../components/projects/useProjectState";
import type { SkillLibrary } from "./useSkillLibrary";
import { ActionExit, type ActionHandle, type StatusReporter, type TranslateFn } from "./useStatusReporter";
import type { CollisionPart, SyncOrchestration, SyncPolicy } from "./useSyncOrchestration";
import type { ToolCatalog } from "./useToolCatalog";
import type { CountVerb, DialogDoor } from "./useSurfaceState";

/**
 * Every verb the skills world has, with what each takes beyond the skills it
 * acts on. A verb without a confirming option (`target`, `mode`,
 * `confirmed`) opens its dialog instead of acting: the door decides where
 * confirmation happens. `answering: "collision"` (Retry / Enable of one
 * pair) says the verb answers a collision recorded in a report (the
 * Notifications panel's Overwrite…, by the route that met it): the door
 * first checks the pair as it is now (`staleCollision`) and says so instead
 * of acting when the report went stale.
 */
export type SkillVerbOptions = {
  /**
   * Sync to Tools: one pair (`tool`, the pill), or — `confirmed` — the given
   * set (`tools`), else the effective targets. Without `tool` or `confirmed`
   * it opens the count-bearing Tool pick (`confirmCount`), for one skill or
   * several.
   */
  syncToTargets: { tool?: string; tools?: readonly string[]; confirmed?: boolean };
  /**
   * A re-sync from the central copy of the failed / missing / stale pairs:
   * one `tool`, or those of `tools` (a health row's Tools), else every such
   * row of each skill.
   */
  retry: { tool?: string; tools?: readonly string[]; answering?: "collision" };
  /**
   * Disable at pair grain (`tool`) or skill grain. Over several skills
   * without `confirmed` it opens the count-bearing confirmation; one skill is
   * never asked (reversible, and its switch shows the state — D8).
   */
  disable: { tool?: string; confirmed?: boolean };
  /** Enable at pair grain (`tool`) or skill grain; over several, `confirmed` as Disable. */
  enable: { tool?: string; confirmed?: boolean; answering?: "collision" };
  /** Over several skills (or "all") without `confirmed`: the count-bearing confirmation. */
  update: { confirmed?: boolean };
  restore: Record<string, never>;
  /** Without `target`: open Change source (on `preselect`, else the skill's kind). */
  repoint: { preselect?: RepointKind; target?: RepointTarget };
  /** Without `confirmed`: open the Detach confirmation (it states the consequence). */
  detach: { confirmed?: boolean };
  /**
   * Without `confirmed`: open the Delete confirmation (D16, one skill or
   * several). Confirmed: one `delete_managed_skill` per skill under one
   * action, folded as one run; a Delete that left anything behind turns the
   * dialog into its report.
   */
  delete: { confirmed?: boolean };
  /** Without `mode`: open the Invocation dialog; `null` clears the override. */
  editInvocation: { mode?: InvocationMode | null };
  /** The operator's Dismiss of a stored health kind (`import_failed` only). */
  dismissHealth: { kind: SkillOutcomeKind };
  /** Without `projectId`: open the Assign-to-project picker (one skill or several). */
  assignToProject: { projectId?: string };
  unassignFromProject: { projectId: string };
  /** A group's Deploy all: Enable what is disabled, sync what is not deployed. */
  deployGroup: Record<string, never>;
};
export type SkillVerb = keyof SkillVerbOptions;

/** The skills a verb acts on: records, ids (resolved at click time) or the whole library. */
export type SkillSelection = readonly ManagedSkill[] | readonly string[] | "all";

type VerbArgs<V extends SkillVerb> =
  Record<string, never> extends SkillVerbOptions[V]
    ? [opts?: SkillVerbOptions[V]]
    : [opts: SkillVerbOptions[V]];

/**
 * What a verb answers its caller. Only Re-point answers anything: its final
 * command's whole-command refusal (D15 — the dialog shows it by its choice,
 * never a toast over it), or null when there is none to show (it settled,
 * was cancelled, or never ran). Target fan-out failures stay report data,
 * folded like any Re-point's.
 */
export type SkillVerbAnswer<V extends SkillVerb> = V extends "repoint" ? CommandError | null : void;

/** The door's one entry point — the handle card, inspector, bulk rail and palette call. */
export type RunSkillAction = <V extends SkillVerb>(
  verb: V,
  skills: SkillSelection,
  ...opts: VerbArgs<V>
) => Promise<SkillVerbAnswer<V>>;

/**
 * The door's reload rule, per verb: a thrown request re-reads the catalog
 * when it may already have settled rows before throwing (a sync seam's
 * first batch before its overwrite retry, an Update's settlement before a
 * catalog read). A thrown Edit, Disable, Delete, Detach or project verb does
 * not reload (AGENTS.md: thrown Edit does not; project verbs converge on
 * their own view). A Delete never throws as a whole: each skill's thrown
 * command is a settlement of its run, which reloads through its fold.
 */
export const RELOAD_ON_THROW: Record<SkillVerb, boolean> = {
  syncToTargets: true,
  retry: true,
  deployGroup: true,
  enable: true,
  update: true,
  restore: true,
  repoint: true,
  disable: false,
  detach: false,
  delete: false,
  editInvocation: false,
  dismissHealth: false,
  assignToProject: false,
  unassignFromProject: false,
};

export type SkillActionsDeps = {
  t: TranslateFn;
  reporter: Pick<StatusReporter, "runAction" | "applyOutcome" | "notify" | "notifyError" | "reportProgress">;
  /**
   * The sync world: the asking seams for a one-batch verb, and — for a verb
   * that settles several reports or a report of another shape — the
   * non-asking collectors plus the one ask over every part
   * (`settleCollisionParts`).
   */
  sync: Pick<
    SyncOrchestration,
    | "autoSyncEnabled"
    | "effectiveSyncTargetIds"
    | "requestSharedDirConfirmation"
    | "syncSkillsToTools"
    | "enableSkillTargets"
    | "collectSync"
    | "collectEnable"
    | "settleCollisionParts"
  >;
  /** The Tool catalog: labels for reports and progress. */
  catalog: Pick<ToolCatalog, "toolLabelById">;
  /** The surface module: the dialogs a verb opens for its confirmation. */
  surface: Pick<DialogDoor, "openDialog" | "dismissDialog" | "updateDialog">;
  /** The skills world's catalog: resolution, replacement and reload. */
  library: Pick<SkillLibrary, "managedSkills" | "loadManagedSkills" | "replaceCatalog">;
  /** The project world's verbs (any project, the view applied) and its view re-read. */
  projects: Pick<ProjectState, "assignToProject" | "unassignFromProject" | "refreshView">;
  /** The showing surface's rows in display order (`lens.displayed`) — Delete's focus rule reads it. */
  order: readonly string[];
};

/** The `{skill_id, name, source_path}` batch item for a managed skill. */
const toSyncItem = (skill: ManagedSkill) => ({
  skill_id: skill.id,
  name: skill.name,
  source_path: skill.central_path,
});

/** Every door sync replaces identical content silently; a differing folder is asked (D12). */
const SYNC_POLICY: SyncPolicy = { overwriteIfSameContent: true };

/** What the operator reads when Overwrite… on a recorded collision no longer applies. */
const STALE_COLLISION_KEY: Record<StaleCollision, string> = {
  skillDisabled: "staleCollision.skillDisabled",
  pairDisabled: "staleCollision.pairDisabled",
  resolved: "staleCollision.resolved",
  changed: "staleCollision.changed",
};

/** The target statuses a central-to-target re-sync repairs. */
const RETRYABLE = new Set(["error", "missing", "stale"]);

/**
 * The skill action door (D7 candidate 2): every verb of the skills world
 * acts on a set of skills through one `run(verb, skills, opts)`. The door
 * decides one batch vs one skill at a time, where confirmation happens (the
 * verb's dialog, the shared-dir ask for a pair), which fold settles the
 * report, the reload rule (`RELOAD_ON_THROW`) and the post-mutation focus
 * (`focusAfter`, D21). Callers — card, inspector, bulk rail, palette,
 * binder — hold only the handle. The backend is reached only through
 * `invokeTauri` and the sync world's seam; every action runs under the
 * reporter's lifecycle (one at a time, per-subject pending) and every report
 * is executed by `applyOutcome`.
 */
export function useSkillActions({
  t,
  reporter,
  sync,
  catalog,
  surface,
  library,
  projects,
  order,
}: SkillActionsDeps) {
  const { runAction, applyOutcome, notify, notifyError, reportProgress } = reporter;
  const {
    autoSyncEnabled,
    effectiveSyncTargetIds,
    requestSharedDirConfirmation,
    syncSkillsToTools,
    enableSkillTargets,
    collectSync,
    collectEnable,
    settleCollisionParts,
  } = sync;
  const { toolLabelById } = catalog;
  const { openDialog, dismissDialog, updateDialog } = surface;
  const { managedSkills, loadManagedSkills, replaceCatalog } = library;
  const { assignToProject, unassignFromProject, refreshView } = projects;

  // Where focus goes when the Delete dialog closes over a removed skill.
  const afterDialogRef = useRef<FocusTarget | null>(null);

  const foldContext = { t, toolLabelById };
  const reload = () => loadManagedSkills();
  // Each skill's global Tools as the operator acted on them: a Refresh
  // report's synced global target outside this set is a re-assert addition,
  // announced in the run's report ("also deployed to Cursor, Pi").
  const priorGlobalTools = (skills: readonly ManagedSkill[]) =>
    Object.fromEntries(
      skills.map((skill) => [skill.id, skill.targets.filter((target) => target.status !== "disabled").map((target) => target.tool)]),
    );

  const newRefreshProgressChannel = () =>
    createProgressChannel<RefreshProgressDto>((progress) => {
      reportProgress({
        done: progress.index,
        total: progress.total,
        // The token is honoured between acquisitions only: once a skill is
        // being applied, nothing would stop (D20) — Cancel goes.
        ...(progress.phase === "applying" ? { cancellable: false as const } : {}),
        message: t(
          progress.phase === "acquiring" ? "actions.refreshFetchStep" : "actions.refreshApplyStep",
          { index: progress.index, total: progress.total, name: progress.skill_name },
        ),
      });
    });

  // Typed at the handle (`RunSkillAction`): each verb's answer is `SkillVerbAnswer`.
  const run = (async (verb: SkillVerb, selection: SkillSelection, ...rest: [opts?: unknown]): Promise<CommandError | null | void> => {
    const opts = (rest[0] ?? {}) as Partial<SkillVerbOptions[SkillVerb]> & Record<string, unknown>;
    const all = selection === "all";
    const skills: ManagedSkill[] = all
      ? managedSkills
      : selection.flatMap((entry) => {
          const id = typeof entry === "string" ? entry : entry.id;
          const found = managedSkills.find((skill) => skill.id === id);
          return found ? [found] : [];
        });
    // Every exit answers the verb's `SkillVerbAnswer`: Re-point's is
    // `CommandError | null`, so its no-op and dialog-opening exits say null.
    if (skills.length === 0 && verb !== "update") return verb === "repoint" ? null : undefined;
    const single = skills.length === 1 ? skills[0] : null;
    const skillIds = skills.map((skill) => skill.id);
    // A batch names its skills: each row carries the pending mark (D19).
    const subject = single ? ({ kind: "skill", id: single.id } as const) : ({ kind: "batch", ids: skillIds } as const);
    const trigger = typeof document !== "undefined" ? document.activeElement : null;

    // A wide verb (several skills, or the library) states its counts before
    // it runs: without `confirmed` it opens the count-bearing confirmation,
    // whose Confirm re-runs it confirmed. Sync to Tools always asks — its
    // confirmation is also the Tool pick.
    const confirmed = opts.confirmed === true;
    const wide = all || skills.length > 1;
    const asksCount =
      !confirmed &&
      ((verb === "syncToTargets" && opts.tool === undefined) ||
        (wide && (verb === "disable" || verb === "enable" || verb === "update")));
    if (asksCount) {
      openDialog({ kind: "confirmCount", verb: verb as CountVerb, skillIds });
      return;
    }
    // A count confirmation has done its work once confirmed: it closes as
    // the verb starts (focus returns to its opener, the rail's button), so
    // the run shows in the header progress over a browsable library (D19) —
    // never under a modal. Its counts describe a state that is about to go.
    if (confirmed) dismissDialog("confirmCount");

    // A recorded collision answered now: only while the pair still holds it
    // (the report is history). A stale one runs nothing and says why.
    if (opts.answering === "collision" && (verb === "retry" || verb === "enable")) {
      const tool = opts.tool as string | undefined;
      if (!single || tool === undefined) return;
      const stale = staleCollision(single, tool, verb === "enable" ? "enable" : "sync");
      if (stale) {
        notify("info", t(STALE_COLLISION_KEY[stale], { name: single.name, tool: toolLabelById[tool] ?? tool }));
        return;
      }
    }

    /** Run `body` under the lifecycle, with the verb's reload rule. */
    const settle = async <T,>(
      runOpts: Parameters<typeof runAction>[0],
      body: (action: ActionHandle) => Promise<T | ActionExit>,
    ): Promise<T | undefined> => {
      const value = await runAction(runOpts, async (action) => {
        try {
          return await body(action);
        } catch (error) {
          if (RELOAD_ON_THROW[verb]) await loadManagedSkills();
          throw error;
        }
      });
      restoreFocus();
      return value;
    };

    /**
     * D21: after the action, focus goes where the rule says — the same
     * control, or Restore's health row — once the settled render is on
     * screen (a control disabled while pending dropped focus to `<body>`;
     * an overwrite ask raised mid-action returned it to nothing). A verb
     * confirmed in a dialog leaves focus to the Dialog primitive's return;
     * focus the operator moved elsewhere is never taken back. One placement,
     * no hold: no layer competes for focus once the action settled (the
     * progress is the header's, never a layer — D19).
     */
    const restoreFocus = () => {
      if (verb === "delete" || typeof window === "undefined") return;
      // (The inspector's sheet is a dialog too, but the verbs in it are its
      // own controls, not a confirmation: the rule applies there.)
      if (trigger?.closest("[role='dialog']:not([data-inspector]), [role='alertdialog']")) return;
      const target = focusAfter({ verb, removed: false }, { skillId: single?.id ?? null, order, surface: "skills" });
      const chain = [target, { kind: "trigger" } as const, ...(single ? [{ kind: "row", skillId: single.id } as const] : [])];
      const place = (settling: boolean) => {
        const active = document.activeElement;
        if (!isFocusLost(active) && active !== trigger) return;
        // The settled render re-enables the control a frame later at most:
        // until then the rule's own target waits (one frame), then the chain.
        if (settling && !resolveFirst([target], trigger)) {
          window.requestAnimationFrame(() => place(false));
          return;
        }
        resolveFirst(chain, trigger)?.focus();
      };
      window.setTimeout(() => place(true), 0);
    };

    /**
     * The shared-dir ask for a pair: a Tool sharing its skills dir affects
     * the group. A failed ask is reported here — `run` is fired and
     * forgotten (`void run(...)`), so nothing may escape it — and acts as a no.
     */
    const confirmPair = async (tool: string | undefined) => {
      if (tool === undefined) return true;
      try {
        return await requestSharedDirConfirmation(tool);
      } catch (error) {
        notifyError(error);
        return false;
      }
    };

    /**
     * One operator action's reports through the one overwrite ask (D12):
     * every part's collisions asked once, the answered pairs retried, each
     * part back with its rows settled. The ask and its retries run no
     * token-taking command, so Cancel is withdrawn meanwhile (D20). After a
     * retry the catalog read before it is stale: re-read it, and the views
     * of the projects the retry touched.
     */
    const settleParts = async <P extends readonly CollisionPart[]>(action: ActionHandle, parts: P) => {
      const settled = await action.uncancellable(() => settleCollisionParts(parts));
      if (settled.retried) {
        await loadManagedSkills();
        for (const projectId of settled.projectIds) await refreshView(projectId);
      }
      return settled;
    };

    /** A Refresh / Update report's collisions (Propagation, the re-assert), asked once. */
    const settleRefresh = async (action: ActionHandle, report: RefreshReport, catalogNow: readonly ManagedSkill[]) => {
      const { parts: [part], retried } = await settleParts(action, [
        { kind: "refresh", report, skills: catalogNow.map(toSyncItem) },
      ] as const);
      return { report: part.report, declined: part.declined, retried };
    };

    const refreshOne = (skill: ManagedSkill, copy: { message: string; success: string; label: string }, request: (action: ActionHandle) => Promise<SkillMutationResultDto | ActionExit>, cancel: "updateManagedSkill" | "repointSkillSource") =>
      settle({ message: copy.message, activity: copy.message, label: copy.label, subject: { kind: "skill", id: skill.id }, cancel }, async (action) => {
        const response = await request(action);
        if (response instanceof ActionExit) return response;
        replaceCatalog(response.skills);
        const { report, declined } = await settleRefresh(action, response.report, response.skills);
        return applyOutcome(
          refreshOutcome(report, {
            ...foldContext,
            single: { name: skill.name, success: copy.success },
            declined,
            priorGlobalTools: priorGlobalTools([skill]),
          }),
        );
      });

    switch (verb) {
      case "syncToTargets": {
        const { tool, tools } = opts as SkillVerbOptions["syncToTargets"];
        if (tool !== undefined) {
          if (!single || !(await confirmPair(tool))) return;
          const toolLabel = toolLabelById[tool] ?? tool;
          await settle({ message: t("actions.syncing", { name: single.name, tool: toolLabel }), subject, label: t("lifecycle.verbs.sync"), activity: t("activity.syncing") }, async () => {
            const report = await syncSkillsToTools([toSyncItem(single)], [tool], { overwriteIfSameContent: true });
            await applyOutcome(syncOutcome(report, { ...foldContext, action: "toggle" }), { reload });
          });
          return;
        }
        // Zero work is reported, never silent: an operator who selected no
        // tools clicked a deploy button and must learn why nothing happened.
        const targets = tools ?? effectiveSyncTargetIds;
        if (targets.length === 0) {
          notify("warning", t("noSyncTargets"));
          return;
        }
        await settle({ subject, label: t("lifecycle.verbs.sync"), activity: t("activity.syncing") }, async () => {
          const report = await syncSkillsToTools(skills.map(toSyncItem), [...targets], { overwriteIfSameContent: true });
          await applyOutcome(syncOutcome(report, { ...foldContext, action: "bulk" }), { reload });
        });
        return;
      }
      case "retry": {
        const { tool, tools } = opts as SkillVerbOptions["retry"];
        // Exactly the pairs to repair: each skill's own Tools, never the
        // product of every skill with every Tool.
        const bySet = new Map<string, { tools: string[]; items: BatchSyncSkillDto[] }>();
        for (const skill of skills) {
          const toolIds =
            tool !== undefined
              ? [tool]
              : skill.targets
                  .filter((target) => RETRYABLE.has(target.status) && (!tools || tools.includes(target.tool)))
                  .map((target) => target.tool);
          if (toolIds.length === 0) continue;
          const key = toolIds.join("\n");
          const batch = bySet.get(key) ?? { tools: toolIds, items: [] };
          batch.items.push(toSyncItem(skill));
          bySet.set(key, batch);
        }
        if (bySet.size === 0) return;
        await settle({ subject, label: t("lifecycle.verbs.sync"), activity: t("activity.syncing") }, async (action) => {
          // Every batch first, then one ask over all their collisions.
          const parts: CollisionPart[] = [];
          for (const batch of bySet.values()) {
            const report = await collectSync(batch.items, batch.tools, SYNC_POLICY);
            parts.push({ kind: "sync", report, skills: batch.items, policy: SYNC_POLICY });
          }
          const { parts: settled } = await settleParts(action, parts);
          const report = settled.flatMap((part) => (part.kind === "sync" ? part.report : []));
          await applyOutcome(syncOutcome(report, { ...foldContext, action: single ? "toggle" : "bulk" }), { reload });
        });
        return;
      }
      case "disable": {
        const { tool } = opts as SkillVerbOptions["disable"];
        if (tool !== undefined && !(single && (await confirmPair(tool)))) return;
        const scope: DisableScope =
          tool !== undefined && single
            ? { kind: "skill_tool", skill_id: single.id, tool }
            : single && !all
              ? { kind: "skill", skill_id: single.id }
              : { kind: "skills", skill_ids: skills.map((skill) => skill.id) };
        const action = scope.kind === "skill_tool" ? "toggle" : scope.kind === "skill" ? "skill" : "all";
        await settle(
          {
            subject,
            label: t("lifecycle.verbs.disable"),
            activity: t("activity.disabling"),
            ...(single && tool !== undefined
              ? { message: t("actions.unsyncing", { name: single.name, tool: toolLabelById[tool] ?? tool }) }
              : {}),
          },
          async () => {
            const report = await invokeTauri("disableSkillTargets", scope);
            await applyOutcome(removalOutcome(report, { ...foldContext, action }), { reload });
          },
        );
        return;
      }
      case "enable": {
        const { tool } = opts as SkillVerbOptions["enable"];
        if (tool !== undefined && !(single && (await confirmPair(tool)))) return;
        const scope: DisableScope =
          tool !== undefined && single
            ? { kind: "skill_tool", skill_id: single.id, tool }
            : single
              ? { kind: "skill", skill_id: single.id }
              : { kind: "skills", skill_ids: skills.map((skill) => skill.id) };
        await settle({ subject, label: t("lifecycle.verbs.enable"), activity: t("activity.enabling") }, async () => {
          const report = await enableSkillTargets(scope);
          await applyOutcome(enableOutcome(report, { ...foldContext, batch: scope.kind === "skills" }), { reload });
        });
        return;
      }
      case "update": {
        // One skill is a batch of one (its catalog comes back with it); a
        // set — or the whole library — is one Refresh batch.
        if (single && !all) {
          await refreshOne(
            single,
            {
              message: t("actions.updating", { name: single.name }),
              success: t("status.updated", { name: single.name }),
              label: t("lifecycle.verbs.update"),
            },
            async () =>
              invokeTauri("updateManagedSkill", single.id, { reassert_auto_sync: autoSyncEnabled }, await newRefreshProgressChannel()),
            "updateManagedSkill",
          );
          return;
        }
        if (managedSkills.length === 0) return;
        const refreshed = all ? managedSkills : skills;
        await settle({
          // The rows it will refresh pend (a skill with no source is not in the run).
          subject: { kind: "batch", ids: refreshed.filter((skill) => skill.refreshable).map((skill) => skill.id) },
          label: t("lifecycle.verbs.refresh"),
          activity: t("activity.refreshing"),
          cancel: "refreshManagedSkills",
        }, async (action) => {
          const settled = await invokeTauri(
            "refreshManagedSkills",
            all ? null : skills.map((skill) => skill.id),
            { reassert_auto_sync: autoSyncEnabled },
            await newRefreshProgressChannel(),
          );
          const { report, declined, retried } = await settleRefresh(action, settled, managedSkills);
          // A retry already re-read the catalog.
          await applyOutcome(refreshOutcome(report, { ...foldContext, declined, priorGlobalTools: priorGlobalTools(refreshed) }), {
            reload: retried ? undefined : reload,
          });
        });
        return;
      }
      case "restore": {
        // Restore an Unlocatable skill whose central copy is gone: the same
        // batch of one — the backend re-acquires it and Propagation follows.
        for (const skill of skills) {
          await refreshOne(
            skill,
            {
              message: t("actions.restoring", { name: skill.name }),
              success: t("status.restored", { name: skill.name }),
              label: t("lifecycle.verbs.restore"),
            },
            async () =>
              invokeTauri("updateManagedSkill", skill.id, { reassert_auto_sync: autoSyncEnabled }, await newRefreshProgressChannel()),
            "updateManagedSkill",
          );
        }
        return;
      }
      case "repoint": {
        if (!single) return null;
        const { preselect, target } = opts as SkillVerbOptions["repoint"];
        if (!target) {
          openDialog({ kind: "repoint", skillId: single.id, name: single.name, preselect: preselect ?? repointKind(single) });
          return null;
        }
        // A folder may be typed with `~`; the backend expands it.
        const trimmed: RepointTarget =
          target.kind === "git"
            ? { ...target, url: target.url.trim() }
            : { kind: "local", path: target.path.trim() };
        // The final command's own refusal (a clone that fails after a good
        // preview, a folder gone since) is answered to the dialog, which
        // shows it by the chosen target — one action, no toast over it. A
        // cancellation stays silent; report failures stay the fold's.
        let refusal: CommandError | null = null;
        const completion = await refreshOne(
          single,
          {
            message: t("actions.repointing", { name: single.name }),
            success: t("status.repointed", { name: single.name }),
            label: t("lifecycle.verbs.repoint"),
          },
          async (action) => {
            try {
              return await invokeTauri("repointSkillSource", single.id, trimmed, { reassert_auto_sync: autoSyncEnabled });
            } catch (err) {
              const error = toCommandError(err);
              if (error.code === "CANCELLED") throw err;
              // A thrown Re-point may have settled rows first (RELOAD_ON_THROW).
              await loadManagedSkills();
              refusal = error;
              return action.handOff();
            }
          },
          "repointSkillSource",
        );
        if (completion?.closeModal) dismissDialog("repoint");
        return refusal;
      }
      case "detach": {
        if (!single) return;
        if (!confirmed) {
          openDialog({ kind: "detachSkill", skillId: single.id });
          return;
        }
        await settle(
          {
            subject,
            label: t("lifecycle.verbs.detach"),
            message: t("actions.detaching", { name: single.name }),
            successToast: t("status.detached", { name: single.name }),
          },
          async () => {
            await invokeTauri("detachSkillFromSource", single.id);
            await loadManagedSkills();
            dismissDialog("detachSkill");
          },
        );
        return;
      }
      case "delete": {
        if (!confirmed) {
          openDialog({ kind: "deleteSkill", skillIds });
          return;
        }
        afterDialogRef.current = null;
        // One skill pends on its row; several pend on theirs, and the Delete
        // dialog — not the progress surface — shows the count (D16, D19).
        const deleteSubject = single ? subject : ({ kind: "skills", ids: skillIds } as const);
        await settle(
          {
            subject: deleteSubject,
            label: t("lifecycle.verbs.delete"),
            activity: single ? t("actions.removing", { name: single.name }) : t("activity.removing"),
            message: single ? t("actions.removing", { name: single.name }) : undefined,
          },
          async () => {
            // The backend removes one skill per command (a multi-skill
            // delete command is a follow-up): each settles on its own, a
            // thrown skill is reported, never the end of the run.
            const settled: DeleteSettlement[] = [];
            for (const [index, skill] of skills.entries()) {
              if (!single)
                reportProgress({
                  done: index,
                  total: skills.length,
                  message: t("actions.removing", { name: skill.name }),
                });
              try {
                settled.push({ skillId: skill.id, name: skill.name, report: await invokeTauri("deleteManagedSkill", skill.id) });
              } catch (error) {
                settled.push({ skillId: skill.id, name: skill.name, error: toCommandError(error) });
              }
            }
            // Decided before the dialog closes: its opener (the row) is gone.
            const removedIds = settled.flatMap((part) =>
              "report" in part && part.report.record_deleted ? [part.skillId] : [],
            );
            afterDialogRef.current =
              removedIds.length > 0
                ? focusAfter({ verb: "delete", removed: true }, { skillId: removedIds[0], removedIds, order, surface: "skills" })
                : null;
            const outcome = deleteOutcome(settled, foldContext);
            // Anything left behind: the same dialog becomes the report (before
            // the reload, so the confirmation never re-renders in between).
            if (!outcome.completion.closeModal) updateDialog({ kind: "deleteSkill", skillIds, report: settled });
            await applyOutcome(outcome, { reload, closeModal: () => dismissDialog("deleteSkill") });
          },
        );
        return;
      }
      case "editInvocation": {
        if (!single) return;
        const { mode } = opts as SkillVerbOptions["editInvocation"];
        if (mode === undefined) {
          openDialog({ kind: "invocationEdit", skillId: single.id });
          return;
        }
        await settle({ subject, label: t("lifecycle.verbs.editInvocation") }, async (action) => {
          const { report, skills: catalogAfter } = await invokeTauri("setSkillInvocationOverride", single.id, mode);
          replaceCatalog(catalogAfter);
          // Propagation's collisions are the one ask too.
          const skillNow = catalogAfter.find((row) => row.id === single.id) ?? single;
          const { parts: [part] } = await settleParts(action, [
            { kind: "propagation", report: report.propagation, skill: toSyncItem(skillNow) },
          ] as const);
          await applyOutcome(
            invocationEditOutcome({ ...report, propagation: part.report }, { ...foldContext, declined: part.declined }),
            { closeModal: () => dismissDialog("invocationEdit") },
          );
        });
        return;
      }
      case "dismissHealth": {
        const { kind } = opts as SkillVerbOptions["dismissHealth"];
        await settle({ subject, label: t("lifecycle.verbs.dismiss") }, async () => {
          for (const skill of skills) await invokeTauri("clearSkillHealth", skill.id, kind);
          await loadManagedSkills();
        });
        return;
      }
      case "assignToProject":
      case "unassignFromProject": {
        const { projectId } = opts as SkillVerbOptions["assignToProject"];
        if (projectId === undefined) {
          if (verb === "assignToProject") openDialog({ kind: "assignProject", skillIds });
          return;
        }
        const ids = skills.map((skill) => skill.id);
        await settle({ subject, label: t("lifecycle.verbs.assign"), activity: t("activity.assigning") }, async () => {
          // The project world assigns the set (the backend command is per
          // skill) and asks about every collision once; one report, one fold.
          if (verb === "assignToProject") {
            const report = await assignToProject(projectId, ids);
            await applyOutcome(projectSyncOutcome(report, { ...foldContext, action: "bulkAssign" }));
          } else {
            const report = await unassignFromProject(projectId, ids);
            await applyOutcome(removalOutcome(report, { ...foldContext, action: "bulkUnassign" }));
          }
        });
        return;
      }
      case "deployGroup": {
        // Runnable work is decided before anything runs: Enable what is
        // disabled; sync what never deployed — only when there are targets.
        const toEnable = skills.filter((skill) => deployAction(skill).kind === "enable");
        const toSync = skills.filter((skill) => deployAction(skill).kind === "sync");
        const targets = [...effectiveSyncTargetIds];
        const syncRunnable = toSync.length > 0 && targets.length > 0;
        if (toEnable.length === 0 && toSync.length === 0) return;
        if (toEnable.length === 0 && !syncRunnable) {
          notify("warning", t("noSyncTargets"));
          return;
        }
        await settle({ subject: { kind: "batch", ids: [...toEnable, ...toSync].map((skill) => skill.id) }, label: t("lifecycle.verbs.deploy"), activity: t("activity.deploying") }, async (action) => {
          // Both reports first, then one ask over both, one fold, one run.
          const scope: DisableScope = { kind: "skills", skill_ids: toEnable.map((skill) => skill.id) };
          const items = toSync.map(toSyncItem);
          const enabled = toEnable.length > 0 ? await collectEnable(scope) : null;
          const synced = syncRunnable ? await collectSync(items, targets, SYNC_POLICY) : [];
          const {
            parts: [enablePart, syncPart],
          } = await settleParts(action, [
            { kind: "enable", report: enabled ?? { global: [], projects: { items: [], skipped_projects: [] } }, scope },
            { kind: "sync", report: synced, skills: items, policy: SYNC_POLICY },
          ] as const);
          // One composite count line over both parts ("1 synced · 1 enabled"), toned
          // by the strongest outcome of either.
          const outcome = combineOutcomes(
            [
              ...(syncRunnable ? [syncOutcome(syncPart.report, { ...foldContext, action: "bulk" })] : []),
              ...(enabled ? [enableOutcome(enablePart.report, foldContext)] : []),
            ],
            foldContext,
          );
          // The undeployed part with no target to go to is reported, never silent.
          if (toSync.length > 0 && !syncRunnable)
            outcome.warnings.push({
              title: t("noSyncTargets"),
              message: t("lifecycle.deployNoTargets", { count: toSync.length }),
            });
          // Enable rewrote rows: one reload after every mutating phase.
          await applyOutcome(outcome, { reload });
        });
        return;
      }
    }
  }) as RunSkillAction;

  /**
   * The Delete dialog's `fallbackFocus` (D21): its opener (the row) is gone
   * after a completed Delete, so the next row — the previous one when it was
   * last, the surface heading when the list emptied. Null leaves the
   * Dialog primitive's own chain (layer beneath, main landmark).
   */
  const dialogFallbackFocus = useCallback(
    (usable?: (element: Element) => boolean) =>
      afterDialogRef.current
        ? resolveFirst([afterDialogRef.current, { kind: "heading", surface: "skills" }], null, usable)
        : null,
    [],
  );

  /**
   * A health row's repair, as the verb it is (`skillHealth`'s `repairs`):
   * the one mapping every reader of the fold (card, ladder, banner) uses.
   * A missing source folder re-points onto a folder.
   */
  const repair = (skill: ManagedSkill, row: HealthRow, key: HealthRepair): Promise<void> => {
    switch (key) {
      case "restore":
        return run("restore", [skill]);
      case "repoint":
        // Without a target it only opens the dialog: nothing to answer.
        return run("repoint", [skill], row.kind === "source_missing" ? { preselect: "local" } : {}).then(() => undefined);
      case "detach":
        return run("detach", [skill]);
      case "update":
        return run("update", [skill]);
      case "editInvocation":
        return run("editInvocation", [skill]);
      case "delete":
        return run("delete", [skill]);
      case "dismiss":
        return row.kind === "import_failed" ? run("dismissHealth", [skill], { kind: row.kind }) : Promise.resolve();
      case "retryTargets":
        // The row's own Tools, re-synced from the central copy.
        return run("retry", [skill], row.tools.length > 0 ? { tools: row.tools } : {});
      case "enable":
        return run("enable", [skill]);
      case "deploy":
        // The repair names its targets (the effective set): no pick.
        return run("syncToTargets", [skill], { confirmed: true });
      case "disable":
        // Finish the skill-grain Disable its failed targets interrupted.
        return run("disable", [skill]);
    }
  };

  // The handle is stable across renders (callers are memoised rows); each
  // call runs this render's implementation, which reads this render's
  // catalog — a skill id resolves at click time.
  const latest = useRef({ run, repair });
  useLayoutEffect(() => {
    latest.current = { run, repair };
  });
  return useMemo(
    () => ({
      run: ((...args: Parameters<RunSkillAction>) => latest.current.run(...args)) as RunSkillAction,
      repair: (...args: Parameters<typeof repair>) => latest.current.repair(...args),
      dialogFallbackFocus,
    }),
    [dialogFallbackFocus],
  );
}

export type SkillActions = ReturnType<typeof useSkillActions>;
