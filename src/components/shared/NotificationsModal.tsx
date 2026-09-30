import { memo, useCallback, useEffect, useId, useMemo, useRef, useState, type ReactNode } from "react";
import {
  Bell,
  Check,
  ChevronRight,
  CircleAlert,
  CircleCheck,
  CircleDot,
  Copy,
  Info,
  TriangleAlert,
} from "lucide-react";
import type { TFunction } from "i18next";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { focusRing } from "@/components/ui/styles";
import { cn } from "@/lib/utils";
import type { SkillActions } from "../../hooks/useSkillActions";
import type { CopyToClipboardFn, Notification, NotificationKind } from "../../hooks/useStatusReporter";
import type { SurfaceState } from "../../hooks/useSurfaceState";
import {
  groupRuns,
  historyText,
  rowActions,
  rowProject,
  rowText,
  type ProjectPrefix,
  type RowAction,
  type RunGroup,
} from "../../lib/notificationRuns";
import { formatRelativeTime } from "../../lib/skillPresentation";

type NotificationsModalProps = {
  open: boolean;
  /** The run "View report" opened the panel on (expanded and focused); else the newest. */
  runId: number | null;
  /** Newest first — the reporter's own order. */
  notifications: Notification[];
  /** The catalog now: a row's skill actions exist only while its skill does. */
  skillNames: Readonly<Record<string, string>>;
  /** Labels for a project row ("in monorepo: …"). */
  projectLabelById: Readonly<Record<string, string>>;
  /** The skill action door: Retry, Overwrite…, Change source…, Remove… */
  actions: Pick<SkillActions, "run">;
  /** Surface state: Show (focus the skill) and Settings → GitHub. */
  surface: Pick<SurfaceState, "focus">;
  /** Why a mutation cannot start now (D19), or null. */
  blockedReason: string | null;
  onRequestClose: () => void;
  onClear: () => void;
  /**
   * The reporter's clipboard helper: it reports a failure like any other
   * notification; the button's own "done" state is local.
   */
  copyToClipboard: CopyToClipboardFn;
  /**
   * The header progress at the panel's foot while an action runs (a Retry
   * started here, or a Refresh the panel was opened over): the page behind
   * the modal is inert, so its Cancel must be the panel's (D19/D20).
   */
  progress?: ReactNode;
  t: TFunction;
};

const KIND_ICON: Record<NotificationKind, typeof CircleAlert> = {
  error: CircleAlert,
  warning: TriangleAlert,
  success: CircleCheck,
  info: Info,
};

/** Kind as colour: the state tokens (measured ≥ 4.5:1 on the popover in both themes). */
const KIND_TEXT: Record<NotificationKind, string> = {
  error: "text-failed",
  warning: "text-stale",
  success: "text-ok",
  info: "text-muted-foreground",
};

/** How long a copy button shows its "done" state. */
const COPIED_FEEDBACK_MS = 1500;

const ACTION_LABEL_KEY: Record<RowAction["kind"], string> = {
  show: "runReport.actions.show",
  retry: "runReport.actions.retry",
  overwrite: "runReport.actions.overwrite",
  update: "runReport.actions.retry",
  delete: "runReport.actions.delete",
  changeSource: "runReport.actions.changeSource",
  githubSettings: "runReport.actions.githubSettings",
};

/** Actions that start a mutation: disabled with the reason while one runs (D19). */
const MUTATES = new Set<RowAction["kind"]>(["retry", "overwrite", "update", "delete", "changeSource"]);

type CopyTarget = string | "all";

/**
 * The session's report, grouped by run (D11): each run is a group row — its
 * composite headline, the action's name and when — that expands to what it
 * reported, and every row carries its actions: Show (the skill, focused, in
 * the inspector — the panel closes), Retry, Overwrite… (the one ask again),
 * Change source… (gone upstream), Open Settings → GitHub (auth / rate
 * limit). Per-row Copy (named by the row), Copy all and Clear. Session-only.
 * A view of the reporter's history (`groupRuns`, `rowActions` are the pure
 * folds); every action goes through surface state or the skill door.
 */
const NotificationsModal = ({
  open,
  runId,
  notifications,
  skillNames,
  projectLabelById,
  actions,
  surface,
  blockedReason,
  onRequestClose,
  onClear,
  copyToClipboard,
  progress,
  t,
}: NotificationsModalProps) => {
  // Show / Settings → GitHub leave the panel for another place: the panel
  // returns focus there (the skill's inspector heading — beside the list, or
  // the sheet's — or the token field), never back to the bell; a place that
  // is not on screen gets nothing, and a sheet opening takes focus itself.
  const [leaveTo, setLeaveTo] = useState<{ selector: string } | null>(null);
  const [wasOpen, setWasOpen] = useState(open);
  if (open !== wasOpen) {
    setWasOpen(open);
    if (open) setLeaveTo(null);
  }

  const leave = useCallback(
    (selector: string, go: () => void) => {
      setLeaveTo({ selector });
      go();
      onRequestClose();
    },
    [onRequestClose],
  );
  const finalFocus = useMemo(
    () => (leaveTo ? () => document.querySelector<HTMLElement>(leaveTo.selector) ?? false : undefined),
    [leaveTo],
  );

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) onRequestClose();
      }}
    >
      <DialogContent
        size="lg"
        finalFocus={finalFocus}
        initialFocus={() => document.querySelector<HTMLElement>("[data-run-initial]")}
      >
        <DialogHeader>
          <DialogTitle>{t("notifications.title")}</DialogTitle>
        </DialogHeader>
        {/* Keyed by the run asked for: "View report" on another run starts fresh. */}
        <ReportBody
          key={runId ?? "all"}
          runId={runId}
          notifications={notifications}
          skillNames={skillNames}
          projectLabelById={projectLabelById}
          actions={actions}
          surface={surface}
          blockedReason={blockedReason}
          leave={leave}
          onClear={onClear}
          copyToClipboard={copyToClipboard}
          t={t}
        />
        {progress}
      </DialogContent>
    </Dialog>
  );
};

type ReportBodyProps = Omit<NotificationsModalProps, "open" | "onRequestClose" | "progress"> & {
  /** Close the panel for another place, whose element (`selector`) then takes focus. */
  leave: (selector: string, go: () => void) => void;
};

function ReportBody({
  runId,
  notifications,
  skillNames,
  projectLabelById,
  actions,
  surface,
  blockedReason,
  leave,
  onClear,
  copyToClipboard,
  t,
}: ReportBodyProps) {
  const groups = useMemo(() => groupRuns(notifications), [notifications]);
  // Expanded by default: the run asked for, else the newest — decided as the
  // panel opens, so a run recorded while it stands open (a Retry from here)
  // never collapses the one being read. The operator's toggles win.
  const [targetKey] = useState(
    () => ((runId !== null && groups.find((group) => group.runId === runId)) || groups[0])?.key ?? null,
  );
  const [toggled, setToggled] = useState<ReadonlyMap<string, boolean>>(new Map());
  const isExpanded = (group: RunGroup) => toggled.get(group.key) ?? group.key === targetKey;

  const [copied, setCopied] = useState<CopyTarget | null>(null);
  const copiedTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(
    () => () => {
      if (copiedTimer.current) clearTimeout(copiedTimer.current);
    },
    [],
  );
  const copy = useCallback(
    async (key: CopyTarget, text: string) => {
      if (!(await copyToClipboard(text))) return;
      setCopied(key);
      if (copiedTimer.current) clearTimeout(copiedTimer.current);
      copiedTimer.current = setTimeout(() => {
        setCopied(null);
        copiedTimer.current = null;
      }, COPIED_FEEDBACK_MS);
    },
    [copyToClipboard],
  );

  const exists = useCallback((skillId: string) => skillId in skillNames, [skillNames]);
  // The one project-aware presentation: heading, item and both copies.
  const inProject = useCallback<ProjectPrefix>(
    (projectId) => t("runReport.inProject", { project: projectLabelById[projectId] ?? projectId }),
    [projectLabelById, t],
  );
  const { run } = actions;
  const { focus } = surface;
  const act = (action: RowAction) => {
    switch (action.kind) {
      case "show":
        leave(`[data-inspector-heading="${CSS.escape(action.skillId)}"]`, () =>
          focus({ kind: "skill", id: action.skillId }),
        );
        return;
      case "githubSettings":
        leave("#settings-github-token", () => focus({ kind: "settings", pane: "github" }));
        return;
      case "retry":
        void run("retry", [action.skillId], { tool: action.tool });
        return;
      case "overwrite":
        // Answer the recorded collision by the route that produced it (a sync
        // re-syncs; an Enable enables the pair): the folder still in the way
        // raises the one ask (D12). The door checks the pair's state now and
        // says so when the report went stale.
        void run(action.via === "enable" ? "enable" : "retry", [action.skillId], {
          tool: action.tool,
          answering: "collision",
        });
        return;
      case "update":
        void run("update", [action.skillId]);
        return;
      case "delete":
        void run("delete", [action.skillId]);
        return;
      case "changeSource":
        void run("repoint", [action.skillId], { preselect: "git" });
        return;
    }
  };

  const isEmpty = groups.length === 0;
  return (
    <>
      <DialogBody variant="flush" className="max-h-[min(70vh,560px)]">
        {isEmpty ? (
          <div className="flex flex-col items-center gap-1.5 px-8 pt-11 pb-10 text-center">
            <Bell aria-hidden="true" className="mb-1.5 size-7 text-faint" />
            {/* An empty panel opens on what it says (Clear and Copy all stay
                focusable while disabled, so they are not the landing). */}
            <p data-run-initial="" tabIndex={-1} className={cn("m-0 rounded-sm text-sm font-semibold text-foreground", focusRing)}>
              {t("notifications.empty")}
            </p>
            <p className="m-0 max-w-[36ch] text-xs text-muted-foreground">{t("notifications.emptyHint")}</p>
          </div>
        ) : (
          <ul aria-label={t("runReport.runs")} className="m-0 list-none p-0">
            {groups.map((group) => (
              <RunRowView
                key={group.key}
                group={group}
                initial={group.key === targetKey}
                expanded={isExpanded(group)}
                onToggle={() =>
                  setToggled((current) => new Map(current).set(group.key, !isExpanded(group)))
                }
                exists={exists}
                skillNames={skillNames}
                inProject={inProject}
                blockedReason={blockedReason}
                copied={copied}
                onCopy={copy}
                onAct={act}
                t={t}
              />
            ))}
          </ul>
        )}
      </DialogBody>
      <DialogFooter className="justify-between">
        {/* Clear empties what it acts on: it stays focusable while disabled,
            so focus stays on it (D21) instead of dropping to <body>. */}
        <Button onClick={onClear} disabled={isEmpty} focusableWhenDisabled>
          {t("notifications.clear")}
        </Button>
        <Button onClick={() => void copy("all", historyText(groups, inProject))} disabled={isEmpty} focusableWhenDisabled>
          {copied === "all" ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
          {copied === "all" ? t("copied") : t("notifications.copyAll")}
        </Button>
      </DialogFooter>
    </>
  );
}

type RowSharedProps = {
  exists: (skillId: string) => boolean;
  skillNames: Readonly<Record<string, string>>;
  inProject: ProjectPrefix;
  blockedReason: string | null;
  copied: CopyTarget | null;
  onCopy: (key: CopyTarget, text: string) => void;
  onAct: (action: RowAction) => void;
  t: TFunction;
};

/** One run: its group row (a disclosure when it has items), then its items. */
function RunRowView({
  group,
  initial,
  expanded,
  onToggle,
  ...shared
}: RowSharedProps & { group: RunGroup; initial: boolean; expanded: boolean; onToggle: () => void }) {
  const { t, inProject } = shared;
  const itemsId = useId();
  const { head, items } = group;
  const Icon = KIND_ICON[group.kind];
  const at = new Date(group.at);
  // A project item promoted to the heading keeps its project (N1).
  const project = rowProject(head, inProject);
  const title = (
    <span className="flex min-w-0 flex-col">
      <span className="text-sm font-semibold break-words text-foreground">
        {project ? <span className="font-normal text-muted-foreground">{project} </span> : null}
        {head.title}
      </span>
      <span className="text-xs text-muted-foreground">
        {group.label ? `${group.label} · ` : null}
        <time dateTime={at.toISOString()} title={at.toLocaleString()}>
          {formatRelativeTime(group.at, t)}
        </time>
        {items.length > 0 ? ` · ${t("runReport.items", { count: items.length })}` : null}
      </span>
    </span>
  );
  return (
    <li data-run={group.runId ?? undefined} className="border-b border-border last:border-b-0">
      <div className="grid grid-cols-[16px_minmax(0,1fr)_auto] items-start gap-x-3 px-5 py-3">
        <span role="img" aria-label={t(`notifications.kind.${group.kind}`)} className={cn("mt-0.5 inline-flex", KIND_TEXT[group.kind])}>
          <Icon aria-hidden="true" className="size-4" />
        </span>
        <div className="flex min-w-0 flex-col gap-1.5">
          {items.length > 0 ? (
            <button
              type="button"
              data-run-initial={initial || undefined}
              aria-expanded={expanded}
              aria-controls={itemsId}
              onClick={onToggle}
              className={cn("-mx-1 flex min-w-0 items-start gap-1 rounded-sm px-1 text-left hover:bg-accent", focusRing)}
            >
              <ChevronRight
                aria-hidden="true"
                className={cn("mt-0.5 size-4 shrink-0 text-muted-foreground", expanded && "rotate-90")}
              />
              {title}
            </button>
          ) : (
            <div data-run-initial={initial || undefined} tabIndex={initial ? -1 : undefined} className={cn("rounded-sm", focusRing)}>
              {title}
            </div>
          )}
          {head.message ? (
            <p className="m-0 text-xs break-words whitespace-pre-wrap text-muted-foreground">{head.message}</p>
          ) : null}
          {/* A run headed by its first item (no headline) keeps that item's actions. */}
          {head.role !== "headline" ? <RowActionsView row={head} {...shared} /> : null}
        </div>
        <CopyButton rowKey={`${group.key}:head`} row={head} {...shared} />
      </div>
      {items.length > 0 && expanded ? (
        <ul id={itemsId} aria-label={head.title} className="m-0 list-none border-t border-border bg-card p-0">
          {items.map((item) => (
            <ItemRow key={item.id} item={item} {...shared} />
          ))}
        </ul>
      ) : null}
    </li>
  );
}

/** One item of a run: kind, words (a project row names its project), actions, Copy. */
function ItemRow({ item, ...shared }: RowSharedProps & { item: Notification }) {
  const { t, inProject } = shared;
  const Icon = item.review ? CircleDot : KIND_ICON[item.kind];
  const kindLabel = item.review ? t("runReport.kindReview") : t(`notifications.kind.${item.kind}`);
  const project = rowProject(item, inProject);
  return (
    <li className="grid grid-cols-[16px_minmax(0,1fr)_auto] items-start gap-x-3 py-2.5 pr-5 pl-10">
      <span role="img" aria-label={kindLabel} className={cn("mt-0.5 inline-flex", item.review ? "text-stale" : KIND_TEXT[item.kind])}>
        <Icon aria-hidden="true" className="size-4" />
      </span>
      <div className="flex min-w-0 flex-col gap-1">
        <p className="m-0 text-sm break-words text-foreground">
          {project ? <span className="text-muted-foreground">{project} </span> : null}
          {item.title}
        </p>
        {item.message ? (
          <p className="m-0 text-xs break-words whitespace-pre-wrap text-muted-foreground">{item.message}</p>
        ) : null}
        <RowActionsView row={item} {...shared} />
      </div>
      <CopyButton rowKey={`item:${item.id}`} row={item} {...shared} />
    </li>
  );
}

function RowActionsView({ row, exists, skillNames, blockedReason, onAct, t }: RowSharedProps & { row: Notification }) {
  const list = rowActions(row, exists);
  if (list.length === 0) return null;
  const skillName = row.subject?.skillId ? (skillNames[row.subject.skillId] ?? row.subject.skillName) : null;
  return (
    <div className="flex flex-wrap items-center gap-1.5">
      {list.map((action) => {
        const label = t(ACTION_LABEL_KEY[action.kind]);
        const blocked = MUTATES.has(action.kind) && blockedReason !== null;
        return (
          <Button
            key={action.kind}
            size="sm"
            variant={action.kind === "show" ? "outline" : "ghost"}
            tone={action.kind === "show" ? "neutral" : "accent"}
            aria-label={skillName && action.kind !== "githubSettings" ? t("runReport.actionNamed", { action: label, name: skillName }) : undefined}
            disabled={blocked}
            focusableWhenDisabled
            title={blocked ? (blockedReason ?? undefined) : undefined}
            aria-description={blocked ? (blockedReason ?? undefined) : undefined}
            onClick={() => onAct(action)}
          >
            {label}
          </Button>
        );
      })}
    </div>
  );
}

/** Copy one row: named by the row it copies ("Copy: alpha failed"). */
function CopyButton({ rowKey, row, copied, onCopy, inProject, t }: RowSharedProps & { rowKey: string; row: Notification }) {
  const name = t("notificationCopy.named", { title: row.title });
  return (
    <Button variant="ghost" size="icon-sm" aria-label={name} title={name} onClick={() => onCopy(rowKey, rowText(row, inProject))}>
      {copied === rowKey ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
    </Button>
  );
}

export default memo(NotificationsModal);
