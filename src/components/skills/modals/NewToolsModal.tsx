import { memo } from 'react'
import type { TFunction } from 'i18next'
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'

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
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) onLater()
      }}
    >
      <DialogContent showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>{t('newToolsTitle')}</DialogTitle>
        </DialogHeader>
        <DialogBody>
          {t('newToolsBody', {
            tools: toolsLabelText,
          })}
        </DialogBody>
        <DialogFooter>
          <button className="btn btn-secondary" onClick={onLater} disabled={loading}>
            {t('later')}
          </button>
          <button className="btn btn-primary" onClick={onSyncAll} disabled={loading}>
            {t('syncAll')}
          </button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

export default memo(NewToolsModal)
