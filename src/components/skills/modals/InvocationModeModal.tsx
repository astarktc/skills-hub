import { useId, useState } from "react";
import type { TFunction } from "i18next";
import Modal from "../../shared/Modal";
import type { InvocationMode, ManagedSkill } from "../types";
import { INVOCATION_MODES, INVOCATION_LABEL_KEY, INVOCATION_TOOLTIP_KEY } from "../../../lib/skillPresentation";

type Props = {
  skill: ManagedSkill;
  loading: boolean;
  onRequestClose: () => void;
  onConfirm: (mode: InvocationMode | null) => void;
  t: TFunction;
};

export default function InvocationModeModal({ skill, loading, onRequestClose, onConfirm, t }: Props) {
  const current = skill.invocation_override?.mode ?? null;
  const [selection, setSelection] = useState<InvocationMode | null>(current);
  const id = useId();
  const base = t(INVOCATION_LABEL_KEY[skill.invocation_override?.base_mode ?? skill.invocation_mode]);
  // Saving the same override is meaningful when it resolves a conflict.
  const unchanged = selection === current && !skill.invocation_override?.conflict;
  return (
    <Modal
      open
      title={t("invocationEdit.title", { name: skill.name })}
      onRequestClose={onRequestClose}
      closeDisabled={loading}
      footer={<>
        <button type="button" className="btn btn-secondary" disabled={loading} onClick={onRequestClose}>{t("cancel")}</button>
        <button type="submit" form={id} className="btn btn-primary" disabled={loading || unchanged}>{t("invocationEdit.save")}</button>
      </>}
    >
      <form id={id} onSubmit={(event) => { event.preventDefault(); if (!loading && !unchanged) onConfirm(selection); }}>
        {skill.invocation_override?.conflict ? (
          <p className="invocation-edit-warning" role="status">{t("invocationEdit.conflictBanner", { mode: base })}</p>
        ) : null}
        <fieldset className="invocation-edit-options" disabled={loading} aria-describedby={`${id}-note`}>
          <legend className="label">{t("invocationEdit.choose")}</legend>
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
    </Modal>
  );
}
