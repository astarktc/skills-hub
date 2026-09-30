import { useEffect } from "react";
import type { TFunction } from "i18next";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import type { SkillPlacementValue } from "../../projects/useProjectState";
import type { SkillActions } from "../../../hooks/useSkillActions";
import type { ManagedSkill } from "../types";
import InspectorSection from "./InspectorSection";

type ProjectsSectionProps = {
  skill: ManagedSkill;
  /** Where the skill is assigned: the project world's held views (kept current by every assignment it settles). */
  placement: SkillPlacementValue;
  /** The explicit read of every project's view (stable). */
  loadPlacementViews: () => Promise<void>;
  /** How many projects exist (0 = nothing to assign to). */
  projectCount: number;
  labelOf: (tool: string) => string;
  actions: Pick<SkillActions, "run">;
  blockedReason: string | null;
  t: TFunction;
};

/** Assignment status words: a problem is coloured, the rest stay quiet. */
const TROUBLE = new Set(["error", "missing"]);

/**
 * Where the skill is assigned across projects (the matrix stays the
 * per-project view, D26): each project with its assigned Tools and their
 * status, a per-project Remove, and **Assign to project…** — the door's
 * `assignToProject`, whose picker honours the project's configured Tools.
 */
export default function ProjectsSection({
  skill,
  placement,
  loadPlacementViews,
  projectCount,
  labelOf,
  actions,
  blockedReason,
  t,
}: ProjectsSectionProps) {
  const busy = blockedReason !== null;

  // The explicit read: on opening, for another skill, when a skills-world
  // verb touched its rows (`updated_at` / `disabled_at` — those answer no
  // project view), or when projects come or go. An assignment made here
  // settles through the views it returned: no read follows it.
  useEffect(() => {
    void loadPlacementViews();
  }, [loadPlacementViews, skill.id, skill.updated_at, skill.disabled_at, projectCount]);

  const assigned = placement.rows?.filter((row) => row.assignments.length > 0) ?? [];

  return (
    <InspectorSection title={t("inspector.sections.projects")}>
      {/* A failed re-read keeps the last-good rows under a note; only a read
          that never landed shows the error alone. */}
      {placement.failed && placement.rows !== null ? (
        <p role="status" className="m-0 text-sm text-failed">{t("inspector.projects.stale")}</p>
      ) : null}
      {placement.failed && placement.rows === null ? (
        <p className="m-0 text-sm text-failed">{t("inspector.projects.loadError")}</p>
      ) : placement.rows === null ? (
        <p className="m-0 text-sm text-muted-foreground">{t("inspector.projects.loading")}</p>
      ) : projectCount === 0 ? (
        <p className="m-0 text-sm text-muted-foreground">{t("inspector.projects.noProjects")}</p>
      ) : assigned.length === 0 ? (
        <p className="m-0 text-sm text-muted-foreground">{t("inspector.projects.empty")}</p>
      ) : (
        <ul className="m-0 flex list-none flex-col p-0">
          {assigned.map(({ project, assignments }) => (
            <li key={project.id} className="flex items-start gap-2 border-b border-border py-1.5 last:border-b-0">
              <div className="flex min-w-0 flex-1 flex-col">
                <span className="truncate text-sm text-foreground" title={project.path}>
                  {project.name}
                </span>
                <span className="flex flex-wrap gap-x-3 text-xs">
                  {assignments.map((row) => (
                    <span key={row.id} className={cn(TROUBLE.has(row.status) ? "text-failed" : "text-muted-foreground")}>
                      {t("inspector.projects.toolStatus", {
                        tool: labelOf(row.tool),
                        status: t(`inspector.projects.status.${row.status}`),
                      })}
                    </span>
                  ))}
                </span>
              </div>
              <Button
                size="sm"
                variant="ghost"
                aria-label={t("inspector.projects.unassignLabel", { name: skill.name, project: project.name })}
                disabled={busy}
                title={blockedReason ?? undefined}
                onClick={() => void actions.run("unassignFromProject", [skill], { projectId: project.id })}
              >
                {t("inspector.projects.unassign")}
              </Button>
            </li>
          ))}
        </ul>
      )}
      <div className="flex flex-wrap gap-2">
        <Button
          size="sm"
          disabled={busy || projectCount === 0}
          title={blockedReason ?? undefined}
          aria-haspopup="dialog"
          onClick={() => void actions.run("assignToProject", [skill])}
        >
          {t("inspector.projects.assign")}
        </Button>
      </div>
    </InspectorSection>
  );
}
