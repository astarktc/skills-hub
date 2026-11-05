import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { TFunction } from "i18next";
import { CommandPalette as CommandPalettePrimitive, CommandPaletteKey } from "@/components/ui/command-palette";
import { mainLandmarkFocus } from "@/components/ui/focus-fallback";
import type { SkillActions } from "../../hooks/useSkillActions";
import type { SurfaceState } from "../../hooks/useSurfaceState";
import { dialogFallback, isFocusLost, resolveFirst } from "../../lib/actionLifecycle";
import {
  openKeyboardShortcuts,
  openSettingsPane,
  paletteGroups,
  rememberRecent,
  type PaletteArea,
  type PaletteEntry,
  type PaletteIntent,
  isApplePlatform,
  isPaletteChord,
} from "../../lib/commandPalette";
import type { ManagedSkill } from "../skills/types";

/** How long a chosen intent waits for the palette's focus return (it takes one frame). */
const FOCUS_RETURN_FRAMES = 5;

/** ⌘ on Apple platforms, Ctrl elsewhere (`isPaletteChord` in `lib/commandPalette.ts` holds the rule). */
const APPLE_PLATFORM = isApplePlatform(typeof navigator === "undefined" ? "" : navigator.platform);

type CommandPaletteProps = {
  surface: Pick<
    SurfaceState,
    "area" | "focused" | "selection" | "stack" | "isOpen" | "openDialog" | "closeDialog" | "focus"
  >;
  /** The library (the skills world's catalog). */
  skills: readonly ManagedSkill[];
  actions: Pick<SkillActions, "run">;
  /** The lifecycle's refusal in words (D19), or null. */
  blockedReason: string | null;
  /** Open an area as the sidebar does (Projects returns to the project the world holds). */
  onOpenArea: (area: PaletteArea) => void;
  onAddSkill: () => void;
  onConfigureHarnesses: () => void;
  t: TFunction;
};

/**
 * The ⌘K palette (round-19 ticket 23): mounted once by the binder. It is a
 * dialog of the surface module (`commandPalette`), opened by ⌘K over an empty
 * stack and closed by ⌘K, Escape or a choice; it never reaches the hash (D5).
 *
 * What it lists is the pure fold `paletteGroups` (`lib/commandPalette.ts`).
 * A choice closes the palette **first** — focus back on the element that
 * opened it (or the D21 chain when that is gone) — and only then runs, so a
 * confirmation a verb opens records that element as its opener and returns
 * focus there, exactly as from the bulk rail. Verbs run through the door
 * (`actions.run`), places through the surface module, Settings through
 * `openSettingsPane` and Keyboard shortcuts through `openKeyboardShortcuts`
 * (dialogs: each takes focus itself).
 */
const CommandPalette = ({
  surface,
  skills,
  actions,
  blockedReason,
  onOpenArea,
  onAddSkill,
  onConfigureHarnesses,
  t,
}: CommandPaletteProps) => {
  const { area, focused, selection, stack, isOpen, openDialog, closeDialog, focus } = surface;
  const open = isOpen("commandPalette");
  const [query, setQuery] = useState("");
  // Session memory: the component lives as long as the app.
  const [recent, setRecent] = useState<readonly string[]>([]);
  const chosen = useRef<PaletteIntent | null>(null);

  // ⌘K toggles the palette: it opens only over an empty stack (a dialog
  // owns the keyboard), and closes when it is the top layer.
  const latest = useRef({ open, layers: stack.length });
  useLayoutEffect(() => {
    latest.current = { open, layers: stack.length };
  });
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (!isPaletteChord(event, APPLE_PLATFORM) || event.repeat) return;
      const { open: isShown, layers } = latest.current;
      if (isShown) {
        event.preventDefault();
        closeDialog("commandPalette");
        return;
      }
      if (layers > 0) return;
      event.preventDefault();
      setQuery("");
      openDialog({ kind: "commandPalette" });
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [closeDialog, openDialog]);

  // The showing surface's selection (D4): My Skills' rows, the matrix's;
  // Explore's candidates are not managed skills and take no door verb.
  const selected = useMemo(() => {
    const ids = area === "skills" ? selection.skills : area === "projects" ? selection.matrix : null;
    return ids && ids.size > 0 ? skills.filter((skill) => ids.has(skill.id)) : [];
  }, [area, selection.matrix, selection.skills, skills]);

  const groups = useMemo(
    () => (open ? paletteGroups({ query, skills, selected, blockedReason, recent, t }) : []),
    [blockedReason, open, query, recent, selected, skills, t],
  );

  const onSelect = useCallback(
    (entry: PaletteEntry) => {
      setRecent((current) => rememberRecent(current, entry.id));
      chosen.current = entry.intent;
      closeDialog("commandPalette");
    },
    [closeDialog],
  );

  const { run } = actions;
  /** After a place changes: focus its heading unless the place took focus itself. */
  const settleOnPlace = useCallback((go: () => void, heading: string | null) => {
    const returned = document.activeElement;
    go();
    window.requestAnimationFrame(() => {
      const active = document.activeElement;
      if (active !== returned && !isFocusLost(active)) return;
      const target = heading ? document.querySelector<HTMLElement>(`[data-surface-heading="${heading}"]`) : null;
      (target ?? mainLandmarkFocus())?.focus();
    });
  }, []);

  const execute = useCallback(
    (intent: PaletteIntent) => {
      switch (intent.kind) {
        case "skill":
          // The binder places focus on the inspector's heading (or the sheet takes it).
          focus({ kind: "skill", id: intent.skillId });
          return;
        case "area":
          settleOnPlace(() => onOpenArea(intent.area), intent.area === "explore" ? null : intent.area);
          return;
        case "settings":
          openSettingsPane(surface, intent.pane);
          return;
        case "shortcuts":
          openKeyboardShortcuts(surface);
          return;
        case "selection":
          void run(intent.verb, intent.skillIds);
          return;
        case "updateAll":
          // As the header's Update all: no count confirmation.
          void run("update", "all", { confirmed: true });
          return;
        case "disableAll":
          // D8's library-wide undeploy: the count-bearing Disable confirmation.
          void run("disable", "all");
          return;
        case "add":
          onAddSkill();
          return;
        case "configureHarnesses":
          onConfigureHarnesses();
          return;
      }
    },
    [focus, onAddSkill, onConfigureHarnesses, onOpenArea, run, settleOnPlace, surface],
  );

  // Base UI reports the close complete before it hands focus back (a frame
  // later): the intent waits until focus has left the palette, so a dialog
  // the intent opens records the returned element as its opener.
  const onOpenChangeComplete = useCallback(
    (isShown: boolean) => {
      if (isShown) return;
      const intent = chosen.current;
      chosen.current = null;
      if (!intent) return;
      let frames = 0;
      const whenReturned = () => {
        const active = document.activeElement;
        const inside = active instanceof Element && active.closest("[data-command-palette]") !== null;
        if (inside && active.isConnected && frames++ < FOCUS_RETURN_FRAMES) {
          window.requestAnimationFrame(whenReturned);
          return;
        }
        execute(intent);
      };
      whenReturned();
    },
    [execute],
  );

  // Opened with focus nowhere, or its opener gone: D21's chain.
  const focusedSkillId = focused?.kind === "skill" ? focused.id : null;
  const fallbackFocus = useCallback(
    (usable: (element: Element) => boolean) =>
      resolveFirst(
        dialogFallback({ focusedSkillId, surface: area === "projects" ? "projects" : "skills" }),
        null,
        usable,
      ),
    [area, focusedSkillId],
  );

  return (
    <CommandPalettePrimitive
      open={open}
      onOpenChange={(next) => {
        if (!next) closeDialog("commandPalette");
      }}
      onOpenChangeComplete={onOpenChangeComplete}
      label={t("palette.title")}
      inputLabel={t("palette.inputLabel")}
      placeholder={t("palette.placeholder")}
      query={query}
      onQueryChange={setQuery}
      groups={groups}
      emptyText={t("palette.empty")}
      onSelect={onSelect}
      fallbackFocus={fallbackFocus}
      footer={
        <>
          <span className="flex items-center gap-1.5">
            <CommandPaletteKey>{"\u2191\u2193"}</CommandPaletteKey>{" "}
            {t("palette.footer.move")}
          </span>{" "}
          <span className="flex items-center gap-1.5">
            <CommandPaletteKey>{t("shell.shortcutEnter")}</CommandPaletteKey>{" "}
            {t("palette.footer.run")}
          </span>{" "}
          <span className="flex items-center gap-1.5">
            <CommandPaletteKey>{t("shell.shortcutEscape")}</CommandPaletteKey>{" "}
            {t("palette.footer.close")}
          </span>
        </>
      }
    />
  );
};

export default memo(CommandPalette);
