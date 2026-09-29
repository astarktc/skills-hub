import { memo, useId } from 'react'
import type { TFunction } from 'i18next'
import { Dialog, DialogContent } from '@/components/ui/dialog'

type LoadingOverlayProps = {
  loading: boolean
  actionMessage: string | null
  loadingStartAt: number | null
  onCancel?: () => void
  t: TFunction
}

const LoadingOverlay = ({
  loading,
  actionMessage,
  loadingStartAt,
  onCancel,
  t,
}: LoadingOverlayProps) => {
  // Until ticket 14 replaces it with header progress (D19), the overlay is a
  // modal layer like any other: named, focus-trapped, top of the stack while
  // it runs (a confirmation raised mid-action opens above it), never
  // dismissed by Escape. The spinner and shimmer stop under reduced motion
  // (index.css base layer); the words carry the state.
  const titleId = useId()
  const stageId = useId()

  return (
    <Dialog open={loading} dismissible={false} onOpenChange={() => {}}>
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
            {actionMessage ?? t('processingTipShort')}
          </div>
          {loadingStartAt ? (
            <div className="loading-subtext loading-subtext-delayed">
              {t('processingTipLong')}
            </div>
          ) : null}
          <div className="progress-bar">
            <div className="progress-fill" />
          </div>
          {onCancel ? (
            <button
              className="btn btn-secondary loading-cancel-btn"
              type="button"
              onClick={onCancel}
            >
              {t('cancel')}
            </button>
          ) : null}
        </div>
      </DialogContent>
    </Dialog>
  )
}

export default memo(LoadingOverlay)
