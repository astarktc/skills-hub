import { useId, useState } from "react";
import type { TFunction } from "i18next";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
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
          <form id={id} onSubmit={(event) => { event.preventDefault(); if (!loading && !unchanged) onConfirm(selection); }}>
            {conflict ? (
              <>
                <p className="invocation-edit-warning" role="status">{t("invocationEdit.conflictBanner", { mode: base })}</p>
                {/* The plain choice (ticket 12): keep the Edit, or take the skill's own setting. */}
                <div role="group" aria-label={t("invocationConflict.legend")} className="mb-3 flex flex-wrap gap-2">
                  <Button type="button" variant="solid" tone="accent" disabled={loading} onClick={() => onConfirm(conflict.mode)}>
                    {t("invocationConflict.keepYours", { mode: t(INVOCATION_LABEL_KEY[conflict.mode]) })}
                  </Button>
                  <Button type="button" disabled={loading} onClick={() => onConfirm(null)}>
                    {t("invocationConflict.useSkills", { mode: base })}
                  </Button>
                </div>
              </>
            ) : null}
            <fieldset className="invocation-edit-options" disabled={loading} aria-describedby={`${id}-note`}>
              <legend className="label">{conflict ? t("invocationConflict.other") : t("invocationEdit.choose")}</legend>
              <label className="invocation-edit-option">
                <input type="radio" name={`${id}-mode`} checked={selection === null} onChange={() => setSelection(null)} />
                <span>{t("invocationEdit.follow", { mode: base })}</span>
              </label>
              {INVOCATION_MODES.map((mode) => (
                <label key={mode} className="invocation-edit-option">
                  <input type="radio" name={`${id}-mode`} checked={selection === mode} onChange={() => setSelection(mode)} />
                  <span><span>{t(INVOCATION_LABEL_KEY[mode])}</span><span className="helper-text">{t(INVOCATION_TOOLTIP_KEY[mode])}</span></span>
                </label>
              ))}
            </fieldset>
            <p id={`${id}-note`} className="helper-text">{t("invocationEdit.note")}</p>
          </form>
        </DialogBody>
        <DialogFooter>
          <button type="button" className="btn btn-secondary" disabled={loading} onClick={onRequestClose}>{t("cancel")}</button>
          <button type="submit" form={id} className="btn btn-primary" disabled={loading || unchanged}>{t("invocationEdit.save")}</button>
        </DialogFooter>
    </>
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
  fallbackFocus?: () => HTMLElement | null;
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
