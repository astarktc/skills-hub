import { memo } from 'react'
import { TriangleAlert } from 'lucide-react'
import type { TFunction } from 'i18next'
import Modal from '../../shared/Modal'

type DeleteModalProps = {
  open: boolean
  loading: boolean
  skillName: string | null
  onRequestClose: () => void
  onConfirm: () => void
  t: TFunction
}

const DeleteModal = ({
  open,
  loading,
  skillName,
  onRequestClose,
  onConfirm,
  t,
}: DeleteModalProps) => {
  return (
    <Modal
      open={open}
      onRequestClose={onRequestClose}
      aria-label={t('deleteTitle')}
      className="modal-delete"
      bodyClassName="delete-body"
      footerClassName="space-between"
      footer={
        <>
          <button
            className="btn btn-secondary"
            onClick={onRequestClose}
            disabled={loading}
          >
            {t('cancel')}
          </button>
          <button
            className="btn btn-danger-solid"
            onClick={onConfirm}
            disabled={loading}
          >
            {t('delete.confirmButton')}
          </button>
        </>
      }
    >
          <div className="delete-title">
            <TriangleAlert size={20} />
            {t('deleteTitle')}
          </div>
          <div className="delete-desc">
            {skillName ? (
              <>
                {t('delete.confirmPrefix')}
                <strong>{skillName}</strong>
                {t('delete.confirmSuffix')}
              </>
            ) : (
              t('deleteBody')
            )}
          </div>
          <div className="delete-warning">
            <ul>
              <li>{t('delete.warningRemoveFromTools')}</li>
              <li>{t('delete.warningDeleteFromHub')}</li>
            </ul>
          </div>
    </Modal>
  )
}

export default memo(DeleteModal)
