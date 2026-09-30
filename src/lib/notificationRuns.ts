/**
 * The Notifications panel's pure folds (D11): the session history grouped by
 * run, and the actions each row offers. The reporter's `recordRun` is the one
 * writer of a run (every row of one fold call shares its `runId`); this module
 * only reads the history. No DOM, no i18n: the panel renders the words.
 */

import type { Notification, NotificationKind } from "../hooks/useNotificationHistory";
import type { SkillTargetDto } from "../bindings";
import type { CollisionRoute } from "./reportOutcome";

/** One run as the panel shows it: its group row, and the items it expands to. */
export type RunGroup = {
  /** Stable React key: `run:<runId>`, or `note:<id>` for a standalone notification. */
  key: string;
  runId: number | null;
  /** The action the run was ("Refresh"), when the reporter knew it. */
  label: string | null;
  /** When the run was recorded (ms since epoch). */
  at: number;
  /** The group row's kind: the headline's, else the strongest of its items. */
  kind: NotificationKind;
  /**
   * The group row: the run's composite headline — or, for a run that settled
   * without one (a single failed toggle), its first item, which then is not
   * repeated among the items.
   */
  head: Notification;
  /** What the run reported, in report order (empty for a standalone notification). */
  items: Notification[];
  /** Items of the "to review" kind (an unanswered collision, D10). */
  reviewCount: number;
};

const KIND_RANK: Record<NotificationKind, number> = { error: 3, warning: 2, info: 1, success: 0 };

const strongestKind = (rows: readonly Notification[]): NotificationKind =>
  rows.reduce<NotificationKind>(
    (best, row) => (KIND_RANK[row.kind] > KIND_RANK[best] ? row.kind : best),
    "success",
  );

/**
 * The history (newest first) as runs, newest first. Rows of one run are
 * contiguous in the ring (`recordRun` writes a block), so a run is the rows
 * sharing its `runId`; a row without one is a standalone notification of its
 * own. A run whose oldest rows fell off the bounded ring keeps what is left.
 */
export function groupRuns(history: readonly Notification[]): RunGroup[] {
  const groups: RunGroup[] = [];
  const byRun = new Map<number, Notification[]>();
  const order: Array<number | Notification> = [];
  for (const row of history) {
    if (row.runId === undefined) {
      order.push(row);
      continue;
    }
    const rows = byRun.get(row.runId);
    if (rows) rows.push(row);
    else {
      byRun.set(row.runId, [row]);
      order.push(row.runId);
    }
  }
  for (const entry of order) {
    if (typeof entry !== "number") {
      groups.push({
        key: `note:${entry.id}`,
        runId: null,
        label: null,
        at: entry.at,
        kind: entry.kind,
        head: entry,
        items: [],
        reviewCount: entry.review ? 1 : 0,
      });
      continue;
    }
    const rows = byRun.get(entry) ?? [];
    const headline = rows.find((row) => row.role === "headline") ?? null;
    const entries = rows.filter((row) => row.role !== "headline");
    const head = headline ?? entries[0];
    const items = headline ? entries : entries.slice(1);
    groups.push({
      key: `run:${entry}`,
      runId: entry,
      label: head.runLabel ?? null,
      at: head.at,
      kind: headline ? headline.kind : strongestKind(entries),
      head,
      items,
      reviewCount: entries.filter((row) => row.review).length,
    });
  }
  return groups;
}

/** What a row offers (D11); the panel maps each onto surface state or the door. */
export type RowAction =
  /** Focus that skill (the inspector opens on it) and close the panel. */
  | { kind: "show"; skillId: string }
  /** Re-sync the pair from the central copy (`run("retry", …, { tool })`). */
  | { kind: "retry"; skillId: string; tool: string }
  /**
   * Answer the collision by the route that met it (`via`: a re-sync, or the
   * pair's Enable): the folder still in the way raises the one overwrite ask (D12).
   */
  | { kind: "overwrite"; skillId: string; tool: string; via: CollisionRoute }
  /** Run the skill's Update again. */
  | { kind: "update"; skillId: string }
  /** Open the Delete dialog again to finish removing it (D16). */
  | { kind: "delete"; skillId: string }
  /** The skill is gone upstream (404): Change source… */
  | { kind: "changeSource"; skillId: string }
  /** Auth or rate limit: the GitHub token lives in Settings. */
  | { kind: "githubSettings" };

/**
 * Why answering a recorded collision no longer applies: the report is
 * history, the pair is now. `skillDisabled` / `pairDisabled` — the operator
 * disabled it since (never overridden by an Overwrite); `resolved` — it is
 * deployed there already; `changed` — an Enable's pair is no longer the
 * disabled marker the collision left, so there is nothing of it to answer.
 */
export type StaleCollision = "skillDisabled" | "pairDisabled" | "resolved" | "changed";

/**
 * Whether Overwrite… on a recorded collision still meets it (null), checked
 * against the skill as the catalog has it now. A sync's collision is answered
 * by re-syncing the pair, which a Disable made since must stop; an Enable's
 * left the pair `disabled` (a marker owning nothing) until answered, so it is
 * answered only while the pair still is that marker and the skill is on.
 */
export function staleCollision(
  skill: { disabled_at: number | null; targets: readonly Pick<SkillTargetDto, "tool" | "status">[] },
  tool: string,
  via: CollisionRoute,
): StaleCollision | null {
  if (skill.disabled_at !== null) return "skillDisabled";
  const status = skill.targets.find((target) => target.tool === tool)?.status;
  if (status === "synced") return "resolved";
  if (via === "enable") return status === "disabled" ? null : "changed";
  return status === "disabled" ? "pairDisabled" : null;
}

/** A failure the GitHub token can fix: authentication, or the API's rate limit. */
function isGithubAccessError(row: Notification): boolean {
  const error = row.error;
  return error?.code === "RATE_LIMITED" || (error?.code === "GIT_CLONE_FAILED" && error.kind === "auth");
}

/**
 * The actions of one row, in the order the panel shows them: Show first,
 * then the row's repair, then Settings → GitHub. A skill that left the
 * catalog offers no skill action (`exists` is the catalog now); a project
 * row offers Show only — its assignment is answered from its matrix.
 */
export function rowActions(row: Notification, exists: (skillId: string) => boolean): RowAction[] {
  const actions: RowAction[] = [];
  const skillId = row.subject?.skillId;
  const present = skillId !== undefined && exists(skillId);
  if (present) actions.push({ kind: "show", skillId });
  if (present && row.error?.code === "GITHUB_SKILL_NOT_FOUND") actions.push({ kind: "changeSource", skillId });
  else if (present && row.repair && row.subject?.projectId === undefined) {
    const { repair } = row;
    switch (repair.verb) {
      case "retry":
        actions.push({ kind: "retry", skillId, tool: repair.tool });
        break;
      case "overwrite":
        actions.push({ kind: "overwrite", skillId, tool: repair.tool, via: repair.via });
        break;
      case "update":
      case "delete":
        actions.push({ kind: repair.verb, skillId });
        break;
    }
  }
  if (isGithubAccessError(row)) actions.push({ kind: "githubSettings" });
  return actions;
}

/**
 * The words that place a project row in its project ("in monorepo:"), for a
 * project id; the panel supplies them (it owns the i18n and the labels).
 */
export type ProjectPrefix = (projectId: string) => string;

/**
 * A row's project prefix, or null for a row about no project. The one
 * project-aware presentation: the panel's run heading and item, and both
 * copies, read it — a project item promoted to its run's heading (a single
 * failed toggle has no headline) keeps its project.
 */
export function rowProject(row: Notification, inProject: ProjectPrefix): string | null {
  const projectId = row.subject?.projectId;
  // A title that names its project already (resync of every project) is not prefixed twice.
  return projectId === undefined || row.subject?.projectInTitle ? null : inProject(projectId);
}

/** A row's title as read out of the panel: its project prefix, then the title. */
export function rowTitle(row: Notification, inProject: ProjectPrefix): string {
  const project = rowProject(row, inProject);
  return project ? `${project} ${row.title}` : row.title;
}

/** What "Copy" copies for one row: its (project-placed) title, then its message beneath it. */
export function rowText(row: Notification, inProject: ProjectPrefix): string {
  const title = rowTitle(row, inProject);
  return row.message ? `${title}\n${row.message}` : title;
}

/**
 * What "Copy all" copies: every run, newest first — the group row with an
 * absolute timestamp and its action's name, its items indented beneath —
 * so it can be pasted into a bug report.
 */
export function historyText(groups: readonly RunGroup[], inProject: ProjectPrefix): string {
  return groups
    .map((group) => {
      const label = group.label ? ` ${group.label}:` : "";
      const head = `${new Date(group.at).toISOString()} [${group.kind}]${label} ${rowTitle(group.head, inProject)}`;
      const lines = [head, ...(group.head.message ? [`    ${group.head.message}`] : [])];
      for (const item of group.items) {
        lines.push(`  - [${item.review ? "review" : item.kind}] ${rowTitle(item, inProject)}`);
        if (item.message) lines.push(`      ${item.message}`);
      }
      return lines.join("\n");
    })
    .join("\n");
}
