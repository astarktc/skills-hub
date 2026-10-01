import { Fragment, memo, useId, type KeyboardEvent } from "react";
import { FolderOpen, MoreHorizontal, Plus, Settings2, Trash2, TriangleAlert } from "lucide-react";
import type { TFunction } from "i18next";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { ProjectDto } from "./types";

type ProjectListProps = {
  projects: ProjectDto[];
  selectedProjectId: string | null;
  loading: boolean;
  loadFailed: boolean;
  onSelectProject: (id: string) => void;
  onAddProject: () => void;
  onEditProject: (id: string) => void;
  onRemoveProject: (id: string) => void;
  onUpdatePath: (id: string) => void;
  /** Why mutation controls are disabled: another action is running (D19); null = they may run. */
  blockedReason: string | null;
  t: TFunction;
};

/**
 * A folder path that wraps only after a separator — never mid-name and
 * never truncated (D23: the project's path is read whole).
 */
export function PathText({ path }: { path: string }) {
  const parts = path.split(/(?<=[/\\])/);
  return (
    <>
      {parts.map((part, index) => (
        <Fragment key={index}>
          {part}
          {index < parts.length - 1 ? <wbr /> : null}
        </Fragment>
      ))}
    </>
  );
}

/** The selection buttons, in list order (arrow keys walk them). */
const SELECT_SELECTOR = "[data-project-select]";

/**
 * Arrow keys / Home / End move focus between the projects' selection
 * buttons (never the selection itself — Enter or Space selects); Tab keeps
 * its native order through each project and its overflow.
 */
function moveFocus(event: KeyboardEvent<HTMLButtonElement>) {
  const keys = ["ArrowDown", "ArrowUp", "Home", "End"];
  if (!keys.includes(event.key)) return;
  const list = event.currentTarget.closest("ul");
  if (!list) return;
  const buttons = Array.from(list.querySelectorAll<HTMLButtonElement>(SELECT_SELECTOR));
  const index = buttons.indexOf(event.currentTarget);
  const next =
    event.key === "Home"
      ? buttons[0]
      : event.key === "End"
        ? buttons[buttons.length - 1]
        : buttons[index + (event.key === "ArrowDown" ? 1 : -1)];
  if (!next) return;
  event.preventDefault();
  next.focus();
}

/**
 * The project list (≥ the shell breakpoint; below it the header's select,
 * D23): one named selection button per project — `aria-current` marks the
 * selected one (a `raised` ground and a 3 px iris edge), the focus ring the
 * focused one, so the two are never confused — and a per-project overflow
 * (Project settings…, Update path…, Remove project…), separate from
 * selection.
 */
const ProjectList = ({
  projects,
  selectedProjectId,
  loading,
  loadFailed,
  onSelectProject,
  onAddProject,
  onEditProject,
  onRemoveProject,
  onUpdatePath,
  blockedReason,
  t,
}: ProjectListProps) => {
  const headingId = useId();
  return (
    <aside aria-labelledby={headingId} className="flex w-64 shrink-0 flex-col border-r border-border bg-card">
      <div className="flex h-12 shrink-0 items-center justify-between gap-2 border-b border-border px-4">
        <h1
          id={headingId}
          data-surface-heading="projects"
          tabIndex={-1}
          className="m-0 rounded-sm text-lg font-semibold text-foreground"
        >
          {t("projectsSurface.heading")}
        </h1>
        <Button variant="ghost" size="icon" aria-label={t("projects.addProject")} title={t("projects.addProject")} onClick={onAddProject}>
          <Plus />
        </Button>
      </div>

      {loadFailed ? (
        <p role="alert" className="m-0 px-4 py-3 text-sm text-failed">
          {t("projects.loadError")}
        </p>
      ) : loading ? (
        <div aria-hidden="true" className="flex flex-col gap-2 px-4 py-3">
          <div className="h-10 rounded-md bg-raised" />
          <div className="h-10 rounded-md bg-raised" />
          <div className="h-10 rounded-md bg-raised" />
        </div>
      ) : (
        <ul aria-labelledby={headingId} className="m-0 flex min-h-0 flex-1 list-none flex-col overflow-y-auto p-0">
          {projects.map((project) => (
            <ProjectItem
              key={project.id}
              project={project}
              selected={selectedProjectId === project.id}
              onSelect={onSelectProject}
              onEdit={onEditProject}
              onRemove={onRemoveProject}
              onUpdatePath={onUpdatePath}
              blockedReason={blockedReason}
              t={t}
            />
          ))}
        </ul>
      )}
    </aside>
  );
};

type ProjectItemProps = {
  project: ProjectDto;
  selected: boolean;
  onSelect: (id: string) => void;
  onEdit: (id: string) => void;
  onRemove: (id: string) => void;
  onUpdatePath: (id: string) => void;
  blockedReason: string | null;
  t: TFunction;
};

const ProjectItem = memo(({ project, selected, onSelect, onEdit, onRemove, onUpdatePath, blockedReason, t }: ProjectItemProps) => {
  const nameId = useId();
  const detailId = useId();
  const missing = !project.path_exists;
  return (
    <li className="relative border-b border-border">
      <button
        type="button"
        data-project-select=""
        aria-current={selected ? "true" : undefined}
        aria-labelledby={nameId}
        aria-describedby={detailId}
        onClick={() => onSelect(project.id)}
        onKeyDown={moveFocus}
        className={cn(
          "relative flex w-full cursor-default flex-col gap-0.5 py-2 pr-11 pl-4 text-left hover:bg-raised",
          "outline-none focus-visible:outline-2 focus-visible:outline-solid focus-visible:-outline-offset-2 focus-visible:outline-ring",
          selected &&
            "bg-raised before:pointer-events-none before:absolute before:inset-y-0 before:left-0 before:w-[3px] before:bg-ring",
        )}
      >
        <span id={nameId} className="text-sm font-medium break-words text-foreground">
          {project.name}
        </span>
        <span id={detailId} className="flex flex-col gap-0.5">
          <span className="font-mono text-xs break-words text-faint">
            <PathText path={project.path} />
          </span>
          {missing ? (
            <span className="flex items-center gap-1 text-xs text-stale">
              <TriangleAlert aria-hidden="true" className="size-3.5 shrink-0" />
              {t("projectsSurface.list.folderMissing")}
            </span>
          ) : (
            <span className="text-xs text-muted-foreground">
              {t("projectsSurface.list.meta", {
                tools: t("projectsSurface.list.tools", { count: project.tool_count }),
                skills: t("projectsSurface.list.skills", { count: project.skill_count }),
              })}
              {project.sync_status === "error" || project.sync_status === "stale" ? (
                <>
                  {" \u00b7 "}
                  <span className={project.sync_status === "error" ? "text-failed" : "text-stale"}>
                    {t(`projectsSurface.list.status.${project.sync_status}`)}
                  </span>
                </>
              ) : null}
            </span>
          )}
        </span>
      </button>
      <div className="absolute top-1.5 right-2">
        <DropdownMenu>
          <DropdownMenuTrigger
            render={
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={t("projectsSurface.header.more", { project: project.name })}
                title={t("projectsSurface.header.more", { project: project.name })}
              />
            }
          >
            <MoreHorizontal />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="min-w-48">
            <ProjectMenuItems
              project={project}
              onEdit={onEdit}
              onRemove={onRemove}
              onUpdatePath={onUpdatePath}
              blockedReason={blockedReason}
              t={t}
            />
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </li>
  );
});
ProjectItem.displayName = "ProjectItem";

/**
 * One project's own actions — the list's overflow and, below the shell
 * breakpoint, the header's: Project settings…, Update path… (a missing
 * folder), Remove project…. A path update is a mutation: refused with the
 * reason while another action runs.
 */
export function ProjectMenuItems({
  project,
  onEdit,
  onRemove,
  onUpdatePath,
  blockedReason,
  t,
}: Pick<ProjectItemProps, "project" | "onEdit" | "onRemove" | "onUpdatePath" | "blockedReason" | "t">) {
  return (
    <>
      {!project.path_exists ? (
        <DropdownMenuItem
          disabled={blockedReason !== null}
          title={blockedReason ?? undefined}
          onClick={() => onUpdatePath(project.id)}
        >
          <FolderOpen />
          {t("projectsSurface.header.updatePath")}
        </DropdownMenuItem>
      ) : null}
      <DropdownMenuItem onClick={() => onEdit(project.id)}>
        <Settings2 />
        {t("projectsSurface.header.configureProject")}
      </DropdownMenuItem>
      <DropdownMenuSeparator />
      <DropdownMenuItem variant="destructive" onClick={() => onRemove(project.id)}>
        <Trash2 />
        {t("projectsSurface.header.remove")}
      </DropdownMenuItem>
    </>
  );
}

export default memo(ProjectList);
