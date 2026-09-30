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
import type { LocalSkillCandidate } from '../types'

type LocalPickModalProps = {
  open: boolean
  loading: boolean
  localCandidates: LocalSkillCandidate[]
  localCandidateSelected: Record<string, boolean>
  onRequestClose: () => void
  onCancel: () => void
  onToggleAll: (checked: boolean) => void
  onToggleCandidate: (subpath: string, checked: boolean) => void
  onInstall: () => void
  /** The header progress at the dialog's foot while its action runs (D19; the page behind is inert). */
  progress?: ReactNode
  t: TFunction
}

const LocalPickModal = ({
  open,
  loading,
  localCandidates,
  localCandidateSelected,
  onRequestClose,
  onCancel,
  onToggleAll,
  onToggleCandidate,
  onInstall,
  progress,
  t,
}: LocalPickModalProps) => {
  const selectedCount = localCandidates.filter(
    (c) => localCandidateSelected[c.subpath],
  ).length
  const selectableCount = localCandidates.filter((c) => c.valid).length

  const mapReason = (code?: string | null) => {
    if (!code) return t('localSkillInvalid.unknown')
    if (code === 'missing_skill_md') return t('localSkillInvalid.missingSkillMd')
    if (code === 'invalid_frontmatter') return t('localSkillInvalid.invalidFrontmatter')
    if (code === 'missing_name') return t('localSkillInvalid.missingName')
    if (code === 'read_failed') return t('localSkillInvalid.readFailed')
    if (code === 'inside_tool_dir') return t('localSkillInvalid.insideToolDir')
    return t('localSkillInvalid.unknown')
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) onRequestClose()
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('localPickTitle')}</DialogTitle>
        </DialogHeader>
        <DialogBody>
          <p className="label">{t('localPickBody')}</p>
          <div className="pick-toolbar">
            <label className="inline-checkbox">
              <input
                type="checkbox"
                checked={
                  selectableCount > 0 &&
                  localCandidates
                    .filter((c) => c.valid)
                    .every((c) => localCandidateSelected[c.subpath])
                }
                onChange={(e) => onToggleAll(e.target.checked)}
              />
              {t('selectAll')}
            </label>
            <span className="pick-toolbar-count">
              {t('selectedCount', {
                selected: selectedCount,
                total: selectableCount,
              })}
            </span>
          </div>
          <div className="pick-list">
            {localCandidates.map((c) => (
              <div
                className={`pick-item${c.valid ? '' : ' disabled'}`}
                key={c.subpath}
              >
                <label className="pick-item-checkbox">
                  <input
                    type="checkbox"
                    checked={Boolean(localCandidateSelected[c.subpath])}
                    onChange={(e) => onToggleCandidate(c.subpath, e.target.checked)}
                    disabled={!c.valid}
                  />
                </label>
                <div className="pick-item-main">
                  <div className="pick-item-title">{c.name}</div>
                  {c.description ? (
                    <div className="pick-item-desc">{c.description}</div>
                  ) : null}
                  <div className="pick-item-path">{c.subpath}</div>
                  {!c.valid ? (
                    <div className="pick-item-reason">
                      {t('localPickInvalidReason', { reason: mapReason(c.reason) })}
                    </div>
                  ) : null}
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

export default memo(LocalPickModal)
