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
import { cn } from "@/lib/utils";
import type { SkillPlacement, SkillPlacementValue } from "../../projects/useProjectState";
import type { ManagedSkill } from "../types";

type AssignProjectModalProps = {
  /** The skill to assign; null = closed (one dialog identity for its life). */
  skill: ManagedSkill | null;
  loading: boolean;
  /** Where the skill is assigned: the project world's held views. */
  placement: SkillPlacementValue;
  /** The explicit read of every project's view (stable), made as the picker opens. */
  loadPlacementViews: () => Promise<void>;
  labelOf: (tool: string) => string;
  /** Assign to the chosen project's configured Tools (the door's `assignToProject`). */
  onConfirm: (projectId: string) => void;
  onRequestClose: () => void;
  t: TFunction;
};

/**
 * Assign to project…: one project, and the skill lands on every Tool that
 * project is configured with — each project shows its Tools, so the choice
 * is made knowing where it goes. A project with no Tools cannot take it; a
 * project that already has the skill says so. Per-Tool choices stay in the
 * project's matrix (D26).
 */
export default function AssignProjectModal({
  skill,
  loading,
  placement,
  loadPlacementViews,
  labelOf,
  onConfirm,
  onRequestClose,
  t,
}: AssignProjectModalProps) {
  const [choice, setChoice] = useState<{ skillId: string; projectId: string } | null>(null);
  const formId = useId();
  const skillId = skill?.id ?? null;

  useEffect(() => {
    if (skillId) void loadPlacementViews();
  }, [loadPlacementViews, skillId]);

  // undefined while the views are read; null when the read failed.
  const rows = placement.failed ? null : (placement.rows ?? undefined);
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
  const open = skill !== null;
  // A choice counts only for this skill, and only while that project can take it.
  const canTake = (row: SkillPlacement) =>
    row.tools.some((tool) => !row.assignments.some((assignment) => assignment.tool === tool.tool));
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
          <DialogTitle>{t("assignProject.title", { name: skill?.name ?? "" })}</DialogTitle>
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
              <FieldSet>
                <FieldLegend>{t("assignProject.legend")}</FieldLegend>
                <ul data-assign-list="" className="m-0 flex max-h-80 list-none flex-col overflow-y-auto rounded-md border border-border p-0">
                  {rows.map(({ project, tools, assignments }) => {
                    const noTools = tools.length === 0;
                    // Assigning adds the skill to the Tools it is not on yet.
                    const missing = tools.filter((tool) => !assignments.some((row) => row.tool === tool.tool));
                    const assigned = !noTools && missing.length === 0;
                    const partly = assignments.length > 0 && missing.length > 0;
                    const unavailable = noTools || assigned;
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
                            {assigned ? <span className="text-xs text-muted-foreground">{t("assignProject.assigned")}</span> : null}
                            {partly ? (
                              <span className="text-xs text-muted-foreground">
                                {t("assignProject.adds", { tools: missing.map((tool) => labelOf(tool.tool)).join(", ") })}
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
