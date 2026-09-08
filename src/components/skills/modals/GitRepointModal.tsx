import { useId, useState } from "react";
import type { TFunction } from "i18next";
import Modal from "../../shared/Modal";

type GitRepointModalProps = {
  skillName: string;
  loading: boolean;
  onRequestClose: () => void;
  onConfirm: (url: string) => void;
  t: TFunction;
};

export default function GitRepointModal({
  skillName, loading, onRequestClose, onConfirm, t,
}: GitRepointModalProps) {
  const [url, setUrl] = useState("");
  const id = useId();
  return (
    <Modal
      open
      title={t("gitRepoint.title", { name: skillName })}
      onRequestClose={onRequestClose}
      closeDisabled={loading}
      footer={
        <>
          <button type="button" className="btn btn-secondary" onClick={onRequestClose} disabled={loading}>
            {t("cancel")}
          </button>
          <button type="submit" form={id} className="btn btn-primary" disabled={loading || !url.trim()}>
            {t("gitRepoint.confirm")}
          </button>
        </>
      }
    >
      <form id={id} onSubmit={(event) => {
        event.preventDefault();
        if (!loading && url.trim()) onConfirm(url.trim());
      }}>
        <div className="form-group">
          <label className="label" htmlFor={`${id}-url`}>{t("gitRepoint.urlLabel")}</label>
          <input
            id={`${id}-url`}
            className="input"
            type="url"
            required
            value={url}
            onChange={(event) => setUrl(event.target.value)}
            placeholder={t("gitRepoint.urlPlaceholder")}
            aria-describedby={`${id}-help`}
            disabled={loading}
            autoComplete="off"
            spellCheck={false}
          />
          <div id={`${id}-help`} className="helper-text">{t("gitRepoint.help")}</div>
        </div>
      </form>
    </Modal>
  );
}
