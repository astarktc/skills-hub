import { memo, useId } from "react";
import { MoreHorizontal, Plus, RefreshCw, Settings, TriangleAlert } from "lucide-react";
import type { TFunction } from "i18next";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Field, FieldLabel, Select } from "@/components/ui/field";
import { formatRelativeTime } from "../../lib/skillPresentation";
import { PathText, ProjectMenuItems } from "./ProjectList";
import type { ProjectDto } from "./types";

type ProjectHeaderProps = {
  /** Above the shell breakpoint the list selects; below it this header's select does (D23). */
  wide: boolean;
  projects: ProjectDto[];
  project: ProjectDto | null;
  /** Newest `synced_at` over the project's assignments; null = never synced. */
  lastSyncAt: number | null;
  /** The project has configured Tools (Sync is meaningful). */
  hasTools: boolean;
  blockedReason: string | null;
  onSelectProject: (id: string) => void;
  onAddProject: () => void;
  onResyncProject: () => void;
  onResyncAll: () => void;
  onConfigureTools: () => void;
  onEditProject: (id: string) => void;
  onRemoveProject: (id: string) => void;
  onUpdatePath: (id: string) => void;
  t: TFunction;
};

/**
 * The selected project's header. Its identity — name (`h2`), path, "Last
 * synced" — never truncates: name and path wrap (the path breaks anywhere,
 * it is one token), "Last synced" stays on one line, and the actions take
 * only the room left beside them. Below the shell breakpoint (D23) the
 * project list becomes the select in front of it and the toolbar collapses
 * to Sync Project plus an overflow (Configure Tools…, Sync All, then the
 * project's own actions); above it the toolbar shows Configure Tools · Sync
 * All · Sync Project inline and the overflow holds the project's actions.
 * Every mutation control stays focusable while refused, with the reason.
 */
const ProjectHeader = ({
  wide,
  projects,
  project,
  lastSyncAt,
  hasTools,
  blockedReason,
  onSelectProject,
  onAddProject,
  onResyncProject,
  onResyncAll,
  onConfigureTools,
  onEditProject,
  onRemoveProject,
  onUpdatePath,
  t,
}: ProjectHeaderProps) => {
  const headingId = useId();
  const missing = project ? !project.path_exists : false;
  const blocked = blockedReason !== null;
  const syncReason = missing ? t("projects.syncDisabledMissing") : blockedReason;
  const refused = {
    focusableWhenDisabled: true,
    title: blockedReason ?? undefined,
    "aria-description": blockedReason ?? undefined,
  };
  const syncProject = (
    <Button
      variant="solid"
      tone="accent"
      onClick={onResyncProject}
      disabled={missing || blocked || !hasTools}
      focusableWhenDisabled
      title={syncReason ?? undefined}
      aria-description={syncReason ?? undefined}
    >
      <RefreshCw />
      {t("projects.syncProject")}
    </Button>
  );
  // Beside the list with nothing selected there is nothing to head.
  if (wide && !project) return null;
  return (
    <header className="flex shrink-0 flex-col gap-2 border-b border-border px-4 py-2">
      {!wide ? (
        <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
          <h1
            id={headingId}
            data-surface-heading="projects"
            tabIndex={-1}
            className="m-0 rounded-sm text-lg font-semibold text-foreground"
          >
            {t("projectsSurface.heading")}
          </h1>
          <Field className="flex-row items-center gap-2">
            <FieldLabel className="sr-only">{t("projectsSurface.header.select")}</FieldLabel>
            <Select
              data-project-header-select=""
              className="w-56"
              value={project?.id ?? ""}
              onChange={(event) => onSelectProject(event.target.value)}
            >
              {project ? null : <option value="">{t("projects.selectProject")}</option>}
              {projects.map((option) => (
                <option key={option.id} value={option.id}>
                  {option.path_exists ? option.name : `${option.name} ${t("projectsSurface.header.missingSuffix")}`}
                </option>
              ))}
            </Select>
          </Field>
          <Button variant="ghost" size="icon" aria-label={t("projects.addProject")} title={t("projects.addProject")} onClick={onAddProject}>
            <Plus />
          </Button>
        </div>
      ) : null}

      {project ? (
        <div className="flex flex-wrap items-start justify-between gap-x-4 gap-y-2">
          <div className="flex min-w-0 flex-1 basis-64 flex-col gap-0.5">
            <h2
              {...(wide ? { id: headingId } : {})}
              data-project-heading=""
              tabIndex={-1}
              className="m-0 rounded-sm text-base font-semibold break-words text-foreground"
            >
              {project.name}
            </h2>
            <p className="m-0 flex flex-wrap items-baseline gap-x-3 gap-y-0.5 text-xs">
              <span className="min-w-0 font-mono break-words text-muted-foreground">
                <PathText path={project.path} />
              </span>
              <span className="whitespace-nowrap text-faint">
                {lastSyncAt
                  ? t("projects.lastSyncTime", { time: formatRelativeTime(lastSyncAt, t) })
                  : t("projects.lastSyncNever")}
              </span>
            </p>
            {missing ? (
              <p className="m-0 flex items-center gap-1 text-xs text-stale">
                <TriangleAlert aria-hidden="true" className="size-3.5 shrink-0" />
                {t("projectsSurface.header.folderMissing")}
              </p>
            ) : null}
          </div>

          <div className="flex shrink-0 items-center gap-2">
            {wide ? (
              <>
                <Button variant="ghost" onClick={onConfigureTools}>
                  <Settings />
                  {t("projectsSurface.header.configureTools")}
                </Button>
                <Button onClick={onResyncAll} disabled={blocked} {...refused}>
                  <RefreshCw />
                  {t("projects.syncAll")}
                </Button>
              </>
            ) : null}
            {syncProject}
            <DropdownMenu>
              <DropdownMenuTrigger
                render={
                  <Button
                    variant="ghost"
                    size="icon"
                    aria-label={t("projectsSurface.header.more", { project: project.name })}
                    title={t("projectsSurface.header.more", { project: project.name })}
                  />
                }
              >
                <MoreHorizontal />
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="min-w-52">
                {!wide ? (
                  <>
                    <DropdownMenuItem onClick={onConfigureTools}>
                      <Settings />
                      {t("projectsSurface.header.configureTools")}
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      disabled={blocked}
                      title={blockedReason ?? undefined}
                      onClick={onResyncAll}
                    >
                      <RefreshCw />
                      {t("projects.syncAll")}
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                  </>
                ) : null}
                <ProjectMenuItems
                  project={project}
                  onEdit={onEditProject}
                  onRemove={onRemoveProject}
                  onUpdatePath={onUpdatePath}
                  blockedReason={blockedReason}
                  t={t}
                />
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
      ) : (
        <p className="m-0 text-sm text-muted-foreground">{t("projects.selectProject")}</p>
      )}
    </header>
  );
};

export default memo(ProjectHeader);
