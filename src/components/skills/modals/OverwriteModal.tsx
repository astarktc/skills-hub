import { memo, useRef } from 'react'
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
import type { OverwritePending } from '../../../hooks/useSyncOrchestration'

type OverwriteModalProps = {
  /** The one pending overwrite ask (`surface.pendingAsk('overwrite')`). */
  pending: OverwritePending | null
  t: TFunction
}

/**
 * Raised mid-action by the sync seam, while the progress overlay runs: it
 * opens above the overlay (the most recently opened layer is on top) and its
 * buttons are never disabled on `loading` — the action is waiting on
 * exactly this answer.
 *
 * Three answers, not two: "Overwrite" (`confirmed`), the explicit "Keep
 * existing" (`declined` — recorded: the backend marks the pairs disabled)
 * and a dismissal — Escape or an outside press (`dismissed` — records
 * nothing). ConfirmDialog funnels Escape into its cancel callback, so this
 * ask composes the Dialog primitives itself to keep the two apart.
 */
const OverwriteModal = ({ pending, t }: OverwriteModalProps) => {
  const confirmRef = useRef<HTMLButtonElement>(null)
  return (
    <Dialog
      open={Boolean(pending)}
      onOpenChange={(next) => {
        if (!next) pending?.answer('dismissed')
      }}
    >
      <DialogContent size="md" showCloseButton={false} initialFocus={confirmRef}>
        <DialogHeader>
          <DialogTitle>{t('overwrite.title')}</DialogTitle>
        </DialogHeader>
        {pending ? (
          <DialogBody>
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
          </DialogBody>
        ) : null}
        <DialogFooter>
          <Button
            variant="outline"
            tone="neutral"
            onClick={() => pending?.answer('declined')}
            disabled={!pending}
          >
            {t('overwrite.cancel')}
          </Button>
          <Button
            ref={confirmRef}
            variant="solid"
            tone="accent"
            onClick={() => pending?.answer('confirmed')}
            disabled={!pending}
          >
            {t('overwrite.confirm')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

export default memo(OverwriteModal)
