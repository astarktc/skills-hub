import { useId, useState } from "react";
import type { TFunction } from "i18next";
import { TriangleAlert } from "lucide-react";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { FocusFallback } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { FieldLegend, FieldSet } from "@/components/ui/field";
import { focusRing } from "@/components/ui/styles";
import { cn } from "@/lib/utils";
import type { InvocationMode, ManagedSkill } from "../types";
import { INVOCATION_MODES, INVOCATION_LABEL_KEY, INVOCATION_TOOLTIP_KEY } from "../../../lib/skillPresentation";

type Props = {
  skill: ManagedSkill;
  loading: boolean;
  onRequestClose: () => void;
  onConfirm: (mode: InvocationMode | null) => void;
  t: TFunction;
};

function InvocationBody({ skill, loading, onRequestClose, onConfirm, t }: Props) {
  const current = skill.invocation_override?.mode ?? null;
  const [selection, setSelection] = useState<InvocationMode | null>(current);
  // The plain choice pressed: it shows the pending action and keeps focus
  // while it runs (a disabled control would drop focus to <body>).
  const [chosen, setChosen] = useState<"keep" | "use" | null>(null);
  const id = useId();
  const base = t(INVOCATION_LABEL_KEY[skill.invocation_override?.base_mode ?? skill.invocation_mode]);
  // Saving the same override is meaningful when it resolves a conflict.
  const unchanged = selection === current && !skill.invocation_override?.conflict;
  const conflict = skill.invocation_override?.conflict ? skill.invocation_override : null;
  return (
    <>
        <DialogHeader>
          <DialogTitle>{t("invocationEdit.title", { name: skill.name })}</DialogTitle>
        </DialogHeader>
        <DialogBody>
          <form
            id={id}
            className="flex flex-col gap-3"
            onSubmit={(event) => { event.preventDefault(); if (!loading && !unchanged) onConfirm(selection); }}
          >
            {conflict ? (
              <>
                <p className="m-0 flex items-start gap-2 text-stale" role="status">
                  <TriangleAlert aria-hidden="true" className="mt-0.5 size-4 shrink-0" />
                  <span>{t("invocationEdit.conflictBanner", { mode: base })}</span>
                </p>
                {/* The plain choice (ticket 12): keep the Edit, or take the skill's own setting. */}
                <div role="group" aria-label={t("invocationConflict.legend")} className="flex flex-wrap gap-2">
                  <Button
                    type="button"
                    variant="solid"
                    tone="accent"
                    disabled={loading}
                    pending={loading && chosen === "keep"}
                    onClick={() => {
                      setChosen("keep");
                      onConfirm(conflict.mode);
                    }}
                  >
                    {t("invocationConflict.keepYours", { mode: t(INVOCATION_LABEL_KEY[conflict.mode]) })}
                  </Button>
                  <Button
                    type="button"
                    disabled={loading}
                    pending={loading && chosen === "use"}
                    onClick={() => {
                      setChosen("use");
                      onConfirm(null);
                    }}
                  >
                    {t("invocationConflict.useSkills", { mode: base })}
                  </Button>
                </div>
              </>
            ) : null}
            <FieldSet disabled={loading} aria-describedby={`${id}-note`} className="gap-0.5">
              <FieldLegend>{conflict ? t("invocationConflict.other") : t("invocationEdit.choose")}</FieldLegend>
              <InvocationOption
                id={`${id}-follow`}
                name={`${id}-mode`}
                checked={selection === null}
                onChoose={() => setSelection(null)}
                label={t("invocationEdit.follow", { mode: base })}
              />
              {INVOCATION_MODES.map((mode) => (
                <InvocationOption
                  key={mode}
                  id={`${id}-${mode}`}
                  name={`${id}-mode`}
                  checked={selection === mode}
                  onChoose={() => setSelection(mode)}
                  label={t(INVOCATION_LABEL_KEY[mode])}
                  hint={t(INVOCATION_TOOLTIP_KEY[mode])}
                />
              ))}
            </FieldSet>
            <p id={`${id}-note`} className="m-0 text-xs text-muted-foreground">{t("invocationEdit.note")}</p>
          </form>
        </DialogBody>
        <DialogFooter>
          <Button type="button" disabled={loading} onClick={onRequestClose}>{t("cancel")}</Button>
          <Button type="submit" form={id} variant="solid" tone="accent" disabled={loading || unchanged}>
            {t("invocationEdit.save")}
          </Button>
        </DialogFooter>
    </>
  );
}

/**
 * One invocation choice: a native radio (the group's one Tab stop; arrows
 * move and select) named by its label and described by its hint — the
 * Import review's described-radio row.
 */
function InvocationOption({
  id,
  name,
  checked,
  onChoose,
  label,
  hint,
}: {
  id: string;
  name: string;
  checked: boolean;
  onChoose: () => void;
  label: string;
  hint?: string;
}) {
  return (
    <label htmlFor={id} className="flex cursor-default items-start gap-2.5 rounded-md px-2 py-1.5 hover:bg-raised">
      <input
        id={id}
        type="radio"
        name={name}
        checked={checked}
        onChange={onChoose}
        aria-labelledby={`${id}-label`}
        aria-describedby={hint ? `${id}-hint` : undefined}
        className={cn("mt-0.5 size-4 shrink-0 accent-primary", focusRing)}
      />
      <span className="flex min-w-0 flex-col gap-0.5">
        <span id={`${id}-label`} className="text-foreground">{label}</span>
        {hint ? <span id={`${id}-hint`} className="text-xs text-muted-foreground">{hint}</span> : null}
      </span>
    </label>
  );
}

/**
 * The Invocation dialog. Its shell stays mounted (one popup identity, so
 * closing returns focus to the opener — the row's pencil, the inspector's
 * Edit / Resolve); the body is keyed per skill.
 */
export default function InvocationModeModal({
  skill,
  fallbackFocus,
  ...rest
}: Omit<Props, "skill"> & {
  skill: ManagedSkill | null;
  /** Where focus goes when the opener is gone after closing (D21: the focused row). */
  fallbackFocus?: FocusFallback;
}) {
  return (
    <Dialog
      open={skill !== null}
      dismissible={!rest.loading}
      onOpenChange={(next) => {
        if (!next) rest.onRequestClose();
      }}
    >
      <DialogContent fallbackFocus={fallbackFocus}>{skill ? <InvocationBody key={skill.id} skill={skill} {...rest} /> : null}</DialogContent>
    </Dialog>
  );
}
