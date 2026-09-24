import { memo } from 'react'
import type { TFunction } from 'i18next'
import Modal from '../../shared/Modal'
import type { OverwritePending } from '../../../hooks/useOverwriteConfirmation'

type OverwriteModalProps = {
  /** The one pending overwrite confirmation, from useOverwriteConfirmation. */
  pending: OverwritePending | null
  onCancel: () => void
  t: TFunction
}

/**
 * Raised mid-action by the sync seam, under the loading overlay: the
 * backdrop layers above the overlay and the buttons are never disabled on
 * `loading` — the action is waiting on exactly this answer.
 */
const OverwriteModal = ({ pending, onCancel, t }: OverwriteModalProps) => {
  return (
    <Modal
      open={Boolean(pending)}
      title={t('overwrite.title')}
      onRequestClose={onCancel}
      showCloseButton={false}
      backdropClassName="modal-backdrop-over-loading"
      footer={
        <>
          <button className="btn btn-secondary" onClick={onCancel}>
            {t('overwrite.cancel')}
          </button>
          <button
            className="btn btn-primary"
            onClick={() => pending?.resolve(true)}
            disabled={!pending}
          >
            {t('overwrite.confirm')}
          </button>
        </>
      }
    >
      {pending ? (
        <>
          <p>{t('overwrite.body', { count: pending.rows.length })}</p>
          <ul className="overwrite-list">
            {pending.rows.map((row) => (
              <li key={`${row.skillId}\n${row.toolKey}`} className="overwrite-row">
                <span className="overwrite-pair">
                  {row.skillName} → {row.toolLabel}
                </span>
                <span className="overwrite-path">{row.path}</span>
              </li>
            ))}
          </ul>
        </>
      ) : null}
    </Modal>
  )
}

export default memo(OverwriteModal)
