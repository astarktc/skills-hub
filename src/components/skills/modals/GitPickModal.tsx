import { memo, type ReactNode } from 'react'
import type { TFunction } from 'i18next'
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import type { GitSkillCandidate } from '../types'

type GitPickModalProps = {
  open: boolean
  loading: boolean
  gitCandidates: GitSkillCandidate[]
  gitCandidateSelected: Record<string, boolean>
  onRequestClose: () => void
  onCancel: () => void
  onToggleAll: (checked: boolean) => void
  onToggleCandidate: (subpath: string, checked: boolean) => void
  onInstall: () => void
  /** The header progress at the dialog's foot while its action runs (D19; the page behind is inert). */
  progress?: ReactNode
  t: TFunction
}

const GitPickModal = ({
  open,
  loading,
  gitCandidates,
  gitCandidateSelected,
  onRequestClose,
  onCancel,
  onToggleAll,
  onToggleCandidate,
  onInstall,
  progress,
  t,
}: GitPickModalProps) => {
  const selectedCount = gitCandidates.filter(
    (c) => gitCandidateSelected[c.subpath],
  ).length

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) onRequestClose()
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('gitPickTitle')}</DialogTitle>
        </DialogHeader>
        <DialogBody>
          <p className="label">{t('gitPickBody')}</p>
          <div className="pick-toolbar">
            <label className="inline-checkbox">
              <input
                type="checkbox"
                checked={
                  gitCandidates.length > 0 &&
                  gitCandidates.every((c) => gitCandidateSelected[c.subpath])
                }
                onChange={(e) => onToggleAll(e.target.checked)}
              />
              {t('selectAll')}
            </label>
            <span className="pick-toolbar-count">
              {t('selectedCount', {
                selected: selectedCount,
                total: gitCandidates.length,
              })}
            </span>
          </div>
          <div className="pick-list">
            {gitCandidates.map((c) => (
              <div className="pick-item" key={c.subpath}>
                <label className="pick-item-checkbox">
                  <input
                    type="checkbox"
                    checked={Boolean(gitCandidateSelected[c.subpath])}
                    onChange={(e) => onToggleCandidate(c.subpath, e.target.checked)}
                  />
                </label>
                <div className="pick-item-main">
                  <div className="pick-item-title">{c.name}</div>
                  {c.description ? (
                    <div className="pick-item-desc">{c.description}</div>
                  ) : null}
                  <div className="pick-item-path">{c.subpath}</div>
                </div>
              </div>
            ))}
          </div>
        </DialogBody>
        <DialogFooter>
          <button className="btn btn-secondary" onClick={onCancel} disabled={loading}>
            {t('cancel')}
          </button>
          <button className="btn btn-primary" onClick={onInstall} disabled={loading}>
            {t('installSelected')}
          </button>
        </DialogFooter>
        {progress}
      </DialogContent>
    </Dialog>
  )
}

export default memo(GitPickModal)
