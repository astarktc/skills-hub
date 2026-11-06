import { useId, useLayoutEffect, useRef, type ReactNode } from "react";
import type { TFunction } from "i18next";
import { ChevronRight, CircleCheck, CircleMinus, CircleX } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Dialog, DialogBody, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Field, FieldDescription, FieldLabel } from "@/components/ui/field";
import { focusRing } from "@/components/ui/styles";
import { cn } from "@/lib/utils";
import type { ImportReviewView, ImportStep } from "../../../hooks/useImportReview";
import {
  COPY_FATE_KEY,
  GROUP_FATE_KEY,
  type ImportResultRow,
  type ImportReviewModel,
  type ImportRow,
  type ManagedRow,
  type ReviewCopy,
} from "../../../lib/importReview";
import { importReviewFallback, resolveFirst } from "../../../lib/actionLifecycle";
import { formatRelativeTime } from "../../../lib/skillPresentation";

type ImportReviewDialogProps = {
  /** The import flow's view (`useAddSkillFlow().importReview`): its values and intents. */
  review: ImportReviewView;
  /** Any action runs (the import's own included): verbs wait. */
  busy: boolean;
  /** Why a verb is refused while another action runs (D19), in words. */
  blockedReason: string | null;
  /** The header progress at the dialog's foot while the import runs (D19; the page behind is inert). */
  progress: ReactNode;
  t: TFunction;
};

const STEPS: readonly ImportStep[] = ["review", "result"];

/**
 * The Import review (ticket 21, D13 / D14): one stepped Dialog, Review →
 * Result. Review lists the plan's groups as rows — new names (Match / a
 * Conflict whose copies are radios, each copy's fate stated), names the
 * Library already holds (Leave / Replace, each copy compared with the managed
 * copy) and invalid manifests (disabled, the reason in words) — over the
 * policy line that says what auto-deploy or the originals checkbox will do,
 * and a button named by the action. Result shows the composite count line
 * and one row per sent group, Retry for a failed one. The one overwrite ask
 * (D12) sits above it while the import's collisions are answered. The shell
 * stays mounted so closing returns focus to the control that opened it.
 */
export default function ImportReviewDialog({ review, busy, blockedReason, progress, t }: ImportReviewDialogProps) {
  const { step, model, result } = review;
  const countsRef = useRef<HTMLHeadingElement | null>(null);
  const resultRef = useRef<HTMLHeadingElement | null>(null);
  const running = review.running;
  const blocked = !running && busy ? (blockedReason ?? "") : null;
  const stepIndex = STEPS.indexOf(step);

  // The Result takes focus in the commit that shows it (D21): its heading.
  const shownStep = useRef<ImportStep | null>(null);
  useLayoutEffect(() => {
    if (!review.open) {
      shownStep.current = null;
      return;
    }
    if (shownStep.current !== null && shownStep.current !== step) {
      (step === "result" ? resultRef : countsRef).current?.focus();
    }
    shownStep.current = step;
  }, [review.open, step]);

  // A focused control a render removed (a Retry whose row settled, the
  // footer's Cancel withdrawn) hands focus to the step's heading, never <body>.
  const lastFocus = useRef<Element | null>(null);
  useLayoutEffect(() => {
    const last = lastFocus.current;
    if (!review.open || !last || last.isConnected) return;
    lastFocus.current = null;
    (step === "result" ? resultRef : countsRef).current?.focus();
  });

  return (
    <Dialog
      open={review.open}
      dismissible={!running}
      onOpenChange={(next) => {
        if (!next) review.close();
      }}
    >
      <DialogContent
        size="xl"
        initialFocus={countsRef}
        // The opener may be gone (the import consumed the banner) or never
        // captured: the surviving Review invitation, else My Skills' heading.
        fallbackFocus={(usable) => resolveFirst(importReviewFallback(), null, usable)}
      >
        <div
          className="contents"
          onFocusCapture={(event) => {
            lastFocus.current = event.target;
          }}
        >
          <DialogHeader>
            <DialogTitle>{t("importReview.title")}</DialogTitle>
            <ol aria-label={t("importReview.steps.label")} className="m-0 flex list-none items-center gap-1 p-0 text-xs">
              {STEPS.map((name, index) => (
                <li key={name} className="flex items-center gap-1">
                  {index > 0 ? <ChevronRight aria-hidden="true" className="size-3 text-faint" /> : null}
                  <span
                    aria-current={index === stepIndex ? "step" : undefined}
                    className={cn(index === stepIndex ? "font-medium text-foreground" : "text-muted-foreground")}
                  >
                    {t(`importReview.steps.${name}`)}
                  </span>
                </li>
              ))}
            </ol>
          </DialogHeader>
          {step === "review" || !result ? (
            model ? (
              <ReviewStep review={review} model={model} running={running} blocked={blocked} countsRef={countsRef} t={t} />
            ) : null
          ) : (
            <ResultStep review={review} result={result} running={running} blocked={blocked} headingRef={resultRef} t={t} />
          )}
          {progress}
        </div>
      </DialogContent>
    </Dialog>
  );
}

// ---------------------------------------------------------------------------
// Review
// ---------------------------------------------------------------------------

type StepProps = {
  review: ImportReviewView;
  /** The import runs: its controls wait. */
  running: boolean;
  /** Another action runs: the import is refused with this reason (D19). */
  blocked: string | null;
  t: TFunction;
};

const listSeparator = (t: TFunction) => t("common.listSeparator");

function ReviewStep({
  review,
  model,
  running,
  blocked,
  countsRef,
  t,
}: StepProps & { model: ImportReviewModel; countsRef: React.RefObject<HTMLHeadingElement | null> }) {
  const newId = useId();
  const managedId = useId();
  const invalidId = useId();
  const statusId = useId();
  const refusalId = useId();
  const { counts, selection } = model;
  const allState =
    selection.imports === 0 ? "none" : selection.imports === selection.importable ? "all" : "some";
  // Only invalid manifests left (#98): the line counts what is there.
  const invalidOnly = counts.skills === 0 && counts.invalid > 0;
  const countsLine = [
    ...(invalidOnly
      ? []
      : [
          t("importReview.counts.skills", { count: counts.skills }),
          t("importReview.counts.copies", { count: counts.copies, harnesses: t("importReview.counts.harnesses", { count: counts.harnesses }) }),
        ]),
    ...(counts.invalid > 0 ? [t("importReview.counts.invalid", { count: counts.invalid })] : []),
  ].join(" \u00b7 ");
  const refusal = model.action.refusal;
  const disabled = running || blocked !== null || refusal !== null;

  return (
    <>
      <DialogBody className="gap-5">
        <div className="flex flex-col gap-1">
          <h3
            ref={countsRef}
            tabIndex={-1}
            className={cn("m-0 w-fit rounded-sm text-sm font-semibold text-foreground tabular-nums", focusRing)}
          >
            {countsLine}
          </h3>
          <p className="m-0">{t(invalidOnly ? "importReview.introInvalidOnly" : "importReview.intro")}</p>
        </div>

        {model.importRows.length > 0 ? (
          <section aria-labelledby={newId} className="flex flex-col gap-2">
            <div className="flex min-h-6 flex-wrap items-center justify-between gap-x-3 gap-y-1">
              <h4 id={newId} className="m-0 text-sm font-semibold text-foreground">
                {t("importReview.sections.new")}
              </h4>
              <div className="flex items-center gap-3">
                {selection.importable > 1 ? (
                  <Field className="flex-row items-center gap-2">
                    <Checkbox
                      checked={allState === "all"}
                      indeterminate={allState === "some"}
                      onCheckedChange={(checked) => review.toggleAll(checked)}
                      disabled={running}
                      aria-describedby={statusId}
                    />
                    <FieldLabel>{t("importReview.selectAll")}</FieldLabel>
                  </Field>
                ) : null}
                <p id={statusId} role="status" className="m-0 text-xs text-muted-foreground tabular-nums">
                  {t("importReview.selected", { checked: selection.imports, total: selection.importable })}
                </p>
              </div>
            </div>
            <p className="m-0 text-xs">{t("importReview.legend")}</p>
            <ul className="m-0 flex list-none flex-col rounded-md border border-border p-0">
              {model.importRows.map((row) => (
                <ImportGroupItem key={row.name} row={row} review={review} running={running} t={t} />
              ))}
            </ul>
          </section>
        ) : null}

        {model.managedRows.length > 0 ? (
          <section aria-labelledby={managedId} className="flex flex-col gap-2">
            <h4 id={managedId} className="m-0 text-sm font-semibold text-foreground">
              {t("importReview.sections.managed")}
            </h4>
            <p className="m-0 text-xs">{t("importReview.managed.hint")}</p>
            <ul className="m-0 flex list-none flex-col rounded-md border border-border p-0">
              {model.managedRows.map((row) => (
                <ManagedGroupItem key={row.name} row={row} review={review} running={running} t={t} />
              ))}
            </ul>
          </section>
        ) : null}

        {model.invalidRows.length > 0 ? (
          <section aria-labelledby={invalidId} className="flex flex-col gap-2">
            <h4 id={invalidId} className="m-0 text-sm font-semibold text-foreground">
              {t("importReview.sections.invalid")}
            </h4>
            <ul className="m-0 flex list-none flex-col rounded-md border border-border p-0">
              {model.invalidRows.map((row) => (
                <li key={row.key} className="border-b border-border last:border-b-0">
                  <Field className="flex-row items-start gap-3 px-3 py-2" disabled>
                    <Checkbox className="mt-0.5" checked={false} disabled />
                    <div className="flex min-w-0 flex-col gap-0.5">
                      <FieldLabel>{row.name}</FieldLabel>
                      <FieldDescription className="flex flex-col gap-0.5">
                        <span className="truncate font-mono text-faint" title={row.path}>
                          {t("importReview.copy.where", { harness: row.toolLabel, path: row.path })}
                        </span>
                        <span className="text-stale">{t("importReview.invalidReason", { reason: row.reason })}</span>
                      </FieldDescription>
                    </div>
                  </Field>
                </li>
              ))}
            </ul>
          </section>
        ) : null}
      </DialogBody>

      <PolicyLine review={review} model={model} running={running} t={t} />

      <DialogFooter>
        <Button onClick={review.close} disabled={running}>
          {t("cancel")}
        </Button>
        <Button
          variant="solid"
          tone="accent"
          onClick={() => void review.importSelected()}
          pending={running}
          disabled={disabled}
          focusableWhenDisabled
          title={blocked || refusal || undefined}
          aria-describedby={refusal ? refusalId : undefined}
        >
          {model.action.label}
        </Button>
        {refusal ? (
          <span id={refusalId} className="sr-only">
            {refusal}
          </span>
        ) : null}
      </DialogFooter>
    </>
  );
}

/** "Claude Code — ~/.claude/skills/tdd", "Modified 3 days ago", the description: one copy's facts. */
function CopyFacts({ copy, t }: { copy: ReviewCopy; t: TFunction }) {
  return (
    <>
      {copy.description ? <span className="line-clamp-2 text-muted-foreground">{copy.description}</span> : null}
      <span className="text-faint">
        {copy.modifiedAt !== null
          ? t("importReview.copy.modified", { when: formatRelativeTime(copy.modifiedAt, t) })
          : t("importReview.copy.modifiedUnknown")}
        {copy.linkTarget ? ` \u00b7 ${t("importReview.copy.link", { target: copy.linkTarget })}` : null}
      </span>
    </>
  );
}

/** A copy's fate, in words: the chosen copy in foreground, a removal in its state colour. */
function Fate({ copy, t }: { copy: ReviewCopy; t: TFunction }) {
  if (!copy.fate) return null;
  return (
    <span
      className={cn(
        copy.chosen ? "font-medium text-foreground" : copy.fate === "removed" ? "text-failed" : "text-muted-foreground",
      )}
    >
      {t(COPY_FATE_KEY[copy.fate])}
    </span>
  );
}

function ImportGroupItem({ row, review, running, t }: { row: ImportRow; review: ImportReviewView; running: boolean; t: TFunction }) {
  const radioName = useId();
  const legendId = useId();
  const first = row.copies[0];
  const harnesses = row.harnesses.join(listSeparator(t));
  return (
    <li className="flex flex-col border-b border-border last:border-b-0" data-import-group={row.name}>
      <Field className="flex-row items-start gap-3 px-3 py-2">
        <Checkbox
          className="mt-0.5"
          checked={row.checked}
          onCheckedChange={(next) => review.toggle(row.name, next)}
          disabled={running}
        />
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <FieldLabel className="flex-wrap">
            <span className="min-w-0 truncate">{row.name}</span>
            {row.conflict ? (
              <Badge tone="stale">{t("importReview.badge.conflict")}</Badge>
            ) : row.copies.length > 1 ? (
              <Badge>{t("importReview.badge.match")}</Badge>
            ) : null}
          </FieldLabel>
          <FieldDescription className="flex flex-col gap-0.5">
            <span>
              {row.conflict
                ? t("importReview.copiesDiffer", { count: row.copies.length, harnesses })
                : t("importReview.copiesIn", { count: row.copies.length, harnesses })}
            </span>
            {!row.conflict && first ? <CopyFacts copy={first} t={t} /> : null}
            {row.groupFate ? (
              <span className="font-medium text-foreground">
                {t(GROUP_FATE_KEY[row.groupFate], { count: row.copies.length })}
              </span>
            ) : null}
            {row.lastFailure ? (
              <span className="text-failed">{t("importReview.lastFailure", { reason: row.lastFailure })}</span>
            ) : null}
          </FieldDescription>
        </div>
      </Field>
      {row.conflict ? (
        <div className="flex flex-col gap-2 pr-3 pb-2.5 pl-10">
          <fieldset className="m-0 flex min-w-0 flex-col gap-1 border-0 p-0" role="radiogroup" aria-labelledby={legendId}>
            <legend id={legendId} className="sr-only">
              {t("importReview.chooseCopy", { name: row.name })}
            </legend>
            {row.copies.map((copy) => (
              <VariantRadio
                key={copy.path}
                copy={copy}
                name={radioName}
                checked={copy.path === row.chosenPath}
                onChoose={() => review.chooseVariant(row.name, copy.path)}
                disabled={running}
                t={t}
              />
            ))}
          </fieldset>
          {row.others > 0 ? (
            <Field className="flex-row items-start gap-2">
              <Checkbox
                className="mt-0.5"
                checked={row.removeOthers}
                onCheckedChange={(next) => review.setRemoveOthers(row.name, next)}
                disabled={running}
              />
              <div className="flex min-w-0 flex-col gap-0.5">
                <FieldLabel>{t("importReview.removeOthers", { count: row.others, name: row.name })}</FieldLabel>
                <FieldDescription>
                  {row.removeOthersThen === "deployed"
                    ? t("importReview.removeOthersOn")
                    : row.removeOthersThen === "nothingDeployed"
                      ? t("importReview.removeOthersOnLibraryOnly")
                      : t("importReview.removeOthersOff")}
                </FieldDescription>
              </div>
            </Field>
          ) : null}
        </div>
      ) : null}
    </li>
  );
}

/** One copy of a Conflict: a radio named by its harness and path, described by its facts and fate. */
function VariantRadio({
  copy,
  name,
  checked,
  onChoose,
  disabled,
  t,
}: {
  copy: ReviewCopy;
  name: string;
  checked: boolean;
  onChoose: () => void;
  disabled: boolean;
  t: TFunction;
}) {
  const id = useId();
  return (
    <label htmlFor={id} className="flex cursor-default items-start gap-2.5 rounded-md px-2 py-1.5 hover:bg-raised">
      <input
        id={id}
        type="radio"
        name={name}
        value={copy.path}
        checked={checked}
        onChange={onChoose}
        disabled={disabled}
        aria-labelledby={`${id}-name`}
        aria-describedby={`${id}-facts`}
        className={cn("mt-0.5 size-4 shrink-0 accent-primary", focusRing)}
      />
      <span className="flex min-w-0 flex-col gap-0.5 text-xs">
        {/* The radio's name: harness and path; the facts and fate describe it. */}
        <span id={`${id}-name`} className="min-w-0 text-sm text-foreground">
          <span className="font-medium">{copy.toolLabel}</span>
          <span className="text-muted-foreground"> {"\u2014"} </span>
          <span className="font-mono text-xs break-all text-muted-foreground">{copy.path}</span>
        </span>
        <span id={`${id}-facts`} className="flex min-w-0 flex-col gap-0.5">
          <CopyFacts copy={copy} t={t} />
          <Fate copy={copy} t={t} />
        </span>
      </span>
    </label>
  );
}

function ManagedGroupItem({ row, review, running, t }: { row: ManagedRow; review: ImportReviewView; running: boolean; t: TFunction }) {
  const radioName = useId();
  const legendId = useId();
  const harnesses = row.harnesses.join(listSeparator(t));
  const choice = (value: "leave" | "replace", label: string, hint: string) => {
    const id = `${radioName}-${value}`;
    return (
      <label htmlFor={id} className="flex cursor-default items-start gap-2.5 rounded-md px-2 py-1.5 hover:bg-raised">
        <input
          id={id}
          type="radio"
          name={radioName}
          value={value}
          checked={(value === "replace") === row.replace}
          onChange={() => review.setReplace(row.name, value === "replace")}
          disabled={running}
          aria-labelledby={`${id}-label`}
          aria-describedby={`${id}-hint`}
          className={cn("mt-0.5 size-4 shrink-0 accent-primary", focusRing)}
        />
        <span className="flex min-w-0 flex-col gap-0.5">
          <span id={`${id}-label`} className="text-sm text-foreground">
            {label}
          </span>
          <span id={`${id}-hint`} className="text-xs text-muted-foreground">
            {hint}
          </span>
        </span>
      </label>
    );
  };
  return (
    <li className="flex flex-col gap-1.5 border-b border-border px-3 py-2 last:border-b-0" data-import-group={row.name}>
      <div className="flex min-w-0 flex-wrap items-center gap-2">
        <span className="min-w-0 truncate text-sm font-medium text-foreground">{row.name}</span>
        <Badge>{t("importReview.badge.managed")}</Badge>
      </div>
      <p className="m-0 text-xs">{t("importReview.managed.taken", { name: row.name })}</p>
      <ul className="m-0 flex list-none flex-col gap-1 p-0 text-xs">
        {row.copies.map((copy) => (
          <li key={copy.path} className="flex min-w-0 flex-col">
            <span className="min-w-0">
              <span className="font-medium text-foreground">{copy.toolLabel}</span>
              <span className="text-muted-foreground"> {"\u2014"} </span>
              <span className="font-mono break-all text-muted-foreground">{copy.path}</span>
            </span>
            <span className={cn(copy.sameAsManaged === true ? "text-muted-foreground" : "text-stale")}>
              {copy.sameAsManaged === true
                ? t("importReview.managed.same")
                : copy.sameAsManaged === false
                  ? t("importReview.managed.differs")
                  : t("importReview.managed.unknown")}
              {copy.modifiedAt !== null
                ? ` \u00b7 ${t("importReview.copy.modified", { when: formatRelativeTime(copy.modifiedAt, t) })}`
                : null}
            </span>
            {copy.fate ? (
              <span className="text-muted-foreground">{t(COPY_FATE_KEY[copy.fate])}</span>
            ) : null}
          </li>
        ))}
      </ul>
      {row.lastFailure ? (
        <p className="m-0 text-xs text-failed">{t("importReview.lastFailure", { reason: row.lastFailure })}</p>
      ) : null}
      <fieldset className="m-0 flex min-w-0 flex-col gap-0.5 border-0 p-0" role="radiogroup" aria-labelledby={legendId}>
        <legend id={legendId} className="sr-only">
          {t("importReview.managed.legend", { name: row.name })}
        </legend>
        {choice("leave", t("importReview.managed.leave"), t("importReview.managed.leaveHint"))}
        {choice(
          "replace",
          t("importReview.managed.replace", { count: row.copies.length, harnesses }),
          row.differs > 0
            ? t("importReview.managed.replaceAsks", { count: row.differs })
            : t("importReview.managed.replaceLinks"),
        )}
      </fieldset>
    </li>
  );
}

/**
 * What the import does beyond the rows, said before the button (D13, D49):
 * auto-deploy on — the identical originals become links and the imported
 * skills deploy to the selected harnesses; off — the per-run checkbox about
 * the identical originals, and the skills are added to the Library only.
 */
function PolicyLine({ review, model, running, t }: { review: ImportReviewView; model: ImportReviewModel; running: boolean; t: TFunction }) {
  const { policy, selection } = model;
  const harnesses = policy.identicalHarnesses.join(listSeparator(t));
  if (selection.imports === 0) return null;
  return (
    <div data-slot="import-policy" className="flex shrink-0 flex-col gap-1.5 border-t border-border bg-card px-5 py-3 text-sm">
      {policy.autoDeploy ? (
        <p className="m-0 text-foreground">
          {policy.identical > 0 ? `${t("importReview.policy.linked", { harnesses })} ` : null}
          {policy.deployHarnesses.length > 0
            ? t("importReview.policy.deployed", { harnesses: policy.deployHarnesses.join(listSeparator(t)) })
            : t("importReview.policy.deployedNowhere")}
        </p>
      ) : (
        <>
          {policy.identical > 0 ? (
            <Field className="flex-row items-start gap-2">
              <Checkbox
                className="mt-0.5"
                checked={policy.removeOriginals}
                onCheckedChange={(next) => review.setRemoveOriginals(next)}
                disabled={running}
              />
              <div className="flex min-w-0 flex-col gap-0.5">
                <FieldLabel>{t("importReview.policy.removeOriginals", { count: policy.identical, harnesses })}</FieldLabel>
                <FieldDescription>
                  {policy.removeOriginals ? t("importReview.policy.removeOriginalsOn") : t("importReview.policy.originalsStay")}
                </FieldDescription>
              </div>
            </Field>
          ) : null}
          <p className="m-0 text-muted-foreground">{t("importReview.policy.libraryOnly")}</p>
        </>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Result
// ---------------------------------------------------------------------------

function ResultStep({
  review,
  result,
  running,
  blocked,
  headingRef,
  t,
}: StepProps & {
  result: NonNullable<ImportReviewView["result"]>;
  headingRef: React.RefObject<HTMLHeadingElement | null>;
}) {
  const headingId = useId();
  const failed = result.rows.filter((row) => row.retryable).map((row) => row.name);
  return (
    <>
      <DialogBody>
        <h3
          ref={headingRef}
          id={headingId}
          tabIndex={-1}
          className={cn("m-0 w-fit rounded-sm text-sm font-semibold text-foreground tabular-nums", focusRing)}
        >
          {result.headline}
        </h3>
        <ul aria-labelledby={headingId} className="m-0 flex list-none flex-col rounded-md border border-border p-0">
          {result.rows.map((row) => (
            <ResultItem
              key={row.name}
              row={row}
              running={running}
              blocked={blocked}
              onRetry={() => void review.retry([row.name])}
              t={t}
            />
          ))}
        </ul>
      </DialogBody>
      <DialogFooter>
        {failed.length > 1 ? (
          <Button
            tone="accent"
            onClick={() => void review.retry(failed)}
            disabled={running || blocked !== null}
            focusableWhenDisabled
            title={blocked || undefined}
          >
            {t("importReview.result.retryFailed", { count: failed.length })}
          </Button>
        ) : null}
        <Button variant="solid" tone="accent" onClick={review.close} disabled={running} focusableWhenDisabled>
          {t("close")}
        </Button>
      </DialogFooter>
    </>
  );
}

const RESULT_ICON = {
  imported: CircleCheck,
  replaced: CircleCheck,
  kept: CircleMinus,
  skipped: CircleMinus,
  failed: CircleX,
} as const;

function ResultItem({
  row,
  running,
  blocked,
  onRetry,
  t,
}: {
  row: ImportResultRow;
  running: boolean;
  blocked: string | null;
  onRetry: () => void;
  t: TFunction;
}) {
  const Icon = RESULT_ICON[row.status];
  const tone =
    row.status === "failed"
      ? "text-failed"
      : row.status === "skipped" || row.status === "kept"
        ? "text-muted-foreground"
        : row.attention
          ? "text-stale"
          : "text-ok";
  return (
    <li data-import-result={row.status} className="flex items-start gap-3 border-b border-border px-3 py-2 last:border-b-0">
      <Icon aria-hidden="true" className={cn("mt-0.5 size-4 shrink-0", tone)} />
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <div className="flex min-w-0 items-baseline gap-2">
          <span className="truncate text-sm font-medium text-foreground" title={row.name}>
            {row.name}
          </span>
          <span className={cn("shrink-0 text-xs", tone)}>{t(`importReview.result.status.${row.status}`)}</span>
        </div>
        {row.lines.map((line, index) => (
          <p key={index} className="m-0 text-xs text-muted-foreground">
            {line}
          </p>
        ))}
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
          aria-label={t("importReview.result.retryNamed", { name: row.name })}
        >
          {t("importReview.result.retry")}
        </Button>
      ) : null}
    </li>
  );
}
