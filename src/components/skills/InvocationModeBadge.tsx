import { Bot, EyeOff, User } from "lucide-react";
import type { TFunction } from "i18next";
import type { InvocationMode } from "./types";

type InvocationModeBadgeProps = {
  mode: InvocationMode;
  t: TFunction;
};

const InvocationModeBadge = ({ mode, t }: InvocationModeBadgeProps) => {
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
  const labelKey =
    mode === "user-and-model"
      ? "invocationMode.userAndModel"
      : mode === "user-only"
        ? "invocationMode.userOnly"
        : mode === "model-only"
          ? "invocationMode.modelOnly"
          : "invocationMode.neither";
  const tooltipKey =
    mode === "user-and-model"
      ? "invocationMode.userAndModelTooltip"
      : mode === "user-only"
        ? "invocationMode.userOnlyTooltip"
        : mode === "model-only"
          ? "invocationMode.modelOnlyTooltip"
          : "invocationMode.neitherTooltip";

  return (
    <span
      className={`invocation-badge ${mode}`}
      title={`${t(labelKey)} — ${t(tooltipKey)}`}
      aria-label={t(labelKey)}
    >
      {icon}
    </span>
  );
};

export default InvocationModeBadge;
