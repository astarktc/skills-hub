import { useId, useState } from "react";
import { ChevronRight, TriangleAlert } from "lucide-react";
import type { TFunction } from "i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldDescription, FieldLabel } from "@/components/ui/field";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";
import type { ReaderHarnessDto, ToolInfoDto, ToolStatusDto } from "../skills/types";
import {
  duplicatePairs,
  duplicateWarning,
  harnessListGroups,
  pairsInvolving,
  type UndetectedReader,
} from "../../lib/harnessPresentation";
import { PaneHeader, SettingGroup } from "./PaneDialog";

type HarnessesPaneProps = {
  /** The Tool catalog's global list (`useToolCatalog`): every row comes from it, never a hand-written list. */
  toolStatus: ToolStatusDto | null;
  labelOf: (toolId: string) => string;
  /** The Effective global target set: the saved selection, else every detected harness. */
  selected: readonly string[];
  scanSelectedOnly: boolean;
  autoDeployEnabled: boolean;
  /** Save one harness on or off (D48's ask follows turning one on, when auto-deploy is on). */
  onSelectedChange: (toolId: string, selected: boolean) => void;
  /** Save "scan only the selected harnesses" for Import. */
  onScanSelectedOnlyChange: (scanSelectedOnly: boolean) => void;
  /** Show the General pane (where Auto-deploy is switched). */
  onOpenGeneral: () => void;
  /** Why every mutation control here is held while an action runs (D19) — a save included — or null. */
  blockedReason: string | null;
  t: TFunction;
};

/**
 * Settings → Harnesses (D47; replaces Configure harnesses): one expandable
 * row per detected harness of the Tool catalog, in catalog order — name,
 * detected, and the selection switch (the saved global selection, i.e. where
 * your skills are deployed; saves as it changes). Expanded: the global skills
 * folder in use (and the harnesses sharing it), how it is deployed to (the
 * registry's capability) and the folders it is detected by. A folder read by
 * reader-only harnesses (`.agents/skills`) lists its detected ones under its
 * row — "Read by", with no switch: they have no folder of their own, so
 * deploying to the folder is deploying to them.
 *
 * Every undetected harness — a selected one included (its row explains why it
 * is listed and offers "Remove from selection"), reader-only ones too — sits
 * in one "Not detected (N)" group, closed whenever the pane mounts (Settings
 * opening) and never persisted (round 20 ticket 02).
 *
 * A row whose harness takes part in a cross-read pair with both harnesses
 * selected says so on one line (`harnessPresentation.duplicateWarning`, from
 * the backend's `duplicate_readers`); nothing is deselected. The Import scan
 * scope is its own section: it decides what Import reviews, never where
 * skills go. Auto-deploy's state shows here, where it acts (D49).
 */
export function HarnessesPane({
  toolStatus,
  labelOf,
  selected,
  scanSelectedOnly,
  autoDeployEnabled,
  onSelectedChange,
  onScanSelectedOnlyChange,
  onOpenGeneral,
  blockedReason,
  t,
}: HarnessesPaneProps) {
  const [undetectedOpen, setUndetectedOpen] = useState(false);
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(() => new Set());
  const listId = useId();
  const all = toolStatus?.tools ?? [];
  const installed = toolStatus?.installed ?? [];
  const selectedSet = new Set(selected);
  const groups = harnessListGroups(all);
  const pairs = duplicatePairs(all, selectedSet);
  const selectedCount = all.filter((tool) => selectedSet.has(tool.key)).length;
  const undetectedSelected = groups.undetected.tools.filter((tool) => selectedSet.has(tool.key)).length;

  const toggleExpanded = (key: string) =>
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });

  const row = (tool: ToolInfoDto, readers: readonly ReaderHarnessDto[]) => (
    <HarnessRow
      key={tool.key}
      tool={tool}
      label={labelOf(tool.key)}
      labelOf={labelOf}
      readers={readers}
      selected={selectedSet.has(tool.key)}
      expanded={expanded.has(tool.key)}
      warning={duplicateWarning(pairsInvolving(pairs, tool.key), labelOf, t)}
      onToggleExpanded={() => toggleExpanded(tool.key)}
      onSelectedChange={(on) => onSelectedChange(tool.key, on)}
      blockedReason={blockedReason}
      t={t}
    />
  );

  return (
    <>
      <PaneHeader
        title={t("settingsDialog.panes.harnesses")}
        description={t("settingsDialog.harnesses.description")}
        note={t("settingsDialog.autoSaves")}
      />
      <div className="flex flex-col gap-1.5">
        <p className="m-0 text-sm text-foreground">
          {t("settingsDialog.harnesses.summary", {
            selected: selectedCount,
            total: all.length,
            detected: installed.length,
          })}
        </p>
        <p className="m-0 flex flex-wrap items-center gap-x-2 text-xs text-muted-foreground">
          <span>
            {autoDeployEnabled ? t("settingsDialog.harnesses.autoDeployOn") : t("settingsDialog.harnesses.autoDeployOff")}
          </span>
          <Button variant="ghost" tone="accent" size="sm" onClick={onOpenGeneral}>
            {t("settingsDialog.harnesses.autoDeployLink")}
          </Button>
        </p>
        <p className="m-0 text-xs text-muted-foreground">{t("settingsDialog.harnesses.offKeeps")}</p>
      </div>

      <section className="flex flex-col gap-2" aria-labelledby={`${listId}-heading`}>
        <h4 id={`${listId}-heading`} className="m-0 text-xs font-medium text-faint">
          {t("settingsDialog.harnesses.list")}
        </h4>
        {groups.detected.length === 0 ? (
          <p className="m-0 text-sm text-muted-foreground">{t("settingsDialog.harnesses.noMatches")}</p>
        ) : (
          <ul className="m-0 flex list-none flex-col divide-y divide-border rounded-lg border border-border p-0">
            {groups.detected.map((entry) => row(entry.tool, entry.readers))}
          </ul>
        )}
        {groups.undetected.count > 0 ? (
          <div className="flex flex-col gap-2">
            <Button
              variant="ghost"
              className="self-start"
              aria-expanded={undetectedOpen}
              aria-controls={`${listId}-undetected`}
              onClick={() => setUndetectedOpen((open) => !open)}
            >
              <ChevronRight aria-hidden="true" className={cn(undetectedOpen && "rotate-90")} />
              {t("settingsDialog.harnesses.notDetectedGroup", { count: groups.undetected.count })}
              {undetectedSelected > 0 ? (
                <Badge variant="soft" tone="stale">
                  {t("settingsDialog.harnesses.notDetectedSelected", { count: undetectedSelected })}
                </Badge>
              ) : null}
            </Button>
            {undetectedOpen ? (
              <ul
                id={`${listId}-undetected`}
                className="m-0 flex list-none flex-col divide-y divide-border rounded-lg border border-border p-0"
              >
                {groups.undetected.tools.map((tool) => row(tool, []))}
                {groups.undetected.readers.map((reader) => (
                  <UndetectedReaderRow key={reader.key} reader={reader} labelOf={labelOf} t={t} />
                ))}
              </ul>
            ) : (
              <div id={`${listId}-undetected`} hidden />
            )}
          </div>
        ) : null}
      </section>

      <SettingGroup title={t("settingsDialog.harnesses.scanTitle")}>
        <Field className="flex-row items-start gap-2 py-3">
          <Checkbox
            className="mt-0.5"
            checked={scanSelectedOnly}
            onCheckedChange={(checked) => onScanSelectedOnlyChange(checked)}
            disabled={blockedReason !== null}
            focusableWhenDisabled
            title={blockedReason ?? undefined}
          />
          <div className="flex min-w-0 flex-col gap-1">
            <FieldLabel>{t("globalToolConfigScanSelectedOnly")}</FieldLabel>
            <FieldDescription>{t("settingsDialog.harnesses.scanHint")}</FieldDescription>
          </div>
        </Field>
      </SettingGroup>
    </>
  );
}

function HarnessRow({
  tool,
  label,
  labelOf,
  readers,
  selected,
  expanded,
  warning,
  onToggleExpanded,
  onSelectedChange,
  blockedReason,
  t,
}: {
  tool: ToolInfoDto;
  label: string;
  labelOf: (toolId: string) => string;
  /** The detected reader-only harnesses listed under this row ("Read by"). */
  readers: readonly ReaderHarnessDto[];
  selected: boolean;
  expanded: boolean;
  /** The cross-read warning line, when a pair with both harnesses selected involves this one. */
  warning: string | null;
  onToggleExpanded: () => void;
  onSelectedChange: (selected: boolean) => void;
  blockedReason: string | null;
  t: TFunction;
}) {
  const detailsId = useId();
  const detected = tool.installed;
  const stray = selected && !detected;
  const placement = tool.placement;
  const sharers = tool.shared_with.filter((key) => key !== tool.key).map(labelOf);
  return (
    <li data-harness-row={tool.key}>
      <div className="flex h-row items-center gap-3 pr-3 pl-1">
        <Button
          variant="ghost"
          className="min-w-0 flex-1 justify-start"
          aria-expanded={expanded}
          aria-controls={detailsId}
          onClick={onToggleExpanded}
        >
          <ChevronRight aria-hidden="true" className={cn(expanded && "rotate-90")} />
          <span className="truncate">{label}</span>
        </Button>
        <Badge variant={stray ? "soft" : detected ? "soft" : "outline"} tone={stray ? "stale" : "neutral"}>
          {detected ? t("settingsDialog.harnesses.detected") : t("settingsDialog.harnesses.notDetected")}
        </Badge>
        <span className="contents">
          <Switch
            size="sm"
            checked={selected}
            onCheckedChange={onSelectedChange}
            aria-label={t("settingsDialog.harnesses.select", { harness: label })}
            aria-describedby={warning ? `${detailsId}-warning` : undefined}
            disabled={blockedReason !== null}
            focusableWhenDisabled
            title={blockedReason ?? undefined}
          />
        </span>
      </div>
      {warning ? (
        <p
          id={`${detailsId}-warning`}
          data-harness-warning={tool.key}
          className="m-0 flex items-start gap-1 pr-3 pb-2 pl-10 text-xs text-stale"
        >
          <TriangleAlert aria-hidden="true" className="mt-px size-3.5 shrink-0" />
          {warning}
        </p>
      ) : null}
      {readers.length > 0 ? (
        <div className="flex flex-col gap-1 pr-3 pb-2 pl-10">
          <span id={`${detailsId}-readers`} className="text-xs text-muted-foreground">
            {t("settingsDialog.harnesses.readBy")}
          </span>
          <ul aria-labelledby={`${detailsId}-readers`} className="m-0 flex list-none flex-col gap-1 p-0">
            {readers.map((reader) => (
              <li
                key={reader.key}
                data-harness-reader={reader.key}
                className="flex items-center justify-between gap-3 text-sm text-foreground"
              >
                <span className="truncate">{t(`tools.${reader.key}`, { defaultValue: reader.label })}</span>
                <Badge variant={reader.installed ? "soft" : "outline"} tone="neutral">
                  {reader.installed ? t("settingsDialog.harnesses.detected") : t("settingsDialog.harnesses.notDetected")}
                </Badge>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
      {expanded ? (
        <div id={detailsId} className="flex flex-col gap-3 border-t border-border px-10 py-3">
          {placement ? (
            <dl className="m-0 grid grid-cols-[8rem_1fr] gap-x-4 gap-y-1.5 text-xs">
              <dt className="text-muted-foreground">{t("settingsDialog.harnesses.skillsFolder")}</dt>
              <dd className="m-0 flex min-w-0 flex-col gap-0.5">
                <span className="font-mono break-all text-foreground">{placement.skills_dir}</span>
                {sharers.length > 0 ? (
                  <span className="text-muted-foreground">
                    {t("settingsDialog.harnesses.sharedWith", { harnesses: sharers.join(t("common.listSeparator")) })}
                  </span>
                ) : null}
              </dd>
              <dt className="text-muted-foreground">{t("settingsDialog.harnesses.deploysAs")}</dt>
              <dd className="m-0 text-foreground">
                {placement.supports_symlink ? t("settingsDialog.harnesses.link") : t("settingsDialog.harnesses.copy")}
              </dd>
              <dt className="text-muted-foreground">{t("settingsDialog.harnesses.detectedBy")}</dt>
              <dd className="m-0 font-mono break-all text-foreground">
                {placement.detect_dirs.join(t("common.listSeparator"))}
              </dd>
            </dl>
          ) : null}
          {stray ? (
            <div className="flex flex-col items-start gap-2">
              <p className="m-0 text-xs text-muted-foreground">{t("settingsDialog.harnesses.undetectedSelected")}</p>
              <Button
                size="sm"
                onClick={() => onSelectedChange(false)}
                disabled={blockedReason !== null}
                focusableWhenDisabled
                title={blockedReason ?? undefined}
              >
                {t("settingsDialog.harnesses.remove")}
              </Button>
            </div>
          ) : null}
        </div>
      ) : (
        <div id={detailsId} hidden />
      )}
    </li>
  );
}

/** An undetected reader-only harness in the "Not detected" group: no switch — it has no folder of its own. */
function UndetectedReaderRow({
  reader,
  labelOf,
  t,
}: {
  reader: UndetectedReader;
  labelOf: (toolId: string) => string;
  t: TFunction;
}) {
  return (
    <li data-harness-reader={reader.key} className="flex h-row items-center gap-3 pr-3 pl-10">
      <span className="min-w-0 flex-1 truncate text-sm text-foreground">
        {t(`tools.${reader.key}`, { defaultValue: reader.label })}
      </span>
      <span className="text-xs text-muted-foreground">
        {t("settingsDialog.harnesses.readerOf", { folder: labelOf(reader.reads) })}
      </span>
      <Badge variant="outline" tone="neutral">
        {t("settingsDialog.harnesses.notDetected")}
      </Badge>
    </li>
  );
}
