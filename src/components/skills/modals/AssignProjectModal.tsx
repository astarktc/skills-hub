import { useEffect, useId, useState } from "react";
import type { TFunction } from "i18next";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { FieldLegend, FieldSet } from "@/components/ui/field";
import { focusRing } from "@/components/ui/styles";
import { assignTargetState } from "@/lib/projectPresentation";
import { cn } from "@/lib/utils";
import type { SkillPlacement, SkillPlacementValue } from "../../projects/useProjectState";
import type { ManagedSkill } from "../types";

type AssignProjectModalProps = {
  /** The skills to assign (one, or a selection); empty = closed (one dialog identity for its life). */
  skills: readonly ManagedSkill[];
  loading: boolean;
  /** Where each skill is assigned (aligned with `skills`): the project world's held views. */
  placements: readonly SkillPlacementValue[];
  /** The explicit read of every project's view (stable), made as the picker opens. */
  loadPlacementViews: () => Promise<void>;
  labelOf: (tool: string) => string;
  /** Assign to the chosen project's configured Tools (the door's `assignToProject`). */
  onConfirm: (projectId: string) => void;
  onRequestClose: () => void;
  t: TFunction;
};

/**
 * Assign to project…: one project, and the skill — or every selected skill
 * (the bulk rail) — lands on every Tool that project is configured with; each
 * project shows its Tools and what assigning adds, so the choice is made
 * knowing where it goes (the picker is the selection's count-bearing
 * confirmation). A project with no Tools cannot take it, nor can one whose
 * folder is missing (the backend skips it whole either way — this is the
 * reason in words, not the safety rule); a project that already has every
 * skill on every Tool says so. Per-Tool choices stay in the project's matrix
 * (D26).
 */
export default function AssignProjectModal({
  skills,
  loading,
  placements,
  loadPlacementViews,
  labelOf,
  onConfirm,
  onRequestClose,
  t,
}: AssignProjectModalProps) {
  const [choice, setChoice] = useState<{ skillId: string; projectId: string } | null>(null);
  const formId = useId();
  // One key per request: a choice made for another selection never carries over.
  const skillId = skills.length > 0 ? skills.map((skill) => skill.id).join(",") : null;
  const single = skills.length === 1 ? skills[0] : null;
  // The project list and each project's Tools are the same from every skill's
  // view; a read failure of any is a read failure of the picker.
  const placement: SkillPlacementValue = {
    rows: placements.length > 0 && placements.every((value) => value.rows !== null) ? placements[0].rows : null,
    failed: placements.some((value) => value.failed),
  };
  /** The Tools of `project` each selected skill is not on yet. */
  const missingFor = (row: SkillPlacement) =>
    placements.map((value) => {
      const own = value.rows?.find((candidate) => candidate.project.id === row.project.id);
      return row.tools.filter((tool) => !own?.assignments.some((assignment) => assignment.tool === tool.tool));
    });

  useEffect(() => {
    if (skillId) void loadPlacementViews();
  }, [loadPlacementViews, skillId]);

  // undefined while the views are read; null when no read ever landed and
  // the last one failed. Last-good rows after a failed re-read stay choosable
  // under a read-error note: the assignment itself answers with the truth.
  const rows = placement.rows ?? (placement.failed ? null : undefined);
  const stale = placement.failed && placement.rows !== null;
  // The list arrives after the dialog opened: its first available project
  // takes focus then, unless the operator already moved it.
  const listReady = rows !== undefined && rows !== null;
  useEffect(() => {
    if (!listReady) return;
    const timer = window.setTimeout(() => {
      const active = document.activeElement;
      const popup = active?.closest("[role='dialog']");
      if (!popup || active?.matches("input")) return;
      popup.querySelector<HTMLElement>("[data-assign-list] input:not([disabled])")?.focus();
    }, 0);
    return () => window.clearTimeout(timer);
  }, [listReady, skillId]);
  const open = skills.length > 0;
  // A choice counts only for this request, and only while that project can take it.
  const stateOf = (row: SkillPlacement) => {
    const missingBySkill = missingFor(row);
    return assignTargetState({
      pathExists: row.project.path_exists,
      toolCount: row.tools.length,
      missingToolCount: row.tools.filter((tool) => missingBySkill.some((list) => list.includes(tool))).length,
    });
  };
  const canTake = (row: SkillPlacement) => stateOf(row) === "available";
  const chosen =
    choice && choice.skillId === skillId && rows?.some((row) => row.project.id === choice.projectId && canTake(row))
      ? choice.projectId
      : null;
  const setChosen = (projectId: string | null) =>
    setChoice(projectId && skillId ? { skillId, projectId } : null);

  return (
    <Dialog
      open={open}
      dismissible={!loading}
      onOpenChange={(next) => {
        if (!next) {
          setChosen(null);
          onRequestClose();
        }
      }}
    >
      <DialogContent
        initialFocus={() => document.querySelector<HTMLElement>("[data-assign-list] input:not([disabled])") ?? true}
      >
        <DialogHeader>
          <DialogTitle>
            {single
              ? t("assignProject.title", { name: single.name })
              : t("bulk.assign.title", { count: skills.length })}
          </DialogTitle>
          <DialogDescription>{t("assignProject.body")}</DialogDescription>
        </DialogHeader>
        <DialogBody>
          {rows === undefined ? (
            <p className="m-0 text-sm text-muted-foreground">{t("assignProject.loading")}</p>
          ) : rows === null ? (
            <p className="m-0 text-sm text-failed">{t("inspector.projects.loadError")}</p>
          ) : (
            <form
              id={formId}
              onSubmit={(event) => {
                event.preventDefault();
                if (chosen && !loading) onConfirm(chosen);
              }}
            >
              {stale ? (
                <p role="status" className="m-0 mb-2 text-sm text-failed">
                  {t("inspector.projects.stale")}
                </p>
              ) : null}
              <FieldSet>
                <FieldLegend>{t("assignProject.legend")}</FieldLegend>
                <ul data-assign-list="" className="m-0 flex max-h-80 list-none flex-col overflow-y-auto rounded-md border border-border p-0">
                  {rows.map((row) => {
                    const { project, tools } = row;
                    const state = stateOf(row);
                    const noTools = tools.length === 0;
                    // Assigning adds each skill to the Tools it is not on yet.
                    const missingBySkill = missingFor(row);
                    const missing = tools.filter((tool) => missingBySkill.some((list) => list.includes(tool)));
                    const adding = missingBySkill.filter((list) => list.length > 0).length;
                    const folderMissing = state === "folderMissing";
                    const assigned = state === "assigned";
                    const partly =
                      state === "available" &&
                      (single ? missing.length < tools.length : true);
                    const unavailable = state !== "available";
                    return (
                      <li key={project.id} className="border-b border-border last:border-b-0">
                        <label className={cn("flex items-start gap-3 px-3 py-2", !unavailable && "hover:bg-raised")}>
                          <input
                            type="radio"
                            name={`${formId}-project`}
                            value={project.id}
                            checked={chosen === project.id}
                            disabled={unavailable || loading}

                            onChange={() => setChosen(project.id)}
                            className={cn("mt-1 size-4 shrink-0 accent-primary disabled:opacity-50", focusRing)}
                          />
                          <span className="flex min-w-0 flex-col">
                            <span className="text-sm font-medium text-foreground">{project.name}</span>
                            <span className="truncate font-mono text-xs text-faint" title={project.path}>
                              {project.path}
                            </span>
                            <span className="text-xs text-muted-foreground">
                              {noTools
                                ? t("assignProject.noTools")
                                : t("assignProject.tools", { tools: tools.map((tool) => labelOf(tool.tool)).join(", ") })}
                            </span>
                            {folderMissing ? (
                              <span className="text-xs text-muted-foreground">{t("projectsSurface.assignFolderMissing")}</span>
                            ) : null}
                            {assigned ? <span className="text-xs text-muted-foreground">{t("assignProject.assigned")}</span> : null}
                            {partly ? (
                              <span className="text-xs text-muted-foreground">
                                {single
                                  ? t("assignProject.adds", { tools: missing.map((tool) => labelOf(tool.tool)).join(", ") })
                                  : t("bulk.assign.adds", {
                                      skills: t("bulk.count.skills", { count: adding }),
                                      tools: missing.map((tool) => labelOf(tool.tool)).join(", "),
                                    })}
                              </span>
                            ) : null}
                          </span>
                        </label>
                      </li>
                    );
                  })}
                </ul>
              </FieldSet>
            </form>
          )}
        </DialogBody>
        <DialogFooter>
          <Button
            onClick={() => {
              setChosen(null);
              onRequestClose();
            }}
            disabled={loading}
          >
            {t("cancel")}
          </Button>
          <Button type="submit" form={formId} variant="solid" tone="accent" disabled={!chosen} pending={loading}>
            {t("assignProject.confirm")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
