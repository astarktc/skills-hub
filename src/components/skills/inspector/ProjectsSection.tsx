import { useEffect, useState } from "react";
import type { TFunction } from "i18next";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import type { SkillPlacement } from "../../projects/useProjectState";
import type { SkillActions } from "../../../hooks/useSkillActions";
import type { ManagedSkill } from "../types";
import InspectorSection from "./InspectorSection";

type ProjectsSectionProps = {
  skill: ManagedSkill;
  /** The project world's read of where a skill is assigned (a new identity after any project change). */
  readSkillPlacement: (skillId: string) => Promise<SkillPlacement[]>;
  /** How many projects exist (0 = nothing to assign to). */
  projectCount: number;
  labelOf: (tool: string) => string;
  actions: Pick<SkillActions, "run">;
  blockedReason: string | null;
  t: TFunction;
};

type Placement = { skillId: string; rows: SkillPlacement[] | null; failed: boolean };

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
  readSkillPlacement,
  projectCount,
  labelOf,
  actions,
  blockedReason,
  t,
}: ProjectsSectionProps) {
  const [placement, setPlacement] = useState<Placement | null>(null);
  const busy = blockedReason !== null;

  useEffect(() => {
    let stale = false;
    void (async () => {
      try {
        const rows = await readSkillPlacement(skill.id);
        if (!stale) setPlacement({ skillId: skill.id, rows, failed: false });
      } catch {
        if (!stale) setPlacement({ skillId: skill.id, rows: null, failed: true });
      }
    })();
    return () => {
      stale = true;
    };
    // `updated_at` / targets change when a skill-grain verb touched its rows.
  }, [readSkillPlacement, skill.id, skill.updated_at, skill.disabled_at]);

  const current = placement?.skillId === skill.id ? placement : null;
  const assigned = current?.rows?.filter((row) => row.assignments.length > 0) ?? [];

  return (
    <InspectorSection title={t("inspector.sections.projects")}>
      {current === null ? (
        <p className="m-0 text-sm text-muted-foreground">{t("inspector.projects.loading")}</p>
      ) : current.failed ? (
        <p className="m-0 text-sm text-failed">{t("inspector.projects.loadError")}</p>
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
