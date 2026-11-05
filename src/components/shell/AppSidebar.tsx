import { memo, type ReactNode } from "react";
import { ArrowUpCircle, Bell, Compass, FolderKanban, Keyboard, Layers, Monitor, Moon, Settings, Sun } from "lucide-react";
import type { TFunction } from "i18next";
import { Badge } from "@/components/ui/badge";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { Area } from "../../hooks/useSurfaceState";
import type { AppUpdate } from "../../hooks/useAppUpdate";

type ThemePreference = "system" | "light" | "dark";

type AppSidebarProps = {
  /** The area showing (surface state); `aria-current` marks it. */
  activeArea: Area;
  onViewChange: (area: Area) => void;
  /** Settings is a dialog (D37), opened on General. */
  onOpenSettings: () => void;
  /** Keyboard shortcuts, in the Settings frame (D37). */
  onOpenShortcuts: () => void;
  /** Errors and warnings not yet seen in the notification panel. */
  unreadNotifications: number;
  onOpenNotifications: () => void;
  themePreference: ThemePreference;
  onThemeChange: (theme: ThemePreference) => void;
  /** The update module: a found release is a quiet item here (D8), never a startup modal. */
  update: Pick<AppUpdate, "announced" | "status" | "version" | "open">;
  t: TFunction;
};

/** Past this the badge stops counting; the panel has the real number. */
const BADGE_MAX = 99;

/**
 * One sidebar entry: 32 px, icon + label. Below the shell breakpoint the
 * sidebar is an icon rail: the label stays in the accessibility tree
 * (`sr-only`) and becomes the hover title. The current area carries
 * `aria-current="page"`, a raised ground and an iris icon. Hover is the
 * `sidebar-accent` (panel) tone, the current item `raised` — one value in
 * light, two in dark (ticket 35).
 */
const itemClass = [
  "relative flex h-8 w-full cursor-default items-center gap-2.5 rounded-md px-2 text-left text-sm text-sidebar-foreground",
  "hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
  "aria-[current=page]:bg-raised aria-[current=page]:font-medium aria-[current=page]:text-sidebar-accent-foreground",
  "aria-[current=page]:[&>svg]:text-ring aria-expanded:bg-sidebar-accent",
  "max-shell:justify-center max-shell:px-0",
  "[&>svg]:size-4 [&>svg]:shrink-0",
].join(" ");

const labelClass = "min-w-0 truncate max-shell:sr-only";

function NavItem({
  icon,
  label,
  current,
  opensDialog = false,
  onClick,
  children,
}: {
  icon: ReactNode;
  label: string;
  current: boolean;
  /** The item opens a dialog (Settings, Keyboard shortcuts), not an area. */
  opensDialog?: boolean;
  onClick: () => void;
  children?: ReactNode;
}) {
  return (
    <button
      type="button"
      className={itemClass}
      aria-current={current ? "page" : undefined}
      aria-haspopup={opensDialog ? "dialog" : undefined}
      title={label}
      onClick={onClick}
    >
      {icon}
      <span className={labelClass}>{label}</span>
      {children}
    </button>
  );
}

const THEME_ICON: Record<ThemePreference, ReactNode> = {
  system: <Monitor aria-hidden="true" />,
  light: <Sun aria-hidden="true" />,
  dark: <Moon aria-hidden="true" />,
};

/**
 * The app shell's sidebar (200 px; an icon rail below the shell
 * breakpoint): the three areas, then — at the foot — notifications, theme,
 * keyboard shortcuts and Settings (the last two open dialogs, D37). Every
 * control is named.
 */
const AppSidebar = ({
  activeArea,
  onViewChange,
  onOpenSettings,
  onOpenShortcuts,
  unreadNotifications,
  onOpenNotifications,
  themePreference,
  onThemeChange,
  update,
  t,
}: AppSidebarProps) => {
  const bellLabel =
    unreadNotifications > 0
      ? t("notifications.bellUnread", { count: unreadNotifications })
      : t("notifications.bell");
  const themeLabel = t("shell.theme", { theme: t(`themeOptions.${themePreference}`) });
  const updateLabel =
    update.status === "done"
      ? t("appUpdate.installed")
      : update.status === "installing"
        ? t("appUpdate.installing", { version: update.version })
        : t("appUpdate.available", { version: update.version });
  return (
    <aside className="flex w-50 shrink-0 flex-col border-r border-sidebar-border bg-sidebar max-shell:w-14">
      <div className="flex h-12 shrink-0 items-center gap-2 px-4 max-shell:justify-center max-shell:px-0">
        <img className="size-6 shrink-0 rounded-sm" src="/logo.png" alt="" />
        <span className="truncate text-sm font-semibold text-foreground max-shell:sr-only">{t("appName")}</span>
      </div>

      <nav aria-label={t("shell.nav")} className="px-2">
        <ul className="m-0 flex list-none flex-col gap-0.5 p-0">
          <li>
            <NavItem
              icon={<Layers aria-hidden="true" />}
              label={t("navMySkills")}
              current={activeArea === "skills"}
              onClick={() => onViewChange("skills")}
            />
          </li>
          <li>
            <NavItem
              icon={<Compass aria-hidden="true" />}
              label={t("navExplore")}
              current={activeArea === "explore"}
              onClick={() => onViewChange("explore")}
            />
          </li>
          <li>
            <NavItem
              icon={<FolderKanban aria-hidden="true" />}
              label={t("navProjects")}
              current={activeArea === "projects"}
              onClick={() => onViewChange("projects")}
            />
          </li>
        </ul>
      </nav>

      <div className="mt-auto flex flex-col gap-0.5 border-t border-sidebar-border p-2">
        {/* The passive update affordance (D8): quiet, in the shell, opening the update dialog. */}
        {update.announced ? (
          <button
            type="button"
            className={itemClass}
            title={updateLabel}
            aria-haspopup="dialog"
            data-update-affordance=""
            onClick={update.open}
          >
            <ArrowUpCircle aria-hidden="true" className="text-ring" />
            <span className={labelClass}>{updateLabel}</span>
          </button>
        ) : null}
        <button type="button" className={itemClass} title={bellLabel} onClick={onOpenNotifications}>
          <Bell aria-hidden="true" />
          <span className={labelClass}>{bellLabel}</span>
          {unreadNotifications > 0 ? (
            <Badge
              variant="solid"
              tone="accent"
              aria-hidden="true"
              className="ml-auto max-shell:absolute max-shell:-top-1 max-shell:right-0"
            >
              {unreadNotifications > BADGE_MAX ? `${BADGE_MAX}+` : unreadNotifications}
            </Badge>
          ) : null}
        </button>

        <DropdownMenu>
          <DropdownMenuTrigger render={<button type="button" className={itemClass} title={themeLabel} />}>
            {THEME_ICON[themePreference]}
            <span className={labelClass}>{themeLabel}</span>
          </DropdownMenuTrigger>
          <DropdownMenuContent side="right" align="end" className="min-w-40">
            <DropdownMenuGroup>
              <DropdownMenuLabel>{t("shell.themeMenu")}</DropdownMenuLabel>
              <DropdownMenuRadioGroup
                value={themePreference}
                onValueChange={(value) => onThemeChange(value as ThemePreference)}
              >
                {(["system", "light", "dark"] as const).map((option) => (
                  <DropdownMenuRadioItem key={option} value={option}>
                    {THEME_ICON[option]}
                    {t(`themeOptions.${option}`)}
                  </DropdownMenuRadioItem>
                ))}
              </DropdownMenuRadioGroup>
            </DropdownMenuGroup>
          </DropdownMenuContent>
        </DropdownMenu>

        <NavItem
          icon={<Keyboard aria-hidden="true" />}
          label={t("shell.shortcuts")}
          current={false}
          opensDialog
          onClick={onOpenShortcuts}
        />

        <NavItem
          icon={<Settings aria-hidden="true" />}
          label={t("settings")}
          current={false}
          opensDialog
          onClick={onOpenSettings}
        />
      </div>
    </aside>
  );
};

export default memo(AppSidebar);
