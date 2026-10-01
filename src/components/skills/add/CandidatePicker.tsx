import { useId, type Ref } from "react";
import type { TFunction } from "i18next";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldDescription, FieldLabel, FieldLegend, FieldSet, Input } from "@/components/ui/field";
import { focusRing } from "@/components/ui/styles";
import { cn } from "@/lib/utils";
import type { PickerCandidate } from "../../../lib/addFlow";

type Common = {
  /** Every candidate of the listing, and the ones the search leaves on screen. */
  candidates: readonly PickerCandidate[];
  visible: readonly PickerCandidate[];
  query: string;
  onQueryChange: (query: string) => void;
  /** The group's name (the legend), e.g. "Skills in acme/collide-kit". */
  legend: string;
  /** What the search reports under itself ("3 of 4 checked · 1 hidden by filter"). */
  status: string;
  /** An action runs: the controls wait (they keep their place). */
  disabled?: boolean;
  /** The search box, for the step's first focus. */
  searchRef?: Ref<HTMLInputElement>;
  t: TFunction;
};

type CandidatePickerProps = Common &
  (
    | {
        /** Several may be picked (Add): checkboxes, Select all over what is shown. */
        mode: "multiple";
        selected: Readonly<Record<string, boolean>>;
        onToggle: (id: string, checked: boolean) => void;
        onToggleAll: (checked: boolean) => void;
      }
    | {
        /** Exactly one is picked (Re-point's multi-skill repository): radios. */
        mode: "single";
        chosen: string | null;
        onChoose: (id: string) => void;
      }
  );

/**
 * The one candidate picker (ticket 16): a search box filtering the list
 * (#47), then the candidates — each control named by the candidate's name
 * and described by its path, description and, for one that cannot be
 * picked, the reason in words. `multiple` (Add) checks candidates, and only
 * checked **and visible** ones install; `single` (Re-point) chooses one.
 * The search appears once there is more than one candidate.
 */
export function CandidatePicker(props: CandidatePickerProps) {
  const { candidates, visible, query, onQueryChange, legend, status, disabled = false, searchRef, t } = props;
  const baseId = useId();
  const statusId = `${baseId}-status`;
  const searchable = candidates.length > 1;
  const selectableVisible = visible.filter((candidate) => candidate.unavailable === null);
  const allState =
    props.mode === "multiple" && selectableVisible.length > 0
      ? selectableVisible.every((candidate) => props.selected[candidate.id])
        ? "all"
        : selectableVisible.some((candidate) => props.selected[candidate.id])
          ? "some"
          : "none"
      : "none";

  return (
    <FieldSet className="gap-3" role={props.mode === "single" ? "radiogroup" : undefined}>
      <FieldLegend className="sr-only">{legend}</FieldLegend>
      {searchable ? (
        <div className="flex flex-col gap-1.5">
          <Input
            ref={searchRef}
            data-candidate-search=""
            type="search"
            value={query}
            onChange={(event) => onQueryChange(event.target.value)}
            placeholder={t("add.picker.searchPlaceholder")}
            aria-label={t("add.picker.searchLabel")}
            aria-describedby={statusId}
            autoComplete="off"
            spellCheck={false}
          />
        </div>
      ) : null}
      <div className="flex min-h-6 items-center justify-between gap-3">
        {props.mode === "multiple" && selectableVisible.length > 1 ? (
          <Field className="flex-row items-center gap-2">
            <Checkbox
              checked={allState === "all"}
              indeterminate={allState === "some"}
              onCheckedChange={(checked) => props.onToggleAll(checked)}
              disabled={disabled}
            />
            <FieldLabel>
              {query.trim() ? t("add.picker.selectShown") : t("add.picker.selectAll")}
            </FieldLabel>
          </Field>
        ) : (
          <span />
        )}
        <p id={statusId} role="status" className="m-0 text-xs text-muted-foreground tabular-nums">
          {status}
        </p>
      </div>
      {visible.length === 0 ? (
        <p className="m-0 rounded-md border border-dashed border-border px-3 py-6 text-center text-sm text-muted-foreground">
          {t("add.picker.noMatch", { query: query.trim() })}
        </p>
      ) : (
        <ul className="m-0 flex list-none flex-col rounded-md border border-border p-0">
          {visible.map((candidate) => (
            <li key={candidate.id} className="border-b border-border last:border-b-0">
              {props.mode === "multiple" ? (
                <CheckRow
                  candidate={candidate}
                  checked={Boolean(props.selected[candidate.id])}
                  onToggle={props.onToggle}
                  disabled={disabled}
                  t={t}
                />
              ) : (
                <RadioRow
                  candidate={candidate}
                  name={`${baseId}-choice`}
                  checked={props.chosen === candidate.id}
                  onChoose={props.onChoose}
                  disabled={disabled}
                />
              )}
            </li>
          ))}
        </ul>
      )}
    </FieldSet>
  );
}

/** Path, description and (when it cannot be picked) the reason — the control's description. */
function CandidateFacts({ candidate, t }: { candidate: PickerCandidate; t?: TFunction }) {
  return (
    <>
      {candidate.description ? (
        <span className="line-clamp-2 text-xs text-muted-foreground">{candidate.description}</span>
      ) : null}
      <span className="truncate font-mono text-xs text-faint" title={candidate.subpath}>
        {candidate.subpath}
      </span>
      {candidate.unavailable !== null && t ? (
        <span className="text-xs text-stale">{t("add.picker.unavailable", { reason: candidate.unavailable })}</span>
      ) : null}
    </>
  );
}

function CheckRow({
  candidate,
  checked,
  onToggle,
  disabled,
  t,
}: {
  candidate: PickerCandidate;
  checked: boolean;
  onToggle: (id: string, checked: boolean) => void;
  disabled: boolean;
  t: TFunction;
}) {
  const unavailable = candidate.unavailable !== null;
  return (
    <Field className="flex-row items-start gap-3 px-3 py-2" disabled={unavailable}>
      <Checkbox
        className="mt-0.5"
        checked={checked && !unavailable}
        onCheckedChange={(next) => onToggle(candidate.id, next)}
        disabled={disabled || unavailable}
        data-candidate-id={candidate.id}
      />
      <div className="flex min-w-0 flex-col gap-0.5">
        <FieldLabel>{candidate.name}</FieldLabel>
        <FieldDescription className="flex flex-col gap-0.5">
          <CandidateFacts candidate={candidate} t={t} />
        </FieldDescription>
      </div>
    </Field>
  );
}

function RadioRow({
  candidate,
  name,
  checked,
  onChoose,
  disabled,
}: {
  candidate: PickerCandidate;
  name: string;
  checked: boolean;
  onChoose: (id: string) => void;
  disabled: boolean;
}) {
  const id = useId();
  return (
    <label htmlFor={id} className="flex cursor-default items-start gap-3 px-3 py-2 hover:bg-raised">
      <input
        id={id}
        type="radio"
        name={name}
        value={candidate.id}
        checked={checked}
        onChange={() => onChoose(candidate.id)}
        disabled={disabled || candidate.unavailable !== null}
        aria-describedby={`${id}-facts`}
        className={cn("mt-1 size-4 shrink-0 accent-primary", focusRing)}
      />
      <span className="flex min-w-0 flex-col gap-0.5">
        <span className="text-sm font-medium text-foreground">{candidate.name}</span>
        <span id={`${id}-facts`} className="flex min-w-0 flex-col gap-0.5">
          <CandidateFacts candidate={candidate} />
        </span>
      </span>
    </label>
  );
}
