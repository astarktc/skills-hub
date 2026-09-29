import { memo, type ReactNode } from "react";
import { Bell, Compass, FolderKanban, Keyboard, Layers, Monitor, Moon, Settings, Sun } from "lucide-react";
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
import { Popover, PopoverContent, PopoverTitle, PopoverTrigger } from "@/components/ui/popover";
import type { Area } from "../../hooks/useSurfaceState";

type ThemePreference = "system" | "light" | "dark";

type AppSidebarProps = {
  /** The area showing (surface state); `aria-current` marks it. */
  activeArea: Area;
  onViewChange: (area: Exclude<Area, "settings">) => void;
  onOpenSettings: () => void;
  /** Errors and warnings not yet seen in the notification panel. */
  unreadNotifications: number;
  onOpenNotifications: () => void;
  themePreference: ThemePreference;
  onThemeChange: (theme: ThemePreference) => void;
  t: TFunction;
};

/** Past this the badge stops counting; the panel has the real number. */
const BADGE_MAX = 99;

/**
 * One sidebar entry: 32 px, icon + label. Below the shell breakpoint the
 * sidebar is an icon rail: the label stays in the accessibility tree
 * (`sr-only`) and becomes the hover title. The current area carries
 * `aria-current="page"`, a raised ground and an iris icon.
 */
const itemClass = [
  "relative flex h-8 w-full cursor-default items-center gap-2.5 rounded-md px-2 text-left text-sm text-sidebar-foreground",
  "hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
  "aria-[current=page]:bg-sidebar-accent aria-[current=page]:font-medium aria-[current=page]:text-sidebar-accent-foreground",
  "aria-[current=page]:[&>svg]:text-ring aria-expanded:bg-sidebar-accent",
  "max-shell:justify-center max-shell:px-0",
  "[&>svg]:size-4 [&>svg]:shrink-0",
].join(" ");

const labelClass = "min-w-0 truncate max-shell:sr-only";

function NavItem({
  icon,
  label,
  current,
  onClick,
  children,
}: {
  icon: ReactNode;
  label: string;
  current: boolean;
  onClick: () => void;
  children?: ReactNode;
}) {
  return (
    <button
      type="button"
      className={itemClass}
      aria-current={current ? "page" : undefined}
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
 * keyboard shortcuts and Settings. Every control is named.
 */
const AppSidebar = ({
  activeArea,
  onViewChange,
  onOpenSettings,
  unreadNotifications,
  onOpenNotifications,
  themePreference,
  onThemeChange,
  t,
}: AppSidebarProps) => {
  const bellLabel =
    unreadNotifications > 0
      ? t("notifications.bellUnread", { count: unreadNotifications })
      : t("notifications.bell");
  const themeLabel = t("shell.theme", { theme: t(`themeOptions.${themePreference}`) });
  const shortcuts: [string, string][] = [
    ["/", t("shell.shortcutSearch")],
    [t("shell.shortcutEnter"), t("shell.shortcutOpen")],
    [t("shell.shortcutEscape"), t("shell.shortcutBack")],
  ];
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

        <Popover>
          <PopoverTrigger
            render={<button type="button" className={itemClass} title={t("shell.shortcuts")} />}
          >
            <Keyboard aria-hidden="true" />
            <span className={labelClass}>{t("shell.shortcuts")}</span>
          </PopoverTrigger>
          <PopoverContent side="right" align="end">
            <PopoverTitle>{t("shell.shortcuts")}</PopoverTitle>
            <dl className="m-0 grid grid-cols-[auto_1fr] items-center gap-x-3 gap-y-1.5">
              {shortcuts.map(([key, action]) => (
                <div key={action} className="contents">
                  <dt>
                    <kbd className="inline-flex h-5 min-w-5 items-center justify-center rounded-sm border border-border-strong px-1 font-mono text-xs text-foreground">
                      {key}
                    </kbd>
                  </dt>
                  <dd className="m-0 text-sm text-muted-foreground">{action}</dd>
                </div>
              ))}
            </dl>
          </PopoverContent>
        </Popover>

        <NavItem
          icon={<Settings aria-hidden="true" />}
          label={t("settings")}
          current={activeArea === "settings"}
          onClick={onOpenSettings}
        />
      </div>
    </aside>
  );
};

export default memo(AppSidebar);
