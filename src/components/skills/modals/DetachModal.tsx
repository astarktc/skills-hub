import type { TFunction } from "i18next";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import type { ManagedSkill } from "../types";

type DetachModalProps = {
  /** The skill being detached; null = closed (the dialog stays mounted, D2's one-identity rule). */
  skill: ManagedSkill | null;
  loading: boolean;
  onConfirm: () => void;
  onRequestClose: () => void;
  /** Where focus goes when the opener (Detach) is gone after it ran (D21: the focused row). */
  fallbackFocus?: () => HTMLElement | null;
  t: TFunction;
};

/**
 * Detach states its consequence before it runs: the skill stops following
 * its folder, Skills Hub's copy becomes the source, and Refresh skips it
 * from then on (ADR-0003). The door's `detach` opens it; Confirm runs the
 * verb with `confirmed`.
 */
export default function DetachModal({ skill, loading, onConfirm, onRequestClose, fallbackFocus, t }: DetachModalProps) {
  const name = skill?.name ?? "";
  const path = skill?.source_ref ?? null;
  return (
    <ConfirmDialog
      open={skill !== null}
      title={t("detach.title", { name })}
      confirmLabel={t("detach.confirm")}
      cancelLabel={t("cancel")}
      pending={loading}
      onConfirm={onConfirm}
      onCancel={onRequestClose}
      fallbackFocus={fallbackFocus}
    >
      <p className="m-0 text-sm text-foreground">
        {path ? t("detach.body", { name, path }) : t("detach.bodyNoPath", { name })}
      </p>
    </ConfirmDialog>
  );
}
