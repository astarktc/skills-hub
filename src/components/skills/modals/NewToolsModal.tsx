import { memo } from 'react'
import type { TFunction } from 'i18next'
import { Button } from '@/components/ui/button'
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
          <Button onClick={onLater} disabled={loading}>
            {t('later')}
          </Button>
          <Button variant="solid" tone="accent" onClick={onSyncAll} disabled={loading}>
            {t('syncAll')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

export default memo(NewToolsModal)
