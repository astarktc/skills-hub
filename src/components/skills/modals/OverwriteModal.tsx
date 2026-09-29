import { memo, useRef, useState } from 'react'
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
import { Segmented, SegmentedItem } from '@/components/ui/segmented'
import { overwriteRowKey, type OverwriteChoice, type OverwriteRow } from '../../../lib/overwriteAsk'
import { formatRelativeTime } from '../../../lib/skillPresentation'
import type { OverwritePending } from '../../../hooks/useSyncOrchestration'

type OverwriteModalProps = {
  /** The one pending overwrite ask (`surface.pendingAsk('overwrite')`). */
  pending: OverwritePending | null
  t: TFunction
}

/**
 * The one overwrite ask (D12): every folder in the way of one operator
 * action — an Add batch, a sync, an Enable, a project assign or resync —
 * listed as skill → Tool → path with the folder's modified time and whether
 * it differs from the skill. Several rows are answered per row (Keep /
 * Overwrite, one confirm); "Keep existing" keeps them all.
 *
 * Raised mid-action by the sync seam, while the action's progress runs: it
 * opens above whatever is open (the most recently opened layer is on top)
 * and its buttons are never disabled on `loading` — the action is waiting on
 * exactly this answer.
 *
 * Three kinds of answer: an Overwrite / per-row choice (`confirmed` or
 * `{ choices }`), the explicit "Keep existing" (`declined` — recorded: the
 * backend marks the pairs disabled) and a dismissal — Escape or an outside
 * press (`dismissed` — records nothing). ConfirmDialog funnels Escape into
 * its cancel callback, so this ask composes the Dialog primitives itself to
 * keep the two apart.
 */
const OverwriteModal = ({ pending, t }: OverwriteModalProps) => (
  <Dialog
    open={Boolean(pending)}
    onOpenChange={(next) => {
      if (!next) pending?.answer('dismissed')
    }}
  >
    {/* Keyed by the ask's rows so the per-row choices start fresh for
        every ask (a displacing ask remounts the body). */}
    <OverwriteAsk key={pending ? pending.rows.map(overwriteRowKey).join('\u0000') : 'none'} pending={pending} t={t} />
  </Dialog>
)

function OverwriteAsk({ pending, t }: OverwriteModalProps) {
  const confirmRef = useRef<HTMLButtonElement>(null)
  const rows = pending?.rows ?? []
  // Overwrite is the action the operator asked for; each row can be kept.
  const [choices, setChoices] = useState<OverwriteChoice[]>(() => rows.map(() => 'overwrite'))
  const perRow = rows.length > 1
  const mixed = choices.some((choice) => choice === 'keep')

  const confirm = () => {
    if (!pending) return
    pending.answer(mixed ? { choices } : 'confirmed')
  }

  return (
    <DialogContent size="md" showCloseButton={false} initialFocus={confirmRef}>
      <DialogHeader>
        <DialogTitle>{t('overwrite.title')}</DialogTitle>
      </DialogHeader>
      {pending ? (
        <DialogBody>
          <p>{t('overwrite.body', { count: rows.length })}</p>
          {perRow ? <p className="mt-1">{t('overwriteAsk.perRowHint')}</p> : null}
          <ul className="overwrite-list">
            {rows.map((row, index) => (
              <li key={overwriteRowKey(row)} className="overwrite-row">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <span className="overwrite-pair">
                      {row.skillName} → {row.toolLabel}
                    </span>
                    <span className="overwrite-path">{row.path}</span>
                    <RowFacts row={row} t={t} />
                  </div>
                  {perRow ? (
                    <Segmented
                      aria-label={t('overwriteAsk.choiceLabel', {
                        name: row.skillName,
                        tool: row.toolLabel,
                      })}
                      value={choices[index]}
                      onValueChange={(value) =>
                        setChoices((prev) =>
                          prev.map((choice, i) => (i === index ? (value as OverwriteChoice) : choice)),
                        )
                      }
                      className="shrink-0"
                    >
                      <SegmentedItem value="keep">{t('overwriteAsk.keep')}</SegmentedItem>
                      <SegmentedItem value="overwrite">{t('overwriteAsk.overwrite')}</SegmentedItem>
                    </Segmented>
                  ) : null}
                </div>
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
          onClick={confirm}
          disabled={!pending}
        >
          {mixed ? t('overwriteAsk.apply') : t('overwrite.confirm')}
        </Button>
      </DialogFooter>
    </DialogContent>
  )
}

/** "Modified 2h ago · differs from the skill" — the facts the backend reported. */
function RowFacts({ row, t }: { row: OverwriteRow; t: TFunction }) {
  const parts = [
    ...(row.modifiedAt !== null
      ? [t('overwriteAsk.modified', { when: formatRelativeTime(row.modifiedAt, t) })]
      : []),
    t(row.sameContent ? 'overwriteAsk.identical' : 'overwriteAsk.differs'),
  ]
  return <span className="block text-xs text-muted-foreground">{parts.join(' · ')}</span>
}

export default memo(OverwriteModal)
