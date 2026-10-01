import { useId, useLayoutEffect, useRef, type ReactNode } from "react";
import type { TFunction } from "i18next";
import { Ban, ChevronRight, CircleCheck, CircleDashed, CircleX, Clock, LoaderCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Field, FieldDescription, FieldError, FieldLabel, FieldLegend, FieldSet, Input } from "@/components/ui/field";
import { Popover, PopoverContent, PopoverDescription, PopoverTitle, PopoverTrigger } from "@/components/ui/popover";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { focusRing } from "@/components/ui/styles";
import { cn } from "@/lib/utils";
import type { AddStep, AddView } from "../../../hooks/useAddSkillFlow";
import { pickSummaryText, type SettlementRow, type SettlementRowStatus } from "../../../lib/addFlow";
import { CandidatePicker } from "./CandidatePicker";

type AddSkillDialogProps = {
  /** The Add flow's view (`useAddSkillFlow().add`): its values and intents. */
  add: AddView;
  /** Any action runs (Add's own included): inputs and verbs wait. */
  busy: boolean;
  /** Why a verb is refused while another action runs (D19), in words. */
  blockedReason: string | null;
  /** The header progress, shown in the sticky summary while Add's action runs (its Cancel, D20). */
  progress: ReactNode;
  t: TFunction;
};

const STEPS: readonly AddStep[] = ["source", "candidates", "settlement"];

/**
 * Add as one stepped surface (ticket 16, D27): Source → Candidates →
 * Settlement in one Dialog, under a sticky summary (source · candidates ·
 * Tool scope, and the running action's progress with its Cancel). The only
 * layer above it is the one overwrite ask (D12), pushed by the sync seam.
 * The shell stays mounted for the app's life so closing returns focus to
 * the control that opened it; each step moves focus to its own first
 * control (the field, the search, the settlement heading).
 */
export default function AddSkillDialog({ add, busy, blockedReason, progress, t }: AddSkillDialogProps) {
  const { step } = add;
  const fieldRef = useRef<HTMLInputElement | null>(null);
  const searchRef = useRef<HTMLInputElement | null>(null);
  const candidatesRef = useRef<HTMLDivElement | null>(null);
  const settlementRef = useRef<HTMLHeadingElement | null>(null);
  // Step changes move focus (not the dialog's opening: its `initialFocus`
  // does), in the same commit that removed the control that moved the step —
  // so no frame has focus on <body> (D21).
  const shownStep = useRef<AddStep | null>(null);
  useLayoutEffect(() => {
    if (!add.open) {
      shownStep.current = null;
      return;
    }
    if (shownStep.current === null || shownStep.current === step) {
      shownStep.current = step;
      return;
    }
    shownStep.current = step;
    if (step === "source") fieldRef.current?.focus();
    else if (step === "candidates")
      (searchRef.current ?? candidatesRef.current?.querySelector<HTMLElement>("[role='checkbox']:not([data-disabled])"))?.focus();
    else settlementRef.current?.focus();
  }, [add.open, step]);

  // A focused control a render removed — the summary's Cancel, withdrawn for
  // the ask's retry or gone with the settled action — hands focus to the
  // step's own first control in the same commit, before any fallback could
  // pick the dialog's first tabbable (D21). An inert layer (under the ask)
  // takes no focus, so this never reaches past the top layer.
  const lastFocus = useRef<Element | null>(null);
  useLayoutEffect(() => {
    const last = lastFocus.current;
    if (!add.open || !last || last.isConnected) return;
    lastFocus.current = null;
    if (step === "source") fieldRef.current?.focus();
    else if (step === "candidates") searchRef.current?.focus();
    else settlementRef.current?.focus();
  });

  const stepIndex = STEPS.indexOf(step);
  // Add's own action (the listing, the batch) pends its control and holds
  // the dialog; another action already running refuses its verbs, with why.
  const running = add.running !== null;
  const blocked = !running && busy ? (blockedReason ?? "") : null;

  return (
    <Dialog
      open={add.open}
      dismissible={!running}
      onOpenChange={(next) => {
        if (!next) add.close();
      }}
    >
      <DialogContent
        size="lg"
        // Add always opens on its source step: the field takes focus.
        initialFocus={fieldRef}
      >
        {/* Layout-neutral: records where focus is, for the rule above. */}
        <div
          className="contents"
          onFocusCapture={(event) => {
            lastFocus.current = event.target;
          }}
        >
          <DialogHeader>
            <DialogTitle>{t("add.title")}</DialogTitle>
            <ol aria-label={t("add.steps.label")} className="m-0 flex list-none items-center gap-1 p-0 text-xs">
              {STEPS.map((name, index) => (
                <li key={name} className="flex items-center gap-1">
                  {index > 0 ? <ChevronRight aria-hidden="true" className="size-3 text-faint" /> : null}
                  <span
                    aria-current={index === stepIndex ? "step" : undefined}
                    className={cn(index === stepIndex ? "font-medium text-foreground" : "text-muted-foreground")}
                  >
                    {t(`add.steps.${name}`)}
                  </span>
                </li>
              ))}
            </ol>
          </DialogHeader>

          <AddSummary add={add} running={running} progress={progress} t={t} />

          {step === "source" ? (
            <SourceStep add={add} running={running} blocked={blocked} fieldRef={fieldRef} t={t} />
          ) : step === "candidates" ? (
            <CandidatesStep
              add={add}
              running={running}
              blocked={blocked}
              searchRef={searchRef}
              listRef={candidatesRef}
              t={t}
            />
          ) : (
            <SettlementStep add={add} running={running} blocked={blocked} headingRef={settlementRef} t={t} />
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}

// ---------------------------------------------------------------------------
// The sticky summary
// ---------------------------------------------------------------------------

/**
 * Source · candidates · Tool scope, through every step (D27), and the
 * running action's progress under them. The Tool scope is the effective
 * deploy set; "Edit Tools" opens the catalog's installed Tools as visible,
 * focusable checkboxes.
 */
function AddSummary({ add, running, progress, t }: { add: AddView; running: boolean; progress: ReactNode; t: TFunction }) {
  const { summary } = add;
  const shown = summary.tools.slice(0, 3).map((tool) => tool.label);
  const more = summary.tools.length - shown.length;
  const toolsText = !summary.deploy
    ? t("add.summary.autoSyncOff")
    : summary.tools.length === 0
      ? t("add.summary.noTools")
      : shown.join(t("common.listSeparator"));
  const moreText = summary.deploy && more > 0 ? t("add.summary.more", { count: more }) : null;
  const toolsTitle = summary.tools.map((tool) => tool.label).join(t("common.listSeparator"));
  const candidatesText = summary.candidates
    ? add.step === "settlement"
      ? t("add.summary.sent", { count: summary.candidates.total })
      : pickSummaryText(summary.candidates, t)
    : t("add.summary.none");

  return (
    <div data-slot="add-summary" className="flex shrink-0 flex-col border-b border-border bg-card">
      <dl className="m-0 grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1.4fr)] gap-x-4 px-5 py-3">
        <SummaryItem label={t("add.summary.source")} value={summary.source ?? t("add.summary.none")} muted={!summary.source} />
        <SummaryItem label={t("add.summary.skills")} value={candidatesText} muted={!summary.candidates} wraps />
        <div className="flex min-w-0 flex-col gap-0.5">
          <dt className="text-xs text-muted-foreground">{t("add.summary.tools")}</dt>
          <dd className="m-0 flex min-w-0 items-center gap-2">
            <span className="flex min-w-0 items-baseline gap-1 text-sm text-foreground" title={toolsTitle || undefined}>
              <span className="min-w-0 truncate">{toolsText}</span>
              {moreText ? <span className="shrink-0 text-muted-foreground tabular-nums">{moreText}</span> : null}
            </span>
            <ToolScope add={add} running={running} t={t} />
          </dd>
        </div>
      </dl>
      {progress}
    </div>
  );
}

/** One summary fact; `wraps` lets a count line take a second row instead of clipping (960 px). */
function SummaryItem({ label, value, muted, wraps = false }: { label: string; value: string; muted: boolean; wraps?: boolean }) {
  return (
    <div className="flex min-w-0 flex-col gap-0.5">
      <dt className="text-xs text-muted-foreground">{label}</dt>
      <dd
        className={cn(
          "m-0 min-w-0 text-sm tabular-nums",
          wraps ? "break-words" : "truncate",
          muted ? "text-muted-foreground" : "text-foreground",
        )}
        title={wraps ? undefined : value}
      >
        {value}
      </dd>
    </div>
  );
}

/** "Edit Tools": the deploy-after-add choice over the catalog's installed Tools. */
function ToolScope({ add, running, t }: { add: AddView; running: boolean; t: TFunction }) {
  const skills = add.summary.candidates?.checked ?? 0;
  const tools = add.summary.tools.length;
  return (
    <Popover>
      <PopoverTrigger
        render={
          <Button size="sm" variant="ghost" tone="accent" className="shrink-0" disabled={running} focusableWhenDisabled />
        }
      >
        {t("add.tools.edit")}
      </PopoverTrigger>
      <PopoverContent align="end" className="w-80">
        <PopoverTitle>{t("add.tools.title")}</PopoverTitle>
        <PopoverDescription>{add.summary.deploy ? t("syncAfterCreate") : t("add.tools.autoSyncOff")}</PopoverDescription>
        <FieldSet>
          <FieldLegend className="sr-only">{t("add.tools.legend")}</FieldLegend>
          <div className="grid grid-cols-2 gap-x-4 gap-y-1.5">
            {add.toolChoices.map((tool) => (
              <Field key={tool.id} className="flex-row items-start gap-2">
                <Checkbox
                  className="mt-0.5"
                  checked={Boolean(add.selectedTools[tool.id])}
                  onCheckedChange={(checked) => add.setTool(tool.id, checked)}
                  disabled={running}
                />
                <div className="flex min-w-0 flex-col">
                  <FieldLabel>{tool.label}</FieldLabel>
                  {tool.detected ? null : <FieldDescription>{t("toolNotDetected")}</FieldDescription>}
                </div>
              </Field>
            ))}
          </div>
        </FieldSet>
        <p role="status" className="m-0 text-xs text-muted-foreground tabular-nums">
          {tools === 0
            ? t("add.tools.none")
            : skills > 0
              ? t("add.tools.pairs", { skills: t("add.tools.skills", { count: skills }), tools: t("add.tools.tools", { count: tools }) })
              : t("add.tools.tools", { count: tools })}
        </p>
      </PopoverContent>
    </Popover>
  );
}

// ---------------------------------------------------------------------------
// Steps
// ---------------------------------------------------------------------------

type StepProps = {
  add: AddView;
  /** Add's own action runs: its control pends, the rest waits. */
  running: boolean;
  /** Another action runs: Add's verbs are refused with this reason (D19). */
  blocked: string | null;
  t: TFunction;
};

/** Source: a Git repository or a local folder (real tabs), then Continue lists it. */
function SourceStep({ add, running, blocked, fieldRef, t }: StepProps & { fieldRef: React.RefObject<HTMLInputElement | null> }) {
  const formId = useId();
  const value = add.kind === "git" ? add.draft.url : add.draft.path;
  return (
    <>
      <DialogBody>
        <form
          id={formId}
          className="flex flex-col gap-3"
          onSubmit={(event) => {
            event.preventDefault();
            if (!running && blocked === null) void add.list();
          }}
        >
          <Tabs value={add.kind} onValueChange={(next) => add.setKind(next === "local" ? "local" : "git")}>
            <TabsList variant="segmented" aria-label={t("add.source.kindLabel")}>
              <TabsTrigger value="git">{t("add.source.git")}</TabsTrigger>
              <TabsTrigger value="local">{t("add.source.local")}</TabsTrigger>
            </TabsList>
            <TabsContent value="git" tabIndex={-1}>
              {add.kind === "git" ? (
                <Field invalid={add.sourceError !== null}>
                  <FieldLabel>{t("repositoryUrl")}</FieldLabel>
                  <Input
                    ref={fieldRef}
                    data-add-source-field=""
                    type="text"
                    inputMode="url"
                    value={add.draft.url}
                    onChange={(event) => add.editDraft({ url: event.target.value })}
                    placeholder={t("add.source.urlPlaceholder")}
                    readOnly={running}
                    autoComplete="off"
                    spellCheck={false}
                  />
                  <FieldError>{add.sourceError}</FieldError>
                  <FieldDescription>{t("add.source.urlHelp")}</FieldDescription>
                </Field>
              ) : null}
            </TabsContent>
            <TabsContent value="local" tabIndex={-1}>
              {add.kind === "local" ? (
                <Field invalid={add.sourceError !== null}>
                  <FieldLabel>{t("localFolder")}</FieldLabel>
                  <div className="flex gap-2">
                    <Input
                      ref={fieldRef}
                      data-add-source-field=""
                      type="text"
                      value={add.draft.path}
                      onChange={(event) => add.editDraft({ path: event.target.value })}
                      placeholder={t("add.source.pathPlaceholder")}
                      readOnly={running}
                      autoComplete="off"
                      spellCheck={false}
                    />
                    <Button type="button" onClick={() => void add.browse()} disabled={running}>
                      {t("add.source.browse")}
                    </Button>
                  </div>
                  <FieldError>{add.sourceError}</FieldError>
                  <FieldDescription>{t("add.source.pathHelp")}</FieldDescription>
                </Field>
              ) : null}
            </TabsContent>
          </Tabs>
        </form>
      </DialogBody>
      <DialogFooter>
        <Button onClick={add.close} disabled={running}>
          {t("cancel")}
        </Button>
        <Button
          type="submit"
          form={formId}
          variant="solid"
          tone="accent"
          pending={running}
          disabled={!value.trim() || blocked !== null}
          focusableWhenDisabled
          title={blocked || undefined}
        >
          {t("add.source.continue")}
        </Button>
      </DialogFooter>
    </>
  );
}

/** Candidates: search, check, name a lone one, Add selected. */
function CandidatesStep({
  add,
  running,
  blocked,
  searchRef,
  listRef,
  t,
}: StepProps & {
  searchRef: React.RefObject<HTMLInputElement | null>;
  listRef: React.RefObject<HTMLDivElement | null>;
}) {
  const lone = add.nameApplies ? add.candidates[0] : null;
  const errorId = useId();
  return (
    <>
      <DialogBody>
        {add.preselectMissing ? (
          <p role="status" className="m-0 rounded-md border border-border bg-card px-3 py-2 text-sm text-foreground">
            {t("add.picker.preselectMissing", { name: add.preselectMissing })}
          </p>
        ) : null}
        <div ref={listRef}>
          <CandidatePicker
            mode="multiple"
            candidates={add.candidates}
            visible={add.visible}
            query={add.query}
            onQueryChange={add.setQuery}
            selected={add.selected}
            onToggle={add.toggle}
            onToggleAll={add.toggleAll}
            legend={t("add.picker.legend", { source: add.summary.source ?? "" })}
            status={add.summary.candidates ? pickSummaryText(add.summary.candidates, t) : ""}
            disabled={running}
            searchRef={searchRef}
            t={t}
          />
        </div>
        {lone ? (
          <Field>
            <FieldLabel>{t("add.picker.nameLabel")}</FieldLabel>
            <Input
              value={add.draft.name}
              onChange={(event) => add.editDraft({ name: event.target.value })}
              placeholder={lone.name}
              readOnly={running}
              autoComplete="off"
              spellCheck={false}
            />
            <FieldDescription>{t("add.picker.nameHelp", { name: lone.name })}</FieldDescription>
          </Field>
        ) : null}
        {add.pickError ? (
          <p id={errorId} role="alert" className="m-0 text-sm text-destructive">
            {add.pickError}
          </p>
        ) : null}
      </DialogBody>
      <DialogFooter>
        <Button onClick={add.back} disabled={running}>
          {t("add.back")}
        </Button>
        <Button
          variant="solid"
          tone="accent"
          onClick={() => void add.install()}
          pending={running}
          disabled={blocked !== null}
          focusableWhenDisabled
          title={blocked || undefined}
          aria-describedby={add.pickError ? errorId : undefined}
        >
          {t("add.addSelected")}
        </Button>
      </DialogFooter>
    </>
  );
}

const STATUS_ICON: Record<SettlementRowStatus, typeof CircleCheck> = {
  queued: CircleDashed,
  adding: LoaderCircle,
  awaiting: Clock,
  installed: CircleCheck,
  failed: CircleX,
  cancelled: Ban,
};

/** Settlement: one row per sent candidate — added (ticked), failed (reason + Retry), cancelled. */
function SettlementStep({
  add,
  running,
  blocked,
  headingRef,
  t,
}: StepProps & { headingRef: React.RefObject<HTMLHeadingElement | null> }) {
  const rows = add.settlement;
  const inFlight = rows.some((row) => row.status === "queued" || row.status === "adding" || row.status === "awaiting");
  const installed = rows.filter((row) => row.status === "installed").length;
  const failedIds = rows.filter((row) => row.status === "failed").map((row) => row.id);
  const headingId = useId();
  return (
    <>
      <DialogBody>
        <h3
          ref={headingRef}
          id={headingId}
          tabIndex={-1}
          className={cn("m-0 rounded-sm text-sm font-semibold text-foreground", focusRing)}
        >
          {inFlight
            ? t("add.settlement.headingRunning", { count: rows.length })
            : t("add.settlement.heading", { installed, total: rows.length })}
        </h3>
        <ul aria-labelledby={headingId} className="m-0 flex list-none flex-col rounded-md border border-border p-0">
          {rows.map((row, index) => (
            <SettlementItem
              key={row.id}
              row={row}
              index={index}
              running={running}
              blocked={blocked}
              onRetry={() => void add.retry([row.id])}
              t={t}
            />
          ))}
        </ul>
      </DialogBody>
      <DialogFooter>
        {failedIds.length > 1 ? (
          <Button
            tone="accent"
            onClick={() => void add.retry(failedIds)}
            disabled={running || blocked !== null}
            focusableWhenDisabled
            title={blocked || undefined}
          >
            {t("add.settlement.retryFailed", { count: failedIds.length })}
          </Button>
        ) : null}
        <Button variant="solid" tone={inFlight ? "neutral" : "accent"} onClick={add.close} disabled={running} focusableWhenDisabled>
          {t("close")}
        </Button>
      </DialogFooter>
    </>
  );
}

function SettlementItem({
  row,
  index,
  running,
  blocked,
  onRetry,
  t,
}: {
  row: SettlementRow;
  index: number;
  running: boolean;
  blocked: string | null;
  onRetry: () => void;
  t: TFunction;
}) {
  const Icon = STATUS_ICON[row.status];
  const tone =
    row.status === "installed"
      ? row.attention
        ? "text-stale"
        : "text-ok"
      : row.status === "failed"
        ? "text-failed"
        : "text-muted-foreground";
  return (
    <li data-settlement-row={row.status} className="flex items-start gap-3 border-b border-border px-3 py-2 last:border-b-0">
      {/* The install landing (DESIGN.md § Motion, the rare moment): a settled
          glyph fades in once, 120 ms, staggered 20 ms per row; reduced
          motion shows it at once. Keyed by status so only a change lands. */}
      <Icon
        key={row.status}
        aria-hidden="true"
        className={cn(
          "mt-0.5 size-4 shrink-0",
          tone,
          row.status === "adding" && "motion-safe:animate-spin",
          (row.status === "installed" || row.status === "failed" || row.status === "cancelled") &&
            "transition-opacity duration-(--motion-fast) ease-out starting:opacity-0 motion-reduce:transition-none",
        )}
        style={{ transitionDelay: `${Math.min(index, 8) * 20}ms` }}
      />
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <div className="flex min-w-0 items-baseline gap-2">
          <span className="truncate text-sm font-medium text-foreground" title={row.name}>
            {row.name}
          </span>
          <span className={cn("shrink-0 text-xs", tone)}>{t(`add.settlement.status.${row.status}`)}</span>
        </div>
        {row.detail ? <p className="m-0 text-xs text-muted-foreground">{row.detail}</p> : null}
      </div>
      {row.retryable ? (
        <Button
          size="sm"
          variant="outline"
          tone="accent"
          className="shrink-0"
          onClick={onRetry}
          disabled={running || blocked !== null}
          focusableWhenDisabled
          title={blocked || undefined}
          aria-label={t(row.status === "failed" ? "add.settlement.retryNamed" : "add.settlement.addNamed", { name: row.name })}
        >
          {row.status === "failed" ? t("add.settlement.retry") : t("add.settlement.add")}
        </Button>
      ) : null}
    </li>
  );
}
