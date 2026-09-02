import { Bot, EyeOff, User } from "lucide-react";
import type { TFunction } from "i18next";
import type { InvocationMode } from "./types";

type InvocationModeBadgeProps = {
  mode: InvocationMode;
  t: TFunction;
};

// The default mode (both the user and the model may invoke a skill) is the
// overwhelming majority, so it renders nothing: the badge only appears when a
// skill's frontmatter restricts who can invoke it.
const InvocationModeBadge = ({ mode, t }: InvocationModeBadgeProps) => {
  if (mode === "user-and-model") return null;

  const icon =
    mode === "user-only" ? (
      <User size={11} />
    ) : mode === "model-only" ? (
      <Bot size={11} />
    ) : (
      <EyeOff size={11} />
    );
  const labelKey =
    mode === "user-only"
      ? "invocationMode.userOnly"
      : mode === "model-only"
        ? "invocationMode.modelOnly"
        : "invocationMode.neither";
  const tooltipKey =
    mode === "user-only"
      ? "invocationMode.userOnlyTooltip"
      : mode === "model-only"
        ? "invocationMode.modelOnlyTooltip"
        : "invocationMode.neitherTooltip";

  return (
    <span
      className={`invocation-badge ${mode}`}
      title={t(tooltipKey)}
      aria-label={t(tooltipKey)}
    >
      <span aria-hidden="true">{icon}</span>
      {t(labelKey)}
    </span>
  );
};

export default InvocationModeBadge;
