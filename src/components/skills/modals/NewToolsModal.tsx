import { memo } from 'react'
import type { TFunction } from 'i18next'
import Modal from '../../shared/Modal'

type NewToolsModalProps = {
  open: boolean
  loading: boolean
  toolsLabelText: string
  onLater: () => void
  onSyncAll: () => void
  t: TFunction
}

const NewToolsModal = ({
  open,
  loading,
  toolsLabelText,
  onLater,
  onSyncAll,
  t,
}: NewToolsModalProps) => {
  return (
    <Modal
      open={open}
      title={t('newToolsTitle')}
      onRequestClose={onLater}
      showCloseButton={false}
      footer={
        <>
          <button className="btn btn-secondary" onClick={onLater} disabled={loading}>
            {t('later')}
          </button>
          <button className="btn btn-primary" onClick={onSyncAll} disabled={loading}>
            {t('syncAll')}
          </button>
        </>
      }
    >
      {t('newToolsBody', {
        tools: toolsLabelText,
      })}
    </Modal>
  )
}

export default memo(NewToolsModal)
