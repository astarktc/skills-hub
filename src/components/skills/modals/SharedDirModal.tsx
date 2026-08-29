import { memo } from 'react'
import type { TFunction } from 'i18next'
import Modal from '../../shared/Modal'

type SharedDirModalProps = {
  open: boolean
  loading: boolean
  toolLabel: string
  otherLabels: string
  onRequestClose: () => void
  onConfirm: () => void
  t: TFunction
}

const SharedDirModal = ({
  open,
  loading,
  toolLabel,
  otherLabels,
  onRequestClose,
  onConfirm,
  t,
}: SharedDirModalProps) => {
  return (
    <Modal
      open={open}
      title={t('appName')}
      onRequestClose={onRequestClose}
      showCloseButton={false}
      footer={
        <>
          <button
            className="btn btn-secondary"
            onClick={onRequestClose}
            disabled={loading}
          >
            {t('cancel')}
          </button>
          <button className="btn btn-primary" onClick={onConfirm} disabled={loading}>
            {t('confirm')}
          </button>
        </>
      }
    >
      {t('sharedDirConfirm', {
        tool: toolLabel,
        others: otherLabels,
      })}
    </Modal>
  )
}

export default memo(SharedDirModal)
