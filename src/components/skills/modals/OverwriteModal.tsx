import { memo } from 'react'
import type { TFunction } from 'i18next'
import { ConfirmDialog } from '@/components/ui/confirm-dialog'
import type { OverwritePending } from '../../../hooks/useOverwriteConfirmation'

type OverwriteModalProps = {
  /** The one pending overwrite confirmation, from useOverwriteConfirmation. */
  pending: OverwritePending | null
  onCancel: () => void
  t: TFunction
}

/**
 * Raised mid-action by the sync seam, while the progress overlay runs: it
 * opens above the overlay (the most recently opened layer is on top) and its
 * buttons are never disabled on `loading` — the action is waiting on
 * exactly this answer.
 */
const OverwriteModal = ({ pending, onCancel, t }: OverwriteModalProps) => {
  return (
    <ConfirmDialog
      open={Boolean(pending)}
      title={t('overwrite.title')}
      confirmLabel={t('overwrite.confirm')}
      cancelLabel={t('overwrite.cancel')}
      onConfirm={() => pending?.resolve(true)}
      onCancel={onCancel}
      confirmDisabled={!pending}
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
    </ConfirmDialog>
  )
}

export default memo(OverwriteModal)
