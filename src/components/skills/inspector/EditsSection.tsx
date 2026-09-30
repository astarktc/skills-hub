import type { TFunction } from "i18next";
import { Button } from "@/components/ui/button";
import type { SkillActions } from "../../../hooks/useSkillActions";
import { INVOCATION_LABEL_KEY } from "../../../lib/skillPresentation";
import type { ManagedSkill } from "../types";
import InspectorSection from "./InspectorSection";

type EditsSectionProps = {
  skill: ManagedSkill;
  actions: Pick<SkillActions, "run">;
  blockedReason: string | null;
  t: TFunction;
};

/**
 * The skill's direct Edits — today the invocation mode: the skill's own
 * setting, or the operator's Edit over it (and what the skill says now).
 * After an Update replayed the Edit over a disagreeing upstream, the base is
 * that upstream (`skill_edits::replay_unlocked` re-bases it), so the conflict
 * line and the Edit dialog's two choices read the same mode the Refresh
 * report named. Edit / Resolve opens the Edit dialog through the door.
 */
export default function EditsSection({ skill, actions, blockedReason, t }: EditsSectionProps) {
  const override = skill.invocation_override;
  const busy = blockedReason !== null;
  const centralMissing = skill.unlocatable === "central_missing";
  const mode = (value: keyof typeof INVOCATION_LABEL_KEY) => t(INVOCATION_LABEL_KEY[value]);

  return (
    <InspectorSection title={t("inspector.sections.edits")}>
      <dl className="m-0 grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1 text-sm">
        <dt className="text-muted-foreground">{t("inspector.edits.invocation")}</dt>
        <dd className="m-0 text-foreground">
          {override
            ? t("inspector.edits.overridden", { mode: mode(override.mode), base: mode(override.base_mode) })
            : t("inspector.edits.following", { mode: mode(skill.invocation_mode) })}
        </dd>
      </dl>
      {override?.conflict ? (
        <p className="m-0 text-xs text-stale">
          {t("inspector.edits.conflict", { base: mode(override.base_mode), mode: mode(override.mode) })}
        </p>
      ) : null}
      {centralMissing ? <p className="m-0 text-xs text-muted-foreground">{t("inspector.edits.unavailable")}</p> : null}
      <div className="flex flex-wrap gap-2">
        <Button
          size="sm"
          tone={override?.conflict ? "stale" : "neutral"}
          disabled={busy || centralMissing}
          focusableWhenDisabled={busy}
          aria-description={blockedReason ?? undefined}
          title={blockedReason ?? undefined}
          aria-haspopup="dialog"
          onClick={() => void actions.run("editInvocation", [skill])}
        >
          {override?.conflict ? t("inspector.edits.resolve") : t("inspector.edits.edit")}
        </Button>
      </div>
    </InspectorSection>
  );
}
