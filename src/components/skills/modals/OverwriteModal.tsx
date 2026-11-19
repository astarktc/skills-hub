import { memo, useMemo, useRef, useState, type RefObject } from 'react'
import type { TFunction } from 'i18next'
import { FolderSearch } from 'lucide-react'
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
import {
  askRowCopy,
  askRowLabels,
  overwriteRowKey,
  type OverwriteChoice,
  type OverwriteRow,
} from '../../../lib/overwriteAsk'
import { formatRelativeTime } from '../../../lib/skillPresentation'
import { useSkillDocument } from '../../../hooks/useSkillDocument'
import type { OverwritePending } from '../../../hooks/useSyncOrchestration'
import SkillFiles from '../SkillFiles'

type OverwriteModalProps = {
  /** The one pending overwrite ask (`surface.pendingAsk('overwrite')`). */
  pending: OverwritePending | null
  /** Reveal in Finder (the backend admits only a folder an ask reported). */
  onReveal: (path: string) => void
  t: TFunction
}

/**
 * The one overwrite ask (D12): every folder in the way of one operator
 * action — an Add batch, a sync, an Enable, a project assign or resync, a
 * Propagation, a redeploy — listed as skill → Tool → path with the folder's
 * modified time and whether it differs from the skill. Several rows are
 * answered per row (Keep / Overwrite, one confirm); "Keep existing" keeps
 * them all.
 *
 * Round 20 #70 — an ask the operator can answer knowingly. Each row says
 * **who put the folder there** (a recorded Deployment's artifact, by the
 * backend's evidence, or not ours), offers **Reveal in Finder** and a
 * **content view against the skill** (the file browser over the backend's
 * fenced occupant read: each file marked same / changed / only here /
 * missing here, a changed file as a line diff), and states **what each
 * answer does** (`askRowCopy`). An ask raised after Cancel says the skills
 * were already being added.
 *
 * Raised mid-action by the sync seam, while the action's progress runs: it
 * opens above whatever is open (the most recently opened layer is on top)
 * and its buttons are never disabled on `loading` — the action is waiting on
 * exactly this answer.
 *
 * A redeploy's ask also carries **edited copies** (round 20 #108): an old
 * copy edited after Skills Hub wrote it, answered Delete (`overwrite`) or
 * Leave (`keep`), Leave chosen until the operator picks Delete — an edit is
 * never deleted by default. With any such row the choices are per row.
 *
 * Three kinds of answer: an Overwrite / per-row choice (`confirmed` or
 * `{ choices }`), the explicit "Keep existing" (`declined` — recorded: the
 * backend marks the pairs disabled) and a dismissal — Escape or an outside
 * press (`dismissed` — records nothing). ConfirmDialog funnels Escape into
 * its cancel callback, so this ask composes the Dialog primitives itself to
 * keep the two apart.
 */
const OverwriteModal = ({ pending, onReveal, t }: OverwriteModalProps) => {
  const confirmRef = useRef<HTMLButtonElement>(null)
  return (
    <Dialog
      open={Boolean(pending)}
      onOpenChange={(next) => {
        if (!next) pending?.answer('dismissed')
      }}
    >
      {/* The popup keeps one identity for the dialog's whole life — never key
          it: remounting a popup while it closes (the answer clears `pending`
          in the same render) strands a closed ghost layer that keeps the app
          inert. Only the per-ask body is keyed, so every ask's per-row
          choices start fresh (a displacing ask remounts the body alone). */}
      <DialogContent size="xl" showCloseButton={false} initialFocus={confirmRef}>
        <DialogHeader>
          <DialogTitle>{t('overwrite.title')}</DialogTitle>
        </DialogHeader>
        <OverwriteAsk
          key={pending ? pending.rows.map(overwriteRowKey).join('\u0000') : 'none'}
          pending={pending}
          onReveal={onReveal}
          confirmRef={confirmRef}
          t={t}
        />
      </DialogContent>
    </Dialog>
  )
}

/** One ask's rows, its per-row choices and its answers (body + footer). */
function OverwriteAsk({
  pending,
  onReveal,
  confirmRef,
  t,
}: OverwriteModalProps & { confirmRef: RefObject<HTMLButtonElement | null> }) {
  const rows = useMemo(() => pending?.rows ?? [], [pending])
  const edited = (row: OverwriteRow) => row.question === 'editedCopy'
  // Overwrite is the action the operator asked for; each row can be kept. An
  // edited copy starts at Leave: deleting it is the operator's explicit pick.
  const [choices, setChoices] = useState<OverwriteChoice[]>(() =>
    rows.map((row) => (edited(row) ? 'keep' : 'overwrite')),
  )
  const labels = useMemo(() => askRowLabels(rows, t), [rows, t])
  const anyEdited = rows.some(edited)
  const perRow = rows.length > 1 || anyEdited
  const mixed = anyEdited || choices.some((choice) => choice === 'keep')
  const occupied = rows.filter((row) => !edited(row)).length

  const confirm = () => {
    if (!pending) return
    pending.answer(mixed ? { choices } : 'confirmed')
  }

  return (
    <>
      {pending ? (
        <DialogBody>
          {pending.afterCancel ? (
            <p className="mb-1 text-foreground">{t('overwriteAsk.afterCancel', { count: rows.length })}</p>
          ) : null}
          {occupied > 0 ? <p>{t('overwrite.body', { count: occupied })}</p> : null}
          {anyEdited ? <p className="mt-1">{t('overwriteAsk.editedHint')}</p> : null}
          {perRow ? <p className="mt-1">{t('overwriteAsk.perRowHint')}</p> : null}
          <ul className="m-0 flex max-h-[55vh] list-none flex-col divide-y divide-border overflow-y-auto rounded-md border border-border p-0">
            {rows.map((row, index) => (
              <AskRow
                key={overwriteRowKey(row)}
                row={row}
                label={labels[index]}
                choice={perRow ? choices[index] : null}
                onChoice={(value) =>
                  setChoices((prev) => prev.map((choice, i) => (i === index ? value : choice)))
                }
                onReveal={onReveal}
                t={t}
              />
            ))}
          </ul>
          {perRow && occupied > 0 ? (
            <p className="mt-2 text-xs text-muted-foreground">{t('overwriteAsk.consequence.all')}</p>
          ) : null}
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
    </>
  )
}

/**
 * One row: what is there and who put it there, what each answer does, and
 * the ways to look before answering — Reveal in Finder and its contents
 * against the skill. `choice` is the row's own answer (null: the footer
 * answers the one row, so both consequences are stated).
 */
function AskRow({
  row,
  label,
  choice,
  onChoice,
  onReveal,
  t,
}: {
  row: OverwriteRow
  label: string
  choice: OverwriteChoice | null
  onChoice: (choice: OverwriteChoice) => void
  onReveal: (path: string) => void
  t: TFunction
}) {
  const [showContents, setShowContents] = useState(false)
  const copy = askRowCopy(row, t)
  const heading =
    row.question === 'editedCopy'
      ? t('overwriteAsk.editedChoiceLabel', { name: row.skillName, tool: row.toolLabel })
      : t('overwriteAsk.choiceLabel', { name: row.skillName, tool: row.toolLabel })
  return (
    <li className="flex flex-col gap-1.5 px-3 py-2">
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 flex-col gap-0.5">
          <span className="text-foreground">{heading}</span>
          <span className="font-mono text-xs break-all text-faint">{row.path}</span>
          <RowFacts row={row} t={t} />
          <span className="text-xs text-muted-foreground">{copy.provenance}</span>
        </div>
        {choice !== null ? (
          <Segmented
            aria-label={label}
            value={choice}
            onValueChange={(value) => onChoice(value as OverwriteChoice)}
            className="shrink-0"
          >
            <SegmentedItem value="keep">{copy.keepLabel}</SegmentedItem>
            <SegmentedItem value="overwrite">{copy.overwriteLabel}</SegmentedItem>
          </Segmented>
        ) : null}
      </div>
      {choice !== null ? (
        <p className="m-0 text-xs text-foreground">{choice === 'keep' ? copy.keep : copy.overwrite}</p>
      ) : (
        <div className="flex flex-col gap-0.5 text-xs text-foreground">
          <p className="m-0">{copy.overwrite}</p>
          <p className="m-0">{copy.keep}</p>
        </div>
      )}
      <div className="flex flex-wrap items-center gap-1">
        <Button
          variant="ghost"
          tone="neutral"
          size="sm"
          aria-label={t('overwriteAsk.revealLabel', { label })}
          onClick={() => onReveal(row.path)}
        >
          <FolderSearch aria-hidden="true" />
          {t('overwriteAsk.reveal')}
        </Button>
        <Button
          variant="ghost"
          tone="neutral"
          size="sm"
          aria-expanded={showContents}
          aria-label={`${t(showContents ? 'overwriteAsk.hideContents' : 'overwriteAsk.showContents')}: ${label}`}
          onClick={() => setShowContents((open) => !open)}
        >
          {t(showContents ? 'overwriteAsk.hideContents' : 'overwriteAsk.showContents')}
        </Button>
      </div>
      {showContents ? <OccupantContents row={row} label={label} t={t} /> : null}
    </li>
  )
}

/** The folder's files against the skill's, through the backend's occupant fence. */
function OccupantContents({ row, label, t }: { row: OverwriteRow; label: string; t: TFunction }) {
  const doc = useSkillDocument({ root: row.path, version: 0, comparedWith: row.skillId })
  return (
    <div className="flex min-h-0 flex-col">
      <SkillFiles doc={doc} label={t('overwriteAsk.contentsLabel', { label })} layout="stacked" t={t} />
    </div>
  )
}

/**
 * "Modified 2h ago · differs from the skill" — the facts the backend
 * reported. An edited copy's own words are its provenance and consequences.
 */
function RowFacts({ row, t }: { row: OverwriteRow; t: TFunction }) {
  const parts = [
    ...(row.modifiedAt !== null
      ? [t('overwriteAsk.modified', { when: formatRelativeTime(row.modifiedAt, t) })]
      : []),
    ...(row.question === 'editedCopy' ? [] : [t(row.sameContent ? 'overwriteAsk.identical' : 'overwriteAsk.differs')]),
  ]
  return parts.length > 0 ? <span className="block text-xs text-muted-foreground">{parts.join(' · ')}</span> : null
}

export default memo(OverwriteModal)
