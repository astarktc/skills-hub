import { memo, useId } from 'react'
import { TriangleAlert } from 'lucide-react'
import type { TFunction } from 'i18next'
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
} from '@/components/ui/dialog'
import { mainLandmarkFocus } from '@/components/ui/focus-fallback'

type DeleteModalProps = {
  open: boolean
  loading: boolean
  skillName: string | null
  onRequestClose: () => void
  onConfirm: () => void
  /**
   * Where focus goes when the dialog closes and its opener (the row) is gone:
   * the lifecycle's post-Delete target (D21). Falls back to the main landmark.
   */
  fallbackFocus?: () => HTMLElement | null
  t: TFunction
}

const DeleteModal = ({
  open,
  loading,
  skillName,
  onRequestClose,
  onConfirm,
  fallbackFocus,
  t,
}: DeleteModalProps) => {
  const titleId = useId()
  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) onRequestClose()
      }}
    >
      <DialogContent aria-labelledby={titleId} fallbackFocus={() => fallbackFocus?.() ?? mainLandmarkFocus()}>
        <DialogBody>
          <div className="delete-title" id={titleId}>
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
        </DialogBody>
        <DialogFooter className="justify-between">
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
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

export default memo(DeleteModal)
