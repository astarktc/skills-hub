import { useEffect, useState, type ReactNode } from "react";
import type { TFunction } from "i18next";
import { Checkbox } from "@/components/ui/checkbox";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import { Field, FieldLabel, FieldLegend, FieldSet } from "@/components/ui/field";
import type { CountVerb } from "../../../hooks/useSurfaceState";
import {
  disableCounts,
  enableCounts,
  syncCounts,
  updateCounts,
  type AssignmentsOf,
} from "../../../lib/bulkCounts";
import type { ManagedSkill } from "../types";

type CountConfirmModalProps = {
  /** The wide verb asked about and the skills it acts on; null = closed (one dialog identity). */
  request: { verb: CountVerb; skills: readonly ManagedSkill[] } | null;
  /** The verb is running: Confirm pends, the dialog stays until it settles. */
  pending: boolean;
  /** Why a new action cannot start now, in words (another one is running). */
  blockedReason: string | null;
  /** Where each skill is assigned (the project world's held views); null while unread. */
  assignmentsOf: AssignmentsOf | null;
  /** The explicit read of every project's view, made as a Disable/Enable confirmation opens. */
  loadPlacementViews: () => Promise<void>;
  /** Sync to Tools: the Tools to pick from (installed, labelled) and the ones picked first. */
  toolChoices: readonly { id: string; label: string }[];
  defaultTools: readonly string[];
  /** Run the verb confirmed (Sync to Tools with the picked Tools). */
  onConfirm: (tools?: string[]) => void;
  onRequestClose: () => void;
  t: TFunction;
};

/**
 * A wide verb's count-bearing confirmation (D8; the bulk rail): what the
 * verb will touch, counted by the pure folds in `lib/bulkCounts.ts` before
 * it runs — "Disable 60 skills on 7 Tools — 412 Sync targets are removed and
 * kept disabled; …". Focus opens on Cancel (the answer is large even when it
 * is not destructive). Sync to Tools carries its Tool pick here, the counts
 * following the pick.
 */
export default function CountConfirmModal({
  request,
  pending,
  blockedReason,
  assignmentsOf,
  loadPlacementViews,
  toolChoices,
  defaultTools,
  onConfirm,
  onRequestClose,
  t,
}: CountConfirmModalProps) {
  const verb = request?.verb ?? null;
  const skills = request?.skills ?? [];
  const open = request !== null;

  // The Tool pick, per request (another verb or selection starts from the defaults).
  const requestKey = request ? `${request.verb}:${request.skills.map((skill) => skill.id).join(",")}` : null;
  const [pick, setPick] = useState<{ key: string | null; tools: string[] } | null>(null);
  const picked =
    pick && pick.key === requestKey
      ? pick.tools
      : defaultTools.filter((id) => toolChoices.some((tool) => tool.id === id));
  const setPicked = (tools: string[]) => setPick({ key: requestKey, tools });

  // Disable and Enable count project assignments: read them as it opens.
  const readsProjects = verb === "disable" || verb === "enable";
  useEffect(() => {
    if (open && readsProjects) void loadPlacementViews();
  }, [loadPlacementViews, open, readsProjects]);

  const n = (key: "skills" | "tools" | "targets" | "assignments", count: number) => t(`bulk.count.${key}`, { count });
  // The assignment count once the placements are read; until then the words alone.
  const assignmentsPhrase = (count: number | null) =>
    count === null ? t("bulk.confirm.someAssignments") : n("assignments", count);

  let title = "";
  let confirmLabel = "";
  let body: ReactNode = null;
  let confirmDisabled = skills.length === 0;
  const lines: string[] = [];

  if (verb === "disable") {
    const counts = disableCounts(skills, assignmentsOf);
    title = t("bulk.confirm.disable.title", { count: counts.skills });
    confirmLabel = t("bulk.confirm.disable.confirm");
    lines.push(
      t(counts.assignments === 0 ? "bulk.confirm.disable.bodyNoAssignments" : "bulk.confirm.disable.body", {
        skills: n("skills", counts.skills),
        tools: n("tools", counts.tools),
        targets: n("targets", counts.targets),
        assignments: assignmentsPhrase(counts.assignments),
      }),
    );
    if (counts.assignments === null) lines.push(t("bulk.confirm.readingProjects"));
    if (counts.alreadyDisabled > 0) lines.push(t("bulk.confirm.disable.already", { count: counts.alreadyDisabled }));
  } else if (verb === "enable") {
    const counts = enableCounts(skills, assignmentsOf);
    title = t("bulk.confirm.enable.title", { count: counts.skills });
    confirmLabel = t("bulk.confirm.enable.confirm");
    lines.push(
      t(counts.assignments === 0 ? "bulk.confirm.enable.bodyNoAssignments" : "bulk.confirm.enable.body", {
        skills: n("skills", counts.skills),
        tools: n("tools", counts.tools),
        targets: n("targets", counts.targets),
        assignments: assignmentsPhrase(counts.assignments),
      }),
    );
    if (counts.assignments === null) lines.push(t("bulk.confirm.readingProjects"));
    if (counts.alreadyEnabled > 0) lines.push(t("bulk.confirm.enable.already", { count: counts.alreadyEnabled }));
  } else if (verb === "update") {
    const counts = updateCounts(skills);
    title = t("bulk.confirm.update.title", { count: counts.skills });
    confirmLabel = t("bulk.confirm.update.confirm");
    lines.push(t("bulk.confirm.update.body", { skills: n("skills", counts.refreshable) }));
    if (counts.skipped > 0) lines.push(t("bulk.confirm.update.skipped", { count: counts.skipped }));
    confirmDisabled ||= counts.refreshable === 0;
  } else if (verb === "syncToTargets") {
    const counts = syncCounts(skills, picked);
    title =
      skills.length === 1
        ? t("bulk.confirm.sync.title", { count: 1, name: skills[0].name })
        : t("bulk.confirm.sync.title", { count: skills.length });
    confirmLabel = t("bulk.confirm.sync.confirm");
    confirmDisabled ||= picked.length === 0;
    const parts = [
      t("bulk.confirm.sync.added", { count: counts.added }),
      ...(counts.alreadySynced > 0 ? [t("bulk.confirm.sync.already", { count: counts.alreadySynced })] : []),
      ...(counts.disabled > 0 ? [t("bulk.confirm.sync.disabled", { count: counts.disabled })] : []),
    ];
    body = (
      <div className="flex flex-col gap-3">
        <FieldSet>
          <FieldLegend>{t("bulk.confirm.sync.legend")}</FieldLegend>
          <div className="grid grid-cols-2 gap-x-4 gap-y-1.5">
            {toolChoices.map((tool) => (
              <Field key={tool.id} className="flex-row items-center gap-2">
                <Checkbox
                  checked={picked.includes(tool.id)}
                  onCheckedChange={(checked) =>
                    setPicked(checked ? [...picked, tool.id] : picked.filter((id) => id !== tool.id))
                  }
                  disabled={pending}
                />
                <FieldLabel>{tool.label}</FieldLabel>
              </Field>
            ))}
          </div>
        </FieldSet>
        <p role="status" className="m-0 text-sm text-foreground">
          {picked.length === 0
            ? t("bulk.confirm.sync.none")
            : `${t("bulk.confirm.sync.body", {
                pairs: t("bulk.confirm.sync.pairs", { count: counts.pairs }),
                skills: n("skills", counts.skills),
                tools: n("tools", counts.tools),
              })} ${parts.join(" · ")}`}
        </p>
      </div>
    );
  }

  return (
    <ConfirmDialog
      open={open}
      title={title}
      confirmLabel={confirmLabel}
      cancelLabel={t("cancel")}
      initialFocus="cancel"
      pending={pending}
      confirmDisabled={confirmDisabled || (!pending && blockedReason !== null)}
      onConfirm={() => onConfirm(verb === "syncToTargets" ? picked : undefined)}
      onCancel={onRequestClose}
      size={verb === "syncToTargets" ? "lg" : "md"}
    >
      {body ?? (
        <div className="flex flex-col gap-2 text-sm text-foreground">
          {lines.map((line, index) => (
            <p key={index} className={index === 0 ? "m-0" : "m-0 text-muted-foreground"}>
              {line}
            </p>
          ))}
        </div>
      )}
    </ConfirmDialog>
  );
}
