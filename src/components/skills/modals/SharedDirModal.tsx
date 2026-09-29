import { memo } from 'react'
import type { TFunction } from 'i18next'
import { ConfirmDialog } from '@/components/ui/confirm-dialog'
import type { SharedDirPending } from '../../../hooks/useSharedDirConfirmation'

type SharedDirModalProps = {
  /** The one pending shared-dir ask (`surface.pendingAsk('sharedDir')`). */
  pending: SharedDirPending | null
  loading: boolean
  onCancel: () => void
  t: TFunction
}

const SharedDirModal = ({ pending, loading, onCancel, t }: SharedDirModalProps) => {
  return (
    <ConfirmDialog
      open={Boolean(pending)}
      title={t('sharedDir.title')}
      confirmLabel={t('sharedDir.confirm')}
      cancelLabel={t('sharedDir.cancel')}
      onConfirm={() => pending?.answer('confirmed')}
      onCancel={onCancel}
      cancelDisabled={loading}
      confirmDisabled={loading || !pending}
    >
      {pending
        ? t('sharedDir.body', {
            tool: pending.toolLabel,
            others: pending.labels.join(t('common.listSeparator')),
          })
        : null}
    </ConfirmDialog>
  )
}

export default memo(SharedDirModal)
