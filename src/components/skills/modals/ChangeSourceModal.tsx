import { useId, useState } from "react";
import type { TFunction } from "i18next";
import Modal from "../../shared/Modal";
import type { RepointTarget } from "../types";
import type { RepointKind } from "../../../lib/skillPresentation";

type ChangeSourceModalProps = {
  skillName: string;
  /** The kind the picker opens on (the skill's own, or `local` for a moved folder). */
  preselect: RepointKind;
  loading: boolean;
  onRequestClose: () => void;
  /** The folder dialog; resolves null when the operator cancels it. */
  onPickFolder: () => Promise<string | null>;
  onConfirm: (target: RepointTarget) => void;
  t: TFunction;
};

const KINDS: { kind: RepointKind; labelKey: string }[] = [
  { kind: "git", labelKey: "changeSource.kindGit" },
  { kind: "local", labelKey: "changeSource.kindLocal" },
];

/**
 * Change source: point a managed skill at a GitHub URL or a local folder.
 * Each arm keeps its own input, so switching kinds loses nothing typed.
 */
export default function ChangeSourceModal({
  skillName, preselect, loading, onRequestClose, onPickFolder, onConfirm, t,
}: ChangeSourceModalProps) {
  const [kind, setKind] = useState<RepointKind>(preselect);
  const [url, setUrl] = useState("");
  const [path, setPath] = useState("");
  const id = useId();
  const value = (kind === "git" ? url : path).trim();
  const target: RepointTarget | null = !value
    ? null
    : kind === "git" ? { kind: "git", url: value } : { kind: "local", path: value };

  const handleChooseFolder = async () => {
    const picked = await onPickFolder();
    if (picked) setPath(picked);
  };

  return (
    <Modal
      open
      title={t("changeSource.title", { name: skillName })}
      onRequestClose={onRequestClose}
      closeDisabled={loading}
      footer={
        <>
          <button type="button" className="btn btn-secondary" onClick={onRequestClose} disabled={loading}>
            {t("cancel")}
          </button>
          <button type="submit" form={id} className="btn btn-primary" disabled={loading || !target}>
            {t("changeSource.confirm")}
          </button>
        </>
      }
    >
      <form id={id} onSubmit={(event) => {
        event.preventDefault();
        if (!loading && target) onConfirm(target);
      }}>
        <fieldset className="change-source-kind" disabled={loading}>
          <legend className="label">{t("changeSource.kindLabel")}</legend>
          <div className="tabs">
            {KINDS.map((option) => (
              <label
                key={option.kind}
                className={`tab-item${kind === option.kind ? " active" : ""}`}
              >
                <input
                  className="sr-only"
                  type="radio"
                  name={`${id}-kind`}
                  value={option.kind}
                  checked={kind === option.kind}
                  onChange={() => setKind(option.kind)}
                />
                {t(option.labelKey)}
              </label>
            ))}
          </div>
        </fieldset>
        {kind === "git" ? (
          <div className="form-group">
            <label className="label" htmlFor={`${id}-url`}>{t("changeSource.urlLabel")}</label>
            <input
              id={`${id}-url`}
              className="input"
              type="text"
              inputMode="url"
              required
              value={url}
              onChange={(event) => setUrl(event.target.value)}
              placeholder={t("changeSource.urlPlaceholder")}
              aria-describedby={`${id}-help`}
              disabled={loading}
              autoComplete="off"
              spellCheck={false}
            />
            <div id={`${id}-help`} className="helper-text">{t("changeSource.urlHelp")}</div>
          </div>
        ) : (
          <div className="form-group">
            <label className="label" htmlFor={`${id}-path`}>{t("changeSource.pathLabel")}</label>
            <div className="input-row">
              <input
                id={`${id}-path`}
                className="input"
                type="text"
                required
                value={path}
                onChange={(event) => setPath(event.target.value)}
                placeholder={t("changeSource.pathPlaceholder")}
                aria-describedby={`${id}-help`}
                disabled={loading}
                autoComplete="off"
                spellCheck={false}
              />
              <button
                type="button"
                className="btn btn-secondary input-action"
                onClick={() => void handleChooseFolder()}
                disabled={loading}
              >
                {t("changeSource.chooseFolder")}
              </button>
            </div>
            <div id={`${id}-help`} className="helper-text">{t("changeSource.pathHelp")}</div>
          </div>
        )}
      </form>
    </Modal>
  );
}
