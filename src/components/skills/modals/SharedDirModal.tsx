import { memo } from 'react'
import type { TFunction } from 'i18next'
import Modal from '../../shared/Modal'
import type { SharedDirPending } from '../../../hooks/useSharedDirConfirmation'

type SharedDirModalProps = {
  /** The one pending shared-dir confirmation, from useSharedDirConfirmation. */
  pending: SharedDirPending | null
  loading: boolean
  onCancel: () => void
  t: TFunction
}

const SharedDirModal = ({ pending, loading, onCancel, t }: SharedDirModalProps) => {
  return (
    <Modal
      open={Boolean(pending)}
      title={t('sharedDir.title')}
      onRequestClose={onCancel}
      showCloseButton={false}
      footer={
        <>
          <button className="btn btn-secondary" onClick={onCancel} disabled={loading}>
            {t('sharedDir.cancel')}
          </button>
          <button
            className="btn btn-primary"
            onClick={() => pending?.resolve(true)}
            disabled={loading || !pending}
          >
            {t('sharedDir.confirm')}
          </button>
        </>
      }
    >
      {pending
        ? t('sharedDir.body', {
            tool: pending.toolLabel,
            others: pending.labels.join(t('common.listSeparator')),
          })
        : null}
    </Modal>
  )
}

export default memo(SharedDirModal)
