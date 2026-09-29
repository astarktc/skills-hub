import { memo, useId } from 'react'
import type { TFunction } from 'i18next'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import {
  showsProgressSurface,
  type PendingAction,
} from '../../lib/actionLifecycle'

type LoadingOverlayProps = {
  /** The reporter's running action; the overlay shows for a batch or a streamed/cancellable one. */
  pending: PendingAction | null
  actionMessage: string | null
  /** Offered only when the running action is cancellable (D20). */
  onCancel: () => void
  t: TFunction
}

const LoadingOverlay = ({
  pending,
  actionMessage,
  onCancel,
  t,
}: LoadingOverlayProps) => {
  // Until ticket 14 replaces it with header progress (D19), the overlay is
  // the progress surface for a batch or a streamed action (a small action on
  // one skill or cell pends inline instead). It is a modal layer like any other: named, focus-trapped, top of the stack while
  // it runs (a confirmation raised mid-action opens above it), never
  // dismissed by Escape. The spinner and shimmer stop under reduced motion
  // (index.css base layer); the words carry the state.
  const titleId = useId()
  const stageId = useId()

  return (
    <Dialog open={showsProgressSurface(pending)} dismissible={false} onOpenChange={() => {}}>
      <DialogContent
        size="md"
        showCloseButton={false}
        aria-labelledby={titleId}
        aria-describedby={stageId}
      >
        <div className="loading-content">
          <div className="loader-spinner" aria-hidden="true" />
          <div className="loading-text" id={titleId}>
            {t('processingTitle')}
          </div>
          <div className="loading-stage" id={stageId} role="status">
            {pending?.cancelling ? t('lifecycle.cancelling') : (actionMessage ?? t('processingTipShort'))}
          </div>
          {pending ? (
            <div className="loading-subtext loading-subtext-delayed">
              {t('processingTipLong')}
            </div>
          ) : null}
          <div className="progress-bar">
            <div className="progress-fill" />
          </div>
          {pending?.cancellable ? (
            <button
              className="btn btn-secondary loading-cancel-btn"
              type="button"
              onClick={onCancel}
              disabled={pending.cancelling}
            >
              {pending.cancelling ? t('lifecycle.cancelling') : t('cancel')}
            </button>
          ) : null}
        </div>
      </DialogContent>
    </Dialog>
  )
}

export default memo(LoadingOverlay)
