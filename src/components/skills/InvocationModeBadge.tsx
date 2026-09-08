import { Bot, EyeOff, User } from "lucide-react";
import type { TFunction } from "i18next";
import type { InvocationMode, InvocationOverride } from "./types";
import { INVOCATION_LABEL_KEY, INVOCATION_TOOLTIP_KEY } from "../../lib/skillPresentation";

type InvocationModeBadgeProps = {
  mode: InvocationMode;
  override: InvocationOverride | null;
  disabled: boolean;
  onClick: () => void;
  t: TFunction;
};

const InvocationModeBadge = ({ mode, override, disabled, onClick, t }: InvocationModeBadgeProps) => {
  const icon =
    mode === "user-and-model" ? (
      <>
        <User size={11} aria-hidden="true" />
        <Bot size={11} aria-hidden="true" />
      </>
    ) : mode === "user-only" ? (
      <User size={11} aria-hidden="true" />
    ) : mode === "model-only" ? (
      <Bot size={11} aria-hidden="true" />
    ) : (
      <EyeOff size={11} aria-hidden="true" />
    );
  const label = t(INVOCATION_LABEL_KEY[mode]);
  const overrideNote = override
    ? ` — ${t(override.conflict ? "invocationEdit.conflictTooltip" : "invocationEdit.overrideTooltip", { mode: t(INVOCATION_LABEL_KEY[override.base_mode]) })}`
    : "";

  return (
    <button
      type="button"
      className={`invocation-badge ${mode}${override ? " overridden" : ""}${override?.conflict ? " conflict" : ""}`}
      title={`${label} — ${t(INVOCATION_TOOLTIP_KEY[mode])}${overrideNote}`}
      aria-label={`${label}${override ? ` — ${t("invocationEdit.overridden")}` : ""}`}
      aria-haspopup="dialog"
      disabled={disabled}
      onClick={(event) => { event.stopPropagation(); onClick(); }}
    >
      {icon}
    </button>
  );
};

export default InvocationModeBadge;
