import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import { AlertTriangle } from "lucide-react";
import type { TFunction } from "i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { FocusFallback } from "@/components/ui/dialog";
import { Field, FieldDescription, FieldError, FieldLabel, Input } from "@/components/ui/field";
import { Segmented, SegmentedItem } from "@/components/ui/segmented";
import { describeCommandError } from "../../../commandError";
import { visibleCandidates, type PickerCandidate } from "../../../lib/addFlow";
import { CandidatePicker } from "../add/CandidatePicker";
import type { RepointPreviewAnswer } from "../../../hooks/useSkillLibrary";
import { repointCurrentSource, type RepointKind } from "../../../lib/skillPresentation";
import type { CommandError, GitSkillCandidate, ManagedSkill, RepointPreview, RepointTarget } from "../types";

type RepointBodyProps = {
  skill: ManagedSkill;
  /** The kind the dialog opens on (the skill's own, or `local` for a moved folder). */
  preselect: RepointKind;
  /** An action runs (the preview, the Re-point itself): inputs wait. */
  loading: boolean;
  /** Resolve a target without landing it (D15); a refusal comes back as a value. */
  previewRepoint: (skillId: string, target: RepointTarget) => Promise<RepointPreviewAnswer | undefined>;
  /** The folder dialog; resolves null when the operator cancels it. */
  onPickFolder: () => Promise<string | null>;
  /**
   * The confirmed target: the door's `repoint` runs it and answers its final
   * command's refusal (null when there is none to show) — shown here, by
   * the chosen target, which stays chosen.
   */
  onConfirm: (target: RepointTarget) => Promise<CommandError | null>;
  onRequestClose: () => void;
  t: TFunction;
};

/** A repository candidate as the one picker renders it (single-choice mode). */
const toPicker = (candidate: GitSkillCandidate): PickerCandidate => ({
  id: candidate.subpath,
  name: candidate.name,
  description: candidate.description,
  subpath: candidate.subpath,
  unavailable: null,
});

type Step =
  | { kind: "form" }
  | { kind: "pick"; url: string; candidates: GitSkillCandidate[]; chosen: string | null; query: string }
  | {
      kind: "confirm";
      target: RepointTarget;
      preview: RepointPreview;
      /** The repository's candidates when it holds several: another may be picked. */
      candidates: GitSkillCandidate[] | null;
    };

/**
 * Re-point (D15): the field is pre-filled with the current source; Check
 * resolves the target first (`preview_repoint_source`, writes nothing) and
 * shows "replaces *X*'s bytes with **Y** (subpath @ ref)". A name mismatch is
 * highlighted and needs an explicit Continue — never a refusal. A
 * multi-skill repository lists its skills to pick from; the pick crosses the
 * wire as a `selection` (a default-branch repository has no URL naming the
 * folder). A refusal renders by the field, never as a toast over the dialog
 * — the preview's by the input, the final command's under the confirmed
 * choice, which stays chosen for a retry.
 */
function RepointBody({
  skill,
  preselect,
  loading,
  previewRepoint,
  onPickFolder,
  onConfirm,
  onRequestClose,
  t,
}: RepointBodyProps) {
  // Read once: the dialog is keyed per skill, and edits must not be reset.
  const [currentSource] = useState(() => repointCurrentSource(skill));
  const [kind, setKind] = useState<RepointKind>(preselect);
  const [url, setUrl] = useState(currentSource?.kind === "git" ? currentSource.ref : "");
  const [path, setPath] = useState(currentSource?.kind === "local" ? currentSource.ref : "");
  const [error, setError] = useState<CommandError | null>(null);
  // The final command's refusal, for the confirmation it was raised from.
  const [finalError, setFinalError] = useState<{ step: Step; error: CommandError } | null>(null);
  const [step, setStep] = useState<Step>({ kind: "form" });
  const formId = useId();
  const finalErrorId = useId();
  const pickId = useId();
  const fieldRef = useRef<HTMLInputElement | null>(null);
  const value = (kind === "git" ? url : path).trim();
  // Back to the form (from a pick or a confirmation), or a kind switch: the
  // field takes focus. The dialog's first focus is its own (`initialFocus`),
  // so the opener it returns to is captured untouched.
  const seenForm = useRef(false);
  const summaryRef = useRef<HTMLParagraphElement | null>(null);
  const confirmRef = useRef<HTMLButtonElement | null>(null);
  const pickRef = useRef<HTMLDivElement | null>(null);
  // A pick's choice and search edit the step in place: only entering a step
  // (or a new confirmation) moves focus.
  const stepKey = step.kind === "confirm" ? step : step.kind;
  useEffect(() => {
    // After the dialog's own focus handling settles (the control that moved
    // the step is gone): a match focuses its confirmation; a mismatch its
    // summary — Continue is a deliberate second step (D15); a pick its list.
    const timer = window.setTimeout(() => {
      if (stepKey === "form") {
        if (seenForm.current) fieldRef.current?.focus();
        seenForm.current = true;
      } else if (stepKey === "pick") {
        pickRef.current
          ?.querySelector<HTMLInputElement>("[data-candidate-search], input[type='radio']:checked, input[type='radio']")
          ?.focus();
      } else if (stepKey.preview.name_matches) {
        confirmRef.current?.focus();
      } else {
        summaryRef.current?.focus();
      }
    }, 0);
    return () => window.clearTimeout(timer);
  }, [stepKey]);

  const check = async (target: RepointTarget, candidates: GitSkillCandidate[] | null = null) => {
    setError(null);
    const answer = await previewRepoint(skill.id, target);
    if (!answer) return;
    if (answer.kind === "error") {
      setError(answer.error);
      setStep({ kind: "form" });
      window.setTimeout(() => fieldRef.current?.focus(), 0);
      return;
    }
    const { preview } = answer;
    if (preview.resolved_name === null && preview.candidates) {
      setStep({ kind: "pick", url: target.kind === "git" ? target.url : "", candidates: preview.candidates, chosen: null, query: "" });
      return;
    }
    setStep({ kind: "confirm", target, preview, candidates: preview.candidates ?? candidates });
  };

  const submitForm = () => {
    if (loading || !value) return;
    void check(kind === "git" ? { kind: "git", url: value } : { kind: "local", path: value });
  };

  const submitPick = (current: Extract<Step, { kind: "pick" }>) => {
    // Only a choice still on screen is sent (the search may have hidden it).
    const shown = visibleCandidates(current.candidates.map(toPicker), current.query);
    const candidate = current.candidates.find((c) => c.subpath === current.chosen);
    if (!candidate || loading || !shown.some((c) => c.id === candidate.subpath)) return;
    void check(
      { kind: "git", url: current.url, selection: { subpath: candidate.subpath, resolution: candidate.resolution } },
      current.candidates,
    );
  };

  const handleChooseFolder = async () => {
    const picked = await onPickFolder();
    if (picked) setPath(picked);
  };

  const where = (preview: RepointPreview, target: RepointTarget) => {
    if (target.kind === "local") return target.path;
    const subpath = preview.subpath ?? t("repoint.repositoryRoot");
    return preview.ref
      ? t("repoint.where", { subpath, ref: preview.ref })
      : t("repoint.whereDefaultBranch", { subpath });
  };

  const confirm = (current: Extract<Step, { kind: "confirm" }>) => {
    if (loading) return;
    setFinalError(null);
    void (async () => {
      const refusal = await onConfirm(current.target);
      if (refusal) setFinalError({ step: current, error: refusal });
    })();
  };
  const refusal = finalError && finalError.step === step ? describeCommandError(finalError.error, t) : null;

  const title = t("changeSource.title", { name: skill.name });

  return (
    <>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>
            {t("repoint.current")}:{" "}
            <span className="font-mono text-xs text-foreground">{currentSource?.ref ?? t("repoint.noCurrent")}</span>
          </DialogDescription>
        </DialogHeader>

        {step.kind === "form" ? (
          <>
            <DialogBody>
              <form
                id={formId}
                className="flex flex-col gap-3"
                onSubmit={(event) => {
                  event.preventDefault();
                  submitForm();
                }}
              >
                <Segmented
                  aria-label={t("changeSource.kindLabel")}
                  value={kind}
                  onValueChange={(next) => {
                    setKind(next === "local" ? "local" : "git");
                    setError(null);
                  }}
                  disabled={loading}
                >
                  <SegmentedItem value="git">{t("changeSource.kindGit")}</SegmentedItem>
                  <SegmentedItem value="local">{t("changeSource.kindLocal")}</SegmentedItem>
                </Segmented>
                {kind === "git" ? (
                  <Field invalid={error !== null}>
                    <FieldLabel>{t("changeSource.urlLabel")}</FieldLabel>
                    <Input
                      ref={fieldRef}
                      data-repoint-field=""
                      type="text"
                      inputMode="url"
                      value={url}
                      onChange={(event) => {
                        setUrl(event.target.value);
                        setError(null);
                      }}
                      placeholder={t("changeSource.urlPlaceholder")}
                      disabled={loading}
                      autoComplete="off"
                      spellCheck={false}
                    />
                    <FieldError>{error ? describeCommandError(error, t) : null}</FieldError>
                    <FieldDescription>{t("changeSource.urlHelp")}</FieldDescription>
                  </Field>
                ) : (
                  <Field invalid={error !== null}>
                    <FieldLabel>{t("changeSource.pathLabel")}</FieldLabel>
                    <div className="flex gap-2">
                      <Input
                        ref={fieldRef}
                        data-repoint-field=""
                        type="text"
                        value={path}
                        onChange={(event) => {
                          setPath(event.target.value);
                          setError(null);
                        }}
                        placeholder={t("changeSource.pathPlaceholder")}
                        disabled={loading}
                        autoComplete="off"
                        spellCheck={false}
                      />
                      <Button type="button" onClick={() => void handleChooseFolder()} disabled={loading}>
                        {t("changeSource.chooseFolder")}
                      </Button>
                    </div>
                    <FieldError>{error ? describeCommandError(error, t) : null}</FieldError>
                    <FieldDescription>{t("changeSource.pathHelp")}</FieldDescription>
                  </Field>
                )}
              </form>
            </DialogBody>
            <DialogFooter>
              <Button onClick={onRequestClose} disabled={loading}>
                {t("cancel")}
              </Button>
              <Button type="submit" form={formId} variant="solid" tone="accent" disabled={!value} pending={loading}>
                {t("repoint.check")}
              </Button>
            </DialogFooter>
          </>
        ) : step.kind === "pick" ? (
          <>
            <DialogBody>
              <form
                id={pickId}
                onSubmit={(event) => {
                  event.preventDefault();
                  submitPick(step);
                }}
              >
                <p className="m-0 mb-3 text-sm text-muted-foreground">
                  {t("repoint.pickBody", { count: step.candidates.length, name: skill.name })}
                </p>
                <div ref={pickRef}>
                  <CandidatePicker
                    mode="single"
                    candidates={step.candidates.map(toPicker)}
                    visible={visibleCandidates(step.candidates.map(toPicker), step.query)}
                    query={step.query}
                    onQueryChange={(query) => setStep({ ...step, query })}
                    chosen={step.chosen}
                    onChoose={(id) => setStep({ ...step, chosen: id })}
                    legend={t("repoint.pickLegend")}
                    status={t("repointPick.shown", {
                      shown: visibleCandidates(step.candidates.map(toPicker), step.query).length,
                      total: step.candidates.length,
                    })}
                    disabled={loading}
                    t={t}
                  />
                </div>
              </form>
            </DialogBody>
            <DialogFooter>
              <Button onClick={() => setStep({ kind: "form" })} disabled={loading}>
                {t("repoint.back")}
              </Button>
              <Button type="submit" form={pickId} variant="solid" tone="accent" disabled={step.chosen === null} pending={loading}>
                {t("repoint.pickConfirm")}
              </Button>
            </DialogFooter>
          </>
        ) : (
          <>
            <DialogBody>
              <p ref={summaryRef} tabIndex={-1} className="m-0 rounded-sm text-sm text-foreground">
                {t("repoint.replacesPre")} <em className="font-medium">{skill.name}</em>
                {t("repoint.replacesMid")} <strong className="font-semibold">{step.preview.resolved_name}</strong>{" "}
                <span className="font-mono text-xs text-muted-foreground">({where(step.preview, step.target)})</span>
              </p>
              {!step.preview.name_matches ? (
                <div data-slot="name-mismatch" className="flex flex-col items-start gap-1.5">
                  <Badge tone="stale" size="md">
                    <AlertTriangle aria-hidden="true" />
                    {t("repoint.mismatchBadge")}
                  </Badge>
                  <p className="m-0 text-sm text-foreground">
                    {t("repoint.mismatch", { current: skill.name, next: step.preview.resolved_name ?? "" })}
                  </p>
                </div>
              ) : null}
              <FieldError id={finalErrorId}>{refusal}</FieldError>
              {step.candidates && step.target.kind === "git" ? (
                <Button
                  size="sm"
                  variant="ghost"
                  tone="accent"
                  className="self-start"
                  onClick={() => {
                    const target = step.target;
                    if (target.kind !== "git" || !step.candidates) return;
                    setStep({ kind: "pick", url: target.url, candidates: step.candidates, chosen: target.selection?.subpath ?? null, query: "" });
                  }}
                  disabled={loading}
                >
                  {t("repoint.pickAnother")}
                </Button>
              ) : null}
            </DialogBody>
            <DialogFooter>
              <Button onClick={() => setStep({ kind: "form" })} disabled={loading}>
                {t("repoint.back")}
              </Button>
              <Button
                ref={confirmRef}
                variant="solid"
                tone="accent"
                pending={loading}
                aria-describedby={refusal ? finalErrorId : undefined}
                onClick={() => confirm(step)}
              >
                {step.preview.name_matches
                  ? t("repoint.confirm")
                  : t("repoint.continue", { name: step.preview.resolved_name ?? "" })}
              </Button>
            </DialogFooter>
          </>
        )}
    </>
  );
}

type RepointModalProps = Omit<RepointBodyProps, "skill" | "preselect"> & {
  /** What the dialog is about; null = closed. */
  request: { skill: ManagedSkill; preselect: RepointKind } | null;
  /** Where focus goes when the opener is gone after closing (D21: the focused row). */
  fallbackFocus?: FocusFallback;
  /** The header progress at the dialog's foot while its Re-point runs (its Cancel, D20). */
  progress?: ReactNode;
};

/**
 * The dialog shell stays mounted for the app's life (one popup identity, so
 * closing returns focus to its opener — the Health row's Re-point, the
 * inspector's Change source…); only the body is keyed per skill and kind,
 * so its drafts start fresh for every opening.
 */
export default function RepointModal({ request, loading, onRequestClose, fallbackFocus, progress, ...rest }: RepointModalProps) {
  return (
    <Dialog
      open={request !== null}
      dismissible={!loading}
      onOpenChange={(next) => {
        if (!next) onRequestClose();
      }}
    >
      <DialogContent
        size="lg"
        fallbackFocus={fallbackFocus}
        initialFocus={() => document.querySelector<HTMLElement>("[data-repoint-field]")}
      >
        {request ? (
          <RepointBody
            key={`${request.skill.id}:${request.preselect}`}
            skill={request.skill}
            preselect={request.preselect}
            loading={loading}
            onRequestClose={onRequestClose}
            {...rest}
          />
        ) : null}
        {progress}
      </DialogContent>
    </Dialog>
  );
}
